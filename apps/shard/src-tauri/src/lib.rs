//! Shell Tauri de PDF SHARD (PLAN.md §5).
//! M1: visor completo — apertura con contraseña, outline, búsqueda de texto,
//! selección/copia, recientes e impresión (desktop). PDFium no es
//! thread-safe, así que un hilo dedicado es dueño del `Renderer` y atiende
//! trabajos por canal; cada operación se manda como un closure (`Job`) para
//! no repetir boilerplate de enum por cada comando nuevo.

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{SystemTime, UNIX_EPOCH};

use core_model::{
    DocumentInfo, OpenError, OpenErrorKind, OutlineNode, PageSize, RecentEntry, RectPt, SearchHit,
    SearchResults, TextSelection,
};
use core_render::{RenderError, Renderer};
use tauri::http::{header::CONTENT_TYPE, Response, StatusCode};
use tauri::Manager;

/// Un trabajo pendiente para el hilo de render: recibe el `Renderer` (o el
/// error guardado si no se pudo crear) y es responsable de responder por su
/// propio canal de vuelta.
type Job = Box<dyn FnOnce(Result<&mut Renderer, RenderError>) + Send>;

/// Handle clonable hacia el hilo de render (el hilo es dueño del Renderer).
#[derive(Clone)]
struct RenderService {
    tx: mpsc::Sender<Job>,
}

impl RenderService {
    fn start() -> Self {
        let (tx, rx) = mpsc::channel::<Job>();
        std::thread::spawn(move || {
            // El renderer se crea perezosamente en el primer trabajo para que
            // la app arranque aunque falte la biblioteca PDFium (el error se
            // reporta por operación, no como crash de arranque).
            let mut renderer: Option<Result<Renderer, RenderError>> = None;
            for job in rx {
                match renderer.get_or_insert_with(|| Renderer::new(None)) {
                    Ok(r) => job(Ok(r)),
                    Err(e) => job(Err(e.clone())),
                }
            }
        });
        Self { tx }
    }

    /// Ejecuta `f` en el hilo de render y espera su resultado. `f` debe ser
    /// `'static` (no puede tomar prestado nada de la llamada) porque cruza
    /// al hilo por canal; se le pasan valores clonados/movidos.
    fn call<T: Send + 'static>(
        &self,
        f: impl FnOnce(&mut Renderer) -> Result<T, RenderError> + Send + 'static,
    ) -> Result<T, RenderError> {
        let (reply_tx, reply_rx) = mpsc::channel();
        let job: Job = Box::new(move |r| {
            let result = match r {
                Ok(renderer) => f(renderer),
                Err(e) => Err(e),
            };
            let _ = reply_tx.send(result);
        });
        self.tx
            .send(job)
            .map_err(|_| RenderError::Pdfium("hilo de render no disponible".into()))?;
        reply_rx
            .recv()
            .map_err(|_| RenderError::Pdfium("hilo de render no disponible".into()))?
    }
}

struct AppState {
    render: RenderService,
}

fn to_open_error(e: RenderError) -> OpenError {
    let kind = match &e {
        RenderError::PasswordRequired => OpenErrorKind::PasswordRequired,
        RenderError::WrongPassword => OpenErrorKind::WrongPassword,
        _ => OpenErrorKind::Other,
    };
    OpenError {
        kind,
        message: e.to_string(),
    }
}

fn convert_rect(r: core_render::RectPt) -> RectPt {
    RectPt {
        left: r.left,
        top: r.top,
        right: r.right,
        bottom: r.bottom,
    }
}

fn convert_outline(nodes: Vec<core_render::OutlineNode>) -> Vec<OutlineNode> {
    nodes
        .into_iter()
        .map(|n| OutlineNode {
            title: n.title,
            page_index: n.page_index,
            children: convert_outline(n.children),
        })
        .collect()
}

fn convert_search_results(r: core_render::SearchResults) -> SearchResults {
    SearchResults {
        hits: r
            .hits
            .into_iter()
            .map(|h| SearchHit {
                page_index: h.page_index,
                rects: h.rects.into_iter().map(convert_rect).collect(),
            })
            .collect(),
        truncated: r.truncated,
    }
}

#[tauri::command]
fn open_document(
    path: String,
    password: Option<String>,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<DocumentInfo, OpenError> {
    let path_buf = PathBuf::from(&path);
    let (id, page_count) = state
        .render
        .call({
            let path_buf = path_buf.clone();
            move |r| r.open(&path_buf, password.as_deref())
        })
        .map_err(to_open_error)?;
    let page_sizes = state
        .render
        .call(move |r| r.page_sizes_pt(id))
        .map_err(to_open_error)?
        .into_iter()
        .map(|p| PageSize {
            width_pt: p.width_pt,
            height_pt: p.height_pt,
        })
        .collect();

    record_recent(&app, &path);

    Ok(DocumentInfo {
        id,
        path,
        page_count,
        page_sizes,
    })
}

#[tauri::command]
fn close_document(id: u32, state: tauri::State<'_, AppState>) -> Result<(), String> {
    state
        .render
        .call(move |r| {
            r.close(id);
            Ok(())
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_outline(id: u32, state: tauri::State<'_, AppState>) -> Result<Vec<OutlineNode>, String> {
    state
        .render
        .call(move |r| r.outline(id))
        .map(convert_outline)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn search_text(
    id: u32,
    query: String,
    match_case: bool,
    whole_word: bool,
    state: tauri::State<'_, AppState>,
) -> Result<SearchResults, String> {
    state
        .render
        .call(move |r| r.search(id, &query, match_case, whole_word))
        .map(convert_search_results)
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
fn select_text(
    id: u32,
    page: u16,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    state: tauri::State<'_, AppState>,
) -> Result<Option<TextSelection>, String> {
    state
        .render
        .call(move |r| r.text_selection(id, page, x0, y0, x1, y1))
        .map(|opt| {
            opt.map(|s| TextSelection {
                page_index: s.page_index,
                text: s.text,
                rects: s.rects.into_iter().map(convert_rect).collect(),
            })
        })
        .map_err(|e| e.to_string())
}

const MAX_RECENTS: usize = 10;

fn recents_path(app: &tauri::AppHandle) -> Option<PathBuf> {
    app.path()
        .app_data_dir()
        .ok()
        .map(|d| d.join("recientes.json"))
}

fn load_recents(app: &tauri::AppHandle) -> Vec<RecentEntry> {
    let Some(path) = recents_path(app) else {
        return Vec::new();
    };
    let Ok(bytes) = std::fs::read(&path) else {
        return Vec::new();
    };
    serde_json::from_slice(&bytes).unwrap_or_default()
}

fn save_recents(app: &tauri::AppHandle, entries: &[RecentEntry]) {
    let Some(path) = recents_path(app) else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(bytes) = serde_json::to_vec_pretty(entries) {
        let _ = std::fs::write(path, bytes);
    }
}

/// Registra `path` como el más reciente (best-effort: si no se puede
/// persistir, la apertura del documento igual continúa).
fn record_recent(app: &tauri::AppHandle, path: &str) {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let mut entries = load_recents(app);
    entries.retain(|e| e.path != path);
    entries.insert(
        0,
        RecentEntry {
            path: path.to_string(),
            last_opened_unix_ms: now_ms,
        },
    );
    entries.truncate(MAX_RECENTS);
    save_recents(app, &entries);
}

#[tauri::command]
fn get_recent_documents(app: tauri::AppHandle) -> Vec<RecentEntry> {
    load_recents(&app)
}

#[tauri::command]
fn clear_recent_documents(app: tauri::AppHandle) {
    save_recents(&app, &[]);
}

/// Envía `path` al diálogo de impresión nativo del SO (desktop). Android
/// llega en M10 vía Android Print Framework (PLAN.md §6.1).
#[tauri::command]
fn print_document(path: String) -> Result<(), String> {
    print_native(Path::new(&path))
}

#[cfg(target_os = "windows")]
fn print_native(path: &Path) -> Result<(), String> {
    let escaped = path.to_string_lossy().replace('\'', "''");
    let status = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-WindowStyle",
            "Hidden",
            "-Command",
            &format!("Start-Process -FilePath '{escaped}' -Verb Print"),
        ])
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err("no se pudo iniciar la impresión".into())
    }
}

#[cfg(target_os = "macos")]
fn print_native(path: &Path) -> Result<(), String> {
    let escaped = path.to_string_lossy().replace('"', "\\\"");
    let script = format!("tell application \"Finder\" to print (POSIX file \"{escaped}\")");
    let status = std::process::Command::new("osascript")
        .args(["-e", &script])
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err("no se pudo iniciar la impresión".into())
    }
}

#[cfg(target_os = "linux")]
fn print_native(path: &Path) -> Result<(), String> {
    // No hay un diálogo de impresión estándar invocable por CLI en Linux;
    // se envía directo a la impresora predeterminada vía CUPS. Brecha
    // declarada (ver notas de M1): sin diálogo previo, a diferencia de
    // Windows/macOS.
    let status = std::process::Command::new("lp")
        .arg(path)
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err("no se pudo enviar a la impresora predeterminada (¿hay CUPS/lp instalado?)".into())
    }
}

#[cfg(target_os = "android")]
fn print_native(_path: &Path) -> Result<(), String> {
    Err("la impresión en Android llega en M10 (Android Print Framework)".into())
}

#[cfg(not(any(
    target_os = "windows",
    target_os = "macos",
    target_os = "linux",
    target_os = "android"
)))]
fn print_native(_path: &Path) -> Result<(), String> {
    Err("impresión no soportada en esta plataforma".into())
}

/// Parsea `tile/<doc_id>/<page>/<width>` del path del protocolo shard://
fn parse_tile_path(path: &str) -> Option<(u32, u16, i32)> {
    let mut seg = path.trim_start_matches('/').split('/');
    if seg.next()? != "tile" {
        return None;
    }
    let id: u32 = seg.next()?.parse().ok()?;
    let page: u16 = seg.next()?.parse().ok()?;
    let width: i32 = seg.next()?.parse().ok()?;
    if !(50..=8000).contains(&width) {
        return None;
    }
    Some((id, page, width))
}

fn respond(status: StatusCode, mime: &str, body: Vec<u8>) -> Response<Vec<u8>> {
    Response::builder()
        .status(status)
        .header(CONTENT_TYPE, mime)
        .body(body)
        .unwrap()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            render: RenderService::start(),
        })
        .register_uri_scheme_protocol("shard", |ctx, request| {
            let state = ctx.app_handle().state::<AppState>();
            let Some((id, page, width)) = parse_tile_path(request.uri().path()) else {
                return respond(
                    StatusCode::BAD_REQUEST,
                    "text/plain",
                    b"ruta invalida".to_vec(),
                );
            };
            match state
                .render
                .call(move |r| r.render_page_png(id, page, width))
            {
                Ok(png) => respond(StatusCode::OK, "image/png", png),
                Err(RenderError::UnknownDocument(_)) => respond(
                    StatusCode::NOT_FOUND,
                    "text/plain",
                    b"documento no abierto".to_vec(),
                ),
                Err(e) => respond(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "text/plain",
                    e.to_string().into_bytes(),
                ),
            }
        })
        .invoke_handler(tauri::generate_handler![
            open_document,
            close_document,
            get_outline,
            search_text,
            select_text,
            get_recent_documents,
            clear_recent_documents,
            print_document,
        ])
        .run(tauri::generate_context!())
        .expect("error ejecutando PDF SHARD");
}

#[cfg(test)]
mod tests {
    use super::parse_tile_path;

    #[test]
    fn parsea_ruta_de_tile_valida() {
        assert_eq!(parse_tile_path("/tile/3/0/1200"), Some((3, 0, 1200)));
    }

    #[test]
    fn rechaza_rutas_invalidas() {
        assert_eq!(parse_tile_path("/otra/3/0/1200"), None);
        assert_eq!(parse_tile_path("/tile/3/0/49"), None);
        assert_eq!(parse_tile_path("/tile/x/0/1200"), None);
    }
}

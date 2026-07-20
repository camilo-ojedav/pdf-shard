//! Shell Tauri de PDF SHARD (PLAN.md §5).
//! M0: abrir documento + servir tiles PNG por el protocolo `shard://`.
//! PDFium no es thread-safe, así que un hilo dedicado es dueño del `Renderer`
//! y atiende trabajos por canal (semilla del worker pool de M1).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{mpsc, Mutex};

use core_model::DocumentInfo;
use core_render::Renderer;
use tauri::http::{header::CONTENT_TYPE, Response, StatusCode};
use tauri::Manager;

enum Job {
    Open {
        path: PathBuf,
        reply: mpsc::Sender<Result<u16, String>>,
    },
    RenderPng {
        path: PathBuf,
        page: u16,
        width: i32,
        reply: mpsc::Sender<Result<Vec<u8>, String>>,
    },
}

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
            let mut renderer: Option<Result<Renderer, String>> = None;
            for job in rx {
                let r =
                    renderer.get_or_insert_with(|| Renderer::new(None).map_err(|e| e.to_string()));
                match job {
                    Job::Open { path, reply } => {
                        let res = match r {
                            Ok(rr) => rr.page_count(&path).map_err(|e| e.to_string()),
                            Err(e) => Err(e.clone()),
                        };
                        let _ = reply.send(res);
                    }
                    Job::RenderPng {
                        path,
                        page,
                        width,
                        reply,
                    } => {
                        let res = match r {
                            Ok(rr) => rr
                                .render_page_png(&path, page, width)
                                .map_err(|e| e.to_string()),
                            Err(e) => Err(e.clone()),
                        };
                        let _ = reply.send(res);
                    }
                }
            }
        });
        Self { tx }
    }

    fn open(&self, path: PathBuf) -> Result<u16, String> {
        let (reply, rx) = mpsc::channel();
        self.tx
            .send(Job::Open { path, reply })
            .map_err(|e| e.to_string())?;
        rx.recv().map_err(|e| e.to_string())?
    }

    fn render_png(&self, path: PathBuf, page: u16, width: i32) -> Result<Vec<u8>, String> {
        let (reply, rx) = mpsc::channel();
        self.tx
            .send(Job::RenderPng {
                path,
                page,
                width,
                reply,
            })
            .map_err(|e| e.to_string())?;
        rx.recv().map_err(|e| e.to_string())?
    }
}

struct AppState {
    render: RenderService,
    docs: Mutex<HashMap<u32, PathBuf>>,
    next_id: AtomicU32,
}

#[tauri::command]
fn open_document(path: String, state: tauri::State<'_, AppState>) -> Result<DocumentInfo, String> {
    let path_buf = PathBuf::from(&path);
    let page_count = state.render.open(path_buf.clone())?;
    let id = state.next_id.fetch_add(1, Ordering::SeqCst);
    state.docs.lock().unwrap().insert(id, path_buf);
    Ok(DocumentInfo {
        id,
        path,
        page_count,
    })
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
            docs: Mutex::new(HashMap::new()),
            next_id: AtomicU32::new(1),
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
            let Some(path) = state.docs.lock().unwrap().get(&id).cloned() else {
                return respond(
                    StatusCode::NOT_FOUND,
                    "text/plain",
                    b"documento no abierto".to_vec(),
                );
            };
            match state.render.render_png(path, page, width) {
                Ok(png) => respond(StatusCode::OK, "image/png", png),
                Err(e) => respond(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "text/plain",
                    e.into_bytes(),
                ),
            }
        })
        .invoke_handler(tauri::generate_handler![open_document])
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

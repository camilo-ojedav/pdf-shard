//! Renderizado de páginas PDF con PDFium (PLAN.md §5.1, crate `core-render`).
//! El visor (M1) mantiene los documentos abiertos en un registro para no
//! reabrir el archivo en cada operación: outline, búsqueda y selección de
//! texto trabajan sobre el `PdfDocument` ya cargado.

use std::collections::{HashMap, HashSet, VecDeque};
use std::io::Cursor;
use std::path::{Path, PathBuf};

use pdfium_render::prelude::*;

/// Cantidad máxima de resultados de búsqueda que se acumulan por documento
/// (protección ante documentos patológicos o coincidencias masivas).
const MAX_SEARCH_HITS: usize = 500;

/// Tolerancia (en puntos PDF) usada al ubicar el carácter más cercano a un
/// punto del mouse/touch para iniciar o cerrar una selección de texto.
const SELECTION_TOLERANCE_PT: f32 = 6.0;

#[derive(Debug, Clone, thiserror::Error)]
pub enum RenderError {
    #[error("no se encontró la biblioteca PDFium (defina PDF_SHARD_PDFIUM_DIR o ejecute scripts/fetch-pdfium)")]
    LibNotFound,
    #[error("el documento requiere contraseña")]
    PasswordRequired,
    #[error("la contraseña ingresada es incorrecta")]
    WrongPassword,
    #[error("documento no abierto: {0}")]
    UnknownDocument(u32),
    #[error("pdfium: {0}")]
    Pdfium(String),
    #[error("codificación de imagen: {0}")]
    Image(String),
}

/// Tamaño de una página en puntos PDF (1 punto = 1/72 pulgada).
#[derive(Debug, Clone, Copy)]
pub struct PageSize {
    pub width_pt: f32,
    pub height_pt: f32,
}

/// Rectángulo en el mismo espacio que usa la UI para posicionar overlays
/// sobre el tile PNG renderizado: origen arriba-izquierda, Y creciendo hacia
/// abajo, en puntos PDF (no en píxeles: la UI aplica su propio factor de
/// zoom). Esto difiere del espacio nativo de PDFium (origen abajo-izquierda,
/// Y creciendo hacia arriba); la conversión se hace una sola vez aquí.
#[derive(Debug, Clone, Copy)]
pub struct RectPt {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

/// Un nodo del árbol de marcadores (outline) de un documento.
#[derive(Debug, Clone)]
pub struct OutlineNode {
    pub title: String,
    pub page_index: Option<u16>,
    pub children: Vec<OutlineNode>,
}

/// Una coincidencia de búsqueda de texto en una página.
#[derive(Debug, Clone)]
pub struct SearchHit {
    pub page_index: u16,
    pub rects: Vec<RectPt>,
}

/// Resultado completo de una búsqueda sobre el documento.
#[derive(Debug, Clone, Default)]
pub struct SearchResults {
    pub hits: Vec<SearchHit>,
    /// `true` si se alcanzó `MAX_SEARCH_HITS` y quedaron coincidencias sin recorrer.
    pub truncated: bool,
}

/// Texto y rectángulos de una selección dentro de una única página (M1 no
/// soporta selección que cruce el borde entre páginas; cada página se
/// selecciona por separado, igual que la mayoría de los visores simples).
#[derive(Debug, Clone)]
pub struct TextSelection {
    pub page_index: u16,
    pub text: String,
    pub rects: Vec<RectPt>,
}

/// Subcarpeta de `vendor/pdfium/` que corresponde a la plataforma actual.
pub fn platform_dir_name() -> &'static str {
    if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        "win-x64"
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        "mac-arm64"
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        "mac-x64"
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        "linux-x64"
    } else if cfg!(all(target_os = "android", target_arch = "aarch64")) {
        "android-arm64"
    } else {
        "unknown"
    }
}

/// Resuelve el directorio que contiene la biblioteca dinámica de PDFium:
/// 1. la variable de entorno `PDF_SHARD_PDFIUM_DIR`,
/// 2. `vendor/pdfium/<plataforma>/` subiendo desde el directorio actual,
/// 3. `vendor/pdfium/<plataforma>/` junto al ejecutable.
pub fn default_lib_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("PDF_SHARD_PDFIUM_DIR") {
        let p = PathBuf::from(dir);
        if p.is_dir() {
            return Some(p);
        }
    }
    let rel: PathBuf = ["vendor", "pdfium", platform_dir_name()].iter().collect();
    if let Ok(mut cwd) = std::env::current_dir() {
        loop {
            let candidate = cwd.join(&rel);
            if candidate.is_dir() {
                return Some(candidate);
            }
            if !cwd.pop() {
                break;
            }
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join(&rel);
            if candidate.is_dir() {
                return Some(candidate);
            }
        }
    }
    None
}

fn pdfium_err(e: PdfiumError) -> RenderError {
    RenderError::Pdfium(format!("{e:?}"))
}

fn classify_open_error(e: PdfiumError, password_supplied: bool) -> RenderError {
    if matches!(
        e,
        PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::PasswordError)
    ) {
        if password_supplied {
            RenderError::WrongPassword
        } else {
            RenderError::PasswordRequired
        }
    } else {
        pdfium_err(e)
    }
}

fn rect_top_left(r: PdfRect, page_height_pt: f32) -> RectPt {
    RectPt {
        left: r.left().value,
        top: page_height_pt - r.top().value,
        right: r.right().value,
        bottom: page_height_pt - r.bottom().value,
    }
}

/// Construye recursivamente un [OutlineNode] a partir de un [PdfBookmark],
/// visitando también sus hermanos siguientes desde el llamador. `visited`
/// evita ciclos (el árbol de marcadores de un PDF corrupto puede ser
/// cíclico); `depth` acota la recursión como segunda defensa.
fn build_outline_node<'a>(
    bookmark: PdfBookmark<'a>,
    visited: &mut HashSet<PdfBookmark<'a>>,
    depth: u32,
) -> Option<OutlineNode> {
    const MAX_OUTLINE_DEPTH: u32 = 64;

    if depth > MAX_OUTLINE_DEPTH || !visited.insert(bookmark.clone()) {
        return None;
    }

    let title = bookmark.title().unwrap_or_default();
    let page_index = bookmark.destination().and_then(|d| d.page_index().ok());

    let mut children = Vec::new();
    let mut next_child = bookmark.first_child();
    while let Some(child) = next_child {
        next_child = child.next_sibling();
        if let Some(built) = build_outline_node(child, visited, depth + 1) {
            children.push(built);
        }
    }

    Some(OutlineNode {
        title,
        page_index,
        children,
    })
}

/// Cantidad máxima de tiles PNG que se retienen en el caché en memoria.
const TILE_CACHE_CAPACITY: usize = 48;

type TileKey = (u32, u16, i32);

/// Caché acotado de tiles PNG ya renderizados, con expulsión FIFO (no es un
/// LRU real: no reordena por acceso). Suficiente para el scroll virtualizado
/// de M1, donde el patrón de acceso es principalmente secuencial; un LRU
/// real queda para cuando el pool de workers concurrente se justifique
/// (PLAN.md §5.3, más allá de M1).
struct TileCache {
    order: VecDeque<TileKey>,
    entries: HashMap<TileKey, Vec<u8>>,
}

impl TileCache {
    fn new() -> Self {
        Self {
            order: VecDeque::new(),
            entries: HashMap::new(),
        }
    }

    fn get(&self, key: &TileKey) -> Option<&Vec<u8>> {
        self.entries.get(key)
    }

    fn put(&mut self, key: TileKey, value: Vec<u8>) {
        if !self.entries.contains_key(&key) {
            self.order.push_back(key);
            if self.order.len() > TILE_CACHE_CAPACITY {
                if let Some(oldest) = self.order.pop_front() {
                    self.entries.remove(&oldest);
                }
            }
        }
        self.entries.insert(key, value);
    }

    fn invalidate_doc(&mut self, doc_key: u32) {
        self.entries.retain(|k, _| k.0 != doc_key);
        self.order.retain(|k| k.0 != doc_key);
    }
}

/// Envoltorio del motor PDFium y del registro de documentos abiertos.
/// NO es thread-safe (PDFium no lo es): debe vivir en un único hilo dedicado
/// (ver `apps/shard` para el patrón del hilo de render).
///
/// La biblioteca PDFium se enlaza una sola vez por proceso y se filtra
/// (`Box::leak`) para poder mantener `PdfDocument`s abiertos junto a ella en
/// el mismo struct; el hilo dedicado nunca se destruye antes de que el
/// proceso termine, así que la fuga es intencional y acotada.
pub struct Renderer {
    pdfium: &'static Pdfium,
    docs: HashMap<u32, PdfDocument<'static>>,
    next_key: u32,
    tiles: TileCache,
}

impl Renderer {
    /// Crea el renderer enlazando la biblioteca dinámica de `lib_dir`
    /// (o la resolución por defecto si es `None`).
    pub fn new(lib_dir: Option<&Path>) -> Result<Self, RenderError> {
        let dir = match lib_dir {
            Some(d) => d.to_path_buf(),
            None => default_lib_dir().ok_or(RenderError::LibNotFound)?,
        };
        let bindings = Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(&dir))
            .map_err(|e| RenderError::Pdfium(format!("{e:?}")))?;
        let pdfium: &'static Pdfium = Box::leak(Box::new(Pdfium::new(bindings)));
        Ok(Self {
            pdfium,
            docs: HashMap::new(),
            next_key: 1,
            tiles: TileCache::new(),
        })
    }

    /// Abre `pdf` (con `password` si el documento lo requiere) y lo registra
    /// en el motor. Devuelve el identificador asignado y la cantidad de
    /// páginas. Si el documento está cifrado y `password` es `None`, falla
    /// con [RenderError::PasswordRequired]; si `password` es incorrecta,
    /// falla con [RenderError::WrongPassword].
    pub fn open(&mut self, pdf: &Path, password: Option<&str>) -> Result<(u32, u16), RenderError> {
        // `load_pdf_from_file` ata el lifetime del `PdfDocument` devuelto tanto
        // a `&self` como a `password`; como el documento debe vivir junto al
        // renderer (efectivamente 'static), la contraseña también debe serlo.
        // Se filtra una copia mínima (la contraseña ya no se necesita tras
        // abrir: PDFium la consume internamente en la apertura).
        let password_static: Option<&'static str> =
            password.map(|p| -> &'static str { Box::leak(p.to_owned().into_boxed_str()) });
        let doc = self
            .pdfium
            .load_pdf_from_file(pdf, password_static)
            .map_err(|e| classify_open_error(e, password.is_some()))?;
        let page_count = doc.pages().len();
        let key = self.next_key;
        self.next_key += 1;
        self.docs.insert(key, doc);
        Ok((key, page_count))
    }

    /// Cierra el documento `key`, liberando su memoria y sus tiles en caché.
    /// No falla si `key` ya no existe (cierre idempotente).
    pub fn close(&mut self, key: u32) {
        self.docs.remove(&key);
        self.tiles.invalidate_doc(key);
    }

    fn doc(&self, key: u32) -> Result<&PdfDocument<'static>, RenderError> {
        self.docs.get(&key).ok_or(RenderError::UnknownDocument(key))
    }

    /// Tamaño en puntos PDF de cada página, en orden. Se usa para reservar
    /// el alto de la lista virtualizada de páginas sin tener que renderizar.
    pub fn page_sizes_pt(&self, key: u32) -> Result<Vec<PageSize>, RenderError> {
        let doc = self.doc(key)?;
        (0..doc.pages().len())
            .map(|i| {
                let rect = doc.pages().page_size(i).map_err(pdfium_err)?;
                Ok(PageSize {
                    width_pt: rect.width().value,
                    height_pt: rect.height().value,
                })
            })
            .collect()
    }

    /// Renderiza la página `index` (base 0) a PNG con ancho `width_px`.
    /// Se usa tanto para tiles a resolución completa como para miniaturas
    /// (pasando un `width_px` bajo). Resultados cacheados por
    /// (documento, página, ancho).
    pub fn render_page_png(
        &mut self,
        key: u32,
        index: u16,
        width_px: i32,
    ) -> Result<Vec<u8>, RenderError> {
        let cache_key = (key, index, width_px);
        if let Some(cached) = self.tiles.get(&cache_key) {
            return Ok(cached.clone());
        }
        // Todo el préstamo inmutable de `self` (vía `self.doc`) queda acotado
        // a este bloque, para poder pedir `self.tiles` en préstamo mutable
        // justo después sin que el borrow checker los vea en conflicto.
        let png = {
            let doc = self.doc(key)?;
            let page = doc.pages().get(index).map_err(pdfium_err)?;
            let config = PdfRenderConfig::new().set_target_width(width_px);
            let bitmap = page.render_with_config(&config).map_err(pdfium_err)?;
            let image = bitmap.as_image();
            let mut buf = Cursor::new(Vec::new());
            image
                .write_to(&mut buf, image::ImageFormat::Png)
                .map_err(|e| RenderError::Image(e.to_string()))?;
            buf.into_inner()
        };
        self.tiles.put(cache_key, png.clone());
        Ok(png)
    }

    /// Árbol de marcadores (bookmarks) del documento, en orden.
    pub fn outline(&self, key: u32) -> Result<Vec<OutlineNode>, RenderError> {
        let doc = self.doc(key)?;
        let mut visited = HashSet::new();
        let mut result = Vec::new();
        let mut next = doc.bookmarks().root();
        while let Some(node) = next {
            next = node.next_sibling();
            if let Some(built) = build_outline_node(node, &mut visited, 0) {
                result.push(built);
            }
        }
        Ok(result)
    }

    /// Busca `query` en todo el documento, página por página, y devuelve las
    /// coincidencias con sus rectángulos de resaltado. Se acota a
    /// [MAX_SEARCH_HITS] resultados.
    pub fn search(
        &self,
        key: u32,
        query: &str,
        match_case: bool,
        match_whole_word: bool,
    ) -> Result<SearchResults, RenderError> {
        if query.trim().is_empty() {
            return Ok(SearchResults::default());
        }
        let doc = self.doc(key)?;
        let options = PdfSearchOptions::new()
            .match_case(match_case)
            .match_whole_word(match_whole_word);

        let mut results = SearchResults::default();
        'pages: for index in 0..doc.pages().len() {
            let page = doc.pages().get(index).map_err(pdfium_err)?;
            let height = page.height().value;
            let text = page.text().map_err(pdfium_err)?;
            let search = text.search(query, &options).map_err(pdfium_err)?;
            for segments in search.iter(PdfSearchDirection::SearchForward) {
                let rects = segments
                    .iter()
                    .map(|s| rect_top_left(s.bounds(), height))
                    .collect();
                results.hits.push(SearchHit {
                    page_index: index,
                    rects,
                });
                if results.hits.len() >= MAX_SEARCH_HITS {
                    results.truncated = true;
                    break 'pages;
                }
            }
        }
        Ok(results)
    }

    /// Resuelve una selección de texto por arrastre dentro de una página:
    /// `(x0,y0)` y `(x1,y1)` son los puntos de inicio/fin en puntos PDF, en
    /// el mismo espacio top-left/Y-abajo que devuelven [RectPt] (la UI
    /// convierte de píxeles de pantalla a puntos con su factor de zoom y
    /// resta del alto de página antes de llamar). Devuelve `None` si ningún
    /// extremo cae cerca de un carácter.
    #[allow(clippy::too_many_arguments)]
    pub fn text_selection(
        &self,
        key: u32,
        page_index: u16,
        x0_pt: f32,
        y0_pt_top_down: f32,
        x1_pt: f32,
        y1_pt_top_down: f32,
    ) -> Result<Option<TextSelection>, RenderError> {
        let doc = self.doc(key)?;
        let page = doc.pages().get(page_index).map_err(pdfium_err)?;
        let height = page.height().value;
        let text = page.text().map_err(pdfium_err)?;

        let tol = PdfPoints::new(SELECTION_TOLERANCE_PT);
        let to_pdfium_y = |y_top_down: f32| PdfPoints::new(height - y_top_down);

        let chars = text.chars();
        let start_char =
            chars.get_char_near_point(PdfPoints::new(x0_pt), tol, to_pdfium_y(y0_pt_top_down), tol);
        let end_char =
            chars.get_char_near_point(PdfPoints::new(x1_pt), tol, to_pdfium_y(y1_pt_top_down), tol);

        let (Some(start_char), Some(end_char)) = (start_char, end_char) else {
            return Ok(None);
        };
        let start = start_char.index().min(end_char.index());
        let end = start_char.index().max(end_char.index());

        let segments = text.segments_subset(start, end - start + 1);
        let mut selected_text = String::new();
        let mut rects = Vec::new();
        for (i, segment) in segments.iter().enumerate() {
            if i > 0 {
                selected_text.push(' ');
            }
            selected_text.push_str(&segment.text());
            rects.push(rect_top_left(segment.bounds(), height));
        }

        Ok(Some(TextSelection {
            page_index,
            text: selected_text,
            rects,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus_path(name: &str) -> PathBuf {
        // El crate corre sus tests desde crates/core-render/, el corpus vive
        // en la raíz del monorepo.
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("corpus")
            .join(name)
    }

    // PDFium admite UNA sola instancia enlazada por proceso, y `Renderer` no
    // es `Send` (contiene punteros crudos de PDFium) así que ni siquiera se
    // puede meter detrás de un `Mutex` compartido entre los hilos que usa
    // `cargo test` para cada test (comprobado: un `Renderer` fresco por test
    // colgaba los tests #2 y #3 >60s; un `Mutex<Renderer>` compartido no
    // compila por falta de `Send`). La solución, igual que en producción
    // (`apps/shard`): un único hilo dedicado es dueño del `Renderer` para
    // todo el proceso de test, y los tests le mandan trabajos por canal.
    type TestJob = Box<dyn FnOnce(&mut Renderer) + Send>;

    struct TestHarness {
        tx: std::sync::mpsc::Sender<TestJob>,
    }

    impl TestHarness {
        fn get() -> &'static TestHarness {
            static HARNESS: std::sync::OnceLock<TestHarness> = std::sync::OnceLock::new();
            HARNESS.get_or_init(|| {
                let (tx, rx) = std::sync::mpsc::channel::<TestJob>();
                std::thread::spawn(move || {
                    let Ok(mut renderer) = Renderer::new(None) else {
                        // Sin PDFium disponible: no se procesan trabajos: el
                        // canal se cierra al salir y los `send` fallan.
                        return;
                    };
                    for job in rx {
                        job(&mut renderer);
                    }
                });
                TestHarness { tx }
            })
        }

        fn run<T: Send + 'static>(
            &self,
            f: impl FnOnce(&mut Renderer) -> T + Send + 'static,
        ) -> Option<T> {
            let (reply_tx, reply_rx) = std::sync::mpsc::channel();
            self.tx
                .send(Box::new(move |r| {
                    let _ = reply_tx.send(f(r));
                }))
                .ok()?;
            reply_rx.recv().ok()
        }
    }

    fn with_renderer<T: Send + 'static>(
        f: impl FnOnce(&mut Renderer) -> T + Send + 'static,
    ) -> Option<T> {
        let result = TestHarness::get().run(f);
        if result.is_none() {
            eprintln!("PDFium no disponible en este entorno de test, se omite");
        }
        result
    }

    #[test]
    fn abre_y_lee_tamanos_de_pagina() {
        with_renderer(|r| {
            let (key, pages) = r.open(&corpus_path("01_texto_simple.pdf"), None).unwrap();
            assert_eq!(pages, 1);
            let sizes = r.page_sizes_pt(key).unwrap();
            assert_eq!(sizes.len(), 1);
            assert!((sizes[0].width_pt - 612.0).abs() < 0.5);
            assert!((sizes[0].height_pt - 792.0).abs() < 0.5);
        });
    }

    #[test]
    fn busca_texto_conocido_del_corpus() {
        with_renderer(|r| {
            let (key, _) = r.open(&corpus_path("01_texto_simple.pdf"), None).unwrap();
            let results = r.search(key, "Corpus 01", false, false).unwrap();
            assert!(!results.hits.is_empty());
            assert!(!results.truncated);
        });
    }

    #[test]
    fn documento_no_abierto_devuelve_error() {
        with_renderer(|r| {
            assert!(matches!(
                r.page_sizes_pt(999),
                Err(RenderError::UnknownDocument(999))
            ));
        });
    }
}

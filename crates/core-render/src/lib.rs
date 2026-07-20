//! Renderizado de páginas PDF con PDFium (PLAN.md §5.1, crate `core-render`).
//! En M0: apertura de documento y render de página a PNG. El pool de workers
//! con tiling progresivo llega con el visor completo (M1).

use std::io::Cursor;
use std::path::{Path, PathBuf};

use pdfium_render::prelude::*;

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("no se encontró la biblioteca PDFium (defina PDF_SHARD_PDFIUM_DIR o ejecute scripts/fetch-pdfium)")]
    LibNotFound,
    #[error("pdfium: {0}")]
    Pdfium(String),
    #[error("codificación de imagen: {0}")]
    Image(String),
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

/// Envoltorio del motor PDFium. NO es thread-safe (PDFium no lo es):
/// debe vivir en un único hilo; ver `apps/shard` para el patrón de hilo dedicado.
pub struct Renderer {
    pdfium: Pdfium,
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
        Ok(Self {
            pdfium: Pdfium::new(bindings),
        })
    }

    /// Cantidad de páginas del documento.
    pub fn page_count(&self, pdf: &Path) -> Result<u16, RenderError> {
        let doc = self
            .pdfium
            .load_pdf_from_file(pdf, None)
            .map_err(|e| RenderError::Pdfium(format!("{e:?}")))?;
        Ok(doc.pages().len())
    }

    /// Renderiza la página `index` (base 0) a PNG con ancho `width_px`.
    pub fn render_page_png(
        &self,
        pdf: &Path,
        index: u16,
        width_px: i32,
    ) -> Result<Vec<u8>, RenderError> {
        let doc = self
            .pdfium
            .load_pdf_from_file(pdf, None)
            .map_err(|e| RenderError::Pdfium(format!("{e:?}")))?;
        let page = doc
            .pages()
            .get(index)
            .map_err(|e| RenderError::Pdfium(format!("{e:?}")))?;
        let config = PdfRenderConfig::new().set_target_width(width_px);
        let bitmap = page
            .render_with_config(&config)
            .map_err(|e| RenderError::Pdfium(format!("{e:?}")))?;
        let image = bitmap.as_image();
        let mut buf = Cursor::new(Vec::new());
        image
            .write_to(&mut buf, image::ImageFormat::Png)
            .map_err(|e| RenderError::Image(e.to_string()))?;
        Ok(buf.into_inner())
    }
}

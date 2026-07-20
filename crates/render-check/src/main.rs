//! Verificación headless del pipeline de render (M0):
//! prueba que PDFium enlaza y renderiza sin necesidad de abrir la GUI.
//!
//! Uso: `cargo run -p render-check [-- <pdf> <salida.png>]`

use core_render::Renderer;
use std::path::PathBuf;

fn main() {
    let mut args = std::env::args().skip(1);
    let pdf = PathBuf::from(
        args.next()
            .unwrap_or_else(|| "corpus/01_texto_simple.pdf".to_string()),
    );
    let out = PathBuf::from(
        args.next()
            .unwrap_or_else(|| "target/render-check.png".to_string()),
    );

    let renderer = match Renderer::new(None) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("ERROR creando renderer: {e}");
            std::process::exit(1);
        }
    };
    let pages = renderer.page_count(&pdf).unwrap_or_else(|e| {
        eprintln!("ERROR abriendo {}: {e}", pdf.display());
        std::process::exit(1);
    });
    let png = renderer.render_page_png(&pdf, 0, 1200).unwrap_or_else(|e| {
        eprintln!("ERROR renderizando: {e}");
        std::process::exit(1);
    });
    if let Some(dir) = out.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    std::fs::write(&out, &png).expect("escribiendo PNG");
    println!(
        "OK: {} ({} paginas) -> {} ({} bytes)",
        pdf.display(),
        pages,
        out.display(),
        png.len()
    );
}

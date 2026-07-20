//! Generador del corpus público de PDFs sintéticos (PLAN.md §9.1).
//! Produce 5 archivos deterministas en `corpus/` que cubren los casos base:
//! texto simple, multicolumna, formas vectoriales, formulario AcroForm
//! y escaneado simulado (imagen embebida sin capa de texto).

use lopdf::{dictionary, Document, Object, ObjectId, Stream};
use std::path::Path;

const PAGE_W: f32 = 612.0; // Carta, en puntos
const PAGE_H: f32 = 792.0;

fn main() {
    let out_dir = Path::new("corpus");
    std::fs::create_dir_all(out_dir).expect("no se pudo crear corpus/");

    generar_texto_simple(&out_dir.join("01_texto_simple.pdf"));
    generar_multicolumna(&out_dir.join("02_multicolumna.pdf"));
    generar_formas(&out_dir.join("03_formas.pdf"));
    generar_formulario(&out_dir.join("04_formulario.pdf"));
    generar_escaneado_sim(&out_dir.join("05_escaneado_sim.pdf"));

    println!("Corpus generado en {}", out_dir.display());
}

/// Arma un documento de una página con el content stream dado y recursos extra.
/// Devuelve (doc, page_id) para que el caso de uso agregue anotaciones/AcroForm.
fn documento_base(
    contenido: String,
    extra_xobject: Option<(String, Stream)>,
) -> (Document, ObjectId) {
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();

    let font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });
    let font_bold_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica-Bold",
    });

    let mut resources = dictionary! {
        "Font" => dictionary! { "F1" => font_id, "F2" => font_bold_id },
    };
    if let Some((nombre, stream)) = extra_xobject {
        let xobj_id = doc.add_object(stream);
        resources.set("XObject", dictionary! { nombre.as_str() => xobj_id });
    }

    let content_id = doc.add_object(Stream::new(dictionary! {}, contenido.into_bytes()));
    let page_id = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "MediaBox" => vec![0.into(), 0.into(), PAGE_W.into(), PAGE_H.into()],
        "Contents" => content_id,
        "Resources" => resources,
    });

    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
        }),
    );
    let catalog_id = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", catalog_id);
    (doc, page_id)
}

fn guardar(mut doc: Document, path: &Path) {
    doc.compress();
    doc.save(path)
        .unwrap_or_else(|e| panic!("guardando {}: {e}", path.display()));
    println!("  {}", path.display());
}

fn texto(x: f32, y: f32, font: &str, size: u32, s: &str) -> String {
    format!("BT /{font} {size} Tf {x} {y} Td ({s}) Tj ET\n")
}

fn generar_texto_simple(path: &Path) {
    let mut c = String::new();
    c += &texto(72.0, 720.0, "F2", 18, "PDF SHARD - Corpus 01");
    c += &texto(
        72.0,
        690.0,
        "F1",
        12,
        "Documento de texto simple para pruebas de render.",
    );
    let lorem = "Linea de parrafo con texto determinista para el corpus de pruebas.";
    for i in 0..20 {
        c += &texto(
            72.0,
            660.0 - (i as f32) * 18.0,
            "F1",
            11,
            &format!("{lorem} Nro {i}."),
        );
    }
    let (doc, _) = documento_base(c, None);
    guardar(doc, path);
}

fn generar_multicolumna(path: &Path) {
    let mut c = String::new();
    c += &texto(72.0, 720.0, "F2", 16, "Corpus 02 - Dos columnas");
    for col in 0..2u32 {
        let x = 72.0 + (col as f32) * 240.0;
        for i in 0..25 {
            c += &texto(
                x,
                680.0 - (i as f32) * 14.0,
                "F1",
                9,
                &format!("Col {} linea {i} texto de columna angosta.", col + 1),
            );
        }
    }
    let (doc, _) = documento_base(c, None);
    guardar(doc, path);
}

fn generar_formas(path: &Path) {
    let mut c = String::new();
    c += &texto(72.0, 730.0, "F2", 16, "Corpus 03 - Formas vectoriales");
    // Rectángulo relleno, rectángulo trazado, líneas y "tabla" de líneas.
    c += "0.2 0.5 0.8 rg 72 560 150 100 re f\n";
    c += "0.9 0.2 0.2 RG 3 w 260 560 150 100 re S\n";
    c += "0 0 0 RG 1 w 72 520 m 540 520 l S\n";
    for i in 0..5 {
        let y = 480.0 - (i as f32) * 30.0;
        c += &format!("0.5 0.5 0.5 RG 0.8 w 72 {y} m 400 {y} l S\n");
    }
    for i in 0..4 {
        let x = 72.0 + (i as f32) * 109.3;
        c += &format!("0.5 0.5 0.5 RG 0.8 w {x} 480 m {x} 360 l S\n");
    }
    c += &texto(80.0, 455.0, "F1", 10, "Celda A1");
    c += &texto(190.0, 455.0, "F1", 10, "Celda B1");
    let (doc, _) = documento_base(c, None);
    guardar(doc, path);
}

fn generar_formulario(path: &Path) {
    let mut c = String::new();
    c += &texto(72.0, 720.0, "F2", 16, "Corpus 04 - Formulario AcroForm");
    c += &texto(72.0, 660.0, "F1", 12, "Nombre:");
    c += "0.6 0.6 0.6 RG 1 w 140 650 250 22 re S\n";
    let (mut doc, page_id) = documento_base(c, None);

    let helv_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });
    let field_id = doc.add_object(dictionary! {
        "Type" => "Annot",
        "Subtype" => "Widget",
        "FT" => "Tx",
        "T" => Object::string_literal("nombre"),
        "V" => Object::string_literal(""),
        "F" => 4,
        "Rect" => vec![140.into(), 650.into(), 390.into(), 672.into()],
        "P" => page_id,
        "DA" => Object::string_literal("/Helv 11 Tf 0 g"),
    });
    if let Ok(page) = doc.get_dictionary_mut(page_id) {
        page.set("Annots", vec![field_id.into()]);
    }
    let acroform_id = doc.add_object(dictionary! {
        "Fields" => vec![field_id.into()],
        "DA" => Object::string_literal("/Helv 0 Tf 0 g"),
        "DR" => dictionary! { "Font" => dictionary! { "Helv" => helv_id } },
        "NeedAppearances" => true,
    });
    let root_id = doc
        .trailer
        .get(b"Root")
        .and_then(|o| o.as_reference())
        .expect("Root");
    if let Ok(catalog) = doc.get_dictionary_mut(root_id) {
        catalog.set("AcroForm", acroform_id);
    }
    guardar(doc, path);
}

fn generar_escaneado_sim(path: &Path) {
    // Imagen RGB cruda de 300x200 con franjas y "texto" pixelado: simula un escaneo
    // sin capa de texto (caso de entrada del OCR, PLAN.md §6.5).
    let (w, h) = (300usize, 200usize);
    let mut data = Vec::with_capacity(w * h * 3);
    for y in 0..h {
        for x in 0..w {
            let franja = (y / 25) % 2 == 0;
            let mancha = (x / 10 + y / 10) % 7 == 0;
            let (r, g, b) = if mancha {
                (30u8, 30u8, 30u8)
            } else if franja {
                (235u8, 235u8, 225u8)
            } else {
                (245u8, 245u8, 240u8)
            };
            data.extend_from_slice(&[r, g, b]);
        }
    }
    let img_stream = Stream::new(
        dictionary! {
            "Type" => "XObject",
            "Subtype" => "Image",
            "Width" => w as i64,
            "Height" => h as i64,
            "ColorSpace" => "DeviceRGB",
            "BitsPerComponent" => 8,
        },
        data,
    );

    let mut c = String::new();
    c += &texto(
        72.0,
        740.0,
        "F2",
        14,
        "Corpus 05 - Escaneado simulado (sin capa de texto)",
    );
    c += "q 450 0 0 300 81 380 cm /Im1 Do Q\n";
    let (doc, _) = documento_base(c, Some(("Im1".to_string(), img_stream)));
    guardar(doc, path);
}

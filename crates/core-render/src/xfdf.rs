//! Lectura y escritura de XFDF (ISO 19444-1) para el subconjunto de
//! anotaciones de M2 (PLAN.md §6.2). Módulo puro: no toca PDFium, solo
//! traduce entre XML y [XfdfAnnot], así que se prueba sin la biblioteca.
//!
//! Coordenadas: XFDF usa el espacio nativo PDF (origen abajo-izquierda), igual
//! que los diccionarios de anotación, así que aquí no se convierte nada: los
//! valores pasan tal cual entre el archivo y PDFium.

use quick_xml::escape::{escape, resolve_predefined_entity};
use quick_xml::events::{BytesStart, Event};
use quick_xml::{Reader, XmlVersion};

use crate::AnnotationKind;

const XFDF_NS: &str = "http://ns.adobe.com/xfdf/";

/// Una anotación tal como viaja en XFDF.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct XfdfAnnot {
    pub kind: AnnotationKind,
    pub page_index: u16,
    /// `[llx, lly, urx, ury]` en puntos PDF.
    pub rect: [f32; 4],
    /// QuadPoints (8 valores por cuadrilátero) en el orden en que vienen.
    pub coords: Vec<[f32; 8]>,
    pub color: Option<[u8; 3]>,
    /// Opacidad 0..1 (`/CA`); `None` equivale a opaco.
    pub opacity: Option<f32>,
    /// Autor (`/T`).
    pub title: Option<String>,
    /// Identificador (`/NM`).
    pub name: Option<String>,
    /// Fecha de modificación (`/M`), formato de fecha PDF (`D:AAAAMMDD…`).
    pub date: Option<String>,
    /// Fecha de creación (`/CreationDate`), formato de fecha PDF.
    pub creation_date: Option<String>,
    pub contents: Option<String>,
}

pub(crate) fn tag_for(kind: AnnotationKind) -> &'static str {
    match kind {
        AnnotationKind::Highlight => "highlight",
        AnnotationKind::Underline => "underline",
        AnnotationKind::Strikeout => "strikeout",
        AnnotationKind::Squiggly => "squiggly",
        AnnotationKind::Text => "text",
    }
}

fn kind_for(tag: &[u8]) -> Option<AnnotationKind> {
    match tag {
        b"highlight" => Some(AnnotationKind::Highlight),
        b"underline" => Some(AnnotationKind::Underline),
        b"strikeout" => Some(AnnotationKind::Strikeout),
        b"squiggly" => Some(AnnotationKind::Squiggly),
        b"text" => Some(AnnotationKind::Text),
        _ => None,
    }
}

/// Formatea un número sin ceros sobrantes (`12.5`, `100`), estable para diff.
fn num(v: f32) -> String {
    let s = format!("{v:.4}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" {
        "0".into()
    } else {
        s.into()
    }
}

/// Escapa texto para XML preservando los caracteres que un parser
/// normalizaría: CR (Acrobat lo usa como salto de línea en `/Contents`) y,
/// dentro de atributos, también LF y TAB.
fn escape_text(v: &str) -> String {
    escape(v).replace('\r', "&#13;")
}

fn escape_attr(v: &str) -> String {
    escape_text(v).replace('\n', "&#10;").replace('\t', "&#9;")
}

fn join_nums(values: &[f32]) -> String {
    values.iter().map(|v| num(*v)).collect::<Vec<_>>().join(",")
}

/// `[r, g, b]` → `#RRGGBB`.
pub fn color_to_hex(c: [u8; 3]) -> String {
    format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2])
}

/// `#RRGGBB` → `[r, g, b]` (`None` si el formato no es ese).
pub fn hex_to_color(s: &str) -> Option<[u8; 3]> {
    let h = s.trim().strip_prefix('#')?;
    if h.len() != 6 || !h.is_ascii() {
        return None;
    }
    let p = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).ok();
    Some([p(0)?, p(2)?, p(4)?])
}

/// Serializa las anotaciones como documento XFDF. `pdf_name` (solo el nombre
/// del archivo, sin ruta) va en `<f href>` para que Acrobat lo asocie.
pub(crate) fn write(annots: &[XfdfAnnot], pdf_name: Option<&str>) -> String {
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str(&format!(
        "<xfdf xmlns=\"{XFDF_NS}\" xml:space=\"preserve\">\n"
    ));
    out.push_str("  <annots>\n");
    for a in annots {
        let tag = tag_for(a.kind);
        out.push_str(&format!(
            "    <{tag} page=\"{}\" rect=\"{}\"",
            a.page_index,
            join_nums(&a.rect)
        ));
        let mut attr = |k: &str, v: &str| {
            out.push_str(&format!(" {k}=\"{}\"", escape_attr(v)));
        };
        if let Some(c) = a.color {
            attr("color", &color_to_hex(c));
        }
        if let Some(o) = a.opacity {
            attr("opacity", &num(o));
        }
        if let Some(t) = &a.title {
            attr("title", t);
        }
        if let Some(n) = &a.name {
            attr("name", n);
        }
        if let Some(d) = &a.date {
            attr("date", d);
        }
        if let Some(d) = &a.creation_date {
            attr("creationdate", d);
        }
        attr("flags", "print");
        if !a.coords.is_empty() {
            let flat: Vec<f32> = a.coords.iter().flatten().copied().collect();
            attr("coords", &join_nums(&flat));
        }
        match &a.contents {
            Some(c) => {
                out.push_str(">\n");
                out.push_str(&format!("      <contents>{}</contents>\n", escape_text(c)));
                out.push_str(&format!("    </{tag}>\n"));
            }
            None => out.push_str("/>\n"),
        }
    }
    out.push_str("  </annots>\n");
    if let Some(name) = pdf_name {
        out.push_str(&format!("  <f href=\"{}\"/>\n", escape_attr(name)));
    }
    out.push_str("</xfdf>\n");
    out
}

fn parse_nums(s: &str) -> Vec<f32> {
    s.split(|c: char| c == ',' || c == ';' || c.is_whitespace())
        .filter(|t| !t.is_empty())
        .filter_map(|t| t.parse::<f32>().ok())
        .collect()
}

fn attrs_of(e: &BytesStart<'_>) -> Result<Vec<(String, String)>, String> {
    let mut out = Vec::new();
    for attr in e.attributes() {
        let attr = attr.map_err(|e| format!("XFDF inválido: {e}"))?;
        let key = String::from_utf8_lossy(attr.key.local_name().as_ref()).into_owned();
        let value = attr
            .normalized_value(XmlVersion::Implicit1_0)
            .map_err(|e| format!("XFDF inválido: {e}"))?
            .into_owned();
        out.push((key, value));
    }
    Ok(out)
}

/// Construye la anotación a partir de los atributos de su elemento. Devuelve
/// `None` si le faltan datos imprescindibles (página o rectángulo válidos).
fn annot_from_attrs(kind: AnnotationKind, attrs: &[(String, String)]) -> Option<XfdfAnnot> {
    let get = |k: &str| attrs.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str());
    let page_index = get("page")?.trim().parse::<u16>().ok()?;
    let r = parse_nums(get("rect")?);
    if r.len() != 4 {
        return None;
    }
    let rect = [
        r[0].min(r[2]),
        r[1].min(r[3]),
        r[0].max(r[2]),
        r[1].max(r[3]),
    ];
    let coords = get("coords")
        .map(|c| {
            let nums = parse_nums(c);
            nums.as_chunks::<8>().0.to_vec()
        })
        .unwrap_or_default();
    let non_empty = |k: &str| get(k).filter(|v| !v.is_empty()).map(str::to_owned);
    Some(XfdfAnnot {
        kind,
        page_index,
        rect,
        coords,
        color: get("color").and_then(hex_to_color),
        opacity: get("opacity")
            .and_then(|o| o.trim().parse::<f32>().ok())
            .map(|o| o.clamp(0.0, 1.0)),
        title: non_empty("title"),
        name: non_empty("name"),
        date: non_empty("date"),
        creation_date: non_empty("creationdate"),
        contents: None,
    })
}

/// Lee un documento XFDF y devuelve las anotaciones del subconjunto
/// soportado. Las anotaciones de otros tipos (o sin página/rect válidos) se
/// ignoran en silencio: M2 importa lo que sabe representar sin abortar todo.
pub(crate) fn parse(xml: &str) -> Result<Vec<XfdfAnnot>, String> {
    let mut reader = Reader::from_str(xml);
    let mut out = Vec::new();
    // Anotación abierta (entre su Start y su End) y si estamos dentro de su
    // <contents>. Los hijos que no conocemos (popup, contents-richtext…) se
    // saltan: solo se acumula texto mientras `in_contents`.
    let mut current: Option<XfdfAnnot> = None;
    let mut in_contents = false;
    let mut text = String::new();

    loop {
        let event = reader
            .read_event()
            .map_err(|e| format!("XFDF inválido: {e}"))?;
        match event {
            Event::Start(e) => {
                let local = e.local_name();
                if current.is_none() {
                    if let Some(kind) = kind_for(local.as_ref()) {
                        current = annot_from_attrs(kind, &attrs_of(&e)?);
                        if current.is_none() {
                            // Elemento inválido: se salta entero (con sus hijos).
                            reader
                                .read_to_end(e.name())
                                .map_err(|e| format!("XFDF inválido: {e}"))?;
                        }
                    }
                } else if local.as_ref() == b"contents" {
                    in_contents = true;
                    text.clear();
                }
            }
            Event::Empty(e) => {
                if current.is_none() {
                    if let Some(kind) = kind_for(e.local_name().as_ref()) {
                        if let Some(a) = annot_from_attrs(kind, &attrs_of(&e)?) {
                            out.push(a);
                        }
                    }
                }
            }
            Event::Text(t) if in_contents => {
                text.push_str(
                    &t.xml10_content()
                        .map_err(|e| format!("XFDF inválido: {e}"))?,
                );
            }
            Event::CData(t) if in_contents => {
                text.push_str(&t.decode().map_err(|e| format!("XFDF inválido: {e}"))?);
            }
            Event::GeneralRef(r) if in_contents => {
                if let Some(ch) = r
                    .resolve_char_ref()
                    .map_err(|e| format!("XFDF inválido: {e}"))?
                {
                    text.push(ch);
                } else {
                    let name = r.decode().map_err(|e| format!("XFDF inválido: {e}"))?;
                    match resolve_predefined_entity(&name) {
                        Some(v) => text.push_str(v),
                        None => return Err(format!("XFDF inválido: entidad desconocida &{name};")),
                    }
                }
            }
            Event::End(e) => {
                let local = e.local_name();
                if in_contents && local.as_ref() == b"contents" {
                    in_contents = false;
                    if let Some(a) = current.as_mut() {
                        // Acrobat guarda los saltos de línea como CR (&#13;),
                        // que es también lo que PDF usa dentro de /Contents.
                        a.contents = Some(std::mem::take(&mut text));
                    }
                } else if current.as_ref().map(|a| tag_for(a.kind).as_bytes())
                    == Some(local.as_ref())
                {
                    out.push(current.take().expect("comprobado arriba"));
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if current.is_some() {
        return Err("XFDF inválido: anotación sin cerrar".into());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> XfdfAnnot {
        XfdfAnnot {
            kind: AnnotationKind::Highlight,
            page_index: 2,
            rect: [72.0, 700.0, 300.5, 712.25],
            coords: vec![[72.0, 712.25, 300.5, 712.25, 72.0, 700.0, 300.5, 700.0]],
            color: Some([255, 214, 0]),
            opacity: Some(0.5),
            title: Some("Camilo & co <revisor>\tdos".into()),
            name: Some("shard-1".into()),
            date: Some("D:20261101120000Z".into()),
            creation_date: Some("D:20261101110000Z".into()),
            contents: Some("línea 1\r\"citada\" & <más>\nfin".into()),
        }
    }

    #[test]
    fn round_trip_sin_perdida() {
        let note = XfdfAnnot {
            kind: AnnotationKind::Text,
            page_index: 0,
            rect: [10.0, 20.0, 30.0, 40.0],
            coords: vec![],
            color: None,
            opacity: None,
            title: None,
            name: None,
            date: None,
            creation_date: None,
            contents: None,
        };
        let annots = vec![sample(), note];
        let xml = write(&annots, Some("doc.pdf"));
        assert_eq!(parse(&xml).unwrap(), annots);
    }

    #[test]
    fn lee_xfdf_estilo_acrobat() {
        // Recortado de un export real de Acrobat: elemento autocerrado,
        // coords sin espacios, nota con popup hijo, y saltos &#13;.
        let xml = r##"<?xml version="1.0" encoding="UTF-8"?>
<xfdf xmlns="http://ns.adobe.com/xfdf/" xml:space="preserve">
<annots>
<underline color="#0000FF" creationdate="D:20250101090000-03'00'" flags="print" date="D:20250101090000-03'00'" name="a1b2" page="0" coords="10,30,50,30,10,20,50,20" rect="10,20,50,30" subject="Underline" title="Ana"/>
<text color="#FFFF00" flags="print,nozoom,norotate" name="c3" page="1" rect="100,100,120,120" icon="Comment" title="Ana">
<contents>Hola&#13;mundo &amp; más</contents>
<popup flags="print,nozoom,norotate" open="no" page="1" rect="120,60,300,120"/>
</text>
<circle page="0" rect="0,0,10,10"/>
<highlight page="x" rect="1,2,3,4"><contents>sin página válida</contents></highlight>
</annots>
<f href="Doc.pdf"/>
</xfdf>"##;
        let annots = parse(xml).unwrap();
        assert_eq!(annots.len(), 2);
        let u = &annots[0];
        assert_eq!(u.kind, AnnotationKind::Underline);
        assert_eq!(u.color, Some([0, 0, 255]));
        assert_eq!(
            u.coords,
            vec![[10.0, 30.0, 50.0, 30.0, 10.0, 20.0, 50.0, 20.0]]
        );
        assert_eq!(u.title.as_deref(), Some("Ana"));
        let t = &annots[1];
        assert_eq!(t.kind, AnnotationKind::Text);
        assert_eq!(t.page_index, 1);
        assert_eq!(t.contents.as_deref(), Some("Hola\rmundo & más"));
    }

    #[test]
    fn rechaza_xml_mal_formado() {
        assert!(parse("<xfdf><annots><highlight page=\"0\" rect=\"1,2,3,4\">").is_err());
    }

    #[test]
    fn colores_hex() {
        assert_eq!(hex_to_color("#ff8000"), Some([255, 128, 0]));
        assert_eq!(hex_to_color("ff8000"), None);
        assert_eq!(color_to_hex([1, 2, 255]), "#0102FF");
    }
}

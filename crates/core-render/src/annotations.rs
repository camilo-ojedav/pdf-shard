//! Anotaciones PDF nativas (PLAN.md §6.2, milestone M2): listar, crear,
//! editar el texto, borrar e intercambiar por XFDF. Todo se hace con
//! `FPDFAnnot_*` (vía pdfium-render), así que lo que se guarda son
//! anotaciones PDF estándar que Acrobat/Chrome leen sin nada especial.
//!
//! Los cambios quedan en memoria y marcan el documento como modificado; el
//! archivo solo se escribe con [Renderer::save] (nunca se pisa el PDF del
//! usuario como efecto secundario de anotar).

use chrono::{DateTime, FixedOffset, NaiveDate, TimeZone, Utc};
use pdfium_render::prelude::*;

use crate::xfdf::{self, XfdfAnnot};
use crate::{pdfium_err, rect_top_left, RectPt, RenderError, Renderer};

/// Tipos de anotación soportados (subconjunto inicial de M2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnnotationKind {
    Highlight,
    Underline,
    Strikeout,
    Squiggly,
    Text,
}

impl AnnotationKind {
    fn from_pdfium(t: PdfPageAnnotationType) -> Option<Self> {
        match t {
            PdfPageAnnotationType::Highlight => Some(Self::Highlight),
            PdfPageAnnotationType::Underline => Some(Self::Underline),
            PdfPageAnnotationType::Strikeout => Some(Self::Strikeout),
            PdfPageAnnotationType::Squiggly => Some(Self::Squiggly),
            PdfPageAnnotationType::Text => Some(Self::Text),
            _ => None,
        }
    }

    /// Color por defecto de cada tipo. La nota va en amarillo puro porque
    /// PDFium dibuja su ícono siempre en ese color (ignora `/C`): así lo que
    /// se ve, lo que se guarda y lo que se exporta a XFDF coinciden.
    pub fn default_color(self) -> [u8; 3] {
        match self {
            Self::Highlight => [255, 235, 59],
            Self::Underline => [33, 150, 243],
            Self::Strikeout => [229, 57, 53],
            Self::Squiggly => [67, 160, 71],
            Self::Text => [255, 255, 0],
        }
    }
}

/// Una anotación leída del documento, en el espacio top-left de la UI.
#[derive(Debug, Clone)]
pub struct Annotation {
    /// `"<página>:<índice>"`: posición dentro de `/Annots` de la página. Es
    /// estable mientras no se borren anotaciones anteriores de esa página;
    /// tras cualquier mutación la UI vuelve a pedir la lista completa.
    pub id: String,
    pub page_index: u16,
    pub kind: AnnotationKind,
    /// Un rectángulo por cuadrilátero (markup) o el `/Rect` (nota).
    pub rects: Vec<RectPt>,
    pub contents: Option<String>,
    pub author: Option<String>,
    /// Leído del `/AP`; `None` si la anotación no tiene apariencia.
    pub color: Option<[u8; 3]>,
}

/// Datos para crear una anotación desde la UI (espacio top-left).
#[derive(Debug, Clone)]
pub struct NewAnnotation {
    pub page_index: u16,
    pub kind: AnnotationKind,
    /// Markup: un rectángulo por línea de texto. Nota: un único rectángulo
    /// con el ícono (si llegan varios se usa la unión).
    pub rects: Vec<RectPt>,
    pub contents: Option<String>,
    pub author: Option<String>,
    /// `None` = color por defecto del tipo.
    pub color: Option<[u8; 3]>,
}

/// Lo que se escribe en el diccionario de la anotación, ya en espacio PDF.
struct AnnotSpec {
    kind: AnnotationKind,
    page_index: u16,
    rect: PdfRect,
    quads: Vec<PdfQuadPoints>,
    color: [u8; 3],
    opacity: f32,
    contents: Option<String>,
    author: Option<String>,
    creation_date: Option<DateTime<Utc>>,
    modification_date: Option<DateTime<Utc>>,
}

/// Rectángulo top-left (UI) → QuadPoints en el orden que usan Acrobat y
/// PDFium: superior-izquierda, superior-derecha, inferior-izquierda,
/// inferior-derecha. PDFium arma la apariencia generada con los puntos 2 y 3
/// (`CPDF_Annot::RectFromQuadPoints`), así que otro orden (p. ej. el
/// antihorario de la especificación) produce resaltados de ancho cero.
fn rect_to_quad(r: RectPt, page_height: f32) -> PdfQuadPoints {
    let left = r.left.min(r.right);
    let right = r.left.max(r.right);
    let top = page_height - r.top.min(r.bottom);
    let bottom = page_height - r.top.max(r.bottom);
    PdfQuadPoints::new_from_values(left, top, right, top, left, bottom, right, bottom)
}

fn quad_to_rect_top_left(q: &PdfQuadPoints, page_height: f32) -> RectPt {
    let xs = [q.x1.value, q.x2.value, q.x3.value, q.x4.value];
    let ys = [q.y1.value, q.y2.value, q.y3.value, q.y4.value];
    let min = |v: [f32; 4]| v.into_iter().fold(f32::INFINITY, f32::min);
    let max = |v: [f32; 4]| v.into_iter().fold(f32::NEG_INFINITY, f32::max);
    RectPt {
        left: min(xs),
        top: page_height - max(ys),
        right: max(xs),
        bottom: page_height - min(ys),
    }
}

fn union_pdf_rect(quads: &[PdfQuadPoints]) -> Option<PdfRect> {
    let mut it = quads.iter();
    let first = it.next()?;
    let (mut l, mut b, mut r, mut t) = (
        first.left().value,
        first.bottom().value,
        first.right().value,
        first.top().value,
    );
    for q in it {
        l = l.min(q.left().value);
        b = b.min(q.bottom().value);
        r = r.max(q.right().value);
        t = t.max(q.top().value);
    }
    Some(PdfRect::new_from_values(b, l, t, r))
}

/// Color (`/C`) y opacidad (`/CA`) de una anotación, leídos de su flujo de
/// apariencia (`/AP`): el primer trazado relleno (resaltado, nota) o
/// trazado con línea (subrayado, tachado, ondulado).
///
/// No se usa `PdfPageAnnotationCommon::stroke_color()` a propósito: cuando
/// la anotación tiene `/AP`, `FPDFAnnot_GetColor` falla y pdfium-render 0.8
/// cae a `FPDFPageObj_GetStrokeColor` pasándole el handle de la anotación
/// como si fuera un objeto de página → segfault (reproducido en los tests
/// de este crate). Por eso toda anotación que crea SHARD sale con apariencia
/// generada (ver [Renderer::generate_appearances]) y aquí solo se lee `/AP`.
fn read_color(a: &PdfPageAnnotation<'_>, kind: AnnotationKind) -> Option<([u8; 3], f32)> {
    let prefer_fill = matches!(kind, AnnotationKind::Highlight | AnnotationKind::Text);
    for obj in a.objects().iter() {
        if obj.object_type() != PdfPageObjectType::Path {
            continue;
        }
        let (first, second) = if prefer_fill {
            (obj.fill_color(), obj.stroke_color())
        } else {
            (obj.stroke_color(), obj.fill_color())
        };
        if let Some(c) = first.ok().or(second.ok()) {
            return Some(([c.red(), c.green(), c.blue()], c.alpha() as f32 / 255.0));
        }
    }
    None
}

/// Parsea una fecha PDF (`D:AAAAMMDDHHmmSSOHH'mm'`, todo opcional salvo el
/// año) a UTC. PDF y XFDF comparten este formato.
pub(crate) fn parse_pdf_date(s: &str) -> Option<DateTime<Utc>> {
    let s = s.trim();
    let s = s.strip_prefix("D:").unwrap_or(s);
    let digits: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.len() < 4 {
        return None;
    }
    let field = |from: usize, len: usize, default: u32| -> u32 {
        digits
            .get(from..from + len)
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    };
    let year = digits[0..4].parse::<i32>().ok()?;
    let naive = NaiveDate::from_ymd_opt(year, field(4, 2, 1), field(6, 2, 1))?.and_hms_opt(
        field(8, 2, 0),
        field(10, 2, 0),
        field(12, 2, 0),
    )?;
    let rest = &s[digits.len()..];
    let offset_secs = match rest.chars().next() {
        Some(sign @ ('+' | '-')) => {
            let tz: String = rest[1..].chars().filter(|c| c.is_ascii_digit()).collect();
            let hh: i32 = tz.get(0..2).and_then(|v| v.parse().ok()).unwrap_or(0);
            let mm: i32 = tz.get(2..4).and_then(|v| v.parse().ok()).unwrap_or(0);
            let secs = hh * 3600 + mm * 60;
            if sign == '-' {
                -secs
            } else {
                secs
            }
        }
        _ => 0,
    };
    FixedOffset::east_opt(offset_secs)?
        .from_local_datetime(&naive)
        .single()
        .map(|d| d.with_timezone(&Utc))
}

fn parse_annotation_id(id: &str) -> Result<(u16, usize), RenderError> {
    let invalid = || RenderError::Pdfium(format!("id de anotación inválido: {id}"));
    let (p, a) = id.split_once(':').ok_or_else(invalid)?;
    Ok((
        p.parse().map_err(|_| invalid())?,
        a.parse().map_err(|_| invalid())?,
    ))
}

/// Escribe en `annotation` (recién creada, todavía sin `/AP`) las propiedades
/// comunes y, si corresponde, sus QuadPoints.
macro_rules! fill_annotation {
    ($annotation:expr, $spec:expr, markup) => {{
        let annotation = &mut $annotation;
        for q in &$spec.quads {
            annotation
                .attachment_points_mut()
                .create_attachment_point_at_end(*q)
                .map_err(pdfium_err)?;
        }
        fill_annotation!(*annotation, $spec, common);
    }};
    ($annotation:expr, $spec:expr, common) => {{
        let annotation = &mut $annotation;
        let spec = $spec;
        let [r, g, b] = spec.color;
        let alpha = (spec.opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
        // FPDFAnnot_SetColor guarda /C y además /CA a partir del alfa.
        annotation
            .set_stroke_color(PdfColor::new(r, g, b, alpha))
            .map_err(pdfium_err)?;
        annotation.set_bounds(spec.rect).map_err(pdfium_err)?;
        if let Some(c) = &spec.contents {
            annotation.set_contents(c).map_err(pdfium_err)?;
        }
        if let Some(t) = &spec.author {
            annotation.set_creator(t).map_err(pdfium_err)?;
        }
        if let Some(d) = spec.creation_date {
            annotation.set_creation_date(d).map_err(pdfium_err)?;
        }
        annotation
            .set_modification_date(spec.modification_date.unwrap_or_else(Utc::now))
            .map_err(pdfium_err)?;
        annotation.set_is_printed(true).map_err(pdfium_err)?;
    }};
}

impl Renderer {
    /// Anotaciones soportadas de todo el documento, página por página.
    /// Las de otros tipos (links, widgets, etc.) se omiten.
    pub fn list_annotations(&self, key: u32) -> Result<Vec<Annotation>, RenderError> {
        let doc = self.doc(key)?;
        let mut out = Vec::new();
        for page_index in 0..doc.pages().len() {
            let page = doc.pages().get(page_index).map_err(pdfium_err)?;
            let height = page.height().value;
            for (i, a) in page.annotations().iter().enumerate() {
                let Some(kind) = AnnotationKind::from_pdfium(a.annotation_type()) else {
                    continue;
                };
                let mut rects: Vec<RectPt> = if kind == AnnotationKind::Text {
                    Vec::new()
                } else {
                    a.attachment_points()
                        .iter()
                        .map(|q| quad_to_rect_top_left(&q, height))
                        .collect()
                };
                if rects.is_empty() {
                    rects.push(rect_top_left(a.bounds().map_err(pdfium_err)?, height));
                }
                out.push(Annotation {
                    id: format!("{page_index}:{i}"),
                    page_index,
                    kind,
                    rects,
                    contents: a.contents().filter(|c| !c.is_empty()),
                    author: a.creator().filter(|c| !c.is_empty()),
                    color: read_color(&a, kind).map(|(c, _)| c),
                });
            }
        }
        Ok(out)
    }

    /// Crea una anotación a partir de coordenadas de la UI.
    pub fn create_annotation(&mut self, key: u32, new: NewAnnotation) -> Result<(), RenderError> {
        if new.rects.is_empty() {
            return Err(RenderError::Pdfium("anotación sin rectángulos".into()));
        }
        let height = self
            .doc(key)?
            .pages()
            .page_size(new.page_index)
            .map_err(pdfium_err)?
            .height()
            .value;
        let quads: Vec<PdfQuadPoints> =
            new.rects.iter().map(|r| rect_to_quad(*r, height)).collect();
        let rect = union_pdf_rect(&quads).expect("rects no vacío");
        let spec = AnnotSpec {
            kind: new.kind,
            page_index: new.page_index,
            rect,
            quads: if new.kind == AnnotationKind::Text {
                Vec::new()
            } else {
                quads
            },
            color: new.color.unwrap_or_else(|| new.kind.default_color()),
            opacity: 1.0,
            contents: new.contents.filter(|c| !c.is_empty()),
            author: new.author.filter(|a| !a.is_empty()),
            creation_date: None,
            modification_date: None,
        };
        self.write_annotation(key, &spec)
    }

    /// Cambia el texto (`/Contents`) de una anotación existente.
    pub fn set_annotation_contents(
        &mut self,
        key: u32,
        annotation_id: &str,
        contents: &str,
    ) -> Result<(), RenderError> {
        let (page_index, index) = parse_annotation_id(annotation_id)?;
        let session = self.session_mut(key)?;
        let page = session.doc.pages().get(page_index).map_err(pdfium_err)?;
        let mut annotation = page.annotations().get(index).map_err(pdfium_err)?;
        if AnnotationKind::from_pdfium(annotation.annotation_type()).is_none() {
            return Err(RenderError::Pdfium(format!(
                "la anotación {annotation_id} no es de un tipo editable"
            )));
        }
        annotation.set_contents(contents).map_err(pdfium_err)?;
        annotation
            .set_modification_date(Utc::now())
            .map_err(pdfium_err)?;
        session.dirty = true;
        Ok(())
    }

    /// Borra una anotación. Solo acepta tipos soportados, para que un id
    /// desactualizado nunca termine borrando un link o un campo de formulario.
    pub fn delete_annotation(&mut self, key: u32, annotation_id: &str) -> Result<(), RenderError> {
        let (page_index, index) = parse_annotation_id(annotation_id)?;
        let session = self.session_mut(key)?;
        let mut page = session.doc.pages().get(page_index).map_err(pdfium_err)?;
        page.set_content_regeneration_strategy(PdfPageContentRegenerationStrategy::Manual);
        let annotation = page.annotations().get(index).map_err(pdfium_err)?;
        if AnnotationKind::from_pdfium(annotation.annotation_type()).is_none() {
            return Err(RenderError::Pdfium(format!(
                "la anotación {annotation_id} no es de un tipo soportado"
            )));
        }
        page.annotations_mut()
            .delete_annotation(annotation)
            .map_err(pdfium_err)?;
        session.dirty = true;
        self.tiles.invalidate_doc(key);
        Ok(())
    }

    /// Exporta las anotaciones soportadas como XFDF. `pdf_name` va en
    /// `<f href>` (solo el nombre de archivo).
    pub fn export_annotations_xfdf(
        &self,
        key: u32,
        pdf_name: Option<&str>,
    ) -> Result<String, RenderError> {
        let doc = self.doc(key)?;
        let mut annots = Vec::new();
        for page_index in 0..doc.pages().len() {
            let page = doc.pages().get(page_index).map_err(pdfium_err)?;
            for a in page.annotations().iter() {
                let Some(kind) = AnnotationKind::from_pdfium(a.annotation_type()) else {
                    continue;
                };
                let b = a.bounds().map_err(pdfium_err)?;
                let coords = if kind == AnnotationKind::Text {
                    Vec::new()
                } else {
                    a.attachment_points()
                        .iter()
                        .map(|q| {
                            [
                                q.x1.value, q.y1.value, q.x2.value, q.y2.value, q.x3.value,
                                q.y3.value, q.x4.value, q.y4.value,
                            ]
                        })
                        .collect()
                };
                let color = read_color(&a, kind);
                annots.push(XfdfAnnot {
                    kind,
                    page_index,
                    rect: [
                        b.left().value,
                        b.bottom().value,
                        b.right().value,
                        b.top().value,
                    ],
                    coords,
                    color: color.map(|(c, _)| c),
                    opacity: color.map(|(_, o)| o).filter(|o| *o < 1.0),
                    title: a.creator().filter(|v| !v.is_empty()),
                    name: a.name().filter(|v| !v.is_empty()),
                    date: a.modification_date().filter(|v| !v.is_empty()),
                    creation_date: a.creation_date().filter(|v| !v.is_empty()),
                    contents: a.contents().filter(|v| !v.is_empty()),
                });
            }
        }
        Ok(xfdf::write(&annots, pdf_name))
    }

    /// Importa (agrega) las anotaciones de un XFDF. Devuelve cuántas se
    /// agregaron. Las de páginas inexistentes hacen fallar la importación
    /// completa antes de tocar el documento.
    pub fn import_annotations_xfdf(&mut self, key: u32, xml: &str) -> Result<usize, RenderError> {
        let annots = xfdf::parse(xml).map_err(RenderError::Pdfium)?;
        let page_count = self.doc(key)?.pages().len();
        if let Some(a) = annots.iter().find(|a| a.page_index >= page_count) {
            return Err(RenderError::Pdfium(format!(
                "el XFDF referencia la página {} pero el documento tiene {page_count}",
                a.page_index + 1
            )));
        }
        for a in &annots {
            let quads: Vec<PdfQuadPoints> = a
                .coords
                .iter()
                .map(|q| {
                    PdfQuadPoints::new_from_values(q[0], q[1], q[2], q[3], q[4], q[5], q[6], q[7])
                })
                .collect();
            let [l, b, r, t] = a.rect;
            let spec = AnnotSpec {
                kind: a.kind,
                page_index: a.page_index,
                rect: PdfRect::new_from_values(b, l, t, r),
                quads,
                color: a.color.unwrap_or_else(|| a.kind.default_color()),
                opacity: a.opacity.unwrap_or(1.0),
                contents: a.contents.clone(),
                author: a.title.clone(),
                creation_date: a.creation_date.as_deref().and_then(parse_pdf_date),
                modification_date: a.date.as_deref().and_then(parse_pdf_date),
            };
            self.write_annotation(key, &spec)?;
        }
        Ok(annots.len())
    }

    fn write_annotation(&mut self, key: u32, spec: &AnnotSpec) -> Result<(), RenderError> {
        let session = self.session_mut(key)?;
        let mut page = session
            .doc
            .pages()
            .get(spec.page_index)
            .map_err(pdfium_err)?;
        // Agregar una anotación no cambia el contenido de la página: sin esto
        // pdfium-render regeneraría el content stream completo (re-serializar
        // todos los objetos de la página arriesga fidelidad sin necesidad).
        page.set_content_regeneration_strategy(PdfPageContentRegenerationStrategy::Manual);
        let annotations = page.annotations_mut();
        match spec.kind {
            AnnotationKind::Highlight => {
                let mut a = annotations
                    .create_highlight_annotation()
                    .map_err(pdfium_err)?;
                fill_annotation!(a, spec, markup);
            }
            AnnotationKind::Underline => {
                let mut a = annotations
                    .create_underline_annotation()
                    .map_err(pdfium_err)?;
                fill_annotation!(a, spec, markup);
            }
            AnnotationKind::Strikeout => {
                let mut a = annotations
                    .create_strikeout_annotation()
                    .map_err(pdfium_err)?;
                fill_annotation!(a, spec, markup);
            }
            AnnotationKind::Squiggly => {
                let mut a = annotations
                    .create_squiggly_annotation()
                    .map_err(pdfium_err)?;
                fill_annotation!(a, spec, markup);
            }
            AnnotationKind::Text => {
                let mut a = annotations
                    .create_text_annotation(spec.contents.as_deref().unwrap_or(""))
                    .map_err(pdfium_err)?;
                fill_annotation!(a, spec, common);
            }
        }
        generate_appearances(&page)?;
        session.dirty = true;
        self.tiles.invalidate_doc(key);
        Ok(())
    }
}

/// Hace que PDFium genere y guarde en el diccionario el `/AP` de toda
/// anotación de la página que todavía no lo tenga (`CPDF_Annot` lo genera al
/// armar la lista de anotaciones de un render). Así el PDF guardado se ve
/// igual en visores que no generan apariencias por su cuenta (Preview de
/// macOS, varios visores móviles) y [read_color] tiene de dónde leer.
fn generate_appearances(page: &PdfPage<'_>) -> Result<(), RenderError> {
    let config = PdfRenderConfig::new()
        .set_target_width(8)
        .render_annotations(true);
    page.render_with_config(&config).map_err(pdfium_err)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quad_en_orden_acrobat() {
        // Página de 800 pt; rect UI de (10,100) a (50,120).
        let q = rect_to_quad(
            RectPt {
                left: 10.0,
                top: 100.0,
                right: 50.0,
                bottom: 120.0,
            },
            800.0,
        );
        // Sup-izq, sup-der, inf-izq, inf-der (PDF: y hacia arriba).
        assert_eq!(
            [q.x1.value, q.y1.value, q.x2.value, q.y2.value],
            [10.0, 700.0, 50.0, 700.0]
        );
        assert_eq!(
            [q.x3.value, q.y3.value, q.x4.value, q.y4.value],
            [10.0, 680.0, 50.0, 680.0]
        );
        // PDFium arma el rect de apariencia con (x3,y3)-(x2,y2): no degenerado.
        assert!(q.x2.value > q.x3.value && q.y2.value > q.y3.value);
    }

    #[test]
    fn quad_ida_y_vuelta() {
        let r = RectPt {
            left: 10.0,
            top: 100.0,
            right: 50.0,
            bottom: 120.0,
        };
        let back = quad_to_rect_top_left(&rect_to_quad(r, 800.0), 800.0);
        assert_eq!(
            (back.left, back.top, back.right, back.bottom),
            (10.0, 100.0, 50.0, 120.0)
        );
    }

    #[test]
    fn fechas_pdf() {
        let d = parse_pdf_date("D:20250101090000-03'00'").unwrap();
        assert_eq!(d.to_rfc3339(), "2025-01-01T12:00:00+00:00");
        let d = parse_pdf_date("D:2026").unwrap();
        assert_eq!(d.to_rfc3339(), "2026-01-01T00:00:00+00:00");
        assert!(parse_pdf_date("ayer").is_none());
    }

    #[test]
    fn ids_de_anotacion() {
        assert_eq!(parse_annotation_id("3:12").unwrap(), (3, 12));
        assert!(parse_annotation_id("3").is_err());
        assert!(parse_annotation_id("a:1").is_err());
    }
}

//! Modelo de documento de PDF SHARD.
//! En M1 contiene la información de sesión de un documento abierto (con sus
//! tamaños de página, para la lista virtualizada), el árbol de marcadores,
//! resultados de búsqueda, selección de texto y la lista de recientes.
//! El modelo transaccional con undo/redo llega en milestones posteriores
//! (PLAN.md §5.3).

use serde::{Deserialize, Serialize};

/// Tamaño de una página en puntos PDF (1 punto = 1/72 pulgada).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PageSize {
    pub width_pt: f32,
    pub height_pt: f32,
}

/// Información de un documento abierto, expuesta a la UI vía IPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentInfo {
    /// Identificador de sesión asignado por el registro de documentos.
    pub id: u32,
    /// Ruta absoluta del archivo en disco.
    pub path: String,
    /// Cantidad de páginas.
    pub page_count: u16,
    /// Tamaño de cada página, en el mismo orden. La UI lo usa para reservar
    /// el alto de la lista virtualizada sin esperar a renderizar cada tile.
    pub page_sizes: Vec<PageSize>,
}

/// Motivo por el que `open_document` no pudo abrir el archivo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpenErrorKind {
    /// El documento está cifrado y no se envió contraseña: la UI debe pedirla.
    PasswordRequired,
    /// Se envió una contraseña pero es incorrecta.
    WrongPassword,
    /// Cualquier otro error (archivo corrupto, no encontrado, etc.).
    Other,
}

/// Error estructurado de apertura, para que la UI distinga "pedir
/// contraseña" de un error genérico sin parsear texto libre.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenError {
    pub kind: OpenErrorKind,
    pub message: String,
}

/// Rectángulo en espacio top-left/Y-hacia-abajo, en puntos PDF (ver
/// `core_render::RectPt`, del que estos valores se copian 1:1 al cruzar el
/// límite de IPC).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RectPt {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

/// Un nodo del árbol de marcadores (outline) de un documento.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlineNode {
    pub title: String,
    pub page_index: Option<u16>,
    pub children: Vec<OutlineNode>,
}

/// Una coincidencia de búsqueda de texto en una página.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub page_index: u16,
    pub rects: Vec<RectPt>,
}

/// Resultado completo de una búsqueda sobre el documento.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchResults {
    pub hits: Vec<SearchHit>,
    pub truncated: bool,
}

/// Texto y rectángulos de una selección dentro de una única página.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextSelection {
    pub page_index: u16,
    pub text: String,
    pub rects: Vec<RectPt>,
}

/// Una entrada de la lista de documentos recientes, persistida entre
/// sesiones (PLAN.md §6.1 / milestone M1: "recientes").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentEntry {
    pub path: String,
    /// Milisegundos desde epoch Unix; se usa solo para ordenar (más
    /// reciente primero), la UI la formatea si hace falta.
    pub last_opened_unix_ms: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_info_roundtrip_json() {
        let info = DocumentInfo {
            id: 1,
            path: "C:/tmp/a.pdf".into(),
            page_count: 3,
            page_sizes: vec![PageSize {
                width_pt: 612.0,
                height_pt: 792.0,
            }],
        };
        let json = serde_json::to_string(&info).unwrap();
        let back: DocumentInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, 1);
        assert_eq!(back.page_count, 3);
        assert_eq!(back.page_sizes.len(), 1);
    }

    #[test]
    fn open_error_kind_se_serializa_en_snake_case() {
        let err = OpenError {
            kind: OpenErrorKind::PasswordRequired,
            message: "se requiere contraseña".into(),
        };
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("\"password_required\""));
    }

    #[test]
    fn outline_node_anidado_roundtrip() {
        let node = OutlineNode {
            title: "Capítulo 1".into(),
            page_index: Some(0),
            children: vec![OutlineNode {
                title: "1.1".into(),
                page_index: Some(1),
                children: vec![],
            }],
        };
        let json = serde_json::to_string(&node).unwrap();
        let back: OutlineNode = serde_json::from_str(&json).unwrap();
        assert_eq!(back.children.len(), 1);
        assert_eq!(back.children[0].page_index, Some(1));
    }
}

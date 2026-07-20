//! Modelo de documento de PDF SHARD.
//! En M0 solo contiene la información básica de un documento abierto;
//! el modelo transaccional con undo/redo llega en milestones posteriores (PLAN.md §5.3).

use serde::{Deserialize, Serialize};

/// Información de un documento abierto, expuesta a la UI vía IPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentInfo {
    /// Identificador de sesión asignado por el registro de documentos.
    pub id: u32,
    /// Ruta absoluta del archivo en disco.
    pub path: String,
    /// Cantidad de páginas.
    pub page_count: u16,
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
        };
        let json = serde_json::to_string(&info).unwrap();
        let back: DocumentInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, 1);
        assert_eq!(back.page_count, 3);
    }
}

# Componentes de terceros

PDF SHARD (GPL-3.0) se apoya en los siguientes componentes open source. Reglas completas en [`docs/PLAN.md` §3](docs/PLAN.md): nada AGPL se linkea al binario; los programas GPL/AGPL externos solo se invocan como procesos separados (mera agregación).

| Componente | Licencia | Modo de uso | Estado |
|---|---|---|---|
| PDFium | BSD-3-Clause / Apache-2.0 | Linkeado (motor de render/texto/objetos/formularios) | Planificado (M0) |
| pdfium-render | MIT / Apache-2.0 | Linkeado (bindings Rust) | Planificado (M0) |
| lopdf | MIT | Linkeado (estructura y escritura PDF) | Planificado (M3) |
| qpdf | Apache-2.0 | Linkeado o CLI (cifrado, linearización, chequeo) | Planificado (M3) |
| Tesseract | Apache-2.0 | Linkeado vía FFI (OCR) | Planificado (M5) |
| Leptonica | BSD-2-Clause | Linkeado vía FFI (preprocesado imagen) | Planificado (M5) |
| allsorts / ttf-parser | Apache-2.0 / MIT | Linkeado (fuentes: métricas y subsetting) | Planificado (M4) |
| docx-rs / rust_xlsxwriter | MIT | Linkeado (motor propio PDF→Office) | Planificado (M6) |
| RustCrypto (cms, rsa, p256, sha2) + p12 | MIT / Apache-2.0 | Linkeado (firmas PAdES) | Planificado (M8) |
| React, Zustand, Vite | MIT | Linkeado (UI) | Planificado (M0) |
| Tauri 2 | MIT / Apache-2.0 | Linkeado (shell multiplataforma) | Planificado (M0) |
| Fuentes Liberation y Noto | SIL OFL 1.1 | Empaquetadas (sustitución métrica) | Planificado (M4) |
| pdf2docx (+ PyMuPDF) | AGPL-3.0 (Artifex) | **Sidecar: proceso separado**, solo desktop, interino hasta motor propio | Planificado (M6) |
| LibreOffice | MPL-2.0 | **Programa externo empaquetado** (Office→PDF headless), instalador Windows | Planificado (M6) |
| tessdata_fast (modelos OCR) | Apache-2.0 | Descarga bajo demanda (es+en empaquetados) | Planificado (M5) |

Este archivo se actualiza en cada milestone que agregue, cambie de versión o retire un componente.

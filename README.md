# PDF SHARD

Editor PDF **open source, multiplataforma y 100% offline** — Windows · macOS · Linux · Android.

Alternativa libre a los editores comerciales (PDFelement, Acrobat): edición real de texto e imágenes sobre el PDF, anotaciones, organización de páginas, **conversión de formatos (PDF ↔ Word/Excel/PPT/imágenes)**, OCR, formularios interactivos, protección, redacción y firmas digitales — sin capa de IA, sin nube obligatoria y sin suscripciones.

> 📋 **Estado:** fase de definición y planificación (pre-desarrollo). El plan maestro vivirá en `docs/PLAN.md`.

## Stack

| Capa | Tecnología |
|---|---|
| Núcleo / backend | Rust (Tauri 2) |
| Motor de renderizado PDF | PDFium (vía `pdfium-render`) |
| Interfaz | Frontend web embebido (WebView nativa) |
| Plataformas | Windows, macOS, Linux y Android desde el día 1 |

## Principios

1. **Clean-room:** solo componentes open source legítimos; ningún código propietario.
2. **Local primero:** todo funciona sin conexión; ningún documento sale del equipo.
3. **Sin IA:** OCR y análisis de layout son deterministas (Tesseract + heurísticas geométricas).
4. **Multiplataforma real:** Android con paridad funcional adaptada a pantalla táctil, no una versión recortada.

## Licencia

Por definir (en breve).

# PDF SHARD — Plan Maestro

> Editor PDF open source, multiplataforma y 100% offline con paridad funcional con Wondershare PDFelement 12, **sin funciones de IA**.
> Documento vivo: se actualiza al cierre (E3) de cada milestone.
> Última actualización: 2026-07-20.

---

## 1. Contexto y objetivo

PDFelement 12 ofrece la experiencia de editar un PDF como si fuera un documento de Word: texto con reflow, imágenes y objetos arrastrables, OCR, formularios, conversión de formatos, protección y firmas. Esa experiencia hoy cuesta una licencia anual.

**PDF SHARD** replica esa capacidad funcional (no la marca, no el código, no los assets) usando exclusivamente componentes open source, para Windows, macOS, Linux y **Android con paridad casi total adaptada a táctil**. Se excluye deliberadamente toda la capa de IA de PDFelement (chat, resumidor, traductor, censura inteligente automática, generación de imágenes): todo lo que hace PDF SHARD es determinista y local.

**Base de investigación**: los informes en `C:\Users\camilo\PDF_Elemental\Referencia\` (fuera de este repo) — arquitectura de 4 fases del "lienzo editable", evaluación de motores de render y catálogo de bibliotecas. Conclusión aplicada aquí: motor PDFium + shell Tauri 2 + heurísticas geométricas deterministas.

**Marco legal (clean-room)**: el código fuente filtrado de Wondershare (incidente RepairIt, 2025) **no se consulta ni se usa jamás**. Clonar funcionalidad es legal; copiar código, iconos o marca no. Ninguna dependencia AGPL se linkea al binario (los programas AGPL/GPL externos solo se invocan como procesos separados).

---

## 2. Registro de decisiones

Todas las decisiones fueron tomadas por Camilo el 2026-07-19. Cambiarlas requiere pasar por E1 de un milestone.

| # | Decisión | Elección |
|---|---|---|
| D01 | Stack base | Tauri 2 + Rust + PDFium (`pdfium-render`) |
| D02 | Plataformas v1 | Windows + macOS + Linux + Android, CI en las 4 desde el día 1 |
| D03 | Android | Paridad casi total con desktop, UI adaptada a táctil (no subset recortado) |
| D04 | Cloud/eSign SaaS | Excluido; se reserva el trait `esign-gateway` para un futuro servicio |
| D05 | Nombre | **PDF SHARD** |
| D06 | Visibilidad | GitHub **público** open source |
| D07 | Idioma UI | Solo español al inicio (strings centralizados para i18n futura) |
| D08 | Fidelidad UI | Inspirada en PDFelement con mejoras propias (no réplica visual) |
| D09 | Frontend | React + TypeScript (Vite, Zustand) |
| D10 | OCR | Tesseract nativo vía Rust, único camino en las 4 plataformas |
| D11 | PDF→Office | Híbrido evolutivo: sidecar `pdf2docx` en desktop YA + motor Rust propio en paralelo que al madurar lo reemplaza y habilita Android |
| D12 | Office→PDF | **LibreOffice portable empaquetado** en el instalador desktop (headless) |
| D13 | Licencia | **GPL-3.0** |
| D14 | iOS | No por ahora; se evalúa cuando Android esté maduro |
| D15 | Firma de código | Android con keystore propio; desktop sin firmar al inicio |
| D16 | Canal Android | APK directo en GitHub Releases |
| D17 | Flujo de trabajo | E1 (plan + aprobación) → E2 (ejecución) → E3 (retro + axiomas) por **cada milestone** |
| D18 | Escáner físico | En el milestone de OCR: WIA/TWAIN en desktop, cámara con deskew en Android |
| D19 | Cadencia | Sin fechas; orden y criterios de cierre por milestone |
| D20 | Nombre repo | `pdf-shard` (carpeta local `C:\Users\camilo\PDF_SHARD`) |
| D21 | Impresión Android (E1-M1) | Diferida a M10 (Android Print Framework); M1 imprime solo desktop, invocando el diálogo/impresora nativa por SO (PowerShell `Start-Process -Verb Print` en Windows, `osascript`/Finder en macOS, `lp`/CUPS sin diálogo previo en Linux — brecha declarada) |
| D22 | Modos de página (E1-M1) | Solo scroll continuo en M1; página única y doble página quedan para M9 (son variantes de layout de la misma lista virtual, sin retrabajo) |

**Defaults técnicos adoptados** (revisables en el E1 del milestone que los toque):

- Escritura/estructura PDF: `lopdf` (MIT) + APIs de objetos de PDFium; cifrado y reparación con **qpdf** (Apache-2.0, linkeable).
- Fuentes de sustitución empaquetadas: Liberation (métricas compatibles con Arial/Times/Courier) + subconjunto Noto.
- Tema claro/oscuro desde el diseño inicial (design tokens CSS).
- TTS con voces del sistema operativo (SAPI en Windows, AVSpeech en macOS, speech-dispatcher en Linux, Android TTS).
- Corpus de pruebas: `corpus/` público (PDFs propios/sintéticos) + `corpus-privado/` local ignorado por git.
- Identificador de app: `com.pdfshard.app`. Versionado: `0.N.x` donde N = último milestone cerrado; `1.0.0` al cerrar M10.
- Código e identificadores en inglés; UI y documentación en español; commits en español.
- LibreOffice portable: empaquetado en el instalador de **Windows**; en macOS/Linux se detecta instalación existente y se ofrece descarga guiada (empaquetarlo ahí es inusual y pesado — se revisa en E1 de M6).

---

## 3. Política de licencias y dependencias

PDF SHARD es GPL-3.0. Reglas:

1. **Linkeable al binario**: cualquier licencia permisiva o compatible con GPL-3.0 (BSD, MIT, Apache-2.0, MPL-2.0, LGPL).
2. **Prohibido linkear**: AGPL (MuPDF, Ghostscript, PyMuPDF, iText). Pueden existir solo como **procesos externos** invocados (mera agregación).
3. Todo componente distribuido se registra en `THIRD_PARTY.md` con versión y licencia.
4. Jamás se incorpora código de origen dudoso (filtraciones, decompilados, snippets sin licencia).

| Componente | Rol | Licencia | Modo |
|---|---|---|---|
| PDFium (+ `pdfium-render`) | Render, texto, objetos, formularios | BSD/Apache-2.0 / MIT | Linkeado |
| lopdf | Estructura PDF, escritura incremental | MIT | Linkeado |
| qpdf | Cifrado AES, linearización, reparación | Apache-2.0 | Linkeado (bindings) o CLI |
| Tesseract + Leptonica | OCR + preprocesado de imagen | Apache-2.0 / BSD | Linkeado (FFI, NDK en Android) |
| allsorts / ttf-parser | Subsetting y métricas de fuentes | Apache-2.0 / MIT | Linkeado |
| docx-rs, rust_xlsxwriter | Motor propio PDF→Office | MIT | Linkeado |
| image, resvg | Recompresión, SVG | MIT/Apache | Linkeado |
| RustCrypto (cms, rsa, p256, sha2), p12 | Firmas digitales PAdES | MIT/Apache | Linkeado |
| Liberation + Noto (fuentes) | Sustitución métrica | OFL | Empaquetadas |
| pdf2docx (+ PyMuPDF) | PDF→Word/Excel interino desktop | AGPL (Artifex) | **Sidecar proceso separado** |
| LibreOffice | Office→PDF desktop | MPL-2.0 | **Programa externo empaquetado** |
| React, Zustand, Vite, Tauri 2 | UI y shell | MIT/Apache | Linkeado |

---

## 4. Alcance funcional (paridad PDFelement 12, sin IA)

Leyenda: ✔ = paridad total · ◐ = adaptado/parcial (se explica) · ✘ = fuera de alcance.

| Módulo | Funciones | Desktop | Android |
|---|---|---|---|
| Visor | Pestañas, zoom, modos de página, miniaturas, marcadores/outline, búsqueda, selección/copia, recientes, propiedades, imprimir | ✔ | ✔ (imprimir vía Android Print Framework) |
| Anotaciones | Resaltar/subrayar/tachar/ondulado, notas, texto libre, formas, lápiz/borrador, sellos, stickers, **biblioteca de anotaciones**, respuestas a comentarios, panel lateral, XFDF, aplanar, medición (distancia/perímetro/área) | ✔ | ✔ |
| Edición | **Texto con reflow de párrafo**, propiedades tipográficas, imágenes (mover/rotar/recortar/reemplazar/extraer), objetos vectoriales, enlaces, marca de agua, fondo, encabezado/pie, números Bates | ✔ | ✔ (misma lógica core; UI táctil) |
| Páginas | Insertar/eliminar/rotar/reordenar/extraer/dividir/combinar/recortar, compresión con control DPI | ✔ | ✔ |
| Crear | Desde archivos/imágenes, combinar múltiples, plantillas, escanear (escáner físico / cámara) | ✔ | ✔ (cámara como escáner) |
| Conversión | PDF→Word/Excel/PPT/HTML/TXT/imagen/EPUB; Word/Excel/PPT/imagen→PDF; lotes | ✔ | ◐ PDF→Office llega cuando el motor Rust propio madure (D11); Office→PDF sin LibreOffice queda imagen/HTML/TXT→PDF (brecha declarada — PDFelement Android lo resuelve vía nube, nosotros no usamos nube) |
| OCR | Tesseract, 26+ idiomas descargables, PDF sándwich, lotes | ✔ | ✔ (modelos descargados a almacenamiento de la app) |
| Formularios | Rellenar AcroForms, crear/editar campos, reconocimiento heurístico de campos, FDF/XFDF/CSV, extracción masiva, aplanar | ✔ | ✔ (creación de campos con UI táctil simplificada) |
| Protección | Contraseñas AES-128/256, permisos, **redacción verdadera** manual y por patrones (regex: RUT, correos, teléfonos, tarjetas) | ✔ | ✔ |
| Firmas | Manuscrita, digital PAdES (certificados .pfx/.p12), sello de tiempo TSA, verificación, firma por lotes | ✔ | ✔ (.pfx + Android KeyStore) |
| Comparar | Lado a lado sincronizado + diff visual + diff de texto | ✔ | ◐ (vista adaptada a pantalla chica) |
| Lotes | Centro de procesos: convertir, OCR, marca de agua, cifrar, firmar, comprimir, recortar | ✔ | ✔ |
| Accesorios | Lectura en voz alta (TTS del SO), tema oscuro, atajos de teclado | ✔ | ✔ (sin atajos) |

**Exclusiones permanentes**: Chat con PDF, resumidor, traductor IA, corrector gramatical, detector de IA, reescribir/explicar PDF, censura "inteligente" automática, generación/edición de imágenes con IA, PDFelement Cloud, eSign SaaS (solo queda `esign-gateway` como interfaz), formularios XFA (formato legado abandonado).

---

## 5. Arquitectura técnica

### 5.1 Vista general

```
┌────────────────────────── UI (React + TS, español) ──────────────────────────┐
│  Ribbon adaptativo · Canvas por capas · Paneles (miniaturas/outline/         │
│  comentarios/campos) · Centro de lotes · Diálogos                            │
│  Capas del canvas (de abajo hacia arriba):                                   │
│    1. bitmap PDFium (tiles)   2. capa de selección de texto                  │
│    3. capa de edición (cajas de párrafo/objetos)   4. capa de anotaciones    │
└───────────────▲──────────────────────────────────────────────▲───────────────┘
                │ IPC Tauri (comandos async + eventos)         │ custom protocol
                │                                              │ (tiles PNG/raw, sin base64)
┌───────────────┴──────────────── Núcleo Rust ─────────────────┴───────────────┐
│ core-model   documento abierto, transacciones, undo/redo, sesión             │
│ core-render  pool de workers PDFium, tiling progresivo, caché LRU            │
│ core-layout  glifos→palabras→líneas→párrafos→columnas/tablas (heurística)    │
│ core-edit    content streams, fuentes/subsetting, reflow, guardado           │
│ core-forms   AcroForms (relleno FPDF_FFL, creación, reconocimiento)          │
│ core-secure  qpdf AES/permisos, redacción verdadera, PAdES + TSA             │
│ core-ocr     Leptonica (deskew/binarizado) + Tesseract → capa invisible      │
│ core-convert motor propio (docx-rs/xlsx/html/epub) + orquestador externos    │
│ core-batch   cola de trabajos, progreso, cancelación                         │
│ esign-gateway  SOLO traits (hueco futuro, D04)                               │
└───────┬──────────────────────────────────────────────────────┬───────────────┘
        │ solo desktop                                         │
   [sidecar pdf2docx]                              [LibreOffice headless
    (proceso Python empaquetado)                    empaquetado, D12]
```

### 5.2 Monorepo

```
pdf-shard/
├─ apps/
│  └─ shard/               # proyecto Tauri 2 (desktop + target Android)
│     ├─ src-tauri/        # shell Rust, comandos IPC, empaquetado
│     └─ (usa ui/ como frontend)
├─ ui/                     # React + TS + Vite + Zustand
│  ├─ src/components/      # Ribbon, Canvas, paneles, diálogos
│  ├─ src/state/           # stores Zustand por dominio
│  ├─ src/strings/         # TODOS los textos de UI (es) — pre-i18n (D07)
│  └─ src/theme/           # design tokens claro/oscuro
├─ crates/                 # los 9 crates del núcleo (5.1)
├─ sidecars/               # build scripts de pdf2docx (PyInstaller) — desktop
├─ vendor/                 # binarios PDFium por plataforma, fuentes OFL, tessdata es+en
├─ corpus/                 # PDFs de prueba públicos + generadores
├─ scripts/                # regresión visual, empaquetado LibreOffice, releases
├─ docs/                   # PLAN.md (este), ARCHITECTURE.md, decisiones E3
└─ .github/workflows/      # ci.yml (lint+test), build.yml (matrix 4 SO), release.yml
```

### 5.3 Contratos clave

- **Modelo de documento** (`core-model`): un `DocumentSession` por pestaña; toda mutación es un `Command` con inverso (patrón command) → undo/redo ilimitado por sesión; los comandos se aplican a un overlay mutable y se materializan al guardar (guardado incremental cuando el PDF lo permita, reescritura completa si no).
- **Render** (`core-render`): la UI pide tiles `(página, rect, zoom)`; el pool renderiza con prioridad viewport-primero; caché LRU con presupuesto de memoria configurable; invalidación selectiva por página al editar.
- **IPC**: comandos Tauri con payloads JSON serde; los bitmaps NUNCA viajan por JSON — se sirven por custom protocol `shard://tile/...` (zero-copy hacia la WebView).
- **Android**: mismo núcleo Rust compilado con NDK (`cargo-ndk`); WebView del sistema; archivos vía Storage Access Framework (content://); sin sidecars (D11/D12: las funciones que dependen de ellos declaran su brecha en la matriz §4).

---

## 6. Cómo se hará cada módulo

### 6.1 Visor (M1)

Apertura con `FPDF_LoadDocument` (soporta contraseña); páginas virtualizadas (solo se montan las visibles ±2); zoom por niveles y ajuste a ancho/página con re-render de tiles al nivel real (nunca escalado borroso permanente); miniaturas renderizadas en baja resolución por el mismo pool con prioridad baja; outline vía `FPDFBookmark_*`; búsqueda con `FPDFText_FindStart/FindNext` + resaltado de resultados y navegación; selección de texto con los rects de `FPDFText_CountRects` y copia al portapapeles; impresión: desktop genera un PDF temporal y lo envía al diálogo nativo del SO; Android usa `PrintManager`.

### 6.2 Anotaciones (M2)

Todas las anotaciones se crean/leen/editan con `FPDFAnnot_*` (subtipos: Highlight, Underline, StrikeOut, Squiggly, Text/nota, FreeText, Square, Circle, Line, Polygon, PolyLine, Ink, Stamp, FileAttachment) y quedan guardadas como anotaciones PDF estándar (interoperables con Acrobat). Respuestas y hilos vía `/IRT`. La **biblioteca de anotaciones** (D — feature v12) es una colección local (JSON + assets en el perfil del usuario) de sellos, stickers, estilos de caja y firmas de trazo reutilizables, con drag & drop al canvas. Medición: herramientas línea/perímetro/área con factor de escala configurable por documento (se persiste en un diccionario propio dentro del PDF, estándar `/Measure` cuando aplique). Import/export **XFDF** para intercambio de revisiones. "Aplanar" convierte anotaciones en contenido de página (render a page objects) vía `core-edit`.

### 6.3 Páginas y documento (M3)

Organizador en grilla con drag & drop (reordenar), y operaciones `FPDFPage_New/Delete`, `FPDF_ImportPages` (combinar/insertar desde otro PDF), rotación por página, extracción a nuevo archivo, división por rangos/tamaño/marcadores, recorte editando `CropBox` (v12: recorte por lotes). Compresión: recompresión de imágenes (downsample a DPI objetivo + JPEG/JPX quality) con el crate `image`, deduplicación de objetos y `qpdf --linearize`. Marca de agua (texto/imagen, mosaico/diagonal), fondo, encabezado/pie y **números Bates** se implementan como generadores de contenido que `core-edit` estampa en cada página (con vista previa en vivo). Adjuntos vía `/EmbeddedFiles`. Editor de metadatos (Info + XMP).

### 6.4 Motor de edición de texto (M4 — el corazón del proyecto)

Pipeline de 6 pasos, correspondiente a la arquitectura de 4 fases de la investigación:

1. **Extracción**: `FPDFText_LoadPage` entrega por carácter: unicode, caja (`GetCharBox`), tamaño, fuente (`GetFontInfo`), color, ángulo. Se normaliza a espacio de página.
2. **Clustering** (`core-layout`): caracteres→palabras (brecha horizontal < umbral relativo al tamaño de fuente), palabras→líneas (misma línea base ± tolerancia), líneas→**bloques de párrafo** (interlineado uniforme, alineación consistente, estilo dominante compartido); detección de columnas por valles en el histograma X; los umbrales se calibran contra el corpus y son configurables.
3. **Overlay de edición**: al entrar en modo edición, la UI dibuja cajas sobre los bloques detectados. Al hacer clic, el bloque se vuelve un editor HTML `contenteditable` posicionado exactamente encima, usando la fuente embebida real (extraída con `FPDFFont_GetFontData` y montada como `@font-face`) para que el texto se vea idéntico. El texto original de esa zona se oculta del render (se excluyen sus page objects del tile).
4. **Reflow**: cada pulsación re-mide las líneas con las métricas reales de la fuente (`ttf-parser` en Rust para exactitud, `canvas.measureText` como aproximación en vivo) y recalcula saltos de línea dentro del ancho del bloque, empujando las líneas siguientes; soporta alineación izquierda/centro/derecha/justificado e interlineado.
5. **Fuentes**: si el usuario escribe glifos que el subset embebido no contiene → se re-subsetea desde la fuente original del sistema si está instalada (allsorts) o se sustituye por la fuente métrica compatible empaquetada (Liberation/Noto) avisando en UI.
6. **Serialización**: al confirmar, se eliminan los page objects de texto originales del bloque y se generan los nuevos (`FPDFPageObj_NewTextObj` o reescritura del content stream con `lopdf`), línea por línea con posiciones calculadas; guardado incremental cuando sea posible. Toda edición es un `Command` reversible.

Subfases: **4a** edición dentro del bloque sin reflow entre bloques → **4b** reflow completo + propiedades tipográficas → **4c** imágenes/objetos (mover/rotar/recortar/reemplazar/extraer, via `FPDFImageObj_*`/`FPDFPageObj_Transform`) y enlaces (`/Annots` tipo Link).

Criterio de aceptación duro: sobre el corpus, editar texto y que el resultado pase `qpdf --check` y abra sin advertencias en Acrobat Reader, Chrome y Edge, con el texto restante intacto.

### 6.5 OCR y escáner (M5)

Camino único multiplataforma (D10): raster de página a ~300 DPI → **Leptonica**: detección/corrección de inclinación (deskew), binarización Otsu, limpieza de ruido → **Tesseract** (LSTM) vía FFI con salida TSV/hOCR (palabra + caja + confianza) → `core-ocr` construye la **capa de texto invisible** (text rendering mode 3) posicionada palabra por palabra sobre la imagen original = "PDF sándwich" seleccionable y buscable. Idiomas: `spa`+`eng` empaquetados; el resto (26+, incluidos RTL) se descargan bajo demanda desde tessdata_fast al perfil del usuario. OCR por lotes vía `core-batch`. **Escáner** (D18): Windows WIA (y TWAIN si el driver lo exige), macOS ImageCaptureKit, Linux SANE — cada uno detrás de un trait común; Android: captura con cámara + auto-recorte del documento y deskew con Leptonica. Salida del escáner entra al mismo pipeline OCR.

### 6.6 Conversión (M6)

**PDF→Office (D11, evolutivo)**: etapa 1: sidecar `pdf2docx` (binario PyInstaller, solo desktop) para Word/Excel con calidad conocida desde el primer día. Etapa 2 (arranca en M6 y continúa): motor propio en Rust — los bloques/tablas de `core-layout` se mapean a `docx-rs` (Word), la detección de grillas de líneas a `rust_xlsxwriter` (Excel), páginas como diapositivas (imagen de fondo + cajas de texto) a un escritor PPTX propio (zip+XML). Cuando el motor propio iguale al sidecar sobre el corpus, lo reemplaza y **habilita conversión en Android**. PDF→HTML: generador propio con posicionamiento absoluto + fuentes extraídas. PDF→TXT: extracción en orden de lectura (usando el orden de `core-layout`, no el orden del stream). PDF→imagen: raster PDFium con DPI/formato elegibles. PDF→EPUB: reflujo de bloques de texto a XHTML.

**Office→PDF (D12)**: desktop invoca el **LibreOffice empaquetado** (`soffice --headless --convert-to pdf`) con perfil aislado; el instalador de Windows lo incluye (~250 MB extra); macOS/Linux detectan instalación existente u ofrecen descarga guiada (default técnico, revisable en E1-M6). Imagen/HTML/TXT→PDF: generador nativo propio (también en Android).

Todo pasa por el centro de lotes con cola, progreso por archivo y cancelación.

### 6.7 Formularios (M7)

Relleno interactivo con el entorno form-fill de PDFium (`FPDFDOC_InitFormFillEnvironment` + eventos `FORM_On*` conectados al mouse/teclado/táctil de la UI, render instantáneo del widget). Diseñador de campos: crear/editar/mover/redimensionar campos (texto, multilinea, check, radio, combo, lista, botón, fecha, firma) escribiendo los diccionarios AcroForm/widget con `lopdf`; propiedades (nombre, requerido, formato, validación simple, orden de tabulación, apariencia). **Reconocimiento heurístico de campos** en PDFs planos (sin IA): detección de rectángulos/líneas de subrayado/etiquetas cercanas con la geometría de `core-layout` → propuesta de campos que el usuario confirma. Datos: import/export FDF, XFDF y CSV; **extracción masiva** (N PDFs → una tabla CSV/XLSX); aplanado de formularios.

### 6.8 Protección y redacción (M8)

Cifrado y permisos con qpdf: AES-128/256, contraseña de apertura y de permisos (imprimir/copiar/editar). **Redacción verdadera**: la zona marcada se intersecta con los caracteres (`FPDFText`) para partir/eliminar los text objects afectados, las imágenes se recortan o re-codifican sin la región, los paths se recortan, y se purgan metadatos/XMP de restos; verificación automática re-extrayendo texto de la zona (debe estar vacía). Redacción por **patrones deterministas**: regex predefinidos (RUT chileno, correos, teléfonos, tarjetas) y regex del usuario — sin IA (la "censura inteligente" de v12 queda excluida por D-alcance).

### 6.9 Firmas digitales (M8)

Manuscrita: trazo/imagen como Stamp o como contenido aplanado. Digital **PAdES**: reserva de `/ByteRange` + `/Contents`, hash SHA-256 del documento, firma CMS/PKCS#7 con RustCrypto (`cms`, `rsa`/`p256`), certificados desde archivo .pfx/.p12 (crate `p12`) y, en fase posterior, almacenes del SO (CNG/Keychain/Android KeyStore); sello de tiempo RFC 3161 contra TSA configurable (perfil B-B y B-T; LTV/B-LTA en backlog post-v1). Verificación: cadena de confianza (raíces del SO), integridad del ByteRange y panel de estado de firmas. Firma por **lotes** (v12). Se evaluará el crate `underskrift` en E1-M8; si no está maduro, se implementa con RustCrypto directo.

### 6.10 Comparación, TTS y lotes (M9)

Comparar: dos documentos en vista sincronizada (scroll/zoom espejo) + modo superposición con diff de píxeles (tiles de ambos, resaltando deltas) + diff de texto (Myers sobre el texto extraído, cajas de inserción/eliminación coloreadas). TTS: lectura del texto en orden de `core-layout` con las voces del SO, controles reproducir/pausa/velocidad y resaltado de seguimiento. Centro de lotes: UI unificada sobre `core-batch` para toda operación por lotes (convertir, OCR, marca de agua, cifrar, firmar, comprimir, recortar).

---

## 7. Milestones

Sin fechas (D19). Cada uno se ejecuta con el flujo **E1 → aprobación de Camilo → E2 → E3** (D17). El E1 de cada milestone detalla sus tareas finas; aquí va el alcance y el criterio de cierre.

| M | Alcance | Criterio de cierre (además de CI verde en las 4 plataformas) |
|---|---|---|
| **M0 Fundaciones** | Monorepo, Tauri 2 + React + Vite + Zustand, pdfium-render con binarios de las 4 plataformas, render de la primera página, CI matrix, LICENSE/THIRD_PARTY/CONTRIBUTING, corpus inicial, design tokens | Un PDF del corpus se abre y se ve en Windows, macOS, Linux y un APK Android instalable |
| **M1 Visor** ✅ | §6.1 completo + pestañas, recientes, atajos, tema oscuro, UI táctil base Android | Uso diario como visor es viable; búsqueda y copia correctas sobre el corpus — **cerrado**: apertura con contraseña, scroll continuo virtualizado (±2 páginas), zoom fijo/ajustar-ancho/ajustar-página, outline y miniaturas, búsqueda de texto con resaltado y navegación, selección de texto por arrastre con copia (Ctrl+C y botón), pestañas multi-documento, recientes persistidos, atajos de teclado, tema oscuro con toggle manual, impresión desktop (D21), tamaños de touch target para Android. CI verde (fmt+clippy+test Rust, lint+test+build UI). Diferido: impresión Android (D21→M10), modos página única/doble (D22→M9), pool de workers PDFium concurrente con LRU real (§5.3 — M1 usa un hilo dedicado con caché FIFO acotado, suficiente para el criterio de cierre). QA manual pendiente: verificación visual/táctil real en Android y de los caminos de impresión en macOS/Linux (no pude probarlos desde este entorno Windows) |
| **M2 Anotaciones** | §6.2 completo | Anotaciones creadas se ven bien en Acrobat/Chrome; XFDF round-trip sin pérdida |
| **M3 Páginas y documento** | §6.3 completo | Salidas pasan `qpdf --check`; compresión reduce ≥40% en corpus de escaneados sin artefactos visibles |
| **M4 Edición** | §6.4 en subfases 4a→4b→4c | Criterio duro de §6.4 sobre el corpus completo |
| **M5 OCR + escáner** | §6.5 completo | Texto seleccionable correcto ≥95% en corpus de escaneados limpios a 300 DPI; cámara Android produce PDF sándwich |
| **M6 Conversión** | §6.6: sidecars + rasterizados + HTML/TXT/EPUB propios + LibreOffice empaquetado (Windows) + arranque del motor Rust propio | DOCX resultante abre sin reparación en Word/LibreOffice y conserva párrafos y tablas del corpus |
| **M7 Formularios** | §6.7 completo | Formularios de referencia (IRS W-9 y equivalentes chilenos) se rellenan, guardan y reabren correctos en Acrobat |
| **M8 Protección y firmas** | §6.8 + §6.9 | Firma verificable en Adobe Reader (con TSA válida); texto redactado irrecuperable (verificación automática + extracción externa) |
| **M9 Paridad desktop** | §6.10 + plantillas, updater (tauri-plugin-updater), pulido de rendimiento, instaladores release | Checklist de paridad §4 completo en desktop; release v0.9 pública con instaladores |
| **M10 Android paridad** | Adaptación táctil de edición/formularios/firmas, cámara-escáner pulida, APK firmado en Releases, motor de conversión propio en Android si maduró | Checklist §4 columna Android completo (con sus ◐ declarados); release v1.0 |

**Backlog post-v1**: iOS (D14), Google Play/F-Droid, certificados de firma desktop (D15), LTV/B-LTA, export PDF/A, accesibilidad/etiquetado, i18n inglés, `esign-gateway` real.

---

## 8. CI/CD y releases

- **`ci.yml`** (cada push/PR): `cargo fmt --check`, `clippy -D warnings`, `cargo test`, `eslint`, `vitest`, build de UI. Repo público → minutos ilimitados en runners estándar.
- **`build.yml`** (por milestone y manual): matriz `windows-latest` (NSIS/.msi), `macos-latest` (.dmg universal), `ubuntu-latest` (AppImage + .deb), `ubuntu-latest + NDK` (`cargo-ndk` → .apk firmado con el keystore, D15/D16). Cachés de cargo y node.
- **`release.yml`** (tag `v*`): compila la matriz, adjunta instaladores y APK a GitHub Releases, genera changelog desde los commits.
- El empaquetado de LibreOffice portable (Windows) y del sidecar pdf2docx se hace con `scripts/` que descargan y verifican hashes en build time (no se versionan binarios en git).

## 9. Estrategia de pruebas

1. **Corpus** (`corpus/`): PDFs sintéticos generados por script (texto multi-columna, tablas, CJK, transparencias, formularios, escaneados simulados) + PDFs propios liberados. `corpus-privado/` local (facturas/documentos reales de Camilo) NUNCA se commitea (.gitignore).
2. **Regresión visual**: snapshots dorados por página del corpus (hash de raster a DPI fijo); cualquier cambio de píxeles no intencional rompe CI.
3. **Validación de salida**: todo PDF escrito por PDF SHARD pasa `qpdf --check` en tests; smoke manual en Acrobat Reader/Chrome/Edge por milestone.
4. **Unit**: `cargo test` por crate (clustering de layout con casos borde; reflow; redacción) + `vitest` en UI.
5. **E2E**: Playwright contra la build desktop por milestone (flujos completos: abrir→editar→guardar→reabrir).
6. **QA manual**: guía de prueba por milestone; opcionalmente QuAsimido (agente QA) ejecuta el guion con evidencia.

## 10. Riesgos

| Riesgo | Impacto | Mitigación |
|---|---|---|
| Reflow de texto (M4) más difícil de lo previsto — es EL foso técnico del rubro | Alto | Subfases 4a/4b/4c; corpus desde M0; estudiar el *enfoque* de PDF4QT/ONLYOFFICE (sin copiar código incompatible); criterio de cierre duro |
| Fuentes: subsets sin glifos nuevos | Alto | §6.4 paso 5: re-subset con allsorts o sustitución métrica avisada |
| Android: PDFium/Tesseract vía NDK con fricción de build | Medio | M0 ya compila las 4 plataformas; nada se acepta "solo desktop" sin decisión explícita |
| Rust es nuevo para Camilo | Medio | Capa Rust delgada y estable; iteración diaria en React/TS; flujo E1/E2/E3 con explicación en cada milestone |
| LibreOffice empaquetado engorda el instalador Windows (~250 MB) | Bajo | Decisión consciente (D12); opción "instalador lite" sin LibreOffice como variante de release |
| Sidecar AGPL (pdf2docx) mal integrado comprometería la licencia | Medio | Regla §3: solo proceso separado; el motor propio lo reemplaza (D11) |
| Alcance total = años de trabajo | Alto | Cada milestone entrega una app útil por sí sola; sin fechas (D19); backlog explícito |

## 11. Convenciones

- Código, identificadores y commits de estructura en inglés técnico donde sea estándar; mensajes de commit y docs en español; formato de commit: `modulo: resumen` + `Co-Authored-By: Claude`.
- Ramas: `main` protegida por CI; trabajo en `feature/<tema>`; merge por PR (aunque el autor sea uno solo, el CI es el gate).
- Strings de UI SOLO en `ui/src/strings/` (D07 → i18n futura sin retrabajo).
- Versionado: `0.N.x` (N = milestone cerrado) → `1.0.0` en M10; tags `vX.Y.Z` disparan release.
- Toda decisión nueva o cambiada se registra en §2 con número D##.

## 12. Proceso de trabajo por milestone (D17)

1. **E1**: Claude presenta el plan fino del milestone (tareas, diseño de detalle, dudas nuevas como preguntas cerradas) → **Camilo aprueba**.
2. **E2**: ejecución sin pausas salvo imprevisto genuino; commits atómicos; CI verde.
3. **E3**: retro del milestone — qué se aprendió, axiomas para los siguientes, actualización de este PLAN.md (§2, §7, riesgos) y del checklist de paridad.

---

## 13. Glosario mínimo

- **AcroForm**: formato estándar de formularios interactivos PDF (el que soportamos). **XFA**: formato XML legado de Adobe (excluido).
- **Content stream**: secuencia de operadores de dibujo de una página (`BT/ET`, `Tj`, etc.).
- **PDF sándwich**: imagen escaneada visible + capa de texto OCR invisible debajo (seleccionable/buscable).
- **PAdES**: perfil europeo de firmas digitales en PDF (B-B básica, B-T con sello de tiempo, B-LTA archivado largo plazo).
- **Redacción verdadera**: eliminación física del contenido del archivo, no un rectángulo negro encima.
- **XFDF**: XML estándar para intercambiar anotaciones/datos de formularios entre editores.

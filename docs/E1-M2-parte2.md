# E1 — M2 parte 2: anotaciones completas

> Estado: **propuesta, pendiente de aprobación de Camilo** (D17, §12 del plan maestro).
> Fecha: 2026-11-04. Base: PR #1 (M2 parte 1: marcado, notas, guardado y XFDF).
> Las decisiones aprobadas aquí se registran después en `PLAN.md` §2 como D26 en adelante.

## 1. Punto de partida

La parte 1 (PR #1) ya entrega:

- resaltado, subrayado, tachado y ondulado sobre la selección de texto;
- notas;
- panel lateral;
- edición y eliminación;
- guardado y guardado como, con escritura atómica y aviso de cambios sin guardar;
- importación y exportación XFDF de esos tipos.

Para escribir, la parte 1 usa la API segura de `pdfium-render` 0.8.37 y fuerza la generación de `/AP` con un render mínimo.

Del §6.2 falta: texto libre, formas (rectángulo, elipse, línea y flecha, polígono, polilínea), lápiz y borrador, sellos y stickers, adjuntos, respuestas (`/IRT`), biblioteca de anotaciones, medición, XFDF de todos los tipos y aplanar.

## 2. Hallazgo que condiciona el diseño

Revisé `pdfium-render` 0.8.37, la versión del repo, y 0.9.5, la última.

- **Subtipos que no se pueden crear:** `Circle`, `Line`, `Polygon`, `PolyLine` y `FileAttachment`. Ninguna de las dos versiones los expone, y el propio PDFium (`FPDFAnnot_IsSupportedSubtype`) no deja crear `Line`, `Polygon` ni `PolyLine`.
- **Claves que no se pueden escribir:** `/IRT`, que es una referencia a otro objeto; `/L`, `/Vertices`, `/InkList`, `/IC`, `/LE`, `/Measure` y `/DA`, que son arrays o diccionarios; y tampoco `/NM`. `set_string_value` es privado y los *handles* `FPDF_DOCUMENT`/`FPDF_PAGE`/`FPDF_ANNOTATION` son `pub(crate)`, así que no se puede llamar a la API C directamente.
- Actualizar a 0.9.5 no resuelve ninguna de estas dos cosas. La API de anotaciones y la visibilidad de los *handles* son iguales.
- PDFium genera `/AP` para algunos subtipos (Square, Circle, Ink, Highlight y otros), pero no para Line, Polygon ni PolyLine. Chrome usa PDFium, así que sin `/AP` propio esas anotaciones no se verían en Chrome. Ese es justamente el criterio de cierre de M2.

En resumen: con PDFium solo no se puede cerrar M2. Hace falta un escritor estructural, y `lopdf` (MIT) ya figura como *default* técnico en el §2 del plan.

## 3. Diseño propuesto

### 3.1 Motor de anotaciones: lopdf escribe y PDFium lee y renderiza

- `DocumentSession` guarda los **bytes vigentes** del documento (`Vec<u8>`) y el `PdfDocument` de PDFium cargado desde esos bytes (`load_pdf_from_byte_vec`).
- Cada mutación de anotaciones funciona así:
  - `lopdf` agrega o reemplaza objetos como **actualización incremental** (`IncrementalDocument`): diccionario de la anotación, `/AP`, `/Annots` de la página.
  - Se recarga PDFium desde los bytes nuevos y se invalidan los tiles y las cachés de texto de las páginas afectadas.
  - El costo es O(tamaño del cambio) más una recarga perezosa de PDFium. Se mide en E2 con el PDF más grande del corpus; la meta es menos de 150 ms por operación.
- **Guardar** escribe los bytes vigentes con la escritura atómica que ya existe. Opcionalmente compacta las actualizaciones incrementales (reescritura completa) en "Guardar como".
- Los tipos de la parte 1 también migran a este camino. Así hay un solo escritor, y se resuelven las dos limitaciones de la parte 1: el color de las notas (hoy PDFium las dibuja siempre amarillas) y `/NM`, que hoy no se conserva.
- **Identidad estable**: toda anotación creada o importada lleva `/NM` (UUID v4). El id que viaja a la UI pasa de `"página:índice"`, que cambia al borrar, a `/NM`. Las anotaciones ajenas que no tengan `/NM` reciben uno la primera vez que se modifican.

### 3.2 Generador de apariencias propio (`appearance.rs`)

- Cada anotación que escribimos lleva su `/AP /N`: un Form XObject con un *content stream* generado en Rust (rectángulo, elipse por 4 Béziers, líneas con terminaciones `/LE`, polígonos, trazos de tinta suavizados, cajas de texto, imagen de sello).
- Así se ve igual en PDFium, Chrome y Acrobat, y deja de depender de que cada visor genere la apariencia.
- Transparencia con `/ExtGState` (`/CA` y `/ca`). Borde con `/BS` (`/W`, `/S` sólido o discontinuo, `/D`).
- **Fuente del texto libre:** Helvetica estándar con `WinAnsiEncoding`, que cubre español completo (ñ, tildes, ¿, ¡). Los caracteres fuera de WinAnsi se avisan en la UI. El soporte Unicode completo con fuentes embebidas llega con el subsetting de M4 (pregunta P4).

### 3.3 Tipos nuevos

| Tipo | Subtipo PDF | Detalle |
|---|---|---|
| Texto libre | `FreeText` | Caja con borde opcional, color de texto y fondo, tamaño de fuente y alineación (`/Q`). Lleva `/DA` y `/DS` para que Acrobat la edite. Las llamadas (`/CL`) quedan fuera. |
| Rectángulo / elipse | `Square` / `Circle` | Trazo, relleno (`/IC`), grosor, discontinuo y opacidad. |
| Línea / flecha | `Line` | `/L` y `/LE` (None, OpenArrow, ClosedArrow, Circle, Square). |
| Polígono / polilínea | `Polygon` / `PolyLine` | `/Vertices`. Se cierra con doble clic o tocando el primer vértice. |
| Lápiz | `Ink` | `/InkList` desde *pointer events*: mouse, lápiz y dedo con el mismo código. Simplificación Douglas-Peucker y suavizado en la apariencia. |
| Borrador | — | Borra puntos dentro del radio y **parte el trazo** en los segmentos que quedan. Si no queda ninguno, elimina la anotación. |
| Sello | `Stamp` | Sellos de texto predefinidos (Aprobado, Rechazado, Borrador, Confidencial, Revisado, Recibido) con fecha y autor opcionales, y sellos de imagen (PNG o JPEG como Image XObject en `/AP`). Un sticker es un sello de imagen de la biblioteca. |
| Adjunto | `FileAttachment` | `/FS` con `/EF` embebido, icono PushPin o Paperclip, y descarga desde el panel. |
| Respuesta | `Text` con `/IRT` | Ver §3.5. |
| Medición | `Line` / `PolyLine` / `Polygon` con `/IT` y `/Measure` | Ver §3.6. |

Todas se pueden **mover y redimensionar** en el canvas. Al hacerlo se reescriben `/Rect` y la geometría (`/L`, `/Vertices`, `/InkList`) y se regenera `/AP`. El marcado de texto (resaltado y los demás) solo se mueve o borra; no se redimensiona.

### 3.4 Capa de anotaciones en el canvas (capa 4 del §5.1)

- **Overlay SVG por página**, en coordenadas de página. Se usa para:
  - la vista previa mientras se dibuja;
  - el marco de selección con tiradores;
  - el *hit-testing* (`/Rect` y geometría vienen en el DTO).
- La anotación definitiva siempre la dibuja PDFium en el tile, para que lo que se ve sea lo que se guarda.
- **Ribbon "Comentar"**: herramientas, selector de color y paleta, grosor, opacidad y tamaño de fuente. El panel de propiedades edita la anotación seleccionada.
- **Android**: los mismos *pointer events*. Pulsación larga para seleccionar y tiradores de 44 px (touch target de M1).

### 3.5 Respuestas e hilos

- **Responder**: crea una `Text` con `/IRT` hacia la anotación padre, `/RT /R`, autor y fecha. No se dibuja como icono propio en la página: usa el `/Rect` del padre, y el panel la muestra dentro del hilo. Qué flags usa Acrobat en las respuestas (`Hidden`, `NoZoom`, `NoRotate`) se confirma con un PDF de referencia al inicio de 2c.
- **Estado de revisión**: también es una respuesta, con `/State` (Accepted, Rejected, Cancelled, Completed, None) y `/StateModel /Review`. Se muestra como etiqueta en el hilo.
- **Panel**: agrupa los hilos bajo su anotación raíz, en orden cronológico. Al borrar una raíz se borran sus respuestas, con confirmación.

### 3.6 Medición

- **Herramientas**:
  - distancia: `Line` con `/IT /LineDimension`;
  - perímetro: `PolyLine` con `/IT /PolyLineDimension`;
  - área: `Polygon` con `/IT /PolygonDimension`.
- **Escala por documento**, por ejemplo "1 cm = 2 m". Se guarda en formato **estándar**:
  - cada anotación de medición lleva `/Measure` (`/RL`, `/X`, `/D`, `/A`), que Acrobat lee;
  - la página lleva `/VP` con su `/Measure` para que la escala persista.
  - No se crea un diccionario propio (P5).
- La etiqueta con el valor ("12,40 m", con coma decimal) va en `/Contents` y en la apariencia.
- Al mover un vértice el valor se recalcula.

### 3.7 Biblioteca de anotaciones

- Vive en el perfil del usuario: `app_data_dir()/biblioteca/`, con `biblioteca.json` y `assets/` para PNG y SVG rasterizado.
- **Contenido**:
  - sellos de texto personalizados;
  - stickers (imágenes);
  - estilos guardados (color, grosor, opacidad y relleno de una herramienta);
  - firmas de trazo (tinta reutilizable).
- **Uso**: el panel "Biblioteca" permite arrastrar y soltar al canvas en desktop, o tocar para colocar en Android. Cualquier anotación seleccionada se puede guardar en la biblioteca.
- El formato lleva versión (`"version": 1`) para poder migrarlo. Importar y exportar la biblioteca como archivo queda fuera de M2.

### 3.8 XFDF completo

- **Tipos**: se agregan `square`, `circle`, `line` (`start`/`end`/`head`/`tail`), `polygon` y `polyline` (`<vertices>`), `ink` (`<inklist><gesture>`), `freetext` (`defaultappearance`, `defaultstyle`, `justification`), `stamp` (`icon` y `<imagedata>` base64 para sellos de imagen), `fileattachment` (`<data>` base64) y `measure`.
- **Atributos**: se agregan `name` (ahora sí se conserva), `inreplyto`, `replyType`, `state`, `statemodel`, `width`, `style`, `dashes`, `interior-color` y `opacity`.
- **Prueba de cierre**: exportar, importar en un documento limpio y volver a exportar produce un XFDF equivalente (comparación semántica) para cada tipo del corpus.

### 3.9 Aplanar

- "Aplanar anotaciones" de la página o del documento, con `PdfPage::flatten()` de pdfium-render (`FPDFPage_Flatten`, `FLAT_PRINT`). Funciona sin problemas porque todas nuestras anotaciones tienen `/AP` y el flag Print.
- Se adelanta a `core-edit` (el §6.2 lo ubicaba ahí) porque no requiere edición de *content streams* propia (P6).
- Por ser destructivo, por defecto se ofrece guardar el resultado **como copia**.
- Se prueba que después de aplanar no quedan anotaciones de marcado y que el raster es el mismo con un margen de tolerancia.

### 3.10 Deshacer y rehacer

- El §5.3 pide el patrón *command*. En M2 se implementa para anotaciones con Ctrl+Z y Ctrl+Shift+Z, y botones en el ribbon.
- **Funcionamiento**: cada operación guarda el diccionario anterior y el nuevo de cada anotación afectada (por `/NM`). Deshacer reescribe el anterior como actualización incremental.
- **Límite**: la pila tiene 200 pasos por pestaña y se vacía al cerrar.
- M3 y M4 reutilizan la misma pila (P2).

## 4. Fases de E2

Cada fase es un PR propio con CI verde y se puede revisar por separado.

| Fase | Contenido | Termina cuando |
|---|---|---|
| **2a: base y formas** | Escritor lopdf, recarga, `/NM`, migración de la parte 1, generador de `/AP`, deshacer y rehacer, capa SVG (selección, mover, redimensionar), ribbon "Comentar", rectángulo, elipse, línea y flecha, polígono, polilínea y texto libre | Cada tipo se crea, se mueve, se guarda y al reabrir se ve igual en PDFium (prueba de píxeles) y pasa `qpdf --check` en CI |
| **2b: tinta, sellos y biblioteca** | Lápiz, borrador que parte trazos, sellos de texto e imagen, stickers, adjuntos y biblioteca | Firma de trazo guardada en la biblioteca y reutilizada en otro documento; adjunto descargable después de reabrir |
| **2c: revisión y cierre** | Respuestas y estados, medición con escala, XFDF de todos los tipos, aplanar, guía de QA manual | Criterio de cierre de M2 (§5) y luego E3 |

## 5. Criterio de cierre de M2 (detallado)

1. CI verde: fmt, clippy, tests de Rust, lint, tests y build de la UI, más `qpdf --check` sobre cada PDF que escriben los tests (se agrega `qpdf` al job de Rust con apt).
2. Pruebas de integración por tipo: crear, guardar, reabrir con PDFium, comparar píxeles y leer de vuelta sin pérdida.
3. Prueba de XFDF de todos los tipos sin pérdida (§3.8).
4. QA manual con guion en `docs/qa/M2.md`: un PDF anotado con todos los tipos abre bien en **Acrobat Reader y Chrome**, y Acrobat muestra las respuestas, los estados y las mediciones. Esta parte la haces tú, Camilo, o QuAsimido, porque este entorno no tiene Acrobat.
5. Verificación en Android: dibujar con el dedo, colocar un sello y responder.

## 6. Riesgos

| Riesgo | Mitigación |
|---|---|
| La recarga de PDFium por mutación es lenta en PDFs grandes | Medir en 2a. Si pasa de 150 ms, se agrupan las mutaciones de un mismo gesto y PDFium se recarga una vez por gesto |
| `lopdf` 0.34 descifra pero **no cifra** al escribir | P8: en M2, los PDF con contraseña abren con anotaciones en solo lectura y un aviso. El cifrado de salida llega con qpdf en M8 |
| Una apariencia propia que Acrobat dibuje distinto | Se usan solo operadores básicos (`re`, `c`, `l`, `Tj`, `Do`), más el guion de QA manual |
| `FPDFPage_Flatten` con problemas conocidos (pdfium-render #140) | Solo se aplana lo que tiene `/AP` propio, más una prueba de raster antes y después |
| Alcance grande | Tres PR (2a, 2b, 2c) que se pueden entregar por separado |

## 7. Preguntas cerradas para aprobar

| # | Pregunta | Opciones (la recomendada va primero) |
|---|---|---|
| **P1** | ¿Quién escribe las anotaciones? | **(a)** `lopdf` para todo, con actualización incremental, recarga de PDFium y migración de la parte 1 · (b) híbrido: PDFium para lo que soporta y `lopdf` solo para lo que falta (dos caminos que mantener y `/NM` inconsistente) · (c) actualizar pdfium-render (descartada: 0.9.5 no lo resuelve) |
| **P2** | ¿Deshacer y rehacer en M2? | **(a)** Sí, para anotaciones, con una pila que M3 y M4 reutilizan · (b) Diferir a M4 |
| **P3** | ¿Cómo se integra? | **(a)** Fusionar primero la PR #1 en `main` y hacer 2a, 2b y 2c como PR desde `feature/m2-<fase>` (§11) · (b) Seguir agregando commits a la PR #1 |
| **P4** | ¿Fuente del texto libre? | **(a)** Helvetica estándar WinAnsi (español completo) con aviso para otros caracteres; Unicode completo en M4 · (b) Embeber Liberation Sans completa ya (aprox. 400 KB por documento, sin subsetting) |
| **P5** | ¿Dónde se guarda la escala de medición? | **(a)** Estándar: `/Measure` en cada anotación y `/VP` en la página (Acrobat la entiende) · (b) Diccionario propio de PDF SHARD |
| **P6** | ¿Aplanar ahora? | **(a)** Sí, con `FPDFPage_Flatten` en `core-render`, ofreciendo guardar como copia · (b) Esperar a `core-edit` (M4) |
| **P7** | ¿Sellos predefinidos? | **(a)** Seis sellos de texto en español (Aprobado, Rechazado, Borrador, Confidencial, Revisado, Recibido) dibujados como vectores, sin assets de terceros · (b) Además, un set de stickers ilustrados con licencia OFL o CC0 |
| **P8** | ¿PDF con contraseña? | **(a)** Anotaciones en solo lectura con aviso hasta M8 · (b) Escribirlos sin cifrar con una advertencia explícita (cambia la seguridad del archivo, no recomendado) |

## 8. Fuera de alcance de M2

- Texto enriquecido (`/RC`) editable y llamadas (`/CL`) de texto libre.
- Anotaciones de audio, video, 3D, Caret, Redact (la redacción es M8) y Watermark (M3).
- Importar y exportar la biblioteca como archivo.
- Comparación de revisiones.
- Sincronización de comentarios entre usuarios.

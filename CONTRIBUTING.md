# Contribuir a PDF SHARD

¡Gracias por el interés! PDF SHARD está en desarrollo temprano (ver [`docs/PLAN.md`](docs/PLAN.md), que es la fuente de verdad del proyecto: decisiones, arquitectura y milestones).

## Reglas de oro

1. **Clean-room**: solo código propio o de dependencias open source registradas en [`THIRD_PARTY.md`](THIRD_PARTY.md). Jamás código de origen dudoso (filtraciones, decompilados, snippets sin licencia). Nada con licencia AGPL se linkea al binario.
2. **Licencia**: todo aporte queda bajo [GPL-3.0](LICENSE).
3. **Sin IA en el producto**: OCR y análisis de layout son deterministas. Las funciones tipo "chat con el PDF" están fuera del alcance por diseño (PLAN.md §4).
4. **Multiplataforma**: nada se acepta "solo desktop" sin decisión explícita; Android es plataforma de primera clase.

## Cómo compilar

Requisitos: Rust estable (MSVC en Windows), Node 25+, y en Linux las dependencias de sistema de Tauri.

```bash
# 1. Binario de PDFium para tu plataforma (hash verificado contra scripts/pdfium.lock)
pwsh scripts/fetch-pdfium.ps1 -Target win-x64    # Windows
bash scripts/fetch-pdfium.sh linux-x64           # Linux / macOS (mac-arm64 / mac-x64)

# 2. Dependencias JS
npm install && npm install --prefix ui

# 3. Corpus de pruebas y verificación headless del render
cargo run -p corpus-gen
cargo run -p render-check

# 4. La app en modo desarrollo
cd apps/shard && npx tauri dev
```

## Antes de abrir un PR

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm run lint --prefix ui && npm test --prefix ui
```

- Commits en español, formato `modulo: resumen` (sin coautores automáticos).
- Los textos de UI van SOLO en `ui/src/strings/` (español; la i18n llegará después).
- PDFs de prueba: solo sintéticos o con licencia libre en `corpus/`; documentos reales van en `corpus-privado/` (ignorado por git).

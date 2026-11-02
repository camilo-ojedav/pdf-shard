// URL del protocolo personalizado `shard://` según plataforma:
// Windows y Android exponen los esquemas personalizados de Tauri como
// http://<esquema>.localhost/, macOS y Linux como <esquema>://localhost/.

const usaEsquemaHttp =
  navigator.userAgent.includes("Windows") || navigator.userAgent.includes("Android");

const BASE = usaEsquemaHttp ? "http://shard.localhost" : "shard://localhost";

/** URL del tile PNG de una página (índice base 0) al ancho pedido en píxeles.
 * `revision` cambia cada vez que el documento muta (anotaciones): el backend
 * ignora la query, pero el webview la usa como clave de caché, así que una
 * revisión nueva fuerza a pedir el tile re-renderizado. */
export function tileUrl(docId: number, pageIndex: number, widthPx: number, revision = 0): string {
  const base = `${BASE}/tile/${docId}/${pageIndex}/${widthPx}`;
  return revision > 0 ? `${base}?r=${revision}` : base;
}

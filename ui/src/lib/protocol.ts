// URL del protocolo personalizado `shard://` según plataforma:
// Windows y Android exponen los esquemas personalizados de Tauri como
// http://<esquema>.localhost/, macOS y Linux como <esquema>://localhost/.

const usaEsquemaHttp =
  navigator.userAgent.includes("Windows") || navigator.userAgent.includes("Android");

const BASE = usaEsquemaHttp ? "http://shard.localhost" : "shard://localhost";

/** URL del tile PNG de una página (índice base 0) al ancho pedido en píxeles. */
export function tileUrl(docId: number, pageIndex: number, widthPx: number): string {
  return `${BASE}/tile/${docId}/${pageIndex}/${widthPx}`;
}

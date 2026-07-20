/** `true` en el APK Android (mismo criterio que `protocol.ts` para el esquema shard://). */
export function isAndroid(): boolean {
  return navigator.userAgent.includes("Android");
}

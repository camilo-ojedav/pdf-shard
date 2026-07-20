// TODOS los textos visibles de la UI viven aquí (PLAN.md §11, decisión D07):
// hoy solo español; cuando llegue i18n, este módulo se convierte en el
// catálogo base sin tocar componentes.

export const S = {
  app: {
    titulo: "PDF SHARD",
  },
  visor: {
    abrir: "Abrir PDF",
    sinDocumento: "Abre un documento PDF para comenzar",
    anterior: "Página anterior",
    siguiente: "Página siguiente",
    acercar: "Acercar",
    alejar: "Alejar",
    errorAbrir: "No se pudo abrir el documento",
    paginaDe: (n: number, total: number): string => `Página ${n} de ${total}`,
    zoomPorciento: (z: number): string => `${Math.round(z * 100)}%`,
  },
} as const;

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
    ajustarAncho: "Ajustar al ancho",
    ajustarPagina: "Ajustar a la página",
    errorAbrir: "No se pudo abrir el documento",
    paginaDe: (n: number, total: number): string => `Página ${n} de ${total}`,
    zoomPorciento: (z: number): string => `${Math.round(z * 100)}%`,
    cerrarPestana: "Cerrar pestaña",
    nuevaPestana: "Abrir otro PDF",
  },
  recientes: {
    titulo: "Recientes",
    vacio: "Sin documentos recientes",
    limpiar: "Limpiar recientes",
  },
  buscar: {
    boton: "Buscar",
    placeholder: "Buscar en el documento…",
    coincidenciaMayus: "Mayúsculas/minúsculas",
    palabraCompleta: "Palabra completa",
    sinResultados: "Sin resultados",
    contador: (activo: number, total: number): string => `${activo} de ${total}`,
    truncado: "Se muestran los primeros resultados; refina la búsqueda para ver más",
    anteriorResultado: "Resultado anterior",
    siguienteResultado: "Resultado siguiente",
    cerrar: "Cerrar búsqueda",
  },
  panel: {
    marcadores: "Marcadores",
    miniaturas: "Miniaturas",
    sinMarcadores: "Este documento no tiene marcadores",
    cerrarPanel: "Cerrar panel",
  },
  seleccion: {
    copiar: "Copiar",
    copiado: "Copiado",
  },
  imprimir: {
    boton: "Imprimir",
    error: "No se pudo enviar el documento a imprimir",
  },
  tema: {
    claro: "Tema claro",
    oscuro: "Tema oscuro",
    cambiar: "Cambiar tema",
  },
  password: {
    titulo: "Documento protegido",
    pideContrasena: "Este documento requiere una contraseña para abrirse.",
    incorrecta: "La contraseña ingresada es incorrecta. Inténtalo de nuevo.",
    placeholder: "Contraseña",
    abrir: "Abrir",
    cancelar: "Cancelar",
  },
} as const;

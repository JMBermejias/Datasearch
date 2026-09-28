// Copyright (c) 2026 Jose Manuel Bernabeu Mejias - Licencia MIT
// Iconos SVG en linea. Evitan depender de fuentes o ficheros externos.

const TRAZOS: Record<string, string> = {
  panel:
    '<rect x="3" y="3" width="7" height="9" rx="1.5"/><rect x="3" y="16" width="7" height="5" rx="1.5"/><rect x="14" y="3" width="7" height="5" rx="1.5"/><rect x="14" y="12" width="7" height="9" rx="1.5"/>',
  panelIzq:
    '<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M9 4v16"/>',
  buscar: '<circle cx="11" cy="11" r="7"/><path d="M20 20l-3.5-3.5"/>',
  filtros:
    '<path d="M3 5h18M6 12h12M10 19h4"/><circle cx="7" cy="5" r="1.6"/><circle cx="16" cy="12" r="1.6"/><circle cx="12" cy="19" r="1.6"/>',
  graficos:
    '<path d="M3 21h18"/><rect x="5" y="11" width="3.6" height="7" rx="1"/><rect x="10.2" y="6" width="3.6" height="12" rx="1"/><rect x="15.4" y="14" width="3.6" height="4" rx="1"/>',
  analisis:
    '<path d="M4 20V9M9.3 20V4M14.7 20v-7M20 20V6"/><path d="M2 21h20"/>',
  base: '<ellipse cx="12" cy="5.5" rx="8" ry="3"/><path d="M4 5.5v6c0 1.7 3.6 3 8 3s8-1.3 8-3v-6"/><path d="M4 11.5v6c0 1.7 3.6 3 8 3s8-1.3 8-3v-6"/>',
  escudo:
    '<path d="M12 2.5l7.5 3v6c0 4.7-3.2 8.6-7.5 10-4.3-1.4-7.5-5.3-7.5-10v-6z"/><path d="M9 12l2.2 2.2L15.5 10"/>',
  usuarios:
    '<circle cx="9" cy="8" r="3.2"/><path d="M2.8 20c0-3.4 2.8-5.6 6.2-5.6s6.2 2.2 6.2 5.6"/><path d="M16.5 5.4a3.2 3.2 0 010 5.2M18 14.6c2 .7 3.4 2.5 3.4 5.4"/>',
  engranaje:
    '<circle cx="12" cy="12" r="3.2"/><path d="M12 2.5v2.8M12 18.7v2.8M21.5 12h-2.8M5.3 12H2.5M18.7 5.3l-2 2M7.3 16.7l-2 2M18.7 18.7l-2-2M7.3 7.3l-2-2"/>',
  cerrar: '<path d="M6 6l12 12M18 6L6 18"/>',
  info: '<circle cx="12" cy="12" r="9"/><path d="M12 11v5M12 7.6v.6"/>',
  exito: '<circle cx="12" cy="12" r="9"/><path d="M8 12.3l2.7 2.7L16 9.5"/>',
  aviso:
    '<path d="M12 3.5L21.5 20h-19z"/><path d="M12 10v4M12 17.2v.4"/>',
  pdf: '<path d="M14 3H7a2 2 0 00-2 2v14a2 2 0 002 2h10a2 2 0 002-2V8z"/><path d="M14 3v5h5"/><path d="M8.5 17v-4h1.6a1.2 1.2 0 010 2.4H8.5M13 17v-4h1.2a2 2 0 010 4z"/>',
  excel:
    '<path d="M14 3H7a2 2 0 00-2 2v14a2 2 0 002 2h10a2 2 0 002-2V8z"/><path d="M14 3v5h5"/><path d="M9 12.5l4 5M13 12.5l-4 5"/>',
  csv: '<path d="M14 3H7a2 2 0 00-2 2v14a2 2 0 002 2h10a2 2 0 002-2V8z"/><path d="M14 3v5h5"/><path d="M8.5 13h7M8.5 16h4.5"/>',
  imprimir:
    '<path d="M7 8V3h10v5"/><rect x="3" y="8" width="18" height="8" rx="2"/><path d="M7 14h10v7H7z"/>',
  descargar:
    '<path d="M12 3v12M7.5 11L12 15.5 16.5 11"/><path d="M4 17v2a2 2 0 002 2h12a2 2 0 002-2v-2"/>',
  anadir: '<path d="M12 5v14M5 12h14"/>',
  lapiz:
    '<path d="M4 20l4.5-1 10-10a2.1 2.1 0 00-3-3l-10 10z"/><path d="M14.5 6.5l3 3"/>',
  basura:
    '<path d="M4 7h16M9.5 7V4.5h5V7M6 7l1 13h10l1-13"/><path d="M10 11v6M14 11v6"/>',
  guardar:
    '<path d="M5 3h11l3 3v15H5z"/><path d="M8 3v6h7V3M8 14h8"/>',
  probar:
    '<path d="M9 3h6M10 3v6L5 19a2 2 0 001.8 3h10.4A2 2 0 0019 19l-5-10V3"/><path d="M7.5 14h9"/>',
  refrescar:
    '<path d="M20.5 12a8.5 8.5 0 11-2.6-6.1"/><path d="M20.5 4v5h-5"/>',
  salir: '<path d="M9 21H5a2 2 0 01-2-2V5a2 2 0 012-2h4"/><path d="M16 16l5-4-5-4M21 12H9"/>',
  candado:
    '<rect x="4.5" y="10.5" width="15" height="10" rx="2"/><path d="M8 10.5V7a4 4 0 018 0v3.5"/>',
  usuario: '<circle cx="12" cy="8" r="4"/><path d="M4.5 20.5c0-4 3.4-6.5 7.5-6.5s7.5 2.5 7.5 6.5"/>',
  menu: '<path d="M4 7h16M4 12h16M4 17h16"/>',
  capas:
    '<path d="M12 3l9 4.5-9 4.5-9-4.5z"/><path d="M3 12.5l9 4.5 9-4.5M3 17l9 4.5 9-4.5"/>',
  reloj: '<circle cx="12" cy="12" r="9"/><path d="M12 7v5.4l3.4 2"/>',
  bombilla:
    '<path d="M9 18h6M10 21h4"/><path d="M12 3a6 6 0 00-3.5 10.9V16h7v-2.1A6 6 0 0012 3z"/>',
  actualizar:
    '<path d="M3.5 12a8.5 8.5 0 108.5-8.5A8.4 8.4 0 006 6.3"/><path d="M3.5 3.5v5h5"/>',
  salirActualizacion:
    '<path d="M12 3v10M8 9.5l4 3.5 4-3.5"/><path d="M4 17v2a2 2 0 002 2h12a2 2 0 002-2v-2"/>',
  tabla:
    '<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M3 9.5h18M9 9.5V20M15 9.5V20"/>',
  arrastrar: '<path d="M8 8h.01M8 12h.01M8 16h.01M16 8h.01M16 12h.01M16 16h.01" stroke-width="2.4" stroke-linecap="round"/>',
  ojo: '<path d="M2 12s3.6-6.5 10-6.5S22 12 22 12s-3.6 6.5-10 6.5S2 12 2 12z"/><circle cx="12" cy="12" r="3"/>',
  carpeta:
    '<path d="M3 7a2 2 0 012-2h4l2 2.5h8a2 2 0 012 2V18a2 2 0 01-2 2H5a2 2 0 01-2-2z"/>',
  credencial:
    '<rect x="2.5" y="5" width="19" height="14" rx="2.5"/><circle cx="8.5" cy="11" r="2.2"/><path d="M5 16c.6-1.5 1.9-2.2 3.5-2.2S11.4 14.5 12 16M14.5 10h4M14.5 13.5h3"/>',
};

/** Devuelve el marcado SVG de un icono por su nombre. */
export function icono(nombre: string, clase = ""): string {
  const trazo = TRAZOS[nombre] ?? TRAZOS.info;
  // Los atributos `width` y `height` son el tamano minimo de seguridad: si
  // algun contenedor no tiene regla CSS para `svg`, el icono se dibujaria al
  // 100 % del hueco y empujaria el contenido hacia abajo. Las reglas CSS
  // pisan estos atributos, de modo que el tamano real sigue siendo el que
  // marque cada componente.
  return `<svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor"
    stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" class="ic ${clase}"
    aria-hidden="true">${trazo}</svg>`;
}

/** Nombres de todos los iconos disponibles. */
export const ICONOS = Object.keys(TRAZOS);

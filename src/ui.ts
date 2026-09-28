// Copyright (c) 2026 Jose Manuel Bernabeu Mejias - Licencia MIT
// Utilidades de interfaz: creacion de elementos, avisos, modales, tablas,
// desplegables multiples y descarga de ficheros.

import { icono } from "./iconos";
import { comoError } from "./api";

export { comoError };

// ---------------------------------------------------------------------
// Creacion de elementos
// ---------------------------------------------------------------------

type Atributos = Record<string, string | number | boolean | undefined | null>;

/** Crea un elemento con atributos y contenido. */
export function el<K extends keyof HTMLElementTagNameMap>(
  etiqueta: K,
  atributos: Atributos = {},
  html = "",
): HTMLElementTagNameMap[K] {
  const nodo = document.createElement(etiqueta);
  for (const [clave, valor] of Object.entries(atributos)) {
    if (valor === undefined || valor === null || valor === false) continue;
    if (clave === "class") nodo.className = String(valor);
    else if (clave === "text") nodo.textContent = String(valor);
    else nodo.setAttribute(clave, valor === true ? "" : String(valor));
  }
  if (html) nodo.innerHTML = html;
  return nodo;
}

/** Vacia un nodo. */
export function limpiar(nodo: HTMLElement): HTMLElement {
  while (nodo.firstChild) nodo.removeChild(nodo.firstChild);
  return nodo;
}

/** Escapa el contenido para insertarlo con seguridad en una plantilla. */
export function esc(texto: unknown): string {
  return String(texto ?? "")
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

/** Crea un boton con icono opcional. */
export function boton(
  texto: string,
  opciones: {
    clase?: string;
    iconoNombre?: string;
    alPulsar?: (ev: MouseEvent) => void;
    titulo?: string;
    tipo?: "button" | "submit";
  } = {},
): HTMLButtonElement {
  const b = el("button", {
    class: `btn ${opciones.clase ?? ""}`.trim(),
    type: opciones.tipo ?? "button",
    title: opciones.titulo ?? texto,
  });
  if (opciones.iconoNombre) {
    b.insertAdjacentHTML("beforeend", icono(opciones.iconoNombre));
  }
  if (texto) b.append(document.createTextNode(texto));
  if (opciones.alPulsar) b.addEventListener("click", opciones.alPulsar);
  return b;
}

/** Crea un campo de formulario con etiqueta. */
export function campo(
  etiqueta: string,
  control: HTMLElement,
  ayuda = "",
): HTMLLabelElement {
  const l = el("label", { class: "campo" });
  l.append(el("span", { html: `${esc(etiqueta)}${ayuda ? ` <span class="ayuda">${esc(ayuda)}</span>` : ""}` }));
  l.append(control);
  return l;
}

/** Crea un `select` con opciones. */
export function select(
  opciones: { valor: string; etiqueta: string }[],
  valor = "",
  multiple = false,
): HTMLSelectElement {
  const s = el("select", multiple ? { multiple: "multiple", size: "6" } : {});
  for (const o of opciones) {
    const op = el("option", { value: o.valor, text: o.etiqueta });
    if (o.valor === valor) op.selected = true;
    s.append(op);
  }
  return s;
}

/** Crea una casilla de verificacion con su etiqueta. */
export function casilla(texto: string, marcado = false): { envoltura: HTMLLabelElement; entrada: HTMLInputElement } {
  const entrada = el("input", { type: "checkbox" });
  entrada.checked = marcado;
  const envoltura = el("label", { class: "campo-checkbox" });
  envoltura.append(entrada, el("span", { text: texto }));
  return { envoltura, entrada };
}

// ---------------------------------------------------------------------
// Formato de datos
// ---------------------------------------------------------------------

const FORMATO_ES = new Intl.NumberFormat("es-ES", { maximumFractionDigits: 2 });
const FORMATO_ES4 = new Intl.NumberFormat("es-ES", {
  minimumFractionDigits: 2,
  maximumFractionDigits: 4,
});

/** Convierte texto en numero respetando el formato español e ingles. */
export function aNumero(texto: string | null | undefined): number | null {
  if (texto === null || texto === undefined) return null;
  const t = String(texto).trim();
  if (!t) return null;
  if (/^-?\d+(\.\d+)?$/.test(t)) return Number(t);
  if (/^-?\d{1,3}(\.\d{3})*,\d+$/.test(t)) {
    return Number(t.replace(/\./g, "").replace(",", "."));
  }
  if (/^-?\d{1,3}(,\d{3})*\.\d+$/.test(t)) {
    return Number(t.replace(/,/g, ""));
  }
  const limpio = t.replace(/[^\d.,-]/g, "");
  if (!limpio) return null;
  const normal = limpio.includes(",") && !limpio.includes(".")
    ? limpio.replace(/\./g, "").replace(",", ".")
    : limpio.replace(/,/g, "");
  const n = Number(normal);
  return Number.isFinite(n) ? n : null;
}

/** Formatea un numero con el estilo espanol. */
export function numero(n: number | null | undefined, decimales = 0): string {
  if (n === null || n === undefined || !Number.isFinite(n)) return "-";
  return decimales > 0 ? FORMATO_ES4.format(n) : FORMATO_ES.format(n);
}

/** Formatea un valor segun la familia deducida del texto. */
export function valor(texto: string | null | undefined): string {
  if (texto === null || texto === undefined) return "";
  const t = String(texto);
  const n = aNumero(t);
  if (n !== null && /^[\d.,\s-]+$/.test(t.trim()) && /\d/.test(t)) {
    return t.includes(",") || t.includes(".") ? t : numero(n);
  }
  return t;
}

/** Tamano de fichero legible. */
export function tamano(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
}

/** Fecha legible a partir del ISO-8601 que devuelve el nucleo. */
export function fecha(iso: string | null | undefined): string {
  if (!iso) return "-";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleString("es-ES", {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

/** Iniciales de un nombre, para el avatar. */
export function iniciales(nombre: string): string {
  const partes = nombre.trim().split(/\s+/).filter(Boolean);
  if (!partes.length) return "?";
  if (partes.length === 1) return partes[0].slice(0, 2).toUpperCase();
  return (partes[0][0] + partes[1][0]).toUpperCase();
}

// ---------------------------------------------------------------------
// Avisos flotantes
// ---------------------------------------------------------------------

type TipoAviso = "exito" | "error" | "info" | "aviso";

let contenedorAvisos: HTMLElement | null = null;

function contenedor(): HTMLElement {
  if (!contenedorAvisos) {
    contenedorAvisos = el("div", { class: "avisos" });
    document.body.append(contenedorAvisos);
  }
  return contenedorAvisos;
}

/** Muestra un aviso flotante. */
export function avisar(
  texto: string,
  tipo: TipoAviso = "info",
  milisegundos = 5000,
): void {
  const n = el("div", { class: `aviso ${tipo}` });
  const marca = tipo === "exito" ? "exito" : tipo === "error" ? "aviso" : tipo === "aviso" ? "aviso" : "info";
  n.insertAdjacentHTML("beforeend", icono(marca));
  n.append(el("div", { class: "contenido", text: texto }));
  const cerrar = el("button", { class: "cerrar", text: "×", title: "Cerrar" });
  cerrar.addEventListener("click", () => n.remove());
  n.append(cerrar);
  contenedor().append(n);
  if (milisegundos > 0) {
    setTimeout(() => {
      n.style.transition = "opacity .2s";
      n.style.opacity = "0";
      setTimeout(() => n.remove(), 220);
    }, milisegundos);
  }
}

// ---------------------------------------------------------------------
// Carga
// ---------------------------------------------------------------------

let nodoCarga: HTMLElement | null = null;

/** Muestra el indicador de carga a pantalla completa. */
export function mostrarCarga(texto = "Procesando...", detalle = ""): void {
  ocultarCarga();
  nodoCarga = el("div", { class: "carga" });
  const caja = el("div", { class: "caja-carga" });
  caja.insertAdjacentHTML("beforeend", '<div class="girador"></div>');
  caja.append(el("div", { class: "texto", text: texto }));
  if (detalle) caja.append(el("div", { class: "detalle", text: detalle }));
  nodoCarga.append(caja);
  document.body.append(nodoCarga);
}

/** Actualiza el texto del indicador de carga. */
export function textoCarga(texto: string): void {
  const n = nodoCarga?.querySelector(".texto");
  if (n) n.textContent = texto;
}

/** Alias de `ocultarCarga`, para leerse mejor en los `finally`. */
export const esconderCarga = ocultarCarga;

/** Oculta el indicador de carga. */
export function ocultarCarga(): void {
  nodoCarga?.remove();
  nodoCarga = null;
}

/** Muestra una barra de progreso dentro de un contenedor. */
export function barraProgreso(contenedor: HTMLElement): HTMLElement {
  const barra = el("div", { class: "progreso" }, '<div style="width:0%"></div>');
  contenedor.append(barra);
  return barra.firstElementChild as HTMLElement;
}

/** Actualiza el ancho de una barra de progreso. */
export function actualizarProgreso(barra: HTMLElement, porcentaje: number): void {
  barra.style.width = `${Math.max(0, Math.min(100, porcentaje))}%`;
}

// ---------------------------------------------------------------------
// Ventanas modales
// ---------------------------------------------------------------------

export interface Ventana {
  cerrar(): void;
  cuerpo: HTMLElement;
  pie: HTMLElement;
}

/**
 * Abre una ventana modal.
 *
 * @param opciones.pieSi Si se envia `null`, la ventana no muestra botonera.
 */
export function modal(
  titulo: string,
  contenido: HTMLElement | string,
  opciones: {
    clase?: string;
    iconoNombre?: string;
    pie?: (cerrar: () => void) => HTMLElement[] | null;
    alCerrar?: () => void;
  } = {},
): Ventana {
  const velo = el("div", { class: "velo" });
  const caja = el("div", { class: `modal ${opciones.clase ?? ""}`.trim() });

  const cabecera = el("div", { class: "modal-cabecera" });
  if (opciones.iconoNombre) cabecera.insertAdjacentHTML("beforeend", icono(opciones.iconoNombre));
  cabecera.append(el("h2", { text: titulo }));
  const btnCerrar = el("button", { class: "cerrar", text: "×", title: "Cerrar" });
  cabecera.append(btnCerrar);

  const cuerpo = el("div", { class: "modal-cuerpo" });
  if (typeof contenido === "string") cuerpo.innerHTML = contenido;
  else cuerpo.append(contenido);

  caja.append(cabecera, cuerpo);

  let ventana: Ventana;
  const cerrar = () => {
    velo.remove();
    document.removeEventListener("keydown", alPulsarTecla);
    opciones.alCerrar?.();
  };
  const pie = el("div", { class: "modal-pie" });
  const botones = opciones.pie?.(cerrar);
  if (botones && botones.length) {
    pie.append(...botones);
    caja.append(pie);
  }

  function alPulsarTecla(e: KeyboardEvent) {
    if (e.key === "Escape") cerrar();
  }

  btnCerrar.addEventListener("click", cerrar);
  velo.addEventListener("click", (e) => {
    if (e.target === velo) cerrar();
  });
  document.addEventListener("keydown", alPulsarTecla);

  velo.append(caja);
  document.body.append(velo);
  ventana = { cerrar, cuerpo, pie };
  return ventana;
}

/** Pide confirmacion antes de una accion irreversible. */
export function confirmar(
  titulo: string,
  mensaje: string,
  alConfirmar: () => void | Promise<void>,
  textoBoton = "Confirmar",
  peligroso = true,
): void {
  const cuerpo = el("div", { class: "ayuda-texto", text: mensaje });
  modal(titulo, cuerpo, {
    clase: "estrecho",
    iconoNombre: peligroso ? "aviso" : "info",
    pie: (cerrar) => [
      boton("Cancelar", { clase: "fantasma", alPulsar: cerrar }),
      boton(textoBoton, {
        clase: peligroso ? "peligro" : "principal",
        alPulsar: async () => {
          try {
            await alConfirmar();
          } finally {
            cerrar();
          }
        },
      }),
    ],
  });
}

/** Pide un texto simple al usuario. */
export function pedirTexto(
  titulo: string,
  etiqueta: string,
  valor = "",
  alConfirmar?: (valor: string) => void,
  tipo = "text",
): void {
  const cuerpo = el("div");
  const entrada = el("input", { type: tipo, value: valor });
  cuerpo.append(campo(etiqueta, entrada));
  const v = modal(titulo, cuerpo, {
    clase: "estrecho",
    pie: (cerrar) => [
      boton("Cancelar", { clase: "fantasma", alPulsar: cerrar }),
      boton("Aceptar", {
        clase: "principal",
        alPulsar: () => {
          alConfirmar?.(entrada.value.trim());
          cerrar();
        },
      }),
    ],
  });
  setTimeout(() => {
    entrada.focus();
    entrada.select();
    void v;
  }, 60);
}

// ---------------------------------------------------------------------
// Tabla de datos
// ---------------------------------------------------------------------

/**
 * Dibuja una tabla de datos a partir de las filas de una fuente.
 *
 * @param opciones.alOrdenar Callback con el nombre de la columna pulsada.
 */
export function tablaDatos(
  filas: Record<string, string | null>[],
  opciones: {
    columnas?: string[];
    tipos?: Map<string, { numerica: boolean }>;
    filasPorPagina?: number;
    alOrdenar?: (columna: string) => void;
    columnaOrden?: string;
    ordenDesc?: boolean;
  } = {},
): HTMLElement {
  if (!filas.length) {
    return el(
      "div",
      { class: "vacio-estado" },
      `${icono("tabla")}<div class="titulo">Sin resultados</div>
       <div>Pulse el boton Buscar para consultar las fuentes de datos.</div>`,
    );
  }

  // Recolecta todas las columnas presentes, respetando el orden pedido.
  const vistas: string[] = [...(opciones.columnas ?? [])];
  if (!vistas.length) {
    const conjunto = new Set<string>();
    for (const f of filas) for (const k of Object.keys(f)) conjunto.add(k);
    vistas.push(...conjunto);
  }

  const marco = el("div", { class: "tabla-marco" });
  const tabla = el("table", { class: "datos" });
  const thead = el("thead");
  const trh = el("tr");

  for (const c of vistas) {
    const meta = opciones.tipos?.get(c);
    const th = el("th", {
      class: opciones.alOrdenar ? "ordenable" : "",
      title: opciones.alOrdenar ? `Ordenar por ${c}` : c,
    });
    th.textContent = c;
    if (meta) {
      const tipo = el("span", { class: "tipo-col" });
      tipo.textContent = meta.numerica ? "numero" : "texto";
      th.append(tipo);
    }
    if (opciones.columnaOrden === c) {
      th.insertAdjacentHTML("afterend", "");
      th.textContent = `${c} ${opciones.ordenDesc ? "▼" : "▲"}`;
    }
    if (opciones.alOrdenar) {
      th.addEventListener("click", () => opciones.alOrdenar?.(c));
    }
    trh.append(th);
  }
  thead.append(trh);

  const tbody = el("tbody");
  const porPagina = opciones.filasPorPagina ?? 0;
  const visibles = porPagina > 0 ? filas.slice(0, porPagina) : filas;

  for (const fila of filas) {
    if (visibles && !visibles.includes(fila)) continue;
    const tr = el("tr");
    for (const c of vistas) {
      const bruto = fila[c];
      const meta = opciones.tipos?.get(c);
      const numerica = meta?.numerica && aNumero(bruto) !== null;
      const td = el("td", {
        class: `${bruto === null || bruto === undefined ? "nulo" : ""} ${numerica ? "numero" : ""}`.trim(),
        title: bruto === null || bruto === undefined ? "sin valor" : String(bruto),
      });
      td.textContent =
        bruto === null || bruto === undefined || bruto === ""
          ? "—"
          : numerica
            ? numero(aNumero(bruto))
            : valor(bruto);
      tr.append(td);
    }
    tbody.append(tr);
  }

  tabla.append(thead, tbody);
  marco.append(tabla);
  if (porPagina > 0 && filas.length > porPagina) {
    const pie = el("div", { class: "tarjeta-pie" });
    pie.textContent = `Mostrando ${porPagina} de ${filas.length} filas.`;
    marco.append(pie);
  }
  return marco;
}

// ---------------------------------------------------------------------
// Desplegable multiple con buscador
// ---------------------------------------------------------------------

/**
 * Crea un desplegable de seleccion multiple.
 *
 * Es el control que permite elegir fuentes, esquemas, columnas y valores,
 * con un buscador para listas largas.
 */
export function desplegableMultiple(
  opciones: { valor: string; etiqueta: string; detalle?: string }[],
  seleccionados: string[] = [],
  opcionesExtra: { placeholder?: string; filtrable?: boolean; alCambiar?: (valores: string[]) => void } = {},
): { envoltura: HTMLElement; obtener: () => string[]; limpiar: () => void } {
  const envoltura = el("div", { class: "desplegable" });
  const eleccion = new Set(seleccionados);
  let abierto = false;
  let menu: HTMLElement | null = null;

  const botonAbrir = el("button", {
    class: "btn bloque",
    type: "button",
    style: "justify-content:space-between",
  });
  const textoBoton = el("span", {});
  botonAbrir.append(textoBoton, el("span", { html: "&#9662;" }));

  const refrescarTexto = () => {
    const n = eleccion.size;
    textoBoton.textContent = n === 0
      ? opcionesExtra.placeholder ?? "Seleccione una opcion"
      : n === 1
        ? (opciones.find((o) => o.valor === [...eleccion][0])?.etiqueta ?? "1 seleccionada")
        : `${n} seleccionadas`;
  };

  const construirMenu = () => {
    const m = el("div", { class: "desplegable-lista" });
    const filtrables = opcionesExtra.filtrable ?? opciones.length > 8;
    let filtro = "";

    if (filtrables) {
      const buscador = el("div", { class: "desplegable-buscador" });
      const entrada = el("input", { type: "search", placeholder: "Buscar..." });
      entrada.addEventListener("input", () => {
        filtro = entrada.value.trim().toLowerCase();
        pintar();
      });
      buscador.append(entrada);
      m.append(buscador);
    }

    const lista = el("div", {});
    m.append(lista);

    const pintar = () => {
      limpiar(lista);
      const visibles = filtro
        ? opciones.filter(
            (o) =>
              o.etiqueta.toLowerCase().includes(filtro) ||
              (o.detalle ?? "").toLowerCase().includes(filtro),
          )
        : opciones;
      if (!visibles.length) {
        lista.append(el("div", { class: "ayuda-texto", style: "padding:8px", text: "Sin coincidencias" }));
        return;
      }
      for (const o of visibles) {
        const marcada = eleccion.has(o.valor);
        const op = el("div", { class: `desplegable-opcion ${marcada ? "sel" : ""}`.trim() });
        op.append(el("span", { class: "marca", text: marcada ? "✓" : "" }));
        op.append(el("span", { text: o.etiqueta }));
        if (o.detalle) op.append(el("span", { class: "texto-pequeno texto-suave", text: ` ${o.detalle}` }));
        op.addEventListener("click", () => {
          if (eleccion.has(o.valor)) eleccion.delete(o.valor);
          else eleccion.add(o.valor);
          refrescarTexto();
          pintar();
          opcionesExtra.alCambiar?.([...eleccion]);
        });
        lista.append(op);
      }
    };
    pintar();
    return m;
  };

  const cerrarMenu = () => {
    menu?.remove();
    menu = null;
    abierto = false;
    document.removeEventListener("click", alClicFuera, true);
  };

  function alClicFuera(e: MouseEvent) {
    if (menu && !envoltura.contains(e.target as Node)) cerrarMenu();
  }

  botonAbrir.addEventListener("click", (e) => {
    e.stopPropagation();
    if (abierto) {
      cerrarMenu();
      return;
    }
    abierto = true;
    menu = construirMenu();
    envoltura.append(menu);
    document.addEventListener("click", alClicFuera, true);
  });

  refrescarTexto();
  envoltura.append(botonAbrir);
  const chips = el("div", { class: "lista-multi" });
  envoltura.append(chips);

  const refrescarChips = () => {
    limpiar(chips);
    for (const v of eleccion) {
      const chip = el("span", { class: "chip" });
      chip.append(document.createTextNode(v.length > 26 ? `${v.slice(0, 26)}…` : v));
      const x = el("button", { type: "button", text: "×", title: "Quitar" });
      x.addEventListener("click", () => {
        eleccion.delete(v);
        refrescarTexto();
        refrescarChips();
        if (menu) {
          menu.remove();
          menu = construirMenu();
          envoltura.append(menu);
        }
        opcionesExtra.alCambiar?.([...eleccion]);
      });
      chip.append(x);
      chips.append(chip);
    }
  };
  refrescarChips();

  return {
    envoltura,
    obtener: () => [...eleccion],
    limpiar: () => {
      eleccion.clear();
      refrescarTexto();
      refrescarChips();
    },
  };
}

/** Desplegable simple en linea con la interfaz estandar. */
export function desplegableSimple(
  opciones: { valor: string; etiqueta: string }[],
  valor = "",
  alCambiar?: (valor: string) => void,
): HTMLSelectElement {
  const s = select(opciones, valor);
  if (alCambiar) s.addEventListener("change", () => alCambiar(s.value));
  return s;
}

// ---------------------------------------------------------------------
// Descarga de ficheros
// ---------------------------------------------------------------------

/** Convierte base64 en un `Uint8Array`. */
export function deBase64(base64: string): Uint8Array {
  const limpio = base64.includes(",") ? (base64.split(",").pop() ?? base64) : base64;
  const bin = atob(limpio);
  const bytes = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i += 1) bytes[i] = bin.charCodeAt(i);
  return bytes;
}

/** Descarga un contenido en base64 con el nombre indicado. */
export function descargar(nombre: string, base64: string, mime: string): void {
  const bytes = deBase64(base64);
  const buffer = new Uint8Array(bytes);
  const url = URL.createObjectURL(new Blob([buffer.buffer as ArrayBuffer], { type: mime }));
  const a = el("a", { href: url, download: nombre });
  document.body.append(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 4000);
}

/** Abre una ventana con el documento listo para imprimir. */
export function imprimirHtml(html: string, titulo: string): void {
  const v = window.open("", "_blank", "width=1000,height=760");
  if (!v) {
    avisar(
      "El navegador ha bloqueado la ventana de impresion. Permitela e intentelo de nuevo.",
      "aviso",
    );
    return;
  }
  v.document.open();
  v.document.write(html);
  v.document.close();
  v.document.title = titulo;
  setTimeout(() => {
    v.focus();
    v.print();
  }, 320);
}

// ---------------------------------------------------------------------
// Barra de informacion
// ---------------------------------------------------------------------

/** Barra de progreso de una zona con acciones. */
export function barraInformacion(texto: string, acciones: HTMLElement[] = []): HTMLElement {
  const barra = el("div", { class: "tarjeta-pie" });
  barra.append(el("span", { text: texto }));
  if (acciones.length) {
    const derecha = el("div", { class: "fila", style: "margin-left:auto" });
    derecha.append(...acciones);
    barra.append(derecha);
  }
  return barra;
}

/** Panel de estado vacio con icono. */
export function vacio(titulo: string, detalle: string, iconoNombre = "info"): HTMLElement {
  return el(
    "div",
    { class: "vacio-estado" },
    `${icono(iconoNombre)}<div class="titulo">${esc(titulo)}</div><div>${esc(detalle)}</div>`,
  );
}

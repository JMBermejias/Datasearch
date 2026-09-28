// Copyright (c) 2026 Jose Manuel Bernabeu Mejias - Licencia MIT
// Pagina del panel de control (dashboard) y generador de tableros.
//
// El usuario puede pedir el tablero con un desplegable de esquemas, con una
// caja de texto en lenguaje natural, o con ambas cosas a la vez.

import {
  api,
  type ImagenDocumento,
  type PeticionDashboard,
  type Tablero,
  type WidgetListo,
} from "../api";
import { construirDocumento, estado } from "../estado";
import { dibujar, aPngBase64, lienzo, leyenda } from "../graficos";
import { icono } from "../iconos";
import {
  avisar,
  barraInformacion,
  boton,
  campo,
  comoError,
  desplegableMultiple,
  el,
  esc,
  esconderCarga,
  mostrarCarga,
  fecha,
  modal,
  numero,
  tablaDatos,
  textoCarga,
  vacio,
  descargar,
  tamano,
  imprimirHtml,
} from "../ui";

const ESTADO: {
  fuentesSel: number[];
  esquemasSel: string[];
  nombre: string;
  instruccion: string;
  graficosBase: string[];
  maxWidgets: number;
  autorizado: boolean;
} = {
  fuentesSel: [],
  esquemasSel: [],
  nombre: "",
  instruccion: "",
  graficosBase: ["kpi", "barra", "linea", "tabla"],
  maxWidgets: 6,
  autorizado: false,
};

/** Widgets renderizados, para poder capturarlos como imagen. */
let widgetsRenderizados: { widget: WidgetListo; imagen: ImagenDocumento }[] = [];

/** Graficos disponibles en el desplegable. */
const GRAFICOS = [
  { valor: "kpi", etiqueta: "Cifra destacada (KPI)" },
  { valor: "barra", etiqueta: "Grafico de barras" },
  { valor: "linea", etiqueta: "Grafico de lineas" },
  { valor: "area", etiqueta: "Grafico de areas" },
  { valor: "torta", etiqueta: "Grafico de torta" },
  { valor: "tabla", etiqueta: "Tabla de datos" },
  { valor: "indicador", etiqueta: "Indicador de progreso" },
  { valor: "dispersion", etiqueta: "Grafico de dispersion" },
];

/** Pinta la pagina del panel de control. */
export function paginaTablero(contenedor: HTMLElement): void {
  const tarjeta = el("div", { class: "tarjeta" });
  const cab = el("div", { class: "tarjeta-cabecera" });
  cab.insertAdjacentHTML("beforeend", icono("graficos"));
  cab.append(el("h2", { text: "Generador de panel de control" }));
  tarjeta.append(cab);

  const cuerpo = el("div", { class: "tarjeta-cuerpo" });

  if (!estado.fuentes.filter((f) => f.activa).length) {
    cuerpo.append(
      vacio(
        "No hay fuentes de datos activas",
        "Anada al menos una fuente antes de generar un tablero.",
        "base",
      ),
    );
    tarjeta.append(cuerpo);
    contenedor.append(tarjeta);
    return;
  }

  // --- Desplegable de fuentes y esquemas ---
  const selFuentes = desplegableMultiple(
    estado.fuentes
      .filter((f) => f.activa)
      .map((f) => ({ valor: String(f.id), etiqueta: f.nombre })),
    ESTADO.fuentesSel.map(String),
    {
      placeholder: "Todas las fuentes activas",
      alCambiar: (v) => {
        ESTADO.fuentesSel = v.map(Number);
        ESTADO.esquemasSel = [];
        pintarEsquemas();
      },
    },
  );

  const contenedorEsquemas = el("div", {});
  const rejilla = el("div", { class: "rejilla dos" });
  rejilla.append(campo("Fuentes de datos", selFuentes.envoltura));
  rejilla.append(campo("Esquemas (tablas o colecciones)", contenedorEsquemas));
  cuerpo.append(rejilla);

  // --- Caja de texto en lenguaje natural ---
  const instruccion = el("textarea", {
    placeholder:
      "Describa que quiere ver, por ejemplo: ventas por zona en torta, evolucion mensual del importe, comparativa por region",
  });
  instruccion.value = ESTADO.instruccion;
  instruccion.addEventListener("input", () => {
    ESTADO.instruccion = instruccion.value;
  });
  const ejemplos = el("div", { class: "lista-multi" });
  for (const e of [
    "ventas por zona en torta",
    "evolucion mensual del importe",
    "comparativa por region",
    "total de importes",
  ]) {
    const chip = el("span", { class: "chip", text: e });
    chip.style.cursor = "pointer";
    chip.addEventListener("click", () => {
      instruccion.value = e;
      ESTADO.instruccion = e;
    });
    ejemplos.append(chip);
  }
  const campoTexto = campo("Que quiere que le muestre", instruccion, "opcional si elige los graficos");
  campoTexto.append(ejemplos);
  cuerpo.append(el("div", { class: "separador" }));
  cuerpo.append(campoTexto);

  // --- Graficos base ---
  const selGraficos = el("div", { class: "btn-grupo" });
  for (const g of GRAFICOS) {
    const activo = ESTADO.graficosBase.includes(g.valor);
    const b = boton(g.etiqueta, {
      clase: `pequeno ${activo ? "principal" : ""}`.trim(),
      alPulsar: () => {
        const i = ESTADO.graficosBase.indexOf(g.valor);
        if (i >= 0) ESTADO.graficosBase.splice(i, 1);
        else ESTADO.graficosBase.push(g.valor);
        paginaTablero(contenedor);
      },
    });
    selGraficos.append(b);
  }
  const bloqueGraficos = el("div", {});
  bloqueGraficos.append(el("div", { class: "ayuda-texto", style: "margin-bottom:6px", text: "Graficos del panel principal (se anaden siempre):" }));
  bloqueGraficos.append(selGraficos);
  cuerpo.append(el("div", { class: "separador" }));
  cuerpo.append(bloqueGraficos);

  const nombre = el("input", { type: "text", placeholder: "Panel de ventas", value: ESTADO.nombre });
  nombre.addEventListener("input", () => {
    ESTADO.nombre = nombre.value;
  });
  const maximo = el("input", { type: "number", min: "1", max: "12", value: String(ESTADO.maxWidgets) });
  maximo.addEventListener("change", () => {
    ESTADO.maxWidgets = Math.max(1, Math.min(12, Number(maximo.value) || 6));
  });
  const rej = el("div", { class: "rejilla dos" });
  rej.append(campo("Nombre del tablero", nombre));
  rej.append(campo("Maximo de graficos a generar", maximo));
  cuerpo.append(rej);

  const acciones = el("div", { class: "btn-grupo", style: "margin-top:14px" });
  acciones.append(
    boton("Generar panel de control", {
      clase: "principal",
      iconoNombre: "graficos",
      alPulsar: () => void generar(contenedor),
    }),
  );
  if (estado.tablero) {
    acciones.append(
      boton("Exportar", {
        clase: "fantasma",
        iconoNombre: "descargar",
        alPulsar: () => void exportarTablero(contenedor),
      }),
    );
    acciones.append(
      boton("Imprimir", {
        clase: "fantasma",
        iconoNombre: "imprimir",
        alPulsar: () => void imprimirTablero(),
      }),
    );
  }
  cuerpo.append(acciones);

  tarjeta.append(cuerpo);
  tarjeta.append(
    barraInformacion(
      "Pulse «Generar panel de control» para crear el tablero. Puede pedirlo con la caja de texto, con el desplegable de esquemas, o combinando ambos.",
    ),
  );
  contenedor.append(tarjeta);

  const zona = el("div", { id: "zona-tablero" });
  contenedor.append(zona);

  function pintarEsquemas(): void {
    while (contenedorEsquemas.firstChild) contenedorEsquemas.removeChild(contenedorEsquemas.firstChild);
    const ids = ESTADO.fuentesSel.length
      ? ESTADO.fuentesSel
      : estado.fuentes.filter((f) => f.activa).map((f) => f.id);
    const nombres = new Set<string>();
    for (const id of ids) {
      const fuente = estado.fuentes.find((f) => f.id === id);
      if (fuente) nombres.add(fuente.nombre);
    }
    if (!nombres.size) {
      contenedorEsquemas.append(
        el("div", { class: "ayuda-texto", text: "Seleccione una fuente para ver sus tablas." }),
      );
      return;
    }
    const sel = desplegableMultiple(
      [...nombres].sort().map((n) => ({ valor: n, etiqueta: n })),
      ESTADO.esquemasSel,
      { placeholder: "Todos los esquemas", alCambiar: (v) => (ESTADO.esquemasSel = v) },
    );
    contenedorEsquemas.append(sel.envoltura);
  }

  pintarEsquemas();
  pintarTablero(zona);
}

async function generar(contenedor: HTMLElement): Promise<void> {
  if (!estado.token) return;
  const peticion: PeticionDashboard = {
    nombre: ESTADO.nombre.trim(),
    fuentes: ESTADO.fuentesSel,
    esquemas: ESTADO.esquemasSel,
    instruccion: ESTADO.instruccion.trim(),
    widgetsBase: ESTADO.graficosBase,
    maxWidgets: ESTADO.maxWidgets,
    filtros: [],
  };
  mostrarCarga("Generando el panel de control...");
  textoCarga("Consultando los datos de las fuentes...");
  try {
    const tablero = await api.crearDashboard(estado.token, peticion);
    estado.tablero = tablero;
    estado.documento = {
      titulo: tablero.nombre,
      subtitulo: `Generado el ${fecha(tablero.generadoEn)}`,
      pie: `Fuentes: ${tablero.fuentes.join(", ")}`,
    };
    if (tablero.advertencias.length) {
      avisar(`Algunos graficos no se pudieron crear: ${tablero.advertencias.join(" | ")}`, "aviso", 9000);
    }
    const zona = contenedor.querySelector<HTMLElement>("#zona-tablero");
    if (zona) {
      pintarTablero(zona);
      zona.scrollIntoView({ behavior: "smooth", block: "start" });
    }
    avisar(`Tablero generado con ${tablero.widgets.length} graficos.`, "exito", 5000);
  } catch (e) {
    avisar(comoError(e).message, "error", 10000);
  } finally {
    esconderCarga();
  }
}

/** Pinta el tablero generado. */
function pintarTablero(zona: HTMLElement): void {
  while (zona.firstChild) zona.removeChild(zona.firstChild);
  widgetsRenderizados = [];
  const tablero: Tablero | null = estado.tablero;
  if (!tablero) {
    zona.append(
      vacio(
        "Todavia no hay ningun tablero",
        "Describa arriba que desea ver y pulse «Generar panel de control».",
        "graficos",
      ),
    );
    return;
  }

  const cab = el("div", { class: "tarjeta-cabecera", style: "border:1px solid #bae6fd;border-radius:10px 10px 0 0" });
  cab.insertAdjacentHTML("beforeend", icono("graficos"));
  cab.append(el("h2", { text: tablero.nombre }));
  const acciones = el("div", { class: "acciones" });
  acciones.append(
    boton("Actualizar", {
      clase: "pequeno",
      iconoNombre: "refrescar",
      alPulsar: () => {
        if (ESTADO.instruccion.trim()) generar(zona.parentElement ?? zona);
      },
    }),
    boton("Exportar", { clase: "pequeno", iconoNombre: "descargar", alPulsar: () => void exportarZona(zona) }),
    boton("Imprimir", { clase: "pequeno", iconoNombre: "imprimir", alPulsar: () => void imprimirTablero() }),
  );
  cab.append(acciones);
  zona.append(cab);
  zona.append(
    barraInformacion(
      `${tablero.widgets.length} graficos | generado en ${tablero.duracionMs} ms | ${fecha(tablero.generadoEn)}`,
    ),
  );

  const rejilla = el("div", { class: "rejilla-widget" });
  for (const w of tablero.widgets) {
    rejilla.append(tarjetaWidget(w));
  }
  zona.append(rejilla);

  if (tablero.advertencias.length) {
    const avisos = el("div", { class: "caja-info", style: "border-color:#fde68a" });
    avisos.innerHTML = `<strong>Advertencias</strong><br>${tablero.advertencias.map((a) => esc(a)).join("<br>")}`;
    zona.append(avisos);
  }
}

function tarjetaWidget(w: WidgetListo): HTMLElement {
  const art = el("div", { class: `widget ancho-${w.ancho}` });
  const cab = el("div", { class: "widget-cabecera" });
  cab.append(el("div", { class: "titulo", text: w.titulo }));
  if (w.subtitulo) cab.append(el("div", { class: "sub", text: w.subtitulo }));
  const acciones = el("div", { class: "fila", style: "gap:4px;margin-top:4px" });
  acciones.append(
    boton("", {
      clase: "fantasma pequeno icono",
      iconoNombre: "refrescar",
      titulo: "Recalcular este grafico",
      alPulsar: async () => {
        try {
          const nuevo = await api.recalcularWidget(estado.token!, {
            id: w.id,
            titulo: w.titulo,
            tipo: w.tipo,
            fuenteId: 0,
            esquema: w.esquema,
            columnaGrupo: w.columnaGrupo,
            columnaValor: w.columnaValor,
            agregacion: w.agregacion,
            limite: 15,
            filtros: [],
            ancho: w.ancho,
            formato: w.formato,
            peticionTexto: "",
          });
          const i = estado.tablero!.widgets.findIndex((x) => x.id === w.id);
          if (i >= 0) estado.tablero!.widgets[i] = nuevo;
          const zona = document.querySelector("#zona-tablero");
          if (zona) pintarTablero(zona as HTMLElement);
        } catch (e) {
          avisar(comoError(e).message, "error");
        }
      },
    }),
    boton("", {
      clase: "fantasma pequeno icono",
      iconoNombre: "descargar",
      titulo: "Exportar este grafico como imagen",
      alPulsar: () => {
        const base64 = aPngBase64({
          tipo: w.tipo,
          puntos: w.puntos,
          formato: w.formato,
          valorDestacado: w.valorDestacado,
        });
        if (!base64) {
          avisar("No se ha podido generar la imagen del grafico.", "error");
          return;
        }
        
      },
    }),
  );
  cab.append(acciones);
  art.append(cab);

  const cuerpo = el("div", { class: "widget-cuerpo" });
  if (w.estado) {
    cuerpo.append(vacio(w.estado, "Pruebe con otras columnas o menos filtros.", "info"));
  } else if (w.tipo === "tabla") {
    const tipos = new Map(w.columnas.map((c) => [c.nombre, c]));
    cuerpo.append(
      tablaDatos(w.filas, {
        tipos,
        filasPorPagina: 8,
        columnas: w.columnas.map((c) => c.nombre).slice(0, 5),
      }),
    );
  } else if (w.tipo === "kpi") {
    cuerpo.innerHTML = `<div class="kpi-valor">${
      w.valorDestacado !== null ? numero(w.valorDestacado) : "-"
    }</div><div class="kpi-etiqueta">${esc(w.columnaValor || w.titulo)}${
      w.agregacion ? ` &middot; ${esc(w.agregacion)}` : ""
    }<br>${numero(w.totalFilas)} registros</div>`;
  } else {
    const canvas = lienzo();
    cuerpo.append(canvas);
    // El dibujado se hace tras insertar el nodo para conocer su ancho.
    requestAnimationFrame(() => {
      dibujar(canvas, {
        tipo: w.tipo,
        puntos: w.puntos,
        formato: w.formato,
        valorDestacado: w.valorDestacado,
      });
      if (w.tipo === "torta" && w.puntos.length) {
        const ly = el("div", { class: "leyenda", html: leyenda(w.puntos) });
        cuerpo.append(ly);
      }
    });
  }
  art.append(cuerpo);
  return art;
}

/**
 * Captura todos los graficos del tablero en PNG.
 *
 * Lo usa la exportacion a PDF y a Excel para incrustar las mismas imagenes
 * que se ven en pantalla.
 */
export function capturarTablero(): ImagenDocumento[] {
  return widgetsRenderizados.map((w) => w.imagen);
}

/** Vuelve a renderizar el tablero y guarda las imagenes para exportar. */
export function prepararImagenes(): ImagenDocumento[] {
  const tablero = estado.tablero;
  if (!tablero) return [];
  const salida: ImagenDocumento[] = [];
  for (const w of tablero.widgets) {
    if (w.tipo === "tabla" && w.filas.length) {
      // Los KPIs y graficos de barras si se pueden exportar como imagen.
      continue;
    }
    const base64 = aPngBase64({
      tipo: w.tipo,
      puntos: w.puntos,
      formato: w.formato,
      valorDestacado: w.valorDestacado,
    });
    if (base64) salida.push({ titulo: w.titulo, base64, anchoPx: 720, altoPx: 360 });
  }
  return salida;
}

async function exportarZona(zona: HTMLElement): Promise<void> {
  try {
    const documento = await import("../estado").then((m) => m.construirDocumento());
    const cuerpo = el("div", { class: "columna" });
    cuerpo.append(el("div", { class: "ayuda-texto", text: "Elija el formato de salida del tablero." }));
    const sel = el("select", {});
    for (const f of estado.info?.formatos ?? []) sel.append(el("option", { value: f.valor, text: f.etiqueta }));
    sel.value = "excel";
    cuerpo.append(campo("Formato", sel));
    void zona;
    modal("Exportar tablero", cuerpo, {
      iconoNombre: "descargar",
      pie: (cerrar) => [
        boton("Cancelar", { clase: "fantasma", alPulsar: cerrar }),
        boton("Generar", {
          clase: "principal",
          alPulsar: async () => {
            try {
              const archivo = await api.exportar(estado.token!, documento, sel.value);
                descargar(archivo.nombre, archivo.contenidoBase64, archivo.mime);
              avisar(`Fichero ${archivo.nombre} (${tamano(archivo.tamanoBytes)}).`, "exito");
              cerrar();
            } catch (e) {
              avisar(comoError(e).message, "error", 9000);
            }
          },
        }),
      ],
    });
  } catch (e) {
    avisar(comoError(e).message, "error");
  }
}

async function exportarTablero(contenedor: HTMLElement): Promise<void> {
  const zona = contenedor.querySelector("#zona-tablero") as HTMLElement | null;
  if (zona) await exportarZona(zona);
}

async function imprimirTablero(): Promise<void> {
  try {
        const documento = await construirDocumento();
    const html = await api.prepararImpresion(estado.token!, documento);
    imprimirHtml(html, documento.titulo);
  } catch (e) {
    avisar(comoError(e).message, "error", 9000);
  }
}

// Copyright (c) 2026 Jose Manuel Bernabeu Mejias - Licencia MIT
// Pagina de analisis. El usuario elige que analisis quiere y lo ejecuta
// pulsando el boton de confirmacion, tal y como se exige.

import {
  api,
  type Esquema,
  type PeticionAnalisis,
  type RespuestaAnalisis,
  type SeccionAnalisis,
} from "../api";
import { construirDocumento, estado } from "../estado";
import { icono } from "../iconos";
import {
  avisar,
  barraInformacion,
  boton,
  campo,
  comoError,
  el,
  esc,
  esconderCarga,
  mostrarCarga,
  fecha,
  modal,
  numero,
  tablaDatos,
  textoCarga,
  descargar,
  tamano,
  imprimirHtml,
  vacio,
} from "../ui";

interface VistaAnalisis {
  fuenteId: number;
  esquema: string;
  columnas: string[];
  analyses: string[];
  limite: number;
  comentario: string;
}

const vista: VistaAnalisis = {
  fuenteId: 0,
  esquema: "",
  columnas: [],
  analyses: ["resumen"],
  limite: 10000,
  comentario: "",
};

let esquemasDisponibles: Esquema[] = [];
let ultimaRespuesta: RespuestaAnalisis | null = null;

/** Pinta la pagina de analisis. */
export function paginaAnalisis(contenedor: HTMLElement): void {
  const activas = estado.fuentes.filter((f) => f.activa);
  if (!activas.length) {
    contenedor.append(
      vacio(
        "No hay fuentes de datos para analizar",
        "Anada una fuente de datos antes de solicitar un analisis.",
        "analisis",
      ),
    );
    return;
  }
  if (!vista.fuenteId || !activas.some((f) => f.id === vista.fuenteId)) {
    vista.fuenteId = activas[0].id;
    vista.esquema = "";
    esquemasDisponibles = [];
  }

  const tarjeta = el("div", { class: "tarjeta" });
  const cab = el("div", { class: "tarjeta-cabecera" });
  cab.insertAdjacentHTML("beforeend", icono("analisis"));
  cab.append(el("h2", { text: "Analisis de datos" }));
  tarjeta.append(cab);

  const cuerpo = el("div", { class: "tarjeta-cuerpo" });
  tarjeta.append(cuerpo);

  // --- Fuente y esquema ---
  const selFuente = el("select", {});
  for (const f of activas) {
    selFuente.append(el("option", { value: String(f.id), text: f.nombre, selected: f.id === vista.fuenteId }));
  }
  selFuente.addEventListener("change", async () => {
    vista.fuenteId = Number(selFuente.value);
    vista.esquema = "";
    esquemasDisponibles = [];
    await cargarEsquemas();
    paginaAnalisis(contenedor);
  });

  const selEsquema = el("select", {});
  selEsquema.addEventListener("change", () => {
    vista.esquema = selEsquema.value;
  });
  pintarOpcionesEsquema(selEsquema);

  const rejilla = el("div", { class: "rejilla dos" });
  rejilla.append(campo("Fuente de datos", selFuente));
  rejilla.append(campo("Esquema (tabla o coleccion)", selEsquema));
  cuerpo.append(rejilla);

  // --- Analisis disponibles ---
  cuerpo.append(el("div", { class: "separador" }));
  const opciones = estado.info?.analisis ?? [];
  const rejillaAnalisis = el("div", { class: "rejilla tres" });
  for (const o of opciones) {
    const marcada = vista.analyses.includes(o.valor);
    const caja = el("label", { class: "campo-checkbox" });
    const casilla = el("input", { type: "checkbox" });
    casilla.checked = marcada;
    casilla.addEventListener("change", () => {
      if (casilla.checked) {
        if (!vista.analyses.includes(o.valor)) vista.analyses.push(o.valor);
      } else {
        vista.analyses = vista.analyses.filter((a) => a !== o.valor);
      }
    });
    caja.append(casilla, el("span", { text: o.etiqueta }));
    rejillaAnalisis.append(caja);
  }
  cuerpo.append(el("div", { class: "ayuda-texto", style: "margin-bottom:6px", text: "Analisis que desea ejecutar:" }));
  cuerpo.append(rejillaAnalisis);

  const limite = el("input", { type: "number", min: "100", max: "100000", value: String(vista.limite) });
  limite.addEventListener("change", () => {
    vista.limite = Math.max(100, Math.min(100000, Number(limite.value) || 10000));
  });
  const comentario = el("input", { type: "text", placeholder: "Ejemplo: revision de ventas del primer trimestre" });
  comentario.value = vista.comentario;
  comentario.addEventListener("input", () => {
    vista.comentario = comentario.value;
  });

  const rejilla2 = el("div", { class: "rejilla dos" });
  rejilla2.append(campo("Filas a analizar", limite, "maximo 100.000"));
  rejilla2.append(campo("Comentario", comentario, "opcional"));
  cuerpo.append(rejilla2);

  // --- Confirmacion ---
  cuerpo.append(el("div", { class: "separador" }));
  const cajaConf = el("div", { class: "caja-info" });
  cajaConf.innerHTML = `
    <strong>Se requiere confirmacion</strong><br>
    El analisis se ejecuta sobre los datos de la fuente seleccionada. Puede modificar el
    origen de los datos en la pagina <em>Buscar</em>. Pulse el boton para confirmar y ejecutar.`;
  cuerpo.append(cajaConf);

  const acciones = el("div", { class: "btn-grupo", style: "margin-top:12px" });
  acciones.append(
    boton("Confirmar y analizar", {
      clase: "principal",
      iconoNombre: "exito",
      alPulsar: () => confirmarYAnalizar(contenedor),
    }),
  );
  acciones.append(
    boton("Ver que hace cada analisis", {
      clase: "fantasma",
      iconoNombre: "info",
      alPulsar: () => mostrarAyuda(opciones),
    }),
  );
  cuerpo.append(acciones);

  tarjeta.append(
    barraInformacion(
      `Fuente: ${activas.find((f) => f.id === vista.fuenteId)?.nombre ?? "-"} | Esquema: ${vista.esquema || "todos"}`,
    ),
  );
  contenedor.append(tarjeta);

  const zona = el("div", { id: "zona-analisis" });
  contenedor.append(zona);
  pintarResultado(zona);

  void cargarEsquemas();
}

function pintarOpcionesEsquema(sel: HTMLSelectElement): void {
  while (sel.firstChild) sel.removeChild(sel.firstChild);
  sel.append(el("option", { value: "", text: "Todos los esquemas de la fuente", selected: !vista.esquema }));
  for (const e of esquemasDisponibles) {
    sel.append(el("option", { value: e.nombre, text: `${e.nombre} (${e.columnas.length} columnas)`, selected: e.nombre === vista.esquema }));
  }
  if (!esquemasDisponibles.length) {
    sel.append(el("option", { value: "", text: "(cargando esquemas...)" }));
  }
}

async function cargarEsquemas(): Promise<void> {
  if (!estado.token || !vista.fuenteId) return;
  try {
    esquemasDisponibles = await api.esquemasDeFuente(estado.token, vista.fuenteId);
  } catch {
    esquemasDisponibles = [];
  }
  const zona = document.querySelector("#zona-analisis");
  void zona;
}

const AYUDA_ANALISIS: Record<string, string> = {
  resumen: "Combina la descripcion estadistica, los nulos y las frecuencias. Buen punto de partida.",
  describir: "Minimo, maximo, media, mediana, desviacion, cuartiles y asimetria de cada columna.",
  nulos: "Cuantas filas carecen de valor en cada columna, con su porcentaje.",
  distintos: "Cuantas combinaciones diferentes hay en cada columna y si puede servir de clave.",
  frecuencias: "Los 15 valores mas repetidos de cada columna de texto.",
  histograma: "Reparto de cada columna numerica por cuartiles.",
  correlacion: "Correlacion de Pearson entre todas las columnas numericas.",
  outliers: "Valores que se apartan mas de tres desviaciones tipicas de la media.",
  tendencia: "Regresion lineal que muestra si la serie sube o baja.",
  calidad: "Puntuacion de calidad segun los valores ausentes y los valores sin dato.",
  duplicados: "Filas cuya combinacion de valores ya aparece en otra fila.",
};

/** Muestra la ayuda de los analisis disponibles. */
function mostrarAyuda(opciones: { valor: string; etiqueta: string }[]): void {
  const cuerpo = el("div", { class: "columna" });
  for (const o of opciones) {
    const fila = el("div", { class: "dato-ficha" });
    fila.innerHTML = `<div class="clave">${esc(o.etiqueta)}</div>
      <div class="valor" style="font-weight:400;font-size:12.5px">${esc(
        AYUDA_ANALISIS[o.valor] ?? "",
      )}</div>`;
    cuerpo.append(fila);
  }
  modal("Que hace cada analisis", cuerpo, {
    iconoNombre: "info",
    pie: (cerrar) => [boton("Cerrar", { clase: "principal", alPulsar: cerrar })],
  });
}

/** Pide confirmacion explicita y ejecuta el analisis. */
async function confirmarYAnalizar(contenedor: HTMLElement): Promise<void> {
  if (!estado.token) return;
  if (!vista.analyses.length) {
    avisar("Seleccione al menos un tipo de analisis.", "aviso", 5000);
    return;
  }

  const ejecutar = async () => {
    const peticion: PeticionAnalisis = {
      fuenteId: vista.fuenteId,
      esquema: vista.esquema,
      columnas: vista.columnas,
      analyses: vista.analyses,
      esAnalisisConfirmado: true,
      filtros: [],
      limite: vista.limite,
      comentario: vista.comentario,
    };
    mostrarCarga("Ejecutando el analisis...");
    textoCarga("Calculando indicadores...");
    try {
      const r = await api.analizar(estado.token!, peticion);
      ultimaRespuesta = r;
      estado.documento = {
        titulo: `Analisis - ${r.fuenteNombre}`,
        subtitulo: r.resumen,
        pie: `Esquema: ${r.esquema || "todos"} | ${r.filasAnalizadas} filas`,
      };
      paginaAnalisis(contenedor);
      const zona = contenedor.querySelector("#zona-analisis");
      zona?.scrollIntoView({ behavior: "smooth", block: "start" });
      avisar(r.resumen, "exito", 8000);
    } catch (e) {
      avisar(comoError(e).message, "error", 10000);
    } finally {
      esconderCarga();
    }
  };

  modal(
    "Confirmar el analisis",
    el("div", {
      class: "ayuda-texto",
      html: `Va a ejecutar <strong>${vista.analyses.length}</strong> analisis sobre la fuente
             <strong>${esc(estado.fuentes.find((f) => f.id === vista.fuenteId)?.nombre ?? "-")}</strong>
             ${vista.esquema ? `y el esquema <strong>${esc(vista.esquema)}</strong>` : ""},
             utilizando hasta <strong>${numero(vista.limite)}</strong> filas.<br><br>
             ¿Desea continuar?`,
    }),
    {
      clase: "estrecho",
      iconoNombre: "analisis",
      pie: (cerrar) => [
        boton("Cancelar", { clase: "fantasma", alPulsar: cerrar }),
        boton("Confirmar y analizar", {
          clase: "principal",
          iconoNombre: "exito",
          alPulsar: async () => {
            cerrar();
            await ejecutar();
          },
        }),
      ],
    },
  );
}

function pintarResultado(zona: HTMLElement): void {
  while (zona.firstChild) zona.removeChild(zona.firstChild);
  const r = ultimaRespuesta;
  if (!r) {
    zona.append(
      vacio(
        "Todavia no hay analisis",
        "Seleccione los analisis que necesite y pulse «Confirmar y analizar».",
        "analisis",
      ),
    );
    return;
  }

  const cab = el("div", { class: "tarjeta-cabecera", style: "border:1px solid #bae6fd;border-radius:10px 10px 0 0" });
  cab.insertAdjacentHTML("beforeend", icono("analisis"));
  cab.append(el("h2", { text: "Resultado del analisis" }));
  const acciones = el("div", { class: "acciones" });
  acciones.append(
    boton("Exportar", { clase: "pequeno", iconoNombre: "descargar", alPulsar: () => void exportar(r) }),
    boton("Imprimir", { clase: "pequeno", iconoNombre: "imprimir", alPulsar: () => void imprimir(r) }),
  );
  cab.append(acciones);
  zona.append(cab);
  zona.append(
    barraInformacion(
      `${r.filasAnalizadas} filas analizadas | ${r.columnasAnalizadas.length} columnas | ${r.duracionMs} ms | ${fecha(new Date().toISOString())}`,
    ),
  );

  if (r.advertencias.length) {
    const av = el("div", { class: "caja-info", style: "border-color:#fde68a" });
    av.innerHTML = `<strong>Advertencias</strong><br>${r.advertencias.map((a) => esc(a)).join("<br>")}`;
    zona.append(av);
  }

  for (const s of r.secciones) {
    zona.append(tarjetaSeccion(s));
  }
}

function tarjetaSeccion(s: SeccionAnalisis): HTMLElement {
  const t = el("div", { class: "tarjeta" });
  const cab = el("div", { class: "tarjeta-cabecera" });
  cab.insertAdjacentHTML("beforeend", icono("capas"));
  cab.append(el("h3", { text: s.titulo }));
  t.append(cab);
  const cuerpo = el("div", { class: "tarjeta-cuerpo" });
  cuerpo.append(el("div", { class: "ayuda-texto", style: "margin-bottom:10px", text: s.descripcion }));

  if (s.metricas.length) {
    const rej = el("div", { class: "rejilla cuatro", style: "margin-bottom:12px" });
    for (const m of s.metricas.slice(0, 24)) {
      const d = el("div", { class: "dato-ficha" });
      d.innerHTML = `<div class="clave">${esc(m.nombre)}</div><div class="valor">${esc(m.valor)}${
        m.unidad ? ` ${esc(m.unidad)}` : ""
      }</div>`;
      rej.append(d);
    }
    cuerpo.append(rej);
  }

  if (s.filas.length) {
    const columnas = [...new Set(s.filas.flatMap((f) => Object.keys(f)))];
    cuerpo.append(tablaDatos(s.filas, { filasPorPagina: 40, columnas }));
  } else if (!s.metricas.length) {
    cuerpo.append(el("div", { class: "ayuda-texto", text: "Esta seccion no tiene datos que mostrar." }));
  }
  t.append(cuerpo);
  return t;
}

async function exportar(r: RespuestaAnalisis): Promise<void> {
  try {
    const documento = await construirDocumento(`Analisis - ${r.fuenteNombre}`, r.resumen, false);
    documento.secciones = r.secciones;
    const cuerpo = el("div", { class: "columna" });
    const sel = el("select", {});
    for (const f of estado.info?.formatos ?? []) sel.append(el("option", { value: f.valor, text: f.etiqueta }));
    sel.value = "excel";
    cuerpo.append(campo("Formato", sel));
    modal("Exportar analisis", cuerpo, {
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
              avisar(comoError(e).message, "error");
            }
          },
        }),
      ],
    });
  } catch (e) {
    avisar(comoError(e).message, "error");
  }
}

async function imprimir(r: RespuestaAnalisis): Promise<void> {
  try {
        const documento = await construirDocumento(`Analisis - ${r.fuenteNombre}`, r.resumen, false);
    documento.secciones = r.secciones;
    const html = await api.prepararImpresion(estado.token!, documento);
    imprimirHtml(html, `Analisis - ${r.fuenteNombre}`);
  } catch (e) {
    avisar(comoError(e).message, "error");
  }
}

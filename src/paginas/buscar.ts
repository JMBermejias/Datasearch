// Copyright (c) 2026 Jose Manuel Bernabeu Mejias - Licencia MIT
// Pagina de busqueda: seleccion de fuentes, buscador de texto, filtros
// desplegables y resultados.
//
// Nada se consulta hasta que el usuario pulsa el boton Buscar.

import {
  api,
  type ColumnaInfo,
  type Esquema,
  type Filtro,
  type Fuente,
  type PeticionBusqueda,
} from "../api";
import { construirDocumento, estado, prepararDocumento } from "../estado";
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
  modal,
  numero,
  tablaDatos,
  textoCarga,
  mostrarCarga,
  ocultarCarga,
  vacio,
  descargar,
  tamano,
  imprimirHtml,
} from "../ui";

// Estado local de la pagina, para conservar los filtros al navegar.
interface VistaBusqueda {
  fuentesSel: number[];
  esquemasSel: string[];
  texto: string;
  modo: "y" | "o";
  filtros: Filtro[];
  limite: number;
  ordenarPor: string;
  ordenDesc: boolean;
  bloqueActivo: number;
}

const vista: VistaBusqueda = {
  fuentesSel: [],
  esquemasSel: [],
  texto: "",
  modo: "y",
  filtros: [],
  limite: 200,
  ordenarPor: "",
  ordenDesc: false,
  bloqueActivo: 0,
};

/** Esquemas disponibles por fuente, cacheados mientras se esta en la pagina. */
const cacheEsquemas = new Map<number, Esquema[]>();

/** Columnas de los resultados actuales, para el desplegable de filtros. */
let columnasActuales: ColumnaInfo[] = [];

/** Pinta la pagina de busqueda. */
export function paginaBuscar(contenedor: HTMLElement): void {
  if (!estado.fuentes.length) {
    contenedor.append(
      vacio(
        "Todavia no hay fuentes de datos",
        "Vaya a «Fuentes de datos» y anada tantas fuentes como necesite: bases de datos locales, remotas o recursos de la web.",
        "base",
      ),
    );
    const acciones = boton("Ir a Fuentes de datos", {
      clase: "principal",
      iconoNombre: "anadir",
      alPulsar: () => window.dispatchEvent(new CustomEvent("datasearch:navegar", { detail: "fuentes" })),
    });
    acciones.style.margin = "0 auto";
    contenedor.lastElementChild?.append(acciones);
    return;
  }

  // ---------------------------------------------------------------
  // Panel de control
  // ---------------------------------------------------------------
  const panel = el("div", { class: "tarjeta" });
  const cab = el("div", { class: "tarjeta-cabecera" });
  cab.insertAdjacentHTML("beforeend", icono("buscar"));
  cab.append(el("h2", { text: "Panel de busqueda" }));
  const accionesCab = el("div", { class: "acciones" });
  accionesCab.append(
    boton("Restablecer", {
      clase: "fantasma pequeno",
      iconoNombre: "refrescar",
      alPulsar: () => {
        vista.fuentesSel = [];
        vista.esquemasSel = [];
        vista.texto = "";
        vista.filtros = [];
        vista.modo = "y";
        paginaBuscar(contenedor);
      },
    }),
  );
  cab.append(accionesCab);
  panel.append(cab);

  const cuerpo = el("div", { class: "tarjeta-cuerpo" });
  panel.append(cuerpo);

  // --- Fuentes ---
  const fuentesDisponibles = estado.fuentes.filter((f) => f.activa);
  const selFuentes = desplegableMultiple(
    fuentesDisponibles.map((f) => ({
      valor: String(f.id),
      etiqueta: f.nombre,
      detalle: `(${etiquetaTipo(f)})`,
    })),
    vista.fuentesSel.map(String),
    {
      placeholder: "Todas las fuentes activas",
      alCambiar: (vals) => {
        vista.fuentesSel = vals.map(Number);
        vista.esquemasSel = [];
        for (const id of vista.fuentesSel) void cargarEsquemas(id);
        pintarEsquemas();
      },
    },
  );

  const contenedorEsquemas = el("div", {});
  const contenedorFiltros = el("div", {});

  const rejilla = el("div", { class: "rejilla dos" });
  rejilla.append(campo("Fuentes de datos", selFuentes.envoltura));
  rejilla.append(campo("Esquemas (tablas o colecciones)", contenedorEsquemas));
  cuerpo.append(rejilla);

  const buscador = el("input", {
    type: "search",
    placeholder: 'Ejemplo: pais:ES "San Javier" ventas',
    value: vista.texto,
  });
  buscador.addEventListener("input", () => {
    vista.texto = buscador.value;
  });
  const modoSel = el("select", {});
  for (const [v, t] of [["y", "Y  (todas las condiciones)"], ["o", "O  (alguna condicion)"]]) {
    modoSel.append(el("option", { value: v, text: t, selected: vista.modo === v }));
  }
  modoSel.addEventListener("change", () => {
    vista.modo = modoSel.value as "y" | "o";
  });
  const limite = el("input", { type: "number", min: "1", max: "100000", value: String(vista.limite) });
  limite.addEventListener("change", () => {
    vista.limite = Math.max(1, Math.min(100000, Number(limite.value) || 200));
  });

  cuerpo.append(el("div", { class: "separador" }));
  const filaBuscador = el("div", { class: "rejilla tres" });
  filaBuscador.append(campo("Texto que debe contener", buscador, "admite campo:valor y comillas"));
  filaBuscador.append(campo("Combinacion de condiciones", modoSel));
  filaBuscador.append(campo("Maximo de filas", limite));
  cuerpo.append(filaBuscador);

  // --- Desplegable de filtrado ---
  cuerpo.append(el("div", { class: "separador" }));
  const cabFiltros = el("div", { class: "fila entre", style: "margin-bottom:8px" });
  const tituloFiltros = el("div", { class: "seccion-titulo", style: "margin:0;flex:1" });
  tituloFiltros.insertAdjacentHTML("beforeend", icono("filtros"));
  tituloFiltros.append(el("span", { text: "Filtros" }));
  cabFiltros.append(tituloFiltros);
  cabFiltros.append(
    boton("Anadir filtro", {
      clase: "pequeno",
      iconoNombre: "anadir",
      alPulsar: () => {
        if (!columnasActuales.length) {
          avisar(
            "Busque primero en alguna fuente para conocer sus columnas, o seleccione un esquema en el desplegable.",
            "aviso",
            6000,
          );
        }
        vista.filtros.push({
          campo: columnasActuales[0]?.nombre ?? "",
          operador: "contiene",
          valor: "",
          valor2: "",
          conector: vista.filtros.length ? "y" : "o",
          activo: true,
        });
        pintarFiltros();
      },
    }),
  );
  cuerpo.append(cabFiltros);
  cuerpo.append(contenedorFiltros);

  const sugerencias = el("div", { class: "caja-info oculto" });
  cuerpo.append(sugerencias);

  // --- Boton de buscar ---
  cuerpo.append(el("div", { class: "separador" }));
  const botones = el("div", { class: "btn-grupo" });
  const btnBuscar = boton("Buscar", {
    clase: "principal",
    iconoNombre: "buscar",
    alPulsar: () => void ejecutarBusqueda(contenedor),
  });
  botones.append(btnBuscar);
  botones.append(
    boton("Limpiar resultados", {
      clase: "fantasma",
      iconoNombre: "cerrar",
      alPulsar: () => {
        estado.resultados = [];
        estado.resultadosMeta = { totalFilas: 0, duracionMs: 0, errores: [], sugerencias: [] };
        pintarResultados(contenedor);
      },
    }),
  );
  cuerpo.append(botones);

  panel.append(
    el("div", {
      class: "tarjeta-pie",
      html: `Solo se leen datos al pulsar <strong>Buscar</strong>. Puede anadir tantas fuentes como
             necesite y aplicar todos los filtros que requiera.`,
    }),
  );
  contenedor.append(panel);

  // ---------------------------------------------------------------
  // Resultados
  // ---------------------------------------------------------------
  const zonaResultados = el("div", { id: "zona-resultados" });
  contenedor.append(zonaResultados);

  function pintarEsquemas(): void {
    while (contenedorEsquemas.firstChild) contenedorEsquemas.removeChild(contenedorEsquemas.firstChild);
    const ids = vista.fuentesSel.length ? vista.fuentesSel : fuentesDisponibles.map((f) => f.id);
    const nombres = new Set<string>();
    for (const id of ids) {
      for (const e of cacheEsquemas.get(id) ?? []) nombres.add(e.nombre);
    }
    if (!nombres.size) {
      contenedorEsquemas.append(
        el("div", { class: "ayuda-texto", text: "Seleccione una fuente para ver sus tablas. Si no elige ninguna, se consultaran todas." }),
      );
      return;
    }
    const sel = desplegableMultiple(
      [...nombres].sort().map((n) => ({ valor: n, etiqueta: n })),
      vista.esquemasSel,
      {
        placeholder: "Todos los esquemas",
        alCambiar: (vals) => {
          vista.esquemasSel = vals;
        },
      },
    );
    contenedorEsquemas.append(sel.envoltura);
  }

  function pintarFiltros(): void {
    while (contenedorFiltros.firstChild) contenedorFiltros.removeChild(contenedorFiltros.firstChild);
    if (!vista.filtros.length) {
      contenedorFiltros.append(
        el("div", {
          class: "ayuda-texto",
          html: `Todavia no hay filtros. Use <strong>Anadir filtro</strong> para combinar las
                 condiciones que necesita sobre cualquier columna.`,
        }),
      );
      return;
    }
    vista.filtros.forEach((f, indice) => {
      const fila = el("div", { class: `filtro-fila ${f.activo ? "" : "inactivo"}`.trim() });

      // Conector
      const conector = el("select", { class: "conector-selector" });
      for (const [v, t] of [["y", "Y"], ["o", "O"]]) {
        conector.append(el("option", { value: v, text: t, selected: f.conector === v }));
      }
      conector.addEventListener("change", () => {
        f.conector = conector.value;
      });
      if (indice > 0) fila.append(conector);

      // Columna
      const columna = el("select", {});
      if (!columnasActuales.length) {
        columna.append(el("option", { value: "", text: "columna (busque primero)" }));
      } else {
        for (const c of columnasActuales) {
          const op = el("option", {
            value: c.nombre,
            text: `${c.nombre}  (${c.tipo})`,
            selected: c.nombre === f.campo,
          });
          columna.append(op);
        }
      }
      columna.addEventListener("change", () => {
        f.campo = columna.value;
        pintarFiltros();
      });
      fila.append(campo("Columna", columna));

      // Operador
      const operadores = (estado.info?.operadores ?? []).map((o) => ({ ...o }));
      const opSel = el("select", {});
      for (const o of operadores) {
        opSel.append(el("option", { value: o.valor, text: o.etiqueta, selected: f.operador === o.valor }));
      }
      opSel.addEventListener("change", () => {
        f.operador = opSel.value;
        pintarFiltros();
      });
      fila.append(campo("Condicion", opSel));

      // Valor
      const meta = operadores.find((o) => o.valor === f.operador);
      const grupoValor = el("div", { class: "rejilla dos" });
      if (meta && !meta.unario) {
        const entrada = el("input", {
          type: meta.necesitaLista ? "text" : "text",
          value: f.valor,
          placeholder: meta.necesitaLista ? "valor1, valor2, valor3" : "valor",
        });
        entrada.addEventListener("input", () => {
          f.valor = entrada.value;
        });
        grupoValor.append(campo("Valor", entrada));
        if (meta.necesitaSegundo) {
          const v2 = el("input", { type: "text", value: f.valor2, placeholder: "valor final" });
          v2.addEventListener("input", () => {
            f.valor2 = v2.value;
          });
          grupoValor.append(campo("Hasta", v2));
        }
      } else {
        grupoValor.append(
          el("div", { class: "ayuda-texto", style: "padding:8px 0", text: "Esta condicion no necesita valor." }),
        );
      }
      fila.append(grupoValor);

      // Activar y borrar
      const accionesF = el("div", { class: "fila" });
      const activo = el("input", { type: "checkbox", title: "Activar este filtro" });
      activo.checked = f.activo;
      activo.addEventListener("change", () => {
        f.activo = activo.checked;
        fila.classList.toggle("inactivo", !f.activo);
      });
      accionesF.append(activo);
      accionesF.append(
        boton("", {
          clase: "icono peligro",
          iconoNombre: "basura",
          titulo: "Eliminar filtro",
          alPulsar: () => {
            vista.filtros.splice(indice, 1);
            pintarFiltros();
          },
        }),
      );
      fila.append(accionesF);
      contenedorFiltros.append(fila);
    });

    contenedorFiltros.append(
      boton("Quitar todos los filtros", {
        clase: "fantasma pequeno",
        iconoNombre: "cerrar",
        alPulsar: () => {
          vista.filtros = [];
          pintarFiltros();
        },
      }),
    );
  }

  // Precarga los esquemas de las fuentes activas.
  for (const f of fuentesDisponibles) void cargarEsquemas(f.id);
  pintarEsquemas();
  pintarFiltros();
  pintarResultados(contenedor);
  pintarSugerencias(sugerencias);
}

function etiquetaTipo(f: Fuente): string {
  const t = estado.info?.tiposFuente.find((x) => x.valor === f.tipo);
  return t?.etiqueta ?? f.tipo;
}

async function cargarEsquemas(idFuente: number): Promise<void> {
  if (cacheEsquemas.has(idFuente) || !estado.token) return;
  try {
    const esquemas = await api.esquemasDeFuente(estado.token, idFuente);
    cacheEsquemas.set(idFuente, esquemas);
  } catch {
    cacheEsquemas.set(idFuente, []);
  }
}

/** Ejecuta la busqueda con los criterios que hay en pantalla. */
async function ejecutarBusqueda(contenedor: HTMLElement): Promise<void> {
  if (!estado.token) return;
  const peticion: PeticionBusqueda = {
    fuentes: vista.fuentesSel,
    esquemas: vista.esquemasSel,
    texto: vista.texto,
    filtros: vista.filtros.filter((f) => f.activo),
    campos: [],
    limite: vista.limite,
    desplazamiento: 0,
    ordenarPor: vista.ordenarPor,
    ordenDesc: vista.ordenDesc,
    modo: vista.modo,
    contarTotal: true,
  };

  mostrarCarga("Consultando las fuentes de datos...");
  textoCarga("Ejecutando la consulta...");
  try {
    const r = await api.buscar(estado.token, peticion);
    estado.resultados = r.bloques;
    estado.resultadosMeta = {
      totalFilas: r.totalFilas,
      duracionMs: r.duracionMs,
      errores: r.fuentesConError,
      sugerencias: r.sugerencias,
    };
    // Las columnas conocidas alimentan el desplegable de filtros.
    columnasActuales = r.bloques.flatMap((b) => b.columnas);
    vista.bloqueActivo = 0;
    prepararDocumento(
      `Resultados de busqueda - ${new Date().toLocaleDateString("es-ES")}`,
      peticion.texto ? `Texto buscado: ${peticion.texto}` : "Sin filtro de texto",
      `Fuentes: ${peticion.fuentes.length || "todas"} | Esquemas: ${peticion.esquemas.length || "todos"} | Filtros: ${peticion.filtros.length}`,
    );
    if (r.mensaje) avisar(r.mensaje, "aviso", 7000);
    if (r.fuentesConError.length) {
      avisar(
        `Algunas fuentes no respondieron: ${r.fuentesConError.join(" | ")}`,
        "aviso",
        9000,
      );
    }
    paginaBuscar(contenedor);
    const zona = contenedor.querySelector("#zona-resultados");
    zona?.scrollIntoView({ behavior: "smooth", block: "start" });
  } catch (e) {
    ocultarCarga();
    avisar(comoError(e).message, "error", 9000);
  } finally {
    esconderCarga();
  }
}

/** Muestra las sugerencias de filtrado ofrecidas por el nucleo. */
function pintarSugerencias(nodo: HTMLElement): void {
  while (nodo.firstChild) nodo.removeChild(nodo.firstChild);
  const s = estado.resultadosMeta.sugerencias;
  if (!s.length) {
    nodo.classList.add('oculto');
    return;
  }
  nodo.classList.remove('oculto');
  nodo.innerHTML = `<strong>Sugerencias de filtrado</strong> — pulse una para aplicarla:<br>`;
  const chips = el("div", { class: "lista-multi" });
  for (const g of s) {
    const chip = el("span", { class: "chip" });
    chip.style.cursor = "pointer";
    chip.textContent = g.etiqueta;
    chip.title = `Confianza ${(g.confianza * 100).toFixed(0)} %`;
    chip.addEventListener("click", () => {
      const numerica = columnasActuales.find((c) => c.nombre === g.columna)?.numerica;
      vista.filtros.push({
        campo: g.columna,
        operador: numerica ? "igual" : "igual",
        valor: g.valor,
        valor2: "",
        conector: vista.filtros.length ? "y" : "o",
        activo: true,
      });
      avisar(`Filtro anadido: ${g.etiqueta}`, "exito", 3000);
      window.dispatchEvent(new CustomEvent("datasearch:refrescar-busqueda"));
    });
    chips.append(chip);
  }
  nodo.append(chips);
}

/** Pinta los resultados obtained. */
function pintarResultados(contenedor: HTMLElement): void {
  const zona = contenedor.querySelector("#zona-resultados");
  if (!zona) return;
  while (zona.firstChild) zona.removeChild(zona.firstChild);

  if (!estado.resultados.length) {
    zona.append(
      vacio(
        "Sin resultados todavia",
        "Configure las fuentes y los filtros que necesite y pulse el boton Buscar.",
        "buscar",
      ),
    );
    return;
  }

  const meta = estado.resultadosMeta;
  const cabecera = el("div", { class: "tarjeta-cabecera", style: "border:1px solid #bae6fd;border-radius:10px 10px 0 0" });
  cabecera.insertAdjacentHTML("beforeend", icono("tabla"));
  cabecera.append(
    el("h2", { text: `Resultados (${estado.resultados.length} ${estado.resultados.length === 1 ? "bloque" : "bloques"})` }),
  );
  const acciones = el("div", { class: "acciones" });
  acciones.append(
    boton("Exportar", { clase: "pequeno", iconoNombre: "descargar", alPulsar: () => abrirExportacion() }),
    boton("Imprimir", { clase: "pequeno", iconoNombre: "imprimir", alPulsar: () => void imprimir() }),
  );
  cabecera.append(acciones);

  const info = barraInformacion(
    `${numero(meta.totalFilas)} filas en ${meta.duracionMs} ms`,
    [],
  );
  zona.append(cabecera, info);

  if (meta.errores.length) {
    const err = el("div", { class: "caja-info", style: "border-color:#fecaca" });
    err.innerHTML = `<strong style="color:#dc2626">Fuentes con errores</strong><br>${meta.errores
      .map((e) => esc(e))
      .join("<br>")}`;
    zona.append(err);
  }

  // Selector de bloque cuando hay varias fuentes.
  if (estado.resultados.length > 1) {
    const selector = el("div", { class: "btn-grupo", style: "padding:8px 0" });
    estado.resultados.forEach((b, i) => {
      selector.append(
        boton(`${b.fuenteNombre} (${b.filas.length})`, {
          clase: `pequeno ${i === vista.bloqueActivo ? "principal" : ""}`.trim(),
          alPulsar: () => {
            vista.bloqueActivo = i;
            pintarResultados(contenedor);
          },
        }),
      );
    });
    zona.append(selector);
  }

  const bloque = estado.resultados[Math.min(vista.bloqueActivo, estado.resultados.length - 1)];
  if (!bloque) return;

  const tipos = new Map(bloque.columnas.map((c) => [c.nombre, c]));
  const marco = tablaDatos(bloque.filas, {
    tipos,
    columnas: bloque.columnas.map((c) => c.nombre),
    filasPorPagina: Math.min(bloque.filas.length, vista.limite),
    columnaOrden: vista.ordenarPor,
    ordenDesc: vista.ordenDesc,
    alOrdenar: (columna) => {
      if (vista.ordenarPor === columna) vista.ordenDesc = !vista.ordenDesc;
      else {
        vista.ordenarPor = columna;
        vista.ordenDesc = false;
      }
      void ejecutarBusqueda(contenedor);
    },
  });
  marco.style.border = "1px solid #bae6fd";
  marco.style.borderTop = "none";
  marco.style.borderRadius = "0 0 10px 10px";
  zona.append(marco);

  const pie = el("div", { class: "tarjeta-pie" });
  pie.innerHTML = `Fuente: <strong>${esc(bloque.fuenteNombre)}</strong> (${esc(etiquetaTipoFuente(bloque.tipoFuente))})
    | Esquema: <strong>${esc(bloque.esquema || "-")}</strong>
    | Filas mostradas: <strong>${bloque.filas.length}</strong>
    | Total en el origen: <strong>${numero(bloque.totalFilas)}</strong>
    ${bloque.mensaje ? `<br>${esc(bloque.mensaje)}` : ""}`;
  zona.append(pie);

  const detalle = el("div", { class: "tarjeta", style: "margin-top:16px" });
  const cabDet = el("div", { class: "tarjeta-cabecera" });
  cabDet.append(el("h3", { text: "Columnas disponibles para filtrar" }));
  const accionesDet = el("div", { class: "acciones" });
  accionesDet.append(
    boton("Analizar estos datos", {
      clase: "pequeno principal",
      iconoNombre: "analisis",
      alPulsar: () =>
        window.dispatchEvent(
          new CustomEvent("datasearch:analizar", {
            detail: { fuenteId: bloque.fuenteId, esquema: bloque.esquema },
          }),
        ),
    }),
  );
  cabDet.append(accionesDet);
  detalle.append(cabDet);
  const cuerpoDet = el("div", { class: "tarjeta-cuerpo" });
  cuerpoDet.innerHTML = tablaColumnas(bloque.columnas);
  detalle.append(cuerpoDet);
  zona.append(detalle);
}

function etiquetaTipoFuente(tipo: string): string {
  return estado.info?.tiposFuente.find((x) => x.valor === tipo)?.etiqueta ?? tipo;
}

function tablaColumnas(columnas: ColumnaInfo[]): string {
  if (!columnas.length) return '<p class="ayuda-texto">No se han detectado columnas.</p>';
  const filas = columnas
    .map(
      (c) => `<tr>
        <td><strong>${esc(c.nombre)}</strong></td>
        <td>${esc(c.tipo)}</td>
        <td class="numero">${numero(c.nulos)}</td>
        <td class="numero">${numero(c.distintos)}</td>
        <td>${c.valoresFrecuentes
          .slice(0, 3)
          .map((f) => `<span class="insignia azul">${esc(f.valor.slice(0, 18))}</span>`)
          .join(" ")}</td>
      </tr>`,
    )
    .join("");
  return `<table class="datos">
      <thead><tr>
        <th>Columna</th><th>Tipo</th><th>Nulos</th><th>Distintos</th><th>Valores frecuentes</th>
      </tr></thead>
      <tbody>${filas}</tbody>
    </table>`;
}

// ---------------------------------------------------------------------
// Exportacion e impresion
// ---------------------------------------------------------------------

async function abrirExportacion(): Promise<void> {
    const cuerpo = el("div", { class: "columna" });
  const formatos = estado.info?.formatos ?? [];
  cuerpo.append(el("div", { class: "ayuda-texto", text: "Elija el formato de salida del informe." }));
  const sel = el("select", {});
  for (const f of formatos) sel.append(el("option", { value: f.valor, text: f.etiqueta }));
  sel.value = "excel";
  cuerpo.append(campo("Formato", sel));
  const nombre = el("input", { type: "text", value: estado.documento.titulo });
  cuerpo.append(campo("Titulo del informe", nombre));

  modal("Exportar resultados", cuerpo, {
    iconoNombre: "descargar",
    pie: (cerrar) => [
      boton("Cancelar", { clase: "fantasma", alPulsar: cerrar }),
      boton("Generar fichero", {
        clase: "principal",
        iconoNombre: "descargar",
        alPulsar: async (ev) => {
          const btn = ev.currentTarget as HTMLButtonElement;
          btn.disabled = true;
          try {
            const documento = await construirDocumento(nombre.value.trim());
            const archivo = await api.exportar(estado.token!, documento, sel.value);
            descargar(archivo.nombre, archivo.contenidoBase64, archivo.mime);
            avisar(`Fichero ${archivo.nombre} generado (${tamano(archivo.tamanoBytes)}).`, "exito", 6000);
            cerrar();
          } catch (e) {
            avisar(comoError(e).message, "error", 9000);
            btn.disabled = false;
          }
        },
      }),
    ],
  });
}

async function imprimir(): Promise<void> {
  try {
    const documento = await construirDocumento();
    const html = await api.prepararImpresion(estado.token!, documento);
    imprimirHtml(html, documento.titulo);
  } catch (e) {
    avisar(comoError(e).message, "error", 9000);
  }
}

/** Limpia la cache de esquemas, por ejemplo al anadir una fuente nueva. */
export function limpiarCache(): void {
  cacheEsquemas.clear();
  columnasActuales = [];
}

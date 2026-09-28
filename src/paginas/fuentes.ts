// Copyright (c) 2026 Jose Manuel Bernabeu Mejias - Licencia MIT
// Pagina de gestion de fuentes de datos. Permite anadir tantas fuentes como
// el usuario necesite: ficheros locales, bases de datos remotas y recursos web.

import { api, type ConfigFuente, type Fuente, type SolicitudFuente, type TipoFuente } from "../api";
import { estado } from "../estado";
import { icono } from "../iconos";
import {
  avisar,
  boton,
  campo,
  comoError,
  el,
  esc,
  esconderCarga,
  mostrarCarga,
  fecha,
  modal,
  vacio,
} from "../ui";

/** Campos que muestra el formulario segun el tipo de fuente. */
type Grupo = "servidor" | "fichero" | "web" | "mongo";

const CAMPOS_POR_TIPO: Record<string, Grupo> = {
  sqlite: "fichero",
  csv: "fichero",
  json: "fichero",
  xml: "fichero",
  excel: "fichero",
  postgres: "servidor",
  mysql: "servidor",
  sqlserver: "servidor",
  mongodb: "mongo",
  web: "web",
};

/** Pinta la pagina de fuentes de datos. */
export function paginaFuentes(contenedor: HTMLElement): void {
  if (!estado.usuario) return;

  const tarjeta = el("div", { class: "tarjeta" });
  const cab = el("div", { class: "tarjeta-cabecera" });
  cab.insertAdjacentHTML("beforeend", icono("base"));
  cab.append(el("h2", { text: "Fuentes de datos" }));
  const acciones = el("div", { class: "acciones" });
  acciones.append(
    boton("Anadir fuente", {
      clase: "principal pequeno",
      iconoNombre: "anadir",
      alPulsar: () => abrirFormulario(),
    }),
    boton("Refrescar", {
      clase: "fantasma pequeno",
      iconoNombre: "refrescar",
      alPulsar: () => window.dispatchEvent(new CustomEvent("datasearch:refrescar-fuentes")),
    }),
  );
  cab.append(acciones);
  tarjeta.append(cab);

  const cuerpo = el("div", { class: "tarjeta-cuerpo" });
  if (!estado.fuentes.length) {
    cuerpo.append(
      vacio(
        "No ha anadido ninguna fuente de datos",
        "Pulse «Anadir fuente» para conectar la primera. Puede anadir tantas como necesite.",
        "base",
      ),
    );
  } else {
    const rej = el("div", { class: "lista-doble" });
    for (const f of estado.fuentes) rej.append(tarjetaFuente(f));
    cuerpo.append(rej);
  }
  tarjeta.append(cuerpo);
  tarjeta.append(
    el("div", {
      class: "tarjeta-pie",
      html: `Las fuentes marcadas como <strong>solo administrador</strong> no son visibles para
             el resto de usuarios. Las demas quedan disponibles para todos mientras esten activas.`,
    }),
  );
  contenedor.append(tarjeta);
}

function tarjetaFuente(f: Fuente): HTMLElement {
  const t = el("div", { class: "tarjeta", style: "margin:0" });
  const cab = el("div", { class: "tarjeta-cabecera" });
  cab.insertAdjacentHTML("beforeend", icono("base"));
  const titulos = el("div", {});
  titulos.append(el("div", { style: "font-weight:700;font-size:13.5px", text: f.nombre }));
  const sub = el("div", { class: "texto-pequeno texto-suave" });
  sub.textContent = etiquetaTipo(f.tipo) + (f.descripcion ? ` - ${f.descripcion}` : "");
  titulos.append(sub);
  cab.append(titulos);

  const estadoInsignia = el("span", {
    class: `insignia ${f.activa ? "exito" : "aviso"}`,
  });
  estadoInsignia.append(el("span", { class: `punto-estado ${f.activa ? "activo" : "pendiente"}` }));
  estadoInsignia.append(document.createTextNode(f.activa ? "Activa" : "Inactiva"));
  cab.append(estadoInsignia);
  if (f.soloAdmin) cab.append(el("span", { class: "insignia azul", text: "Solo admin" }));
  t.append(cab);

  const cuerpo = el("div", { class: "tarjeta-cuerpo compacto" });
  const datos: [string, string][] = [
    ["Tipo", etiquetaTipo(f.tipo)],
    ["Servidor / ruta", f.config.host || f.config.uri || f.config.ruta || "-"],
    ["Base de datos", f.config.baseDatos || "-"],
    ["Usuario", f.config.usuario || "-"],
    ["Alta", fecha(f.creadoEn)],
    ["Ultima modificacion", fecha(f.actualizadoEn)],
  ];
  const rej = el("div", { class: "ficha" });
  for (const [k, v] of datos) {
    if (v === "-" || !v) continue;
    const d = el("div", { class: "dato-ficha" });
    d.innerHTML = `<div class="clave">${esc(k)}</div><div class="valor" title="${esc(v)}">${esc(
      v.length > 46 ? `${v.slice(0, 46)}…` : v,
    )}</div>`;
    rej.append(d);
  }
  cuerpo.append(rej);

  const acciones = el("div", { class: "btn-grupo", style: "margin-top:10px" });
  acciones.append(
    boton("Probar conexion", {
      clase: "pequeno",
      iconoNombre: "probar",
      alPulsar: (ev) => void probar(f, ev.currentTarget as HTMLButtonElement),
    }),
    boton("Ver esquemas", {
      clase: "pequeno fantasma",
      iconoNombre: "tabla",
      alPulsar: () => void verEsquemas(f),
    }),
    boton("Editar", {
      clase: "pequeno fantasma",
      iconoNombre: "lapiz",
      alPulsar: () => abrirFormulario(f),
    }),
    boton("Eliminar", {
      clase: "pequeno peligro",
      iconoNombre: "basura",
      alPulsar: () =>
        modal(
          `Eliminar «${f.nombre}»`,
          el("div", {
            class: "ayuda-texto",
            text: "Se eliminara esta fuente de forma permanente. Los resultados ya obtenidos no se veran afectados. Esta accion no se puede deshacer.",
          }),
          {
            clase: "estrecho",
            iconoNombre: "aviso",
            pie: (cerrar) => [
              boton("Cancelar", { clase: "fantasma", alPulsar: cerrar }),
              boton("Eliminar definitivamente", {
                clase: "peligro",
                alPulsar: async () => {
                  try {
                    await api.eliminarFuente(estado.token!, f.id);
                    avisar(`Fuente «${f.nombre}» eliminada.`, "exito");
                    window.dispatchEvent(new CustomEvent("datasearch:refrescar-fuentes"));
                  } catch (e) {
                    avisar(comoError(e).message, "error");
                  }
                },
              }),
            ],
          },
        ),
    }),
  );
  cuerpo.append(acciones);
  t.append(cuerpo);
  return t;
}

function etiquetaTipo(tipo: TipoFuente): string {
  return estado.info?.tiposFuente.find((x) => x.valor === tipo)?.etiqueta ?? tipo;
}

async function probar(f: Fuente, botonPulsado: HTMLButtonElement): Promise<void> {
  botonPulsado.disabled = true;
  mostrarCarga("Comprobando la conexion...");
  try {
    const r = await api.probarFuente(estado.token!, f.id);
    avisar(`${f.nombre}: ${r}`, "exito", 8000);
  } catch (e) {
    avisar(`${f.nombre}: ${comoError(e).message}`, "error", 12000);
  } finally {
    esconderCarga();
    botonPulsado.disabled = false;
  }
}

async function verEsquemas(f: Fuente): Promise<void> {
  mostrarCarga("Leyendo las tablas de la fuente...");
  try {
    const esquemas = await api.esquemasDeFuente(estado.token!, f.id);
    const cuerpo = el("div", { class: "columna" });
    if (!esquemas.length) {
      cuerpo.append(el("div", { class: "ayuda-texto", text: "La fuente no expone ninguna tabla ni coleccion." }));
    }
    for (const e of esquemas) {
      const d = el("div", { class: "dato-ficha" });
      d.innerHTML = `<div class="clave">${esc(e.nombre)} <span class="texto-pequeno">(${
        e.filasEstimadas >= 0 ? `${e.filasEstimadas} filas` : "nuevo de cuenta"
      })</span></div>
        <div class="valor texto-pequeno" style="font-weight:400">${esc(
          e.columnas
            .slice(0, 14)
            .map((c) => `${c.nombre}: ${c.tipo}`)
            .join(" | ") || "sin columnas",
        )}</div>`;
      cuerpo.append(d);
    }
    modal(`Esquemas de «${f.nombre}»`, cuerpo, {
      clase: "ancho",
      iconoNombre: "tabla",
      pie: (cerrar) => [boton("Cerrar", { clase: "principal", alPulsar: cerrar })],
    });
  } catch (e) {
    avisar(comoError(e).message, "error", 10000);
  } finally {
    esconderCarga();
  }
}

// ---------------------------------------------------------------------
// Formulario de alta y edicion
// ---------------------------------------------------------------------

function abrirFormulario(existente?: Fuente): void {
  if (!estado.token) return;
  const esNueva = !existente;
  const tipos = estado.info?.tiposFuente ?? [];

  const cuerpo = el("div", { class: "columna" });
  const nombre = el("input", { type: "text", value: existente?.nombre ?? "", placeholder: "Ventas de produccion" });
  const descripcion = el("input", { type: "text", value: existente?.descripcion ?? "", placeholder: "Descripcion opcional" });
  const selTipo = el("select", {});
  for (const t of tipos) {
    selTipo.append(el("option", { value: t.valor, text: t.etiqueta, selected: existente?.tipo === t.valor }));
  }
  if (esNueva) selTipo.value = "sqlite";

  const config: ConfigFuente = existente
    ? { ...existente.config }
    : {
        host: "",
        puerto: null,
        usuario: "",
        contrasena: "",
        baseDatos: "",
        esquema: "",
        ssl: false,
        tiempoEsperaSeg: 45,
        ruta: "",
        metodo: "GET",
        cabeceras: {},
        cuerpo: "",
        rutaJson: "",
        formatoWeb: "json",
        parametros: "",
        uri: "",
        coleccion: "",
      };
  if (esNueva) {
    const t = selTipo.value as TipoFuente;
    const ejemplo = estado.info?.tiposFuente.find((x) => x.valor === t);
    config.puerto = ejemplo?.puertoPorDefecto ?? null;
  }

  const zonaConfig = el("div", { class: "rejilla dos" });
  const construir = () => {
    while (zonaConfig.firstChild) zonaConfig.removeChild(zonaConfig.firstChild);
    const grupo = CAMPOS_POR_TIPO[selTipo.value] ?? "fichero";
    if (grupo === "fichero") {
      zonaConfig.append(
        campo(
          "Ruta del fichero o URL",
          entradaTexto(
            () => config.ruta,
            (v) => (config.ruta = v),
            "/ruta/al/fichero.csv o https://ejemplo/datos.csv",
          ),
        ),
      );
    } else if (grupo === "web") {
      zonaConfig.append(
        campo("URL del recurso", entradaTexto(() => config.ruta, (v) => (config.ruta = v), "https://api.ejemplo.es/datos")),
        campo(
          "Metodo",
          entradaTexto(() => config.metodo, (v) => (config.metodo = v), "GET, POST, PUT o DELETE"),
        ),
        campo("Formato", entradaTexto(() => config.formatoWeb, (v) => (config.formatoWeb = v), "json o csv")),
        campo(
          "Ruta dentro del JSON",
          entradaTexto(() => config.rutaJson, (v) => (config.rutaJson = v), "data.items"),
        ),
        campo(
          "Parametros",
          entradaTexto(() => config.parametros, (v) => (config.parametros = v), "pagina=1&limite=50"),
        ),
        campo("Cuerpo (POST/PUT)", areaTexto(() => config.cuerpo, (v) => (config.cuerpo = v))),
      );
    } else if (grupo === "mongo") {
      zonaConfig.append(
        campo(
          "URI de conexion",
          entradaTexto(
            () => config.uri,
            (v) => (config.uri = v),
            "mongodb://usuario:clave@servidor:27017",
          ),
          "deje vacio para usar servidor y usuario",
        ),
        campo("Servidor", entradaTexto(() => config.host, (v) => (config.host = v), "localhost")),
        campo("Puerto", entradaNumero(() => config.puerto, (v) => (config.puerto = v), 27017)),
        campo("Base de datos", entradaTexto(() => config.baseDatos, (v) => (config.baseDatos = v), "mi_base")),
      );
    } else {
      zonaConfig.append(
        campo("Servidor", entradaTexto(() => config.host, (v) => (config.host = v), "localhost")),
        campo("Puerto", entradaNumero(() => config.puerto, (v) => (config.puerto = v), config.puerto ?? 5432)),
        campo("Usuario", entradaTexto(() => config.usuario, (v) => (config.usuario = v), "usuario")),
        campo(
          "Contrasena",
          entradaTexto(() => config.contrasena, (v) => (config.contrasena = v), "contrasena", "password"),
          "en blanco se conserva la actual",
        ),
        campo("Base de datos", entradaTexto(() => config.baseDatos, (v) => (config.baseDatos = v), "mi_base")),
        campo(
          "Esquema",
          entradaTexto(() => config.esquema, (v) => (config.esquema = v), "public"),
          "opcional",
        ),
      );
    }
  };
  selTipo.addEventListener("change", construir);
  construir();

  const activa = el("input", { type: "checkbox" });
  activa.checked = existente?.activa ?? true;
  const soloAdmin = el("input", { type: "checkbox" });
  soloAdmin.checked = existente?.soloAdmin ?? false;
  const cajaActiva = el("label", { class: "campo-checkbox" });
  cajaActiva.append(activa, el("span", { text: "Fuente activa" }));
  const cajaSolo = el("label", { class: "campo-checkbox" });
  cajaSolo.append(soloAdmin, el("span", { text: "Visible solo por el administrador" }));

  cuerpo.append(campo("Nombre de la fuente *", nombre));
  cuerpo.append(campo("Descripcion", descripcion));
  cuerpo.append(campo("Tipo de fuente *", selTipo));
  cuerpo.append(el("div", { class: "separador" }));
  cuerpo.append(zonaConfig);
  cuerpo.append(el("div", { class: "separador" }));
  cuerpo.append(cajaActiva, cajaSolo);

  modal(
    esNueva ? "Anadir fuente de datos" : `Editar «${existente?.nombre}»`,
    cuerpo,
    {
      clase: "ancho",
      iconoNombre: "base",
      pie: (cerrar) => [
        boton("Cancelar", { clase: "fantasma", alPulsar: cerrar }),
        boton(esNueva ? "Anadir y probar" : "Guardar cambios", {
          clase: "principal",
          iconoNombre: "guardar",
          alPulsar: async (ev) => {
            if (!nombre.value.trim()) {
              avisar("Indique el nombre de la fuente.", "aviso");
              return;
            }
            const btn = ev.currentTarget as HTMLButtonElement;
            btn.disabled = true;
            const peticion: SolicitudFuente = {
              nombre: nombre.value.trim(),
              descripcion: descripcion.value.trim(),
              tipo: selTipo.value,
              config: { ...config },
              activa: activa.checked,
              soloAdmin: soloAdmin.checked,
              conservarSecreto: !esNueva,
            };
            try {
              if (esNueva) {
                const creada = await api.crearFuente(estado.token!, peticion);
                avisar(`Fuente «${creada.nombre}» anadida.`, "exito");
                // Se comprueba la conexion de forma automatica.
                try {
                  const r = await api.probarFuente(estado.token!, creada.id);
                  avisar(`Conexion correcta: ${r}`, "exito", 8000);
                } catch (e2) {
                  avisar(
                    `La fuente se ha guardado, pero la conexion falla: ${comoError(e2).message}`,
                    "aviso",
                    12000,
                  );
                }
              } else {
                await api.actualizarFuente(estado.token!, existente!.id, peticion);
                avisar(`Fuente «${nombre.value.trim()}» actualizada.`, "exito");
              }
              cerrar();
              window.dispatchEvent(new CustomEvent("datasearch:refrescar-fuentes"));
            } catch (e) {
              avisar(comoError(e).message, "error", 12000);
              btn.disabled = false;
            }
          },
        }),
      ],
    },
  );

}

function entradaTexto(
  obtener: () => string,
  asignar: (v: string) => void,
  marcador = "",
  tipo = "text",
): HTMLInputElement {
  const i = el("input", { type: tipo, value: obtener(), placeholder: marcador });
  i.addEventListener("input", () => asignar(i.value));
  return i;
}

function entradaNumero(
  obtener: () => number | null,
  asignar: (v: number | null) => void,
  porDefecto: number,
): HTMLInputElement {
  const i = el("input", { type: "number", value: obtener() !== null ? String(obtener()) : String(porDefecto) });
  i.addEventListener("input", () => asignar(i.value ? Number(i.value) : null));
  return i;
}

function areaTexto(obtener: () => string, asignar: (v: string) => void): HTMLTextAreaElement {
  const a = el("textarea", { value: obtener() });
  a.addEventListener("input", () => asignar(a.value));
  return a;
}

/** Muestra la ayuda de los tipos de fuente disponibles. */
export function ayudaTipos(): void {
  const cuerpo = el("div", { class: "columna" });
  cuerpo.innerHTML = `
    <div class="ayuda-texto">
      <strong>SQLite</strong>: fichero de base de datos en el equipo.<br>
      <strong>PostgreSQL / MySQL / SQL Server</strong>: servidores de bases de datos, locales o remotos.<br>
      <strong>MongoDB</strong>: bases de datos documentales por colecciones.<br>
      <strong>CSV / JSON / XML / Excel</strong>: ficheros de datos en el equipo o alcanzables por URL.<br>
      <strong>Recurso web</strong>: cualquier API o fichero publicado por HTTP o HTTPS.
    </div>`;
  modal("Tipos de fuente admitidos", cuerpo, {
    clase: "estrecho",
    iconoNombre: "info",
    pie: (cerrar) => [boton("Cerrar", { clase: "principal", alPulsar: cerrar })],
  });
}

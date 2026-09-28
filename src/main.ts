// Copyright (c) 2026 Jose Manuel Bernabeu Mejias - Licencia MIT
// Punto de entrada de la ventana principal.
//
// Estructura: panel lateral de navegacion en azul claro y zona de trabajo
// blanca con el panel de control, el buscador, el tablero y el resto de
// secciones. El pie de pagina lleva el copyright del autor.

import "./styles.css";
import { api, type InfoVersion } from "./api";
import {
  cargarFuentes,
  cargarInfo,
  cerrarSesion,
  esAdmin,
  estado,
  notificar,
  recordarToken,
  sesionActual,
  suscribir,
  tokenGuardado,
  tienePermiso,
} from "./estado";
import { icono } from "./iconos";
import { pantallaAcceso } from "./paginas/acceso";
import { abrirCredenciales } from "./paginas/credenciales";
import { paginaAnalisis } from "./paginas/analisis";
import { paginaBuscar } from "./paginas/buscar";
import { paginaFuentes } from "./paginas/fuentes";
import { paginaTablero } from "./paginas/tablero";
import { paginaUsuarios } from "./paginas/usuarios";
import { paginaAjustes } from "./paginas/ajustes";
import { avisar, boton, el, esc, comoError, esconderCarga } from "./ui";
import { listen } from "@tauri-apps/api/event";

type Seccion = "tablero" | "buscar" | "analisis" | "fuentes" | "usuarios" | "ajustes";

let seccionActual: Seccion = "tablero";
let lateralPlegada = false;
let contenedorApp: HTMLElement;
let lateralAbiertoMovil = false;

interface ItemMenu {
  clave: Seccion;
  titulo: string;
  icono: string;
  requiere?: "admin" | PermisoClave;
}

type PermisoClave =
  | "consultar"
  | "crearFuentes"
  | "modificarFuentes"
  | "eliminarFuentes"
  | "exportar"
  | "imprimir"
  | "analizar"
  | "crearDashboards"
  | "verAuditoria";

const MENU: ItemMenu[] = [
  { clave: "tablero", titulo: "Panel de control", icono: "graficos", requiere: "crearDashboards" },
  { clave: "buscar", titulo: "Buscar datos", icono: "buscar", requiere: "consultar" },
  { clave: "analisis", titulo: "Analisis", icono: "analisis", requiere: "analizar" },
  { clave: "fuentes", titulo: "Fuentes de datos", icono: "base", requiere: "consultar" },
  { clave: "usuarios", titulo: "Usuarios y permisos", icono: "usuarios", requiere: "admin" },
  { clave: "ajustes", titulo: "Ajustes", icono: "engranaje" },
];

// ---------------------------------------------------------------------
// Arranque
// ---------------------------------------------------------------------

async function iniciar(): Promise<void> {
  contenedorApp = (document.querySelector("#app") as HTMLElement) ?? el("div", { id: "app" });
  document.body.append(contenedorApp);

  try {
    await cargarInfo();
  } catch (e) {
    document.body.innerHTML = `<div style="padding:40px;font-family:sans-serif">
      <h1>No se ha podido iniciar DataSearch</h1>
      <p>${esc(comoError(e).message)}</p></div>`;
    return;
  }

  // Se intenta recuperar la sesion guardada.
  const guardado = tokenGuardado();
  if (guardado) {
    const usuario = await sesionActual(guardado);
    if (usuario) {
      estado.token = guardado;
      estado.usuario = usuario;
    } else {
      recordarToken(null);
    }
  }

  pintar();
  notificar();

  // Se avisa al sistema de que la ventana ya puede mostrarse.
  void api.mostrarVentana().catch(() => undefined);

  suscribir(() => pintar());
  registrarEventos();
  await comprobarVersionInicial();
}

/** Suscribe los eventos internos emitidos por el nucleo. */
function registrarEventos(): void {
  // Aviso de actualizacion disponible desde el arranque del nucleo.
  void listen<InfoVersion>("actualizacion:disponible", (e) => {
    estado.version = e.payload;
    pintar();
  });
  void listen<InfoVersion>("actualizacion:sin_novedades", (e) => {
    estado.version = e.payload;
  });

  // Analisis solicitado desde la pagina de busqueda.
  window.addEventListener("datasearch:analizar", () => irA("analisis"));
  window.addEventListener("datasearch:navegar", (e) => {
    const detalle = (e as CustomEvent<{ detail: string }>).detail;
    irA(detalle as unknown as Seccion);
  });
  window.addEventListener("datasearch:refrescar-fuentes", () => void recargarFuentes());
  window.addEventListener("datasearch:refrescar-busqueda", () => pintar());
  window.addEventListener("datasearch:refrescar-usuarios", () => {
    if (seccionActual === "usuarios") pintar();
  });
}

async function recargarFuentes(): Promise<void> {
  await cargarFuentes();
  if (seccionActual === "fuentes" || seccionActual === "tablero" || seccionActual === "buscar") {
    pintar();
  }
}

async function comprobarVersionInicial(): Promise<void> {
  // La comprobacion la hace el nucleo al arrancar; aqui solo se espera su
  // resultado para pintar el indicador del panel lateral.
  setTimeout(async () => {
    try {
      const info = await api.comprobarActualizaciones();
      estado.version = info;
      pintar();
      if (info.hayActualizacion) {
        avisar(
          `Hay una nueva version disponible: v${info.versionDisponible}. Use el indicador de la barra lateral para instalarla.`,
          "info",
          10000,
        );
      }
    } catch {
      // Sin conexion: la aplicacion sigue funcionando con normalidad.
    }
  }, 1400);
}

// ---------------------------------------------------------------------
// Estructura visual
// ---------------------------------------------------------------------

function pintar(): void {
  while (contenedorApp.firstChild) contenedorApp.removeChild(contenedorApp.firstChild);

  if (!estado.usuario) {
    document.body.classList.add("sin-sesion");
    pantallaAcceso(contenedorApp);
    return;
  }
  document.body.classList.remove("sin-sesion");

  const capa = el("div", {
    class: `capa-menu ${lateralAbiertoMovil ? "visible" : ""}`.trim(),
  });
  capa.addEventListener("click", () => {
    lateralAbiertoMovil = false;
    pintar();
  });

  const principal = el("div", { class: "principal" });
  principal.append(cabecera(), zonaTrabajo(), pie());

  contenedorApp.append(lateral(capa), principal, capa);
}

function lateral(capa: HTMLElement): HTMLElement {
  const nav = el("aside", {
    class: `lateral ${lateralPlegada ? "plegado" : ""} ${lateralAbiertoMovil ? "abierta" : ""}`.trim(),
  });

  // Cabecera con marca y version.
  const cab = el("div", { class: "lateral-cabecera" });
  cab.insertAdjacentHTML("beforeend", `<div class="lateral-logo">${icono("graficos")}</div>`);
  const titulo = el("div", { class: "lateral-titulo" });
  titulo.append(el("div", { class: "lateral-nombre", text: estado.info?.nombre ?? "DataSearch" }));
  titulo.append(el("div", { class: "lateral-version", text: `v${estado.info?.version ?? "1.0.0"}` }));
  if (estado.version?.hayActualizacion) {
    const act = el("div", { class: "lateral-actualizacion", title: "Hay una actualizacion disponible" });
    act.insertAdjacentHTML("beforeend", icono("actualizar"));
    act.append(document.createTextNode(`v${estado.version.versionDisponible}`));
    act.addEventListener("click", () => irA("ajustes"));
    titulo.append(act);
  } else {
    const act = el("div", { class: "lateral-actualizacion", title: "Comprobar actualizaciones" });
    act.insertAdjacentHTML("beforeend", icono("actualizar"));
    act.append(document.createTextNode("Actualizar"));
    act.addEventListener("click", () => void comprobarAhora());
    titulo.append(act);
  }
  cab.append(titulo);
  nav.append(cab);

  // Boton de plegado.
  const plegar = el("button", {
    class: "nav-item",
    type: "button",
    title: lateralPlegada ? "Desplegar el menu" : "Plegar el menu",
    style: "margin-top:6px",
  });
  plegar.insertAdjacentHTML("beforeend", icono("panel"));
  plegar.append(el("span", { text: lateralPlegada ? "Menu" : "Plegar menu" }));
  plegar.addEventListener("click", () => {
    lateralPlegada = !lateralPlegada;
    pintar();
  });
  nav.append(plegar);

  // Navegacion.
  const lista = el("nav", { class: "lateral-nav" });
  lista.append(el("div", { class: "lateral-seccion", text: "Navegacion" }));

  const visibles = MENU.filter((m) => {
    if (m.clave === "usuarios") return esAdmin();
    if (m.requiere) return tienePermiso(m.requiere as PermisoClave);
    return true;
  });

  for (const item of visibles) {
    const b = el("button", {
      class: `nav-item ${seccionActual === item.clave ? "activo" : ""}`.trim(),
      type: "button",
    });
    b.insertAdjacentHTML("beforeend", icono(item.icono));
    b.append(el("span", { text: item.titulo }));
    if (item.clave === "fuentes" && estado.fuentes.length) {
      b.append(el("span", { class: "contador", text: String(estado.fuentes.length) }));
    }
    if (item.clave === "usuarios" && estado.usuario?.rol === "administrador") {
      b.append(el("span", { class: "contador", text: "admin" }));
    }
    b.addEventListener("click", () => {
      lateralAbiertoMovil = false;
      irA(item.clave);
    });
    lista.append(b);
  }

  lista.append(el("div", { class: "lateral-seccion", text: "Sesion" }));
  const perfil = el("button", { class: "nav-item perfil", type: "button" });
  perfil.insertAdjacentHTML("beforeend", icono("credencial"));
  perfil.append(el("span", { text: "Credenciales" }));
  perfil.addEventListener("click", () => {
    lateralAbiertoMovil = false;
    void abrirCredenciales();
  });
  lista.append(perfil);

  const salir = el("button", { class: "nav-item", type: "button" });
  salir.insertAdjacentHTML("beforeend", icono("salir"));
  salir.append(el("span", { text: "Cerrar sesion" }));
  salir.addEventListener("click", () => {
    cerrarSesion();
    avisar("Su sesion se ha cerrado.", "info", 3000);
  });
  lista.append(salir);
  nav.append(lista);
  void capa;
  return nav;
}

function cabecera(): HTMLElement {
  const cab = el("header", { class: "cabecera" });
  const item = MENU.find((m) => m.clave === seccionActual) ?? MENU[0];

  const menuMovil = el("button", { class: "btn icono fantasma", title: "Menu" });
  menuMovil.insertAdjacentHTML("beforeend", icono("menu"));
  menuMovil.addEventListener("click", () => {
    lateralAbiertoMovil = !lateralAbiertoMovil;
    pintar();
  });
  cab.append(menuMovil);

  const textos = el("div", {});
  textos.append(el("h1", { text: item.titulo }));
  textos.append(el("div", { class: "sub", text: subtitulo(item.clave) }));
  cab.append(textos);

  const acciones = el("div", { class: "cabecera-acciones" });
  const usuario = estado.usuario;
  if (usuario) {
    const chip = el("span", { class: "insignia azul" });
    chip.insertAdjacentHTML("beforeend", icono("usuario"));
    chip.append(document.createTextNode(usuario.nombre || usuario.usuario));
    acciones.append(chip);
  }
  const credenciales = boton("Credenciales", {
    clase: "fantasma",
    iconoNombre: "credencial",
    titulo: "Ver mis datos y los del autor",
    alPulsar: () => void abrirCredenciales(),
  });
  acciones.append(credenciales);
  cab.append(acciones);
  return cab;
}

function subtitulo(seccion: Seccion): string {
  switch (seccion) {
    case "tablero":
      return "Genere el panel de control que necesite a partir de los datos de sus fuentes.";
    case "buscar":
      return estado.resultados.length
        ? `${estado.resultados.length} bloques de resultados, ${estado.resultadosMeta.totalFilas} filas en ${estado.resultadosMeta.duracionMs} ms.`
        : "Configure el buscador y pulse Buscar para obtener resultados.";
    case "analisis":
      return "Seleccione los analisis y pulse «Confirmar y analizar».";
    case "fuentes":
      return `${estado.fuentes.length} fuentes registradas. Anada tantas como necesite.`;
    case "usuarios":
      return "Verificacion, permisos y actividad de los usuarios registrados.";
    case "ajustes":
      return "Preferencias de la aplicacion y comprobacion de actualizaciones.";
    default:
      return "";
  }
}

function zonaTrabajo(): HTMLElement {
  const zona = el("main", { class: "zona-trabajo" });
  try {
    switch (seccionActual) {
      case "tablero":
        paginaTablero(zona);
        break;
      case "buscar":
        paginaBuscar(zona);
        break;
      case "analisis":
        paginaAnalisis(zona);
        break;
      case "fuentes":
        paginaFuentes(zona);
        break;
      case "usuarios":
        paginaUsuarios(zona);
        break;
      case "ajustes":
        paginaAjustes(zona);
        break;
    }
  } catch (e) {
    zona.append(
      el("div", {
        class: "caja-info",
        style: "border-color:#fecaca",
        html: `<strong style="color:#dc2626">No se ha podido pintar esta seccion</strong><br>${esc(
          comoError(e).message,
        )}`,
      }),
    );
    console.error(e);
  }
  return zona;
}

function pie(): HTMLElement {
  const p = el("footer", { class: "pie" });
  const autor = estado.info?.autorFicha;
  p.innerHTML = `
    <span>Copyright (c) 2026 ${esc(autor?.nombre ?? "Jose Manual Bernabeu Mejias")}</span>
    <span class="sep">|</span>
    <span>Licencia ${esc(estado.info?.licencia ?? "MIT")}</span>
    <span class="sep">|</span>
    <a href="${esc(estado.info?.repositorio ?? "https://github.com/JMBermejias/Datasearch")}"
       target="_blank" rel="noopener">${esc(estado.info?.repositorio ?? "github.com/JMBermejias/Datasearch")}</a>
    <span class="derecha">${esc(estado.info?.nombre ?? "DataSearch")} v${esc(
      estado.info?.version ?? "1.0.0",
    )} &middot; Todos los derechos reservados</span>`;
  return p;
}

// ---------------------------------------------------------------------
// Navegacion
// ---------------------------------------------------------------------

function irA(seccion: Seccion): void {
  seccionActual = seccion;
  try {
    sessionStorage.setItem("datasearch.seccion", seccion);
  } catch {
    /* sin almacenamiento: se mantiene solo en memoria */
  }
  pintar();
}

async function comprobarAhora(): Promise<void> {
  try {
    const info = await api.comprobarActualizaciones();
    estado.version = info;
    pintar();
    if (info.hayActualizacion) irA("ajustes");
    else if (info.consultado) avisar(`Esta es la ultima version (v${info.versionActual}).`, "exito", 6000);
    else avisar(info.notas || "No se ha podido comprobar si hay actualizaciones.", "aviso", 7000);
  } catch (e) {
    avisar(comoError(e).message, "error");
  }
}

// ---------------------------------------------------------------------
// Utilidades de arranque
// ---------------------------------------------------------------------

/** Recupera la seccion recordada de la sesion anterior. */
function seccionRecordada(): Seccion {
  try {
    const v = sessionStorage.getItem("datasearch.seccion");
    if (v && MENU.some((m) => m.clave === v)) return v as Seccion;
  } catch {
    /* sin almacenamiento */
  }
  return "tablero";
}
seccionActual = seccionRecordada();

/** Atajos de teclado habituales. */
document.addEventListener("keydown", (e) => {
  if (!estado.usuario) return;
  const enCampo =
    e.target instanceof HTMLInputElement ||
    e.target instanceof HTMLTextAreaElement ||
    e.target instanceof HTMLSelectElement;
  if (e.ctrlKey && e.key === "1" && !enCampo) irA("tablero");
  else if (e.ctrlKey && e.key === "2" && !enCampo) irA("buscar");
  else if (e.ctrlKey && e.key === "3" && !enCampo) irA("analisis");
  else if (e.ctrlKey && e.key === "4" && !enCampo) irA("fuentes");
});

void iniciar().catch((e) => {
  console.error(e);
  esconderCarga();
  avisar(`Error al iniciar DataSearch: ${comoError(e).message}`, "error", 0);
});

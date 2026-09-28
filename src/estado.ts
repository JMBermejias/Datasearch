// Copyright (c) 2026 Jose Manuel Bernabeu Mejias - Licencia MIT
// Estado global de la aplicacion: sesion, fuentes, resultados y tablero.

import {
  api,
  type BloqueResultados,
  type Fila,
  type Fuente,
  type InfoAplicacion,
  type InfoVersion,
  type Permisos,
  type Tablero,
  type Usuario,
} from "./api";
import { comoError, ErrorNucleo } from "./api";
import { avisar } from "./ui";

/** Clave con la que se guarda el token en el almacenamiento local. */
const CLAVE_TOKEN = "datasearch.token";

export interface Estado {
  info: InfoAplicacion | null;
  token: string | null;
  usuario: Usuario | null;
  fuentes: Fuente[];
  resultados: BloqueResultados[];
  resultadosMeta: {
    totalFilas: number;
    duracionMs: number;
    errores: string[];
    sugerencias: { etiqueta: string; columna: string; valor: string; confianza: number }[];
  };
  tablero: Tablero | null;
  documento: {
    titulo: string;
    subtitulo: string;
    pie: string;
  };
  version: InfoVersion | null;
}

export const estado: Estado = {
  info: null,
  token: null,
  usuario: null,
  fuentes: [],
  resultados: [],
  resultadosMeta: { totalFilas: 0, duracionMs: 0, errores: [], sugerencias: [] },
  tablero: null,
  documento: { titulo: "Informe de DataSearch", subtitulo: "", pie: "" },
  version: null,
};

const oyentes = new Set<() => void>();

/** Suscribe una funcion a los cambios de estado. */
export function suscribir(fn: () => void): () => void {
  oyentes.add(fn);
  return () => oyentes.delete(fn);
}

/** Notifica a los suscriptores. */
export function notificar(): void {
  for (const fn of oyentes) fn();
}

/** Guarda el token para no pedir el acceso en cada arranque. */
export function recordarToken(token: string | null): void {
  estado.token = token;
  try {
    if (token) localStorage.setItem(CLAVE_TOKEN, token);
    else localStorage.removeItem(CLAVE_TOKEN);
  } catch {
    // En modo privado puede fallar: la sesion durara lo que dure la pestana.
  }
}

/** Recupera el token guardado. */
export function tokenGuardado(): string | null {
  try {
    return localStorage.getItem(CLAVE_TOKEN);
  } catch {
    return null;
  }
}

/** Carga la informacion general de la aplicacion. */
export async function cargarInfo(): Promise<InfoAplicacion> {
  if (!estado.info) estado.info = await api.info();
  return estado.info;
}

/** Comprueba si el usuario conectado sigue teniendo permisos. */
export function tienePermiso(p: keyof Permisos): boolean {
  if (!estado.usuario) return false;
  if (estado.usuario.rol === "administrador") return true;
  return estado.usuario.permisos[p];
}

/** Recupera el usuario asociado a un token guardado. */
export async function sesionActual(token: string): Promise<Usuario | null> {
  try {
    return await api.sesionActual(token);
  } catch {
    return null;
  }
}

/** Indica si el usuario conectado es administrador. */
export function esAdmin(): boolean {
  return estado.usuario?.rol === "administrador";
}

/** Cierra la sesion y limpia el estado. */
export function cerrarSesion(): void {
  if (estado.token) void api.cerrarSesion(estado.token).catch(() => undefined);
  try {
    sessionStorage.removeItem("datasearch.token");
  } catch {
    /* sin almacenamiento */
  }
  recordarToken(null);
  estado.usuario = null;
  estado.fuentes = [];
  estado.resultados = [];
  estado.resultadosMeta = { totalFilas: 0, duracionMs: 0, errores: [], sugerencias: [] };
  estado.tablero = null;
  notificar();
}

/** Recarga la lista de fuentes del usuario. */
export async function cargarFuentes(): Promise<Fuente[]> {
  if (!estado.token) return [];
  try {
    estado.fuentes = await api.listarFuentes(estado.token);
  } catch (e) {
    const err = comoError(e);
    if (err.esSesion) {
      cerrarSesion();
      return [];
    }
    avisar(err.message, "error");
  }
  notificar();
  return estado.fuentes;
}

/**
 * Ejecuta una accion que necesita una sesion valida.
 *
 * Si el nucleo responde que la sesion ha caducado, la cierra y avisa, de modo
 * que la interfaz vuelve a la pantalla de acceso sin dejar botones muertos.
 */
export async function conSesion<T>(accion: () => Promise<T>): Promise<T | null> {
  try {
    return await accion();
  } catch (e) {
    const err = comoError(e);
    if (err.esSesion) {
      avisar("Su sesion ha caducado. Vuelva a introducir sus credenciales.", "aviso", 7000);
      cerrarSesion();
      return null;
    }
    throw err;
  }
}

/** Vuelca en el estado el documento que se exportara o imprimira. */
export function prepararDocumento(titulo: string, subtitulo: string, pie: string): void {
  estado.documento = { titulo, subtitulo, pie };
}

/** Construye el documento con los resultados o el tablero en pantalla. */
export async function construirDocumento(
  titulo?: string,
  subtitulo?: string,
  incluirGraficos = true,
): Promise<import("./api").Documento> {
  if (!estado.token) throw new ErrorNucleo("SIN_SESION", "No hay sesion iniciada");
  const base: import("./api").Documento = {
    titulo: titulo ?? estado.documento.titulo,
    subtitulo: subtitulo ?? estado.documento.subtitulo,
    pie: estado.documento.pie,
    bloques: estado.resultados,
    secciones: [],
    widgets: estado.tablero?.widgets ?? [],
    imagenes: [],
    generadoPor: estado.usuario?.usuario ?? "",
    generadoEn: new Date().toISOString(),
    duracionMs: estado.resultadosMeta.duracionMs,
  };
  if (incluirGraficos && estado.tablero) {
    // Los graficos se capturan como PNG para incrustarlos en el PDF y el Excel.
    const { prepararImagenes } = await import("./paginas/tablero");
    base.imagenes = prepararImagenes();
  }
  return api.prepararDocumento(estado.token, base.titulo, base.subtitulo, base);
}

/** Todas las filas de resultados juntas, para busquedas y exportaciones. */
export function todasLasFilas(): Fila[] {
  return estado.resultados.flatMap((b) => b.filas);
}

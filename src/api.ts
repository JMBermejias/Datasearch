// Copyright (c) 2026 Jose Manuel Bernabeu Mejias - Licencia MIT
// Capa de comunicacion con el nucleo de DataSearch a traves de los comandos
// IPC de Tauri. Todos los tipos reflects fielmente los structs de Rust.

import { invoke } from "@tauri-apps/api/core";

// ---------------------------------------------------------------------
// Errores
// ---------------------------------------------------------------------

/** Error devuelto por el nucleo, con su codigo estable y su mensaje. */
export class ErrorNucleo extends Error {
  readonly codigo: string;

  constructor(codigo: string, mensaje: string) {
    super(mensaje);
    this.name = "ErrorNucleo";
    this.codigo = codigo;
  }

  /** `true` si el nucleo pide volver a iniciar sesion. */
  get esSesion(): boolean {
    return this.codigo === "SIN_SESION" || this.codigo === "SESION_CADUCADA";
  }

  /** `true` si es un problema de permisos. */
  get esPermiso(): boolean {
    return this.codigo === "PERMISO_DENEGADO";
  }
}

/** Normaliza cualquier excepcion de Tauri en un `ErrorNucleo`. */
export function comoError(e: unknown): ErrorNucleo {
  if (e instanceof ErrorNucleo) return e;
  if (e && typeof e === "object") {
    const v = e as { codigo?: unknown; mensaje?: unknown; message?: unknown };
    const codigo = typeof v.codigo === "string" ? v.codigo : "INTERNO";
    const mensaje =
      typeof v.mensaje === "string"
        ? v.mensaje
        : typeof v.message === "string"
          ? v.message
          : String(e);
    return new ErrorNucleo(codigo, mensaje);
  }
  return new ErrorNucleo("INTERNO", String(e));
}

/** Invoca un comando y normaliza los errores. */
async function llamar<T>(comando: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(comando, args);
  } catch (e) {
    throw comoError(e);
  }
}

// ---------------------------------------------------------------------
// Tipos
// ---------------------------------------------------------------------

export type Rol = "usuario" | "administrador";
export type Estado = "pendiente" | "activo" | "suspendido";

export interface Permisos {
  consultar: boolean;
  crearFuentes: boolean;
  modificarFuentes: boolean;
  eliminarFuentes: boolean;
  exportar: boolean;
  imprimir: boolean;
  analizar: boolean;
  crearDashboards: boolean;
  verAuditoria: boolean;
}

export interface DatosPersonales {
  nombre: string;
  apellidos: string;
  documento: string;
  email: string;
  telefono: string;
  direccion: string;
  numero: string;
  codigoPostal: string;
  poblacion: string;
  provincia: string;
  pais: string;
  fechaNacimiento: string;
  empresa: string;
  cargo: string;
  motivoSolicitud: string;
}

export interface Usuario {
  id: number;
  usuario: string;
  nombre: string;
  apellidos: string;
  documento: string;
  email: string;
  telefono: string;
  direccion: string;
  numero: string;
  codigoPostal: string;
  poblacion: string;
  provincia: string;
  pais: string;
  fechaNacimiento: string;
  empresa: string;
  cargo: string;
  estado: Estado;
  rol: Rol;
  permisos: Permisos;
  creadoEn: string;
  ultimoAcceso: string | null;
  notasAdmin: string;
}

export interface Sesion {
  token: string;
  usuario: Usuario;
  versionApp: string;
  esAdmin: boolean;
}

export type TipoFuente =
  | "sqlite"
  | "postgres"
  | "mysql"
  | "sqlserver"
  | "mongodb"
  | "csv"
  | "json"
  | "xml"
  | "excel"
  | "web";

export interface ConfigFuente {
  host: string;
  puerto: number | null;
  usuario: string;
  contrasena: string;
  baseDatos: string;
  esquema: string;
  ssl: boolean;
  tiempoEsperaSeg: number | null;
  ruta: string;
  metodo: string;
  cabeceras: Record<string, string>;
  cuerpo: string;
  rutaJson: string;
  formatoWeb: string;
  parametros: string;
  uri: string;
  coleccion: string;
}

export interface Fuente {
  id: number;
  idUsuario: number;
  nombre: string;
  descripcion: string;
  tipo: TipoFuente;
  config: ConfigFuente;
  activa: boolean;
  soloAdmin: boolean;
  creadoEn: string;
  actualizadoEn: string;
  estadoConexion: string;
}

export interface SolicitudFuente {
  nombre: string;
  descripcion: string;
  tipo: string;
  config: ConfigFuente;
  activa: boolean;
  soloAdmin: boolean;
  conservarSecreto: boolean;
}

export interface ColumnaInfo {
  nombre: string;
  tipo: string;
  etiqueta: string;
  numerica: boolean;
  esTexto: boolean;
  esFecha: boolean;
  esBooleano: boolean;
  nulos: number;
  distintos: number;
  valoresFrecuentes: Frecuencia[];
}

export interface Frecuencia {
  valor: string;
  cuenta: number;
  porcentaje: number;
}

export type Fila = Record<string, string | null>;

export interface ColumnaEsquema {
  nombre: string;
  tipo: string;
  obligatorio: boolean;
  clavePrimaria: boolean;
}

export interface Esquema {
  nombre: string;
  tipo: string;
  filasEstimadas: number;
  columnas: ColumnaEsquema[];
}

export interface BloqueResultados {
  fuenteId: number;
  fuenteNombre: string;
  tipoFuente: TipoFuente;
  esquema: string;
  filas: Fila[];
  columnas: ColumnaInfo[];
  totalFilas: number;
  filasOmitidas: number;
  duracionMs: number;
  mensaje: string;
}

export interface Sugerencia {
  etiqueta: string;
  columna: string;
  valor: string;
  confianza: number;
}

export interface RespuestaBusqueda {
  bloques: BloqueResultados[];
  totalFilas: number;
  totalBloques: number;
  duracionMs: number;
  fuentesConError: string[];
  mensaje: string;
  sugerencias: Sugerencia[];
}

export interface Filtro {
  campo: string;
  operador: string;
  valor: string;
  valor2: string;
  conector: string;
  activo: boolean;
}

export interface PeticionBusqueda {
  fuentes: number[];
  esquemas: string[];
  texto: string;
  filtros: Filtro[];
  campos: string[];
  limite: number;
  desplazamiento: number;
  ordenarPor: string;
  ordenDesc: boolean;
  modo: string;
  contarTotal: boolean;
}

export type TipoGrafico =
  | "kpi"
  | "barra"
  | "linea"
  | "area"
  | "torta"
  | "dispersion"
  | "tabla"
  | "indicador"
  | "mapaCalor";

export interface PuntoGrafico {
  etiqueta: string;
  valor: number;
  valorSecundario: number | null;
  categoria: string;
}

export interface WidgetListo {
  id: string;
  titulo: string;
  tipo: TipoGrafico;
  subtitulo: string;
  fuenteNombre: string;
  esquema: string;
  columnaGrupo: string;
  columnaValor: string;
  agregacion: string;
  formato: string;
  ancho: number;
  puntos: PuntoGrafico[];
  columnas: ColumnaInfo[];
  filas: Fila[];
  totalFilas: number;
  valorDestacado: number | null;
  textoDestacado: string | null;
  estado: string;
}

export interface Widget {
  id: string;
  titulo: string;
  tipo: TipoGrafico;
  fuenteId: number;
  esquema: string;
  columnaGrupo: string;
  columnaValor: string;
  agregacion: string;
  limite: number;
  filtros: Filtro[];
  ancho: number;
  formato: string;
  peticionTexto: string;
}

export interface Tablero {
  nombre: string;
  widgets: WidgetListo[];
  generadoEn: string;
  duracionMs: number;
  fuentes: string[];
  esquemas: string[];
  advertencias: string[];
}

export interface PeticionDashboard {
  nombre: string;
  fuentes: number[];
  esquemas: string[];
  instruccion: string;
  widgetsBase: string[];
  maxWidgets: number;
  filtros: Filtro[];
}

export type TipoAnalisis =
  | "describir"
  | "nulos"
  | "distintos"
  | "frecuencias"
  | "histograma"
  | "correlacion"
  | "outliers"
  | "tendencia"
  | "calidad"
  | "duplicados"
  | "resumen";

export interface SeccionAnalisis {
  titulo: string;
  tipo: string;
  descripcion: string;
  filas: Fila[];
  clave: Record<string, string>;
  metricas: Metrica[];
}

export interface Metrica {
  nombre: string;
  valor: string;
  unidad: string;
}

export interface RespuestaAnalisis {
  fuenteNombre: string;
  esquema: string;
  secciones: SeccionAnalisis[];
  filasAnalizadas: number;
  columnasAnalizadas: string[];
  duracionMs: number;
  resumen: string;
  advertencias: string[];
}

export interface PeticionAnalisis {
  fuenteId: number;
  esquema: string;
  columnas: string[];
  analyses: string[];
  esAnalisisConfirmado: boolean;
  filtros: Filtro[];
  limite: number;
  comentario: string;
}

export interface ImagenDocumento {
  titulo: string;
  base64: string;
  anchoPx: number;
  altoPx: number;
}

export interface Documento {
  titulo: string;
  subtitulo: string;
  pie: string;
  bloques: BloqueResultados[];
  secciones: SeccionAnalisis[];
  widgets: WidgetListo[];
  imagenes: ImagenDocumento[];
  generadoPor: string;
  generadoEn: string;
  duracionMs: number;
}

export interface ArchivoExportado {
  nombre: string;
  contenidoBase64: string;
  tamanoBytes: number;
  formato: string;
  mime: string;
}

export interface RegistroAuditoria {
  id: number;
  idUsuario: number;
  usuario: string;
  accion: string;
  detalle: string;
  exito: boolean;
  fecha: string;
}

export interface InfoVersion {
  versionActual: string;
  versionDisponible: string;
  hayActualizacion: boolean;
  urlDescarga: string;
  notas: string;
  publicadaEn: string;
  nombreLanzamiento: string;
  consultado: boolean;
}

export interface OpcionDesplegable {
  valor: string;
  etiqueta: string;
}

export interface InfoAplicacion {
  nombre: string;
  version: string;
  titulo: string;
  repositorio: string;
  licencia: string;
  autor: string;
  copyright: string;
  autorFicha: Record<string, string>;
  pie: string;
  tiposFuente: (OpcionDesplegable & {
    relacional: boolean;
    documental: boolean;
    plano: boolean;
    puertoPorDefecto: number | null;
  })[];
  operadores: (OpcionDesplegable & {
    necesitaSegundo: boolean;
    necesitaLista: boolean;
    unario: boolean;
  })[];
  analisis: OpcionDesplegable[];
  formatos: OpcionDesplegable[];
  graficos: { valor: string; nombre: string }[];
  usuarioAdminDefecto: string;
}

// ---------------------------------------------------------------------
// Comandos
// ---------------------------------------------------------------------

export const api = {
  info: () => llamar<InfoAplicacion>("info_aplicacion"),
  credencialesIniciales: () =>
    llamar<{ usuario: string; aviso: string }>("credenciales_iniciales"),

  registrar: (datos: Record<string, unknown>, autoaprobar = false) =>
    llamar<Usuario>("registrar", { peticion: datos, autoaprobar }),
  usuarioDisponible: (usuario: string) =>
    llamar<boolean>("usuario_disponible", { usuario }),
  iniciarSesion: (usuario: string, contrasena: string) =>
    llamar<Sesion>("iniciar_sesion", { credenciales: { usuario, contrasena } }),
  cerrarSesion: (token: string) => llamar<void>("cerrar_sesion", { token }),
  sesionActual: (token: string) => llamar<Usuario>("sesion_actual", { token }),
  mostrarVentana: () => llamar<void>("mostrar_ventana_principal"),

  miPerfil: (token: string) =>
    llamar<{
      usuario: Usuario;
      fuentes: Fuente[];
      auditoria: RegistroAuditoria[];
      sesionesActivas: number;
      versionApp: string;
      fichaAutor: Record<string, string>;
    }>("mi_perfil", { token }),
  actualizarMiPerfil: (token: string, datos: DatosPersonales) =>
    llamar<Usuario>("actualizar_mi_perfil", { token, datos }),
  cambiarContrasena: (token: string, actual: string, nueva: string) =>
    llamar<void>("cambiar_contrasena", { token, actual, nueva }),

  adminListarUsuarios: (token: string, estado: Estado | null, texto: string) =>
    llamar<Usuario[]>("admin_listar_usuarios", {
      token,
      filtroEstado: estado,
      texto,
    }),
  adminActualizarUsuario: (
    token: string,
    idUsuario: number,
    nuevoEstado: Estado,
    nuevoRol: Rol,
    permisos: Permisos,
    notas: string,
  ) =>
    llamar<Usuario>("admin_actualizar_usuario", {
      token,
      idUsuario,
      nuevoEstado,
      nuevoRol,
      permisos,
      notas,
    }),
  adminEliminarUsuario: (token: string, idUsuario: number) =>
    llamar<void>("admin_eliminar_usuario", { token, idUsuario }),
  adminRestablecerContrasena: (token: string, idUsuario: number, nueva: string) =>
    llamar<void>("admin_restablecer_contrasena", { token, idUsuario, nuevaContrasena: nueva }),
  adminDetalleUsuario: (token: string, idUsuario: number) =>
    llamar<{
      usuario: Usuario;
      fuentes: Fuente[];
      auditoria: RegistroAuditoria[];
      conteoFuentes: number;
      sesionesActivas: number;
    }>("admin_detalle_usuario", { token, idUsuario }),
  adminResumen: (token: string) =>
    llamar<{
      totalUsuarios: number;
      pendientes: number;
      activos: number;
      suspendidos: number;
      administradores: number;
      fuentesPorUsuario: [string, number][];
      ultimaAuditoria: RegistroAuditoria[];
    }>("admin_resumen", { token }),

  listarFuentes: (token: string) => llamar<Fuente[]>("listar_fuentes", { token }),
  crearFuente: (token: string, peticion: SolicitudFuente) =>
    llamar<Fuente>("crear_fuente", { token, peticion }),
  actualizarFuente: (token: string, idFuente: number, peticion: SolicitudFuente) =>
    llamar<Fuente>("actualizar_fuente", { token, idFuente, peticion }),
  eliminarFuente: (token: string, idFuente: number) =>
    llamar<void>("eliminar_fuente", { token, idFuente }),
  probarFuente: (token: string, idFuente: number) =>
    llamar<string>("probar_fuente", { token, idFuente }),
  esquemasDeFuente: (token: string, idFuente: number) =>
    llamar<Esquema[]>("esquemas_de_fuente", { token, idFuente }),
  sugerirFiltros: (token: string, idFuente: number, esquema: string) =>
    llamar<{
      columnas: ColumnaInfo[];
      sugerencias: Sugerencia[];
      valoresMuestra: Fila[];
    }>("sugerir_filtros", { token, idFuente, esquema }),

  buscar: (token: string, peticion: PeticionBusqueda) =>
    llamar<RespuestaBusqueda>("buscar", { token, peticion }),
  mostrarDatos: (token: string, idFuente: number, esquema: string, limite: number) =>
    llamar<Documento>("mostrar_datos", { token, idFuente, esquema, limite }),

  crearDashboard: (token: string, peticion: PeticionDashboard) =>
    llamar<Tablero>("crear_dashboard", { token, peticion }),
  recalcularWidget: (token: string, widget: Widget) =>
    llamar<WidgetListo>("recalcular_widget", { token, widget }),

  analizar: (token: string, peticion: PeticionAnalisis) =>
    llamar<RespuestaAnalisis>("analizar", { token, peticion }),
  // La peticion sin confirmar devuelve un error: sirve para comprobar que el
  // boton de confirmacion es obligatorio.
  analizarSinConfirmar: (token: string, peticion: PeticionAnalisis) =>
    llamar<RespuestaAnalisis>("analizar", {
      token,
      peticion: { ...peticion, esAnalisisConfirmado: false },
    }),

  prepararDocumento: (
    token: string,
    titulo: string,
    subtitulo: string,
    origen: Documento | null,
  ) => llamar<Documento>("preparar_documento", { token, titulo, subtitulo, origen }),
  exportar: (token: string, documento: Documento, formato: string) =>
    llamar<ArchivoExportado>("exportar", { token, documento, formato }),
  prepararImpresion: (token: string, documento: Documento) =>
    llamar<string>("preparar_impresion", { token, documento }),
  carpetaDestino: () => llamar<string>("carpeta_destino"),

  comprobarActualizaciones: () => llamar<InfoVersion>("comprobar_actualizaciones"),
  descargarActualizacion: (url: string, nombre: string | null) =>
    llamar<{ ruta: string; nombre: string; bytes: number; esDeb: boolean }>(
      "descargar_actualizacion",
      { url, nombreFichero: nombre },
    ),
  abrirActualizacion: (ruta: string) => llamar<void>("abrir_actualizacion", { ruta }),
  cerrarParaActualizar: () => llamar<void>("cerrar_para_actualizar"),
};

// Copyright (c) 2026 Jose Manuel Bernabeu Mejias - Licencia MIT
// Boton de credenciales: muestra los datos del autor y del usuario conectado,
// con edicion de los datos propios y cambio de contrasena.

import { api, type DatosPersonales, type Usuario } from "../api";
import { estado, esAdmin } from "../estado";
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
  iniciales,
  modal,
  numero,
  tablaDatos,
} from "../ui";

interface Detalle {
  usuario: Usuario;
  fuentes: { id: number; nombre: string; tipo: string; activa: boolean }[];
  auditoria: { id: number; accion: string; detalle: string; exito: boolean; fecha: string }[];
  sesionesActivas: number;
  versionApp: string;
  fichaAutor: Record<string, string>;
}

/** Abre la ventana de credenciales. */
export function abrirCredenciales(): void {
  const cuerpo = el("div", { class: "columna" });
  cuerpo.append(el("div", { class: "ayuda-texto", text: "Cargando sus datos..." }));

  const ventana = modal("Credenciales", cuerpo, {
    clase: "ancho",
    iconoNombre: "credencial",
  });

  void api
    .miPerfil(estado.token!)
    .then((d) => {
      ventana.cerrar();
      mostrarCredenciales(d);
    })
    .catch((e) => {
      avisar(comoError(e).message, "error");
    });
}

function mostrarCredenciales(d: Detalle): void {
  const cuerpo = el("div", { class: "columna" });

  // --- Ficha del autor de la aplicacion ---
  const sello = el("div", { class: "sello-autor" });
  sello.innerHTML = `
    <div class="avatar">JM</div>
    <div class="nombre">${esc(d.fichaAutor.nombre ?? estado.info?.autor ?? "Jose Manuel Bernabeu Mejias")}</div>
    <div class="detalle">${esc(d.fichaAutor.direccion ?? "")}<br>
      ${esc(d.fichaAutor.codigoPostal ?? "")} ${esc(d.fichaAutor.poblacion ?? "")},
      ${esc(d.fichaAutor.provincia ?? "")} (${esc(d.fichaAutor.pais ?? "")})<br>
      ${esc(d.fichaAutor.correo ?? "")}</div>
    <div class="detalle" style="margin-top:8px">
      Licencia ${esc(d.fichaAutor.licencia ?? estado.info?.licencia ?? "MIT")} &middot;
      Copyright (c) 2026 ${esc(d.fichaAutor.nombre ?? "Jose Manuel Bernabeu Mejias")}
    </div>`;
  cuerpo.append(sello);

  // --- Pestanas: mis datos / autor / actividad ---
  const pestanas = el("div", { class: "pestanas" });
  const panelDatos = el("div", {});
  const panelAutor = el("div", { class: 'oculto' });
  const panelActividad = el("div", { class: 'oculto' });

  const misPestana = el("button", { class: "pestana activa", type: "button", text: "Mis datos" });
  const autPestana = el("button", { class: "pestana", type: "button", text: "Autor y licencia" });
  const actPestana = el("button", { class: "pestana", type: "button", text: "Actividad" });
  pestanas.append(misPestana, autPestana, actPestana);

  const activar = (a: HTMLElement, b: HTMLElement, c: HTMLElement) => {
    a.classList.add("activa");
    b.classList.remove("activa");
    c.classList.remove("activa");
    panelDatos.classList.remove('oculto');
    panelAutor.classList.add('oculto');
    panelActividad.classList.add('oculto');
  };
  misPestana.addEventListener("click", () => activar(misPestana, autPestana, actPestana));
  autPestana.addEventListener("click", () => {
    activar(autPestana, misPestana, actPestana);
    panelDatos.classList.add('oculto');
    panelAutor.classList.remove('oculto');
    panelActividad.classList.add('oculto');
  });
  actPestana.addEventListener("click", () => {
    activar(actPestana, misPestana, autPestana);
    panelDatos.classList.add('oculto');
    panelAutor.classList.add('oculto');
    panelActividad.classList.remove('oculto');
  });

  // --- Panel: mis datos ---
  const u = d.usuario;
  const resumen = el("div", { class: "ficha" });
  const pares: [string, string][] = [
    ["Nombre de usuario", u.usuario],
    ["Nombre y apellidos", `${u.nombre} ${u.apellidos}`.trim() || "-"],
    ["Documento", u.documento || "-"],
    ["Correo electronico", u.email],
    ["Telefono", u.telefono],
    ["Direccion", `${u.direccion}${u.numero ? `, ${u.numero}` : ""}`],
    ["Codigo postal", u.codigoPostal],
    ["Poblacion", u.poblacion],
    ["Provincia", u.provincia],
    ["Pais", u.pais],
    ["Fecha de nacimiento", u.fechaNacimiento || "-"],
    ["Empresa", u.empresa || "-"],
    ["Cargo", u.cargo || "-"],
    ["Rol", u.rol === "administrador" ? "Administrador" : "Usuario"],
    ["Estado", u.estado],
    ["Cuenta creada", fecha(u.creadoEn)],
    ["Ultimo acceso", fecha(u.ultimoAcceso)],
    ["Sesiones activas", String(d.sesionesActivas)],
  ];
  for (const [k, v] of pares) {
    if (!v) continue;
    const ficha = el("div", { class: "dato-ficha" });
    ficha.innerHTML = `<div class="clave">${esc(k)}</div><div class="valor">${esc(v)}</div>`;
    resumen.append(ficha);
  }
  panelDatos.append(resumen);

  const acciones = el("div", { class: "btn-grupo", style: "margin-top:14px" });
  acciones.append(
    boton("Editar mis datos", {
      clase: "principal",
      iconoNombre: "lapiz",
      alPulsar: () => editarDatos(u),
    }),
    boton("Cambiar contrasena", { iconoNombre: "candado", alPulsar: () => cambiarContrasena() }),
    boton("Mis fuentes de datos", { clase: "fantasma", iconoNombre: "base", alPulsar: () => verFuentes(d) }),
  );
  panelDatos.append(acciones);

  // --- Panel: autor ---
  const ficha = el("div", { class: "ficha" });
  const datosAutor: [string, string][] = [
    ["Titular", d.fichaAutor.nombre ?? "-"],
    ["Direccion", d.fichaAutor.direccion ?? "-"],
    ["Codigo postal", d.fichaAutor.codigoPostal ?? "-"],
    ["Poblacion", d.fichaAutor.poblacion ?? "-"],
    ["Provincia", d.fichaAutor.provincia ?? "-"],
    ["Pais", d.fichaAutor.pais ?? "-"],
    ["Correo", d.fichaAutor.correo ?? "-"],
    ["Licencia", d.fichaAutor.licencia ?? "MIT"],
    ["Repositorio", d.fichaAutor.repositorio ?? "-"],
    ["Derechos", d.fichaAutor.derechos ?? "-"],
  ];
  for (const [k, v] of datosAutor) {
    const c = el("div", { class: "dato-ficha" });
    c.innerHTML = `<div class="clave">${esc(k)}</div><div class="valor">${esc(v)}</div>`;
    ficha.append(c);
  }
  panelAutor.append(ficha);
  panelAutor.append(
    el("div", {
      class: "caja-info",
      html: `La aplicacion se distribuye bajo la licencia <strong>MIT</strong>, escrita
             integramente en espanol. El codigo fuente completo esta disponible en el
             repositorio indicado arriba.`,
    }),
  );
  const accionesAutor = el("div", { class: "btn-grupo", style: "margin-top:12px" });
  accionesAutor.append(
    boton("Comprobar actualizaciones", {
      clase: "principal",
      iconoNombre: "actualizar",
      alPulsar: (ev) => void comprobarActualizaciones(ev.currentTarget as HTMLButtonElement),
    }),
  );
  panelAutor.append(accionesAutor);

  // --- Panel: actividad ---
  if (d.auditoria.length) {
    panelActividad.append(
      tablaDatos(
        d.auditoria.map((a) => ({
          Fecha: a.fecha,
          Accion: a.accion,
          Detalle: a.detalle,
          Resultado: a.exito ? "correcto" : "fallido",
        })),
        { filasPorPagina: 40 },
      ),
    );
  } else {
    panelActividad.append(
      el("div", { class: "ayuda-texto", text: "No hay actividad registrada para su cuenta." }),
    );
  }

  cuerpo.append(pestanas, panelDatos, panelAutor, panelActividad);

  modal("Credenciales", cuerpo, {
    clase: "ancho",
    iconoNombre: "credencial",
    pie: (cerrar) => [boton("Cerrar", { clase: "principal", alPulsar: cerrar })],
  });
}

const CAMPOS_EDITABLES: { clave: keyof DatosPersonales; etiqueta: string; tipo?: string }[] = [
  { clave: "nombre", etiqueta: "Nombre" },
  { clave: "apellidos", etiqueta: "Apellidos" },
  { clave: "documento", etiqueta: "Documento" },
  { clave: "fechaNacimiento", etiqueta: "Fecha de nacimiento", tipo: "date" },
  { clave: "email", etiqueta: "Correo electronico", tipo: "email" },
  { clave: "telefono", etiqueta: "Telefono" },
  { clave: "direccion", etiqueta: "Direccion" },
  { clave: "numero", etiqueta: "Numero / piso" },
  { clave: "codigoPostal", etiqueta: "Codigo postal" },
  { clave: "poblacion", etiqueta: "Poblacion" },
  { clave: "provincia", etiqueta: "Provincia" },
  { clave: "pais", etiqueta: "Pais" },
  { clave: "empresa", etiqueta: "Empresa" },
  { clave: "cargo", etiqueta: "Cargo" },
];

function editarDatos(u: Usuario): void {
  const cuerpo = el("div", { class: "columna" });
  const valores: DatosPersonales = {
    nombre: u.nombre,
    apellidos: u.apellidos,
    documento: u.documento,
    email: u.email,
    telefono: u.telefono,
    direccion: u.direccion,
    numero: u.numero,
    codigoPostal: u.codigoPostal,
    poblacion: u.poblacion,
    provincia: u.provincia,
    pais: u.pais,
    fechaNacimiento: u.fechaNacimiento,
    empresa: u.empresa,
    cargo: u.cargo,
    motivoSolicitud: "",
  };
  const rej = el("div", { class: "rejilla dos" });
  for (const c of CAMPOS_EDITABLES) {
    const entrada = el("input", { type: c.tipo ?? "text", value: String(valores[c.clave] ?? "") });
    entrada.addEventListener("input", () => {
      (valores as unknown as Record<string, string>)[c.clave] = entrada.value;
    });
    rej.append(campo(c.etiqueta, entrada));
  }
  cuerpo.append(rej);

  modal("Editar mis datos", cuerpo, {
    clase: "ancho",
    iconoNombre: "lapiz",
    pie: (cerrar) => [
      boton("Cancelar", { clase: "fantasma", alPulsar: cerrar }),
      boton("Guardar", {
        clase: "principal",
        iconoNombre: "guardar",
        alPulsar: async (ev) => {
          const btn = ev.currentTarget as HTMLButtonElement;
          btn.disabled = true;
          try {
            const actualizado = await api.actualizarMiPerfil(estado.token!, valores);
            if (estado.usuario) estado.usuario = actualizado;
            avisar("Sus datos se han actualizado.", "exito");
            cerrar();
            window.dispatchEvent(new CustomEvent("datasearch:refrescar-usuario"));
          } catch (e) {
            avisar(comoError(e).message, "error", 10000);
            btn.disabled = false;
          }
        },
      }),
    ],
  });
}

function cambiarContrasena(): void {
  const cuerpo = el("div", { class: "columna" });
  const actual = el("input", { type: "password" });
  const nueva = el("input", { type: "password" });
  const repetir = el("input", { type: "password" });
  cuerpo.append(
    campo("Contrasena actual", actual),
    campo("Contrasena nueva", nueva, "minimo 8 caracteres con letras y numeros"),
    campo("Repetir contrasena nueva", repetir),
  );

  modal("Cambiar contrasena", cuerpo, {
    clase: "estrecho",
    iconoNombre: "candado",
    pie: (cerrar) => [
      boton("Cancelar", { clase: "fantasma", alPulsar: cerrar }),
      boton("Cambiar", {
        clase: "principal",
        alPulsar: async (ev) => {
          if (nueva.value !== repetir.value) {
            avisar("Las contrasenas nuevas no coinciden.", "aviso");
            return;
          }
          const btn = ev.currentTarget as HTMLButtonElement;
          btn.disabled = true;
          try {
            await api.cambiarContrasena(estado.token!, actual.value, nueva.value);
            avisar("Contrasena actualizada correctamente.", "exito");
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

function verFuentes(d: Detalle): void {
  const cuerpo = el("div", { class: "columna" });
  if (!d.fuentes.length) {
    cuerpo.append(el("div", { class: "ayuda-texto", text: "No tiene ninguna fuente de datos registrada." }));
  } else {
    cuerpo.append(
      tablaDatos(
        d.fuentes.map((f) => ({
          Nombre: f.nombre,
          Tipo: f.tipo,
          Estado: f.activa ? "Activa" : "Inactiva",
        })),
      ),
    );
  }
  modal("Mis fuentes de datos", cuerpo, {
    iconoNombre: "base",
    pie: (cerrar) => [boton("Cerrar", { clase: "principal", alPulsar: cerrar })],
  });
}

async function comprobarActualizaciones(btn: HTMLButtonElement): Promise<void> {
  btn.disabled = true;
  mostrarCarga("Comprobando si hay actualizaciones...");
  try {
    const info = await api.comprobarActualizaciones();
    estado.version = info;
    if (info.hayActualizacion) {
      modal(
        "Actualizacion disponible",
        el("div", {
          class: "ayuda-texto",
          html: `Hay una nueva version disponible: <strong>v${esc(info.versionDisponible)}</strong>
                 (${esc(fecha(info.publicadaEn))}).<br><br>
                 Al reiniciar la aplicacion, o desde la ventana de actualizacion, puede
                 descargarla e instalarla automaticamente.`,
        }),
        {
          clase: "estrecho",
          iconoNombre: "actualizar",
          pie: (cerrar) => [boton("Entendido", { clase: "principal", alPulsar: cerrar })],
        },
      );
    } else if (info.consultado) {
      avisar(`Esta es la ultima version (v${info.versionActual}). No hay actualizaciones.`, "exito", 6000);
    } else {
      avisar(info.notas || "No se ha podido comprobar si hay actualizaciones.", "aviso", 7000);
    }
  } catch (e) {
    avisar(comoError(e).message, "error");
  } finally {
    esconderCarga();
    btn.disabled = false;
  }
}

/** Ficha resumida del autor, para la barra lateral. */
export function resumenAutor(): { iniciales: string; nombre: string } {
  const nombre = estado.info?.autorFicha.nombre ?? "Jose Manuel Bernabeu Mejias";
  return { iniciales: iniciales(nombre), nombre };
}

/** Numero de fuentes activas del usuario, para el contador del menu. */
export function fuentesActivas(): number {
  return estado.fuentes.filter((f) => f.activa).length;
}

/** Devuelve el numero de usuarios pendientes, si el usuario es admin. */
export async function pendientesDeVerificar(): Promise<number> {
  if (!estado.token || !esAdmin()) return 0;
  try {
    const r = await api.adminResumen(estado.token);
    return r.pendientes;
  } catch {
    return 0;
  }
}

/** Texto auxiliar con el numero de filas de la ultima busqueda. */
export function resumenResultados(): string {
  const n = estado.resultadosMeta.totalFilas;
  return n ? `${numero(n)} filas` : "sin busquedas";
}

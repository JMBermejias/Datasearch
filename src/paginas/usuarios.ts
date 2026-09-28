// Copyright (c) 2026 Jose Manuel Bernabeu Mejias - Licencia MIT
// Pagina de administracion: altas de usuarios, verificacion, permisos,
// estado de las cuentas y revision de la actividad de cada usuario.

import { api, type Estado, type Permisos, type Rol, type Usuario } from "../api";
import { estado } from "../estado";
import { icono } from "../iconos";
import {
  avisar,
  boton,
  campo,
  comoError,
  el,
  esc,
  fecha,
  modal,
  numero,
  tablaDatos,
} from "../ui";

const NOMBRES_ESTADO: Record<Estado, string> = {
  pendiente: "Pendiente",
  activo: "Activo",
  suspendido: "Suspendido",
};

const NOMBRES_PERMISO: [keyof Permisos, string][] = [
  ["consultar", "Consultar datos"],
  ["crearFuentes", "Crear fuentes de datos"],
  ["modificarFuentes", "Modificar fuentes de datos"],
  ["eliminarFuentes", "Eliminar fuentes de datos"],
  ["exportar", "Exportar resultados"],
  ["imprimir", "Imprimir resultados"],
  ["analizar", "Analizar datos"],
  ["crearDashboards", "Crear tableros de control"],
  ["verAuditoria", "Ver la auditoria"],
];

let filtroEstado: Estado | "todos" = "todos";
let filtroTexto = "";

/** Pinta la pagina de administracion. */
export function paginaUsuarios(contenedor: HTMLElement): void {
  if (estado.usuario?.rol !== "administrador") {
    contenedor.append(
      el("div", {
        class: "caja-info",
        html: "Esta seccion esta reservada al administrador del sistema.",
      }),
    );
    return;
  }

  // --- Resumen ---
  const resumen = el("div", { class: "rejilla cuatro", style: "margin-bottom:18px" });
  const idResumen = el("div", { class: "rejilla cuatro" });
  resumen.append(idResumen);
  contenedor.append(resumen);

  const tarjeta = el("div", { class: "tarjeta" });
  const cab = el("div", { class: "tarjeta-cabecera" });
  cab.insertAdjacentHTML("beforeend", icono("usuarios"));
  cab.append(el("h2", { text: "Usuarios registrados" }));
  const acciones = el("div", { class: "acciones" });

  const selEstado = el("select", { style: "width:auto" });
  for (const [v, t] of [
    ["todos", "Todos los estados"],
    ["pendiente", "Pendientes de verificar"],
    ["activo", "Activos"],
    ["suspendido", "Suspendidos"],
  ] as [string, string][]) {
    selEstado.append(el("option", { value: v, text: t, selected: filtroEstado === v }));
  }
  selEstado.addEventListener("change", () => {
    filtroEstado = selEstado.value as Estado | "todos";
    cargar();
  });
  const buscador = el("input", { type: "search", placeholder: "Buscar usuario, nombre o correo", style: "width:210px" });
  buscador.addEventListener("input", () => {
    filtroTexto = buscador.value;
    window.clearTimeout(temporizador);
    temporizador = window.setTimeout(() => void cargar(), 320);
  });

  acciones.append(boton("Refrescar", { clase: "fantasma pequeno", iconoNombre: "refrescar", alPulsar: () => void cargar() }));
  cab.append(acciones);
  tarjeta.append(cab);

  const cuerpo = el("div", { class: "tarjeta-cuerpo" });
  const filtros = el("div", { class: "rejilla dos" });
  filtros.append(campo("Estado", selEstado));
  filtros.append(campo("Buscar", buscador));
  cuerpo.append(filtros);
  const zona = el("div", { id: "zona-usuarios", style: "margin-top:12px" });
  cuerpo.append(zona);
  tarjeta.append(cuerpo);
  contenedor.append(tarjeta);

  const actividad = el("div", { class: "tarjeta" });
  const cabAct = el("div", { class: "tarjeta-cabecera" });
  cabAct.insertAdjacentHTML("beforeend", icono("reloj"));
  cabAct.append(el("h3", { text: "Ultima actividad registrada" }));
  actividad.append(cabAct);
  const cuerpoAct = el("div", { class: "tarjeta-cuerpo", id: "zona-auditoria" });
  actividad.append(cuerpoAct);
  contenedor.append(actividad);

  void cargar();

  async function cargar(): Promise<void> {
    try {
      const [r, usuarios] = await Promise.all([
        api.adminResumen(estado.token!),
        api.adminListarUsuarios(
          estado.token!,
          filtroEstado === "todos" ? null : filtroEstado,
          filtroTexto,
        ),
      ]);
      pintarResumen(idResumen, r);
      pintarUsuarios(zona, usuarios);
      pintarAuditoria(cuerpoAct, r.ultimaAuditoria);
    } catch (e) {
      avisar(comoError(e).message, "error", 10000);
    }
  }
}

let temporizador: number | undefined;

function pintarResumen(
  contenedor: HTMLElement,
  r: Awaited<ReturnType<typeof api.adminResumen>>,
): void {
  while (contenedor.firstChild) contenedor.removeChild(contenedor.firstChild);
  const tarjetas: [string, number][] = [
    ["Usuarios registrados", r.totalUsuarios],
    ["Pendientes de verificar", r.pendientes],
    ["Activos", r.activos],
    ["Suspendidos", r.suspendidos],
    ["Administradores", r.administradores],
  ];
  for (const [titulo, valor] of tarjetas) {
    const t = el("div", { class: "dato-ficha" });
    t.innerHTML = `<div class="clave">${esc(titulo)}</div>
      <div class="valor" style="font-size:24px;font-weight:800;color:var(--azul-oscuro)">${numero(valor)}</div>`;
    contenedor.append(t);
  }
  // Fuentes por usuario.
  const t = el("div", { class: "dato-ficha" });
  const max = Math.max(1, ...r.fuentesPorUsuario.map(([, n]) => n));
  t.innerHTML = `<div class="clave">Fuentes de datos por usuario</div>
    <div class="valor texto-pequeno" style="font-weight:400">${r.fuentesPorUsuario
      .map(
        ([u, n]) =>
          `<div style="display:flex;gap:6px;align-items:center;margin-top:2px">
             <span style="flex:0 0 96px;overflow:hidden;text-overflow:ellipsis">${esc(u)}</span>
             <span style="flex:1;height:7px;background:#e0f2fe;border-radius:4px;overflow:hidden">
               <span style="display:block;height:100%;width:${(n / max) * 100}%;background:#0ea5e9"></span>
             </span>
             <span style="flex:0 0 26px;text-align:right">${n}</span>
           </div>`,
      )
      .join("") || "Sin fuentes registradas"}</div>`;
  contenedor.append(t);
}

function pintarUsuarios(contenedor: HTMLElement, usuarios: Usuario[]): void {
  while (contenedor.firstChild) contenedor.removeChild(contenedor.firstChild);
  if (!usuarios.length) {
    contenedor.append(
      el("div", { class: "vacio-estado", html: `${icono("usuarios")}<div class="titulo">No hay usuarios que coincidan</div>` }),
    );
    return;
  }

  const tabla = el("table", { class: "datos" });
  tabla.innerHTML = `<thead><tr>
      <th>Usuario</th><th>Nombre</th><th>Correo</th><th>Rol</th><th>Estado</th>
      <th>Fuentes</th><th>Ultimo acceso</th><th>Acciones</th>
    </tr></thead>`;
  const tbody = el("tbody");
  for (const u of usuarios) {
    const tr = el("tr");
    const insigniaEstado = `<span class="insignia ${
      u.estado === "activo" ? "exito" : u.estado === "pendiente" ? "aviso" : "error"
    }">${NOMBRES_ESTADO[u.estado]}</span>`;
    const insigniaRol =
      u.rol === "administrador"
        ? '<span class="insignia azul">Administrador</span>'
        : '<span class="insignia">Usuario</span>';
    tr.innerHTML = `
      <td><strong>${esc(u.usuario)}</strong></td>
      <td>${esc(`${u.nombre} ${u.apellidos}`.trim() || "-")}</td>
      <td>${esc(u.email)}</td>
      <td>${insigniaRol}</td>
      <td>${insigniaEstado}</td>
      <td class="numero" data-fuentes="${u.id}">-</td>
      <td>${esc(fecha(u.ultimoAcceso))}</td>
      <td></td>`;
    const celdaAcciones = tr.lastElementChild as HTMLElement;
    celdaAcciones.append(
      boton("Gestionar", {
        clase: "pequeno principal",
        iconoNombre: "engranaje",
        alPulsar: () => void gestionar(u),
      }),
      boton("Ver", {
        clase: "pequeno fantasma",
        iconoNombre: "ojo",
        alPulsar: () => void detalle(u),
      }),
    );
    tbody.append(tr);
  }
  tabla.append(tbody);
  const marco = el("div", { class: "tabla-marco" });
  marco.append(tabla);
  contenedor.append(marco);
  void usuarios;
}

function pintarAuditoria(
  contenedor: HTMLElement,
  registros: { id: number; usuario: string; accion: string; detalle: string; exito: boolean; fecha: string }[],
): void {
  while (contenedor.firstChild) contenedor.removeChild(contenedor.firstChild);
  if (!registros.length) {
    contenedor.append(el("div", { class: "ayuda-texto", text: "No hay actividad registrada." }));
    return;
  }
  contenedor.append(
    tablaDatos(
      registros.map((r) => ({
        Fecha: fecha(r.fecha),
        Usuario: r.usuario,
        Accion: r.accion,
        Detalle: r.detalle,
        Resultado: r.exito ? "correcto" : "fallido",
      })),
      { filasPorPagina: 30 },
    ),
  );
}

// ---------------------------------------------------------------------
// Gestion de una cuenta
// ---------------------------------------------------------------------

function gestionar(u: Usuario): void {
  const cuerpo = el("div", { class: "columna" });

  cuerpo.append(
    el("div", {
      class: "sello-autor",
      html: `<div class="avatar">${esc((u.nombre || u.usuario).slice(0, 1).toUpperCase())}</div>
        <div class="nombre">${esc(u.usuario)}</div>
        <div class="detalle">${esc(`${u.nombre} ${u.apellidos}`.trim())} &middot; ${esc(u.email)}<br>
          Registrado el ${esc(fecha(u.creadoEn))}</div>`,
    }),
  );

  const selEstado = el("select", {});
  for (const [v, t] of [
    ["pendiente", "Pendiente de verificacion"],
    ["activo", "Activo (puede iniciar sesion)"],
    ["suspendido", "Suspendido (acceso bloqueado)"],
  ] as [Estado, string][]) {
    selEstado.append(el("option", { value: v, text: t, selected: u.estado === v }));
  }
  const selRol = el("select", {});
  for (const [v, t] of [
    ["usuario", "Usuario"],
    ["administrador", "Administrador"],
  ] as [Rol, string][]) {
    selRol.append(el("option", { value: v, text: t, selected: u.rol === v }));
  }
  const notas = el("textarea", { value: u.notasAdmin, placeholder: "Notas internas sobre esta cuenta" });

  const rej = el("div", { class: "rejilla dos" });
  rej.append(campo("Estado de la cuenta", selEstado));
  rej.append(campo("Rol", selRol));
  cuerpo.append(rej);
  cuerpo.append(campo("Notas del administrador", notas));

  cuerpo.append(el("div", { class: "separador" }));
  cuerpo.append(el("div", { class: "seccion-titulo", text: "Permisos asignados" }));
  const permisos = { ...u.permisos };
  const listaPermisos = el("div", { class: "rejilla dos" });
  for (const [clave, etiqueta] of NOMBRES_PERMISO) {
    const casilla = el("input", { type: "checkbox" });
    casilla.checked = permisos[clave];
    casilla.addEventListener("change", () => {
      permisos[clave] = casilla.checked;
    });
    const caja = el("label", { class: "campo-checkbox" });
    caja.append(casilla, el("span", { text: etiqueta }));
    listaPermisos.append(caja);
  }
  cuerpo.append(listaPermisos);

  const selTodos = el("input", { type: "checkbox" });
  selTodos.addEventListener("change", () => {
    for (const [clave] of NOMBRES_PERMISO) permisos[clave] = selTodos.checked;
    const inputs = listaPermisos.querySelectorAll("input");
    inputs.forEach((i) => ((i as HTMLInputElement).checked = selTodos.checked));
  });
  const cajaTodos = el("label", { class: "campo-checkbox" });
  cajaTodos.append(selTodos, el("span", { text: "Conceder todos los permisos" }));
  cuerpo.append(cajaTodos);

  const accionesExtra = el("div", { class: "btn-grupo", style: "margin-top:14px" });
  accionesExtra.append(
    boton("Restablecer contrasena", {
      clase: "fantasma",
      iconoNombre: "candado",
      alPulsar: () => restablecer(u),
    }),
  );
  if (u.usuario !== "JMBernabeu") {
    accionesExtra.append(
      boton("Eliminar cuenta", {
        clase: "peligro",
        iconoNombre: "basura",
        alPulsar: () =>
          modal(
            `Eliminar la cuenta de ${u.usuario}`,
            el("div", {
              class: "ayuda-texto",
              text: "Se eliminaran la cuenta, sus fuentes de datos y sus sesiones. Esta accion no se puede deshacer.",
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
                      await api.adminEliminarUsuario(estado.token!, u.id);
                      avisar(`La cuenta de ${u.usuario} ha sido eliminada.`, "exito");
                      window.dispatchEvent(new CustomEvent("datasearch:refrescar-usuarios"));
                    } catch (e) {
                      avisar(comoError(e).message, "error", 10000);
                    }
                  },
                }),
              ],
            },
          ),
      }),
    );
  }
  cuerpo.append(accionesExtra);

  modal(`Gestionar la cuenta ${u.usuario}`, cuerpo, {
    clase: "ancho",
    iconoNombre: "usuarios",
    pie: (cerrar) => [
      boton("Cancelar", { clase: "fantasma", alPulsar: cerrar }),
      boton("Guardar cambios", {
        clase: "principal",
        iconoNombre: "guardar",
        alPulsar: async (ev) => {
          const btn = ev.currentTarget as HTMLButtonElement;
          btn.disabled = true;
          try {
            await api.adminActualizarUsuario(
              estado.token!,
              u.id,
              selEstado.value as Estado,
              selRol.value as Rol,
              permisos as Permisos,
              notas.value.trim(),
            );
            avisar(`Cuenta de ${u.usuario} actualizada.`, "exito");
            cerrar();
            window.dispatchEvent(new CustomEvent("datasearch:refrescar-usuarios"));
          } catch (e) {
            avisar(comoError(e).message, "error", 10000);
            btn.disabled = false;
          }
        },
      }),
    ],
  });
}

function restablecer(u: Usuario): void {
  const cuerpo = el("div", { class: "columna" });
  const nueva = el("input", { type: "text", placeholder: "Nueva contrasena" });
  const repetir = el("input", { type: "text", placeholder: "Repita la contrasena" });
  cuerpo.append(
    el("div", {
      class: "ayuda-texto",
      html: `Se establecera una nueva contrasena para <strong>${esc(u.usuario)}</strong> y se
             cerraran todas sus sesiones abiertas.<br>Comunicasela por un canal seguro.`,
    }),
    campo("Nueva contrasena", nueva),
    campo("Repetir contrasena", repetir),
  );
  modal(`Restablecer la contrasena de ${u.usuario}`, cuerpo, {
    clase: "estrecho",
    iconoNombre: "candado",
    pie: (cerrar) => [
      boton("Cancelar", { clase: "fantasma", alPulsar: cerrar }),
      boton("Restablecer", {
        clase: "principal",
        alPulsar: async () => {
          if (nueva.value !== repetir.value) {
            avisar("Las contrasenas no coinciden.", "aviso");
            return;
          }
          try {
            await api.adminRestablecerContrasena(estado.token!, u.id, nueva.value);
            avisar(`Contrasena de ${u.usuario} restablecida.`, "exito");
            cerrar();
          } catch (e) {
            avisar(comoError(e).message, "error");
          }
        },
      }),
    ],
  });
}

async function detalle(u: Usuario): Promise<void> {
  const cuerpo = el("div", { class: "columna" });
  cuerpo.append(el("div", { class: "ayuda-texto", text: "Cargando informacion de la cuenta..." }));
  const ventana = modal(`Ficha de ${u.usuario}`, cuerpo, { clase: "ancho", iconoNombre: "ojo" });

  try {
    const d = await api.adminDetalleUsuario(estado.token!, u.id);
    while (cuerpo.firstChild) cuerpo.removeChild(cuerpo.firstChild);

    const ficha = el("div", { class: "ficha" });
    const datos: [string, string][] = [
      ["Usuario", d.usuario.usuario],
      ["Nombre", `${d.usuario.nombre} ${d.usuario.apellidos}`.trim()],
      ["Documento", d.usuario.documento || "-"],
      ["Email", d.usuario.email],
      ["Telefono", d.usuario.telefono],
      ["Direccion", `${d.usuario.direccion}${d.usuario.numero ? `, ${d.usuario.numero}` : ""}`],
      ["Poblacion", `${d.usuario.codigoPostal} ${d.usuario.poblacion}`],
      ["Provincia", d.usuario.provincia],
      ["Pais", d.usuario.pais],
      ["Empresa", d.usuario.empresa || "-"],
      ["Cargo", d.usuario.cargo || "-"],
      ["Fecha de nacimiento", d.usuario.fechaNacimiento || "-"],
      ["Estado", NOMBRES_ESTADO[d.usuario.estado]],
      ["Rol", d.usuario.rol === "administrador" ? "Administrador" : "Usuario"],
      ["Creada", fecha(d.usuario.creadoEn)],
      ["Ultimo acceso", fecha(d.usuario.ultimoAcceso)],
      ["Sesiones activas", String(d.sesionesActivas)],
      ["Notas", d.usuario.notasAdmin || "-"],
    ];
    for (const [k, v] of datos) {
      if (!v) continue;
      const c = el("div", { class: "dato-ficha" });
      c.innerHTML = `<div class="clave">${esc(k)}</div><div class="valor">${esc(v)}</div>`;
      ficha.append(c);
    }
    cuerpo.append(ficha);

    cuerpo.append(el("div", { class: "seccion-titulo", text: "Fuentes de datos" }));
    cuerpo.append(
      d.fuentes.length
        ? tablaDatos(
            d.fuentes.map((f) => ({
              Nombre: f.nombre,
              Tipo: f.tipo,
              Estado: f.activa ? "Activa" : "Inactiva",
              Creada: fecha(f.creadoEn),
            })),
          )
        : el("div", { class: "ayuda-texto", text: "No ha registrado ninguna fuente." }),
    );

    cuerpo.append(el("div", { class: "seccion-titulo", text: "Actividad reciente" }));
    cuerpo.append(
      d.auditoria.length
        ? tablaDatos(
            d.auditoria.map((a) => ({
              Fecha: fecha(a.fecha),
              Accion: a.accion,
              Detalle: a.detalle,
              Resultado: a.exito ? "correcto" : "fallido",
            })),
            { filasPorPagina: 25 },
          )
        : el("div", { class: "ayuda-texto", text: "Sin actividad registrada." }),
    );
    ventana.cuerpo.scrollTop = 0;
  } catch (e) {
    avisar(comoError(e).message, "error");
  }
}

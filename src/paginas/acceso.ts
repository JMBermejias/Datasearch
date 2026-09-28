// Copyright (c) 2026 Jose Manuel Bernabeu Mejias - Licencia MIT
// Pantalla de acceso: inicio de sesion y registro de cuentas nuevas.

import { api, type DatosPersonales, type Usuario } from "../api";
import { estado, notificar, recordarToken } from "../estado";
import { icono } from "../iconos";
import {
  avisar,
  boton,
  campo,
  comoError,
  el,
  esc,
  modal,
  mostrarCarga,
  ocultarCarga,
} from "../ui";

const CARACTERISTICAS = [
  "Bases de datos locales, remotas y de la web",
  "Filtros combinables con Y / O sobre cualquier columna",
  "Tableros de control generados a medida",
  "Analisis estadistico con confirmacion previa",
  "Exportacion a Excel, PDF, CSV, JSON e impresion",
  "Control de acceso por usuarios y permisos",
];

/** Pinta la pantalla de acceso en el contenedor indicado. */
export function pantallaAcceso(contenedor: HTMLElement): void {
  limpiarPantalla(contenedor);

  const raiz = el("div", { class: "acceso" });

  // -------------------------------------------------- Columna izquierda
  const izq = el("div", { class: "acceso-lateral" });
  const marca = el("div", { class: "acceso-marca" });
  marca.insertAdjacentHTML(
    "beforeend",
    `<div class="logo">${icono("graficos")}</div>
     <div><h1>${esc(estado.info?.nombre ?? "DataSearch")}</h1>
     <span class="version">v${esc(estado.info?.version ?? "1.0.0")}</span></div>`,
  );
  izq.append(marca);
  izq.append(el("h2", { text: "Sus datos, de cualquier origen, en un solo panel" }));
  izq.append(
    el("p", {
      text:
        "Conecte tantas fuentes de datos como necesite, filtre con las condiciones que usted indique y obtenga resultados, analisis y tableros listos para imprimir o exportar.",
    }),
  );
  const lista = el("div", { class: "acceso-caracteristicas" });
  for (const c of CARACTERISTICAS) {
    const fila = el("div");
    fila.insertAdjacentHTML("beforeend", icono("exito"));
    fila.append(el("span", { text: c }));
    lista.append(fila);
  }
  izq.append(lista);

  // -------------------------------------------------- Columna derecha
  const der = el("div", { class: "acceso-derecha" });
  const tarjeta = el("div", { class: "acceso-tarjeta" });

  const pestanas = el("div", { class: "pestanas" });
  const pestanaAcceso = el("button", { class: "pestana activa", type: "button", text: "Acceder" });
  const pestanaRegistro = el("button", { class: "pestana", type: "button", text: "Registrarse" });
  pestanas.append(pestanaAcceso, pestanaRegistro);
  tarjeta.append(pestanas);

  const panelAcceso = el("div", {});
  const panelRegistro = el("div", { class: 'oculto' });
  tarjeta.append(panelAcceso, panelRegistro);

  pintarAcceso(panelAcceso);
  pintarRegistro(panelRegistro);

  pestanaAcceso.addEventListener("click", () => {
    pestanaAcceso.classList.add("activa");
    pestanaRegistro.classList.remove("activa");
    panelAcceso.classList.remove('oculto');
    panelRegistro.classList.add('oculto');
  });
  pestanaRegistro.addEventListener("click", () => {
    pestanaRegistro.classList.add("activa");
    pestanaAcceso.classList.remove("activa");
    panelRegistro.classList.remove('oculto');
    panelAcceso.classList.add('oculto');
  });

  der.append(tarjeta);
  raiz.append(izq, der);
  contenedor.append(raiz);
  setTimeout(() => (panelAcceso.querySelector("input") as HTMLInputElement)?.focus(), 120);
}

function limpiarPantalla(contenedor: HTMLElement): void {
  while (contenedor.firstChild) contenedor.removeChild(contenedor.firstChild);
}

// ---------------------------------------------------------------------
// Acceso
// ---------------------------------------------------------------------

function pintarAcceso(panel: HTMLElement): void {
  panel.innerHTML = `
    <h2>Acceso a la aplicacion</h2>
    <div class="intro">Introduzca su nombre de usuario y su contrasena para entrar.</div>`;

  const usuario = el("input", {
    type: "text",
    placeholder: "Nombre de usuario",
    autocomplete: "username",
  });
  const contrasena = el("input", {
    type: "password",
    placeholder: "Contrasena",
    autocomplete: "current-password",
  });
  const recordar = el("input", { type: "checkbox" });
  const cajaRecordar = el("label", { class: "campo-checkbox" });
  cajaRecordar.append(recordar, el("span", { text: "Recordar mi acceso en este equipo" }));

  panel.append(campo("Usuario", usuario));
  panel.append(campo("Contrasena", contrasena));
  panel.append(cajaRecordar);

  const entrar = boton("Entrar", {
    clase: "principal bloque",
    iconoNombre: "candado",
    tipo: "submit",
  });
  const formulario = el("form", { class: "columna" });
  panel.appendChild(formulario);
  formulario.append(entrar);

  const error = el("div", { class: "caja-info oculto" });
  panel.append(error);

  formulario.addEventListener("submit", async (e) => {
    e.preventDefault();
    error.classList.add('oculto');
    if (!usuario.value.trim() || !contrasena.value) {
      mostrarError(error, "Indique su usuario y su contrasena.");
      return;
    }
    ocultarCarga();
    mostrarCarga("Comprobando credenciales...");
    try {
      const sesion = await api.iniciarSesion(usuario.value.trim(), contrasena.value);
      if (recordar.checked) recordarToken(sesion.token);
      else {
        estado.token = sesion.token;
        try {
          sessionStorage.setItem("datasearch.token", sesion.token);
        } catch {
          /* sin almacenamiento: la sesion durara lo que dure la pestana */
        }
      }
      estado.token = sesion.token;
      estado.usuario = sesion.usuario;
      avisar(`Bienvenido, ${sesion.usuario.nombre || sesion.usuario.usuario}.`, "exito", 3500);
      notificar();
    } catch (e2) {
      const err = comoError(e2);
      mostrarError(error, err.message);
    } finally {
      ocultarCarga();
    }
  });

  // Recordatorio de la cuenta de administrador inicial.
  const recordatorio = el("div", { class: "caja-info" });
  recordatorio.innerHTML = `
    <strong>Primer acceso</strong><br>
    La cuenta de administrador se crea automaticamente la primera vez que se abre la
    aplicacion. Su nombre de usuario es <code>${esc(
      estado.info?.usuarioAdminDefecto ?? "JMBernabeu",
    )}</code> con la contrasena inicial definida por el autor.<br>
    Se recomienda cambiar la contrasena desde <em>Credenciales</em> tras el primer acceso.`;
  panel.append(recordatorio);
}

function mostrarError(nodo: HTMLElement, mensaje: string): void {
  nodo.classList.remove('oculto');
  nodo.innerHTML = `<strong style="color:#dc2626">No se ha podido entrar</strong><br>${esc(mensaje)}`;
}

// ---------------------------------------------------------------------
// Registro
// ---------------------------------------------------------------------

interface DefinicionCampo {
  clave: keyof DatosPersonales;
  etiqueta: string;
  tipo?: string;
  obligatorio?: boolean;
  ancho?: "completo" | "medio";
  ayuda?: string;
}

const CAMPOS_PERSONALES: DefinicionCampo[] = [
  { clave: "nombre", etiqueta: "Nombre", obligatorio: true },
  { clave: "apellidos", etiqueta: "Apellidos", obligatorio: true },
  { clave: "documento", etiqueta: "Documento de identidad", ayuda: "DNI o NIE" },
  { clave: "fechaNacimiento", etiqueta: "Fecha de nacimiento", tipo: "date" },
  { clave: "email", etiqueta: "Correo electronico", tipo: "email", obligatorio: true },
  { clave: "telefono", etiqueta: "Telefono", tipo: "tel", obligatorio: true },
  { clave: "direccion", etiqueta: "Direccion", tipo: "text", obligatorio: true, ancho: "completo" },
  { clave: "numero", etiqueta: "Numero / piso", ayuda: "2 C" },
  { clave: "codigoPostal", etiqueta: "Codigo postal", obligatorio: true },
  { clave: "poblacion", etiqueta: "Poblacion", obligatorio: true },
  { clave: "provincia", etiqueta: "Provincia", obligatorio: true },
  { clave: "pais", etiqueta: "Pais", obligatorio: true },
  { clave: "empresa", etiqueta: "Empresa" },
  { clave: "cargo", etiqueta: "Cargo o puesto" },
];

function pintarRegistro(panel: HTMLElement): void {
  panel.innerHTML = `
    <h2>Crear una cuenta nueva</h2>
    <div class="intro">
      Para acceder a DataSearch es obligatorio registrarse. Indique todos sus datos
      personales: un administrador los verificara antes de activar su cuenta.
    </div>`;

  const usuario = el("input", { type: "text", placeholder: "usuario", autocomplete: "username" });
  const contrasena = el("input", { type: "password", autocomplete: "new-password" });
  const repetir = el("input", { type: "password", autocomplete: "new-password" });
  const motivo = el("textarea", {
    placeholder: "Explique para que necesita utilizar DataSearch",
  });

  const rejilla = el("div", { class: "rejilla dos" });
  for (const c of CAMPOS_PERSONALES) {
    const entrada = el("input", {
      type: c.tipo ?? "text",
      "data-clave": c.clave,
    });
    rejilla.append(campo(`${c.etiqueta}${c.obligatorio ? " *" : ""}`, entrada, c.ayuda ?? ""));
  }
  panel.append(campo("Nombre de usuario *", usuario));
  panel.append(el("div", { class: "rejilla dos" }));
  const dos = panel.lastElementChild as HTMLElement;
  dos.append(campo("Contrasena *", contrasena, "Minimo 8 caracteres con letras y numeros"));
  dos.append(campo("Repetir contrasena *", repetir));
  panel.append(rejilla);
  panel.append(campo("Motivo de la solicitud *", motivo));
  panel.append(
    el("div", {
      class: "ayuda-texto",
      style: "margin:10px 0",
      html: `Los campos marcados con <strong>*</strong> son obligatorios. La cuenta queda
             <strong>pendiente</strong> hasta que el administrador la verifique.`,
    }),
  );

  const enviar = boton("Enviar solicitud de registro", {
    clase: "principal bloque",
    iconoNombre: "guardar",
  });
  panel.append(enviar);

  const error = el("div", { class: "caja-info oculto" });
  panel.append(error);

  // Comprobacion de disponibilidad del nombre de usuario mientras se escribe.
  const avisoDisp = el("div", { class: "ayuda-texto", style: "min-height:16px" });
  panel.insertBefore(avisoDisp, panel.querySelector(".rejilla.dos"));
  let temporizador: number | undefined;
  usuario.addEventListener("input", () => {
    window.clearTimeout(temporizador);
    if (usuario.value.trim().length < 3) {
      avisoDisp.textContent = "";
      return;
    }
    temporizador = window.setTimeout(async () => {
      try {
        const libre = await api.usuarioDisponible(usuario.value.trim());
        avisoDisp.textContent = libre
          ? "Nombre disponible."
          : "Ese nombre de usuario ya esta registrado.";
        avisoDisp.style.color = libre ? "#059669" : "#dc2626";
      } catch {
        avisoDisp.textContent = "";
      }
    }, 320);
  });

  enviar.addEventListener("click", async () => {
    error.classList.add('oculto');
    const datos: Record<string, string> = {};
    for (const c of CAMPOS_PERSONALES) {
      const entrada = rejilla.querySelector(`[data-clave="${c.clave}"]`) as HTMLInputElement;
      datos[c.clave] = entrada?.value.trim() ?? "";
    }

    if (!usuario.value.trim() || !contrasena.value || !motivo.value.trim()) {
      mostrarError(error, "Complete el nombre de usuario, la contrasena y el motivo de la solicitud.");
      return;
    }
    if (contrasena.value.length < 8) {
      mostrarError(error, "La contrasena debe tener al menos 8 caracteres.");
      return;
    }
    if (contrasena.value !== repetir.value) {
      mostrarError(error, "Las contrasenas no coinciden.");
      return;
    }

    mostrarCarga("Enviando la solicitud...");
    try {
      const creado = await api.registrar({
        usuario: usuario.value.trim(),
        contrasena: contrasena.value,
        repetirContrasena: repetir.value,
        motivoSolicitud: motivo.value.trim(),
        ...datos,
      });
      ocultarCarga();
      mostrarRegistroOk(creado, panel);
    } catch (e2) {
      ocultarCarga();
      mostrarError(error, comoError(e2).message);
    }
  });
}

function mostrarRegistroOk(usuario: Usuario, panel: HTMLElement): void {
  const cuerpo = el("div", { class: "columna" });
  cuerpo.innerHTML = `
    <div class="sello-autor">
      <div class="avatar">${esc((usuario.nombre || usuario.usuario).slice(0, 1).toUpperCase())}</div>
      <div class="nombre">Solicitud registrada</div>
      <div class="detalle">
        La cuenta <strong>${esc(usuario.usuario)}</strong> ha sido creada y esta
        <strong>pendiente de verificacion</strong>.<br>
        El administrador debera aprobarla antes de que pueda iniciar sesion.
      </div>
    </div>
    <div class="caja-info">
      Puede cerrar esta ventana. En cuanto el administrador active su cuenta,
      podra entrar con su nombre de usuario y su contrasena.
    </div>`;
  modal("Registro completado", cuerpo, {
    clase: "estrecho",
    iconoNombre: "exito",
    pie: (cerrar) => [boton("Entendido", { clase: "principal", alPulsar: cerrar })],
    alCerrar: () => {
      panel.innerHTML = "";
      panel.append(
        el("div", { class: "caja-info", html: "Su solicitud esta registrada. Quedo a la espera de que el administrador la verifique." }),
      );
    },
  });
}

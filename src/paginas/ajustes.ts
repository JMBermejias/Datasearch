// Copyright (c) 2026 Jose Manuel Bernabeu Mejias - Licencia MIT
// Pagina de ajustes: informacion del sistema, carpeta de descargas,
// comprobacion de actualizaciones e instalacion.

import { api, type InfoVersion } from "../api";
import { estado } from "../estado";
import { icono } from "../iconos";
import {
  avisar,
  barraInformacion,
  barraProgreso,
  boton,
  comoError,
  el,
  esc,
  esconderCarga,
  fecha,
  mostrarCarga,
  numero,
  textoCarga,
  tamano,
  vacio,
} from "../ui";

/** Pinta la pagina de ajustes. */
export function paginaAjustes(contenedor: HTMLElement): void {
  // --- Informacion del sistema ---
  const tarjeta = el("div", { class: "tarjeta" });
  const cab = el("div", { class: "tarjeta-cabecera" });
  cab.insertAdjacentHTML("beforeend", icono("engranaje"));
  cab.append(el("h2", { text: "Informacion de la aplicacion" }));
  tarjeta.append(cab);
  const cuerpo = el("div", { class: "tarjeta-cuerpo" });

  const ficha = el("div", { class: "ficha" });
  const datos: [string, string][] = [
    ["Aplicacion", estado.info?.nombre ?? "DataSearch"],
    ["Version instalada", `v${estado.info?.version ?? "1.0.0"}`],
    ["Titular", estado.info?.autor ?? "Jose Manual Bernabeu Mejias"],
    ["Licencia", estado.info?.licencia ?? "MIT"],
    ["Copyright", estado.info?.copyright ?? ""],
    ["Repositorio", estado.info?.repositorio ?? ""],
    ["Fuentes registradas", String(estado.fuentes.length)],
    ["Sesion iniciada", estado.usuario ? fecha(new Date().toISOString()) : "-"],
  ];
  for (const [k, v] of datos) {
    if (!v) continue;
    const c = el("div", { class: "dato-ficha" });
    c.innerHTML = `<div class="clave">${esc(k)}</div><div class="valor">${esc(v)}</div>`;
    ficha.append(c);
  }
  cuerpo.append(ficha);
  tarjeta.append(cuerpo);
  contenedor.append(tarjeta);

  // --- Carpetas ---
  const carpeta = el("div", { class: "tarjeta" });
  const cab2 = el("div", { class: "tarjeta-cabecera" });
  cab2.insertAdjacentHTML("beforeend", icono("carpeta"));
  cab2.append(el("h2", { text: "Carpetas del sistema" }));
  const acciones2 = el("div", { class: "acciones" });
  const ruta = el("div", { class: "texto-pequeno texto-suave", id: "ruta-descargas", text: " consultando..." });
  acciones2.append(ruta);
  cab2.append(acciones2);
  carpeta.append(cab2);
  const cuerpo2 = el("div", { class: "tarjeta-cuerpo" });
  cuerpo2.append(
    el("div", {
      class: "ayuda-texto",
      html: `Los ficheros exportados y las actualizaciones descargadas se guardan en la carpeta
             de descargas del sistema. En Linux suele ser <code>~/Descargas</code> o
             <code>~/Downloads</code>.`,
    }),
  );
  carpeta.append(cuerpo2);
  contenedor.append(carpeta);
  void api
    .carpetaDestino()
    .then((r) => {
      const n = document.querySelector("#ruta-descargas");
      if (n) n.textContent = r;
    })
    .catch(() => undefined);

  // --- Actualizaciones ---
  const act = el("div", { class: "tarjeta" });
  const cab3 = el("div", { class: "tarjeta-cabecera" });
  cab3.insertAdjacentHTML("beforeend", icono("actualizar"));
  cab3.append(el("h2", { text: "Actualizaciones" }));
  const acciones3 = el("div", { class: "acciones" });
  const btnComprobar = boton("Comprobar ahora", {
    clase: "fantasma pequeno",
    iconoNombre: "refrescar",
    alPulsar: (e) => void comprobar(e.currentTarget as HTMLButtonElement),
  });
  acciones3.append(btnComprobar);
  cab3.append(acciones3);
  act.append(cab3);

  const cuerpo3 = el("div", { class: "tarjeta-cuerpo" });
  const zonaVersion = el("div", { id: "zona-version" });
  cuerpo3.append(zonaVersion);
  act.append(cuerpo3);
  contenedor.append(act);

  pintarVersion(zonaVersion, btnComprobar);

  act.append(
    barraInformacion(
      "La aplicacion comprueba las actualizaciones al arrancar. Si hay una version nueva, puede descargarla e instalarla desde aqui.",
    ),
  );
  contenedor.append(act);

  // --- Licencia ---
  const lic = el("div", { class: "tarjeta" });
  const cab4 = el("div", { class: "tarjeta-cabecera" });
  cab4.insertAdjacentHTML("beforeend", icono("escudo"));
  cab4.append(el("h2", { text: "Licencia y autoría" }));
  lic.append(cab4);
  const cuerpo4 = el("div", { class: "tarjeta-cuerpo" });
  const autor = estado.info?.autorFicha;
  cuerpo4.innerHTML = `
    <div class="sello-autor">
      <div class="avatar">JM</div>
      <div class="nombre">${esc(autor?.nombre ?? "Jose Manuel Bernabeu Mejias")}</div>
      <div class="detalle">${esc(autor?.direccion ?? "")}<br>
        ${esc(autor?.codigoPostal ?? "")} ${esc(autor?.poblacion ?? "")},
        ${esc(autor?.provincia ?? "")} (${esc(autor?.pais ?? "")})</div>
    </div>
    <div class="ayuda-texto" style="line-height:1.75">
      <strong>Copyright (c) 2026 ${esc(autor?.nombre ?? "Jose Manuel Bernabeu Mejias")}.</strong>
      Todos los derechos reservados.<br><br>
      Esta aplicacion se distribuye bajo la licencia <strong>MIT</strong>, redactada
      integramente en espanol. Puede consultar el texto completo en el repositorio
      <a href="${esc(estado.info?.repositorio ?? "")}" target="_blank" rel="noopener">${esc(
        estado.info?.repositorio ?? "",
      )}</a>.<br><br>
      Queda permitido el uso, copia, modificacion, fusion y publicacion, incluida la
      comercial, con la condicion de conservar este aviso de copyright y el archivo de
      licencia en todas las copias o partes sustanciales del software.
    </div>`;
  lic.append(cuerpo4);
  contenedor.append(lic);

  if (!estado.info) {
    contenedor.append(vacio("Informacion no disponible", "Reinicie la aplicacion.", "aviso"));
  }
}

function pintarVersion(zona: HTMLElement, btn: HTMLButtonElement): void {
  while (zona.firstChild) zona.removeChild(zona.firstChild);
  const v: InfoVersion | null = estado.version;
  if (!v) {
    zona.append(
      el("div", {
        class: "ayuda-texto",
        text: "Todavia no se ha comprobado si hay actualizaciones nuevas.",
      }),
    );
    return;
  }

  const ficha = el("div", { class: "ficha" });
  const datos: [string, string][] = [
    ["Version instalada", `v${v.versionActual}`],
    ["Ultima version publicada", v.consultado ? `v${v.versionDisponible}` : "sin consultar"],
    ["Publicada el", v.publicadaEn ? fecha(v.publicadaEn) : "-"],
  ];
  for (const [k, val] of datos) {
    const c = el("div", { class: "dato-ficha" });
    c.innerHTML = `<div class="clave">${esc(k)}</div><div class="valor">${esc(val)}</div>`;
    ficha.append(c);
  }
  zona.append(ficha);

  if (v.hayActualizacion) {
    const caja = el("div", { class: "caja-info", style: "border-color:#7dd3fc;margin-top:12px" });
    caja.innerHTML = `
      <strong style="color:var(--azul-oscuro)">Hay una nueva version disponible: v${esc(v.versionDisponible)}</strong>
      ${v.nombreLanzamiento ? `<br>${esc(v.nombreLanzamiento)}` : ""}
      ${v.publicadaEn ? ` &middot; publicada el ${esc(fecha(v.publicadaEn))}` : ""}
      <br><br>Pulse el boton para descargarla e instalarla.`;
    zona.append(caja);

    const acciones = el("div", { class: "btn-grupo", style: "margin-top:12px" });
    acciones.append(
      boton("Descargar e instalar la actualizacion", {
        clase: "principal",
        iconoNombre: "descargarActualizacion",
        alPulsar: (e) => void instalar(zona, e.currentTarget as HTMLButtonElement),
      }),
    );
    zona.append(acciones);
  } else if (v.consultado) {
    const caja = el("div", { class: "caja-info", style: "margin-top:12px;border-color:#a7f3d0" });
    caja.innerHTML = `<strong style="color:#059669">Esta aplicacion esta actualizada.</strong>
      No hay versiones nuevas publicadas.`;
    zona.append(caja);
  } else {
    const caja = el("div", { class: "caja-info", style: "margin-top:12px;border-color:#fde68a" });
    caja.innerHTML = `<strong style="color:#d97706">No se ha podido comprobar</strong><br>${esc(v.notas)}`;
    zona.append(caja);
  }
  btn.disabled = false;
}

async function comprobar(btn: HTMLButtonElement): Promise<void> {
  btn.disabled = true;
  try {
    const v = await api.comprobarActualizaciones();
    estado.version = v;
    const zona = document.querySelector("#zona-version");
    if (zona) pintarVersion(zona as HTMLElement, btn);
    if (v.hayActualizacion) avisar(`Nueva version disponible: v${v.versionDisponible}.`, "info", 6000);
    else if (v.consultado) avisar("Esta es la ultima version publicada.", "exito", 5000);
    else avisar(v.notas || "No se ha podido comprobar.", "aviso", 7000);
  } catch (e) {
    avisar(comoError(e).message, "error");
    btn.disabled = false;
  }
}

/** Descarga la nueva version y abre el instalador. */
async function instalar(zona: HTMLElement, btn: HTMLButtonElement): Promise<void> {
  const v = estado.version;
  if (!v?.hayActualizacion) {
    avisar("No hay ninguna actualizacion pendiente.", "aviso");
    return;
  }
  btn.disabled = true;
  mostrarCarga("Descargando la actualizacion...");
  textoCarga("Conectando con GitHub...");
  const progreso = barraProgreso(zona);
  let totalEsperado = 0;
  try {
    const descarga = await api.descargarActualizacion(v.urlDescarga, null);
    progreso.style.width = "100%";
    totalEsperado = descarga.bytes;
    esconderCarga();
    avisar(
      `Actualizacion descargada: ${descarga.nombre} (${tamano(totalEsperado)}) en ${descarga.ruta}`,
      "exito",
      0,
    );
    // En Linux se abre el instalador del sistema; en el resto se deja el
    // fichero descargado para que el usuario lo ejecute.
    try {
      await api.abrirActualizacion(descarga.ruta);
      avisar(
        descarga.esDeb
          ? "Se ha abierto el instalador. Sigalo para completar la actualizacion."
          : "Se ha abierto el fichero descargado.",
        "info",
        0,
      );
    } catch {
      avisar(`Abra manualmente el fichero ${descarga.ruta} para instalarlo.`, "aviso", 0);
    }
    zona.append(
      el("div", {
        class: "caja-info",
        style: "margin-top:10px;border-color:#a7f3d0",
        html: `<strong style="color:#059669">Descarga completada.</strong><br>
               Fichero: <code>${esc(descarga.ruta)}</code> (${esc(tamano(totalEsperado))})<br>
               Cierre la aplicacion antes de instalarla.`,
      }),
    );
  } catch (e) {
    esconderCarga();
    avisar(comoError(e).message, "error", 0);
    zona.append(
      el("div", {
        class: "caja-info",
        style: "margin-top:10px;border-color:#fecaca",
        html: `<strong style="color:#dc2626">No se ha podido descargar la actualizacion</strong><br>${esc(
          comoError(e).message,
        )}<br>Puede descargar el fichero manualmente desde
        <a href="${esc(v.urlDescarga)}" target="_blank" rel="noopener">${esc(v.urlDescarga)}</a>.`,
      }),
    );
  } finally {
    btn.disabled = false;
  }
  void numero;
}

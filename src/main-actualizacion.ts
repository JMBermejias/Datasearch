// Copyright (c) 2026 Jose Manuel Bernabeu Mejias - Licencia MIT
// Ventana de actualizacion: comprueba la version publicada, la descarga
// automaticamente e instala la nueva version.

import "./styles.css";
import { api, type InfoVersion } from "./api";
import { listen } from "@tauri-apps/api/event";
import { icono } from "./iconos";
import { avisar, barraProgreso, boton, el, esc, fecha, tamano, actualizarProgreso } from "./ui";

let info: InfoVersion | null = null;
let raiz: HTMLElement;
let descargando = false;

function pintar(): void {
  while (raiz.firstChild) raiz.removeChild(raiz.firstChild);
  const envoltura = el("div", { class: "actualizacion" });
  const tarjeta = el("div", { class: "tarjeta-actualizacion" });

  const cabecera = el("div", { class: "tarjeta-cabecera" });
  cabecera.insertAdjacentHTML("beforeend", icono("actualizar"));
  cabecera.append(el("h2", { text: "Actualizacion de DataSearch" }));
  tarjeta.append(cabecera);

  const cuerpo = el("div", { class: "tarjeta-cuerpo" });
  if (!info) {
    cuerpo.append(
      el("div", { class: "ayuda-texto", text: "Comprobando si hay actualizaciones disponibles..." }),
    );
  } else if (!info.hayActualizacion) {
    const ok = el("div", { class: "caja-info", style: "border-color:#a7f3d0" });
    ok.innerHTML = `<strong style="color:#059669">Esta aplicacion esta actualizada.</strong><br>
      Version instalada: <strong>v${esc(info.versionActual)}</strong>. No hay versiones nuevas publicadas.`;
    cuerpo.append(ok);
  } else {
    cuerpo.append(
      el("div", {
        class: "sello-autor",
        html: `<div class="avatar">DS</div>
          <div class="nombre">DataSearch v${esc(info.versionDisponible)}</div>
          <div class="detalle">${esc(info.nombreLanzamiento || "")}
            ${info.publicadaEn ? `&middot; publicada el ${esc(fecha(info.publicadaEn))}` : ""}<br>
            Version instalada: v${esc(info.versionActual)}</div>`,
      }),
    );
    if (info.notas) {
      cuerpo.append(el("div", { class: "seccion-titulo", text: "Novedades de esta version" }));
      cuerpo.append(el("div", { class: "notas", text: info.notas }));
    }
    const acciones = el("div", { class: "btn-grupo", style: "margin-top:14px" });
    const btn = boton("Descargar e instalar ahora", {
      clase: "principal",
      iconoNombre: "descargarActualizacion",
      alPulsar: (e) => void descargar(e.currentTarget as HTMLButtonElement),
    });
    if (descargando) btn.disabled = true;
    acciones.append(btn);
    acciones.append(
      boton("Ahora no", { clase: "fantasma", iconoNombre: "cerrar", alPulsar: () => cerrar() }),
    );
    cuerpo.append(acciones);
    cuerpo.append(el("div", { id: "progreso-actualizacion" }));
  }
  tarjeta.append(cuerpo);
  tarjeta.append(
    el("div", {
      class: "tarjeta-pie",
      html: `Copyright (c) 2026 Jose Manuel Bernabeu Mejias &middot; Licencia MIT &middot;
             <a href="https://github.com/JMBermejias/Datasearch" target="_blank" rel="noopener">github.com/JMBermejias/Datasearch</a>`,
    }),
  );
  envoltura.append(tarjeta);
  raiz.append(envoltura);
}

async function descargar(btn: HTMLButtonElement): Promise<void> {
  if (!info?.hayActualizacion || descargando) return;
  descargando = true;
  btn.disabled = true;
  const zona = document.querySelector("#progreso-actualizacion") as HTMLElement | null;
  const texto = el("div", { class: "ayuda-texto", style: "margin-top:10px" });
  let barra: HTMLElement | null = null;
  if (zona) {
    while (zona.firstChild) zona.removeChild(zona.firstChild);
    texto.textContent = "Descargando la nueva version...";
    zona.append(texto);
    barra = barraProgreso(zona);
  }

  try {
    const descarga = await api.descargarActualizacion(info.urlDescarga, null);
    if (barra) actualizarProgreso(barra, 100);
    texto.innerHTML = `<strong style="color:#059669">Descarga completada.</strong><br>
      ${esc(descarga.nombre)} &middot; ${esc(tamano(descarga.bytes))}<br>
      ${esc(descarga.ruta)}`;

    const acciones = el("div", { class: "btn-grupo", style: "margin-top:12px" });
    acciones.append(
      boton(descarga.esDeb ? "Instalar ahora" : "Abrir el fichero", {
        clase: "principal",
        iconoNombre: "salirActualizacion",
        alPulsar: async () => {
          try {
            await api.abrirActualizacion(descarga.ruta);
            avisar(
              "Siga las instrucciones del instalador. Cierre esta ventana para continuar.",
              "info",
              0,
            );
          } catch (e) {
            avisar(String(e), "error", 0);
          }
        },
      }),
      boton("Cerrar y reiniciar mas tarde", {
        clase: "fantasma",
        alPulsar: () => cerrar(),
      }),
    );
    zona?.append(acciones);
  } catch (e) {
    if (barra) actualizarProgreso(barra, 0);
    texto.innerHTML = `<strong style="color:#dc2626">No se ha podido descargar la actualizacion</strong><br>
      ${esc(String(e))}<br>
      Puede descargarla manualmente desde
      <a href="${esc(info.urlDescarga)}" target="_blank" rel="noopener">${esc(info.urlDescarga)}</a>.`;
    descargando = false;
    btn.disabled = false;
  }
}

function cerrar(): void {
  void import("@tauri-apps/api/window").then(async ({ getCurrentWindow }) => {
    const w = getCurrentWindow();
    await w.close();
  });
}

async function iniciar(): Promise<void> {
  raiz = (document.querySelector("#raiz") as HTMLElement) ?? el("div", { id: "raiz" });
  document.body.append(raiz);
  pintar();
  try {
    info = await api.comprobarActualizaciones();
  } catch (e) {
    info = null;
    console.error(e);
  }
  pintar();
}

void listen<InfoVersion>("actualizacion:datos", (e) => {
  info = e.payload;
  pintar();
});

void iniciar();

// Copyright (c) 2026 Jose Manuel Bernabeu Mejias - Licencia MIT
// Motor de graficos en canvas.
//
// Se dibuja sobre `canvas` para que, al mismo tiempo, sirva para pintar en
// pantalla y para exportar la misma imagen en PNG e incrustarla en el PDF.

import type { PuntoGrafico, TipoGrafico } from "./api";
import { esc, numero } from "./ui";

/** Paleta de la aplicacion, en el mismo azul del resto de la interfaz. */
const PALETA = [
  "#0ea5e9",
  "#0284c7",
  "#7dd3fc",
  "#38bdf8",
  "#0369a1",
  "#0c4a6e",
  "#a5f3fc",
  "#075985",
];

const EJE = "#94a3b8";
const REJILLA = "#e0f2fe";
const TEXTO = "#0f172a";

/** Ancho de la leyenda de un grafico de torta. */
const PIE_LEYENDA = 150;

export interface OpcionesGrafico {
  tipo: TipoGrafico;
  puntos: PuntoGrafico[];
  formato?: string;
  valorDestacado?: number | null;
  titulo?: string;
}

/**
 * Dibuja un grafico en el canvas indicado.
 *
 * @returns `true` si se ha dibujado algo.
 */
export function dibujar(canvas: HTMLCanvasElement, opciones: OpcionesGrafico): boolean {
  const dpr = Math.min(globalThis.devicePixelRatio || 1, 2);
  const ancho = Math.max(220, canvas.clientWidth || 420);
  const alto = opciones.tipo === "kpi" ? 150 : 260;

  canvas.width = Math.round(ancho * dpr);
  canvas.height = Math.round(alto * dpr);
  canvas.style.height = `${alto}px`;

  const ctx = canvas.getContext("2d");
  if (!ctx) return false;
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, ancho, alto);
  ctx.font = '12px "Segoe UI", Roboto, Ubuntu, Arial, sans-serif';
  ctx.textBaseline = "middle";

  const puntos = opciones.puntos.filter((p) => Number.isFinite(p.valor));
  if (!puntos.length) {
    ctx.fillStyle = "#94a3b8";
    ctx.textAlign = "center";
    ctx.fillText("Sin datos para los criterios indicados", ancho / 2, alto / 2);
    return false;
  }

  switch (opciones.tipo) {
    case "kpi":
      dibujarKpi(ctx, ancho, alto, opciones);
      return true;
    case "barra":
      dibujarBarras(ctx, ancho, alto, puntos, true);
      return true;
    case "linea":
      dibujarBarras(ctx, ancho, alto, puntos, false);
      return true;
    case "area":
      dibujarArea(ctx, ancho, alto, puntos);
      return true;
    case "torta":
      dibujarTorta(ctx, ancho, alto, puntos);
      return true;
    case "indicador":
      dibujarIndicador(ctx, ancho, alto, puntos, opciones);
      return true;
    case "dispersion":
      dibujarDispersion(ctx, ancho, alto, puntos);
      return true;
    default:
      dibujarBarras(ctx, ancho, alto, puntos, true);
      return true;
  }
}

/** Convierte el valor a texto segun el formato pedido. */
function textoValor(v: number, formato?: string): string {
  switch (formato) {
    case "moneda":
      return `${numero(v, 2)} €`;
    case "porcentaje":
      return `${numero(v, 1)} %`;
    case "entero":
      return numero(v, 0);
    default:
      return numero(v, v % 1 === 0 ? 0 : 2);
  }
}

function dibujarKpi(
  ctx: CanvasRenderingContext2D,
  ancho: number,
  alto: number,
  o: OpcionesGrafico,
): void {
  const valor = o.valorDestacado ?? o.puntos[0]?.valor ?? 0;
  ctx.textAlign = "center";
  ctx.fillStyle = "#0284c7";
  ctx.font = '800 40px "Segoe UI", Roboto, Ubuntu, Arial, sans-serif';
  ctx.fillText(textoValor(valor, o.formato), ancho / 2, alto / 2 - 10);
  if (o.puntos.length > 1) {
    ctx.fillStyle = "#64748b";
    ctx.font = '12px "Segoe UI", Roboto, Ubuntu, Arial, sans-serif';
    const total = o.puntos.reduce((s, p) => s + p.valor, 0);
    ctx.fillText(`${o.puntos.length} registros`, ancho / 2, alto / 2 + 24);
    ctx.fillText(`Total ${textoValor(total, o.formato)}`, ancho / 2, alto / 2 + 42);
  }
}

/** Escala los valores al alto disponible y devuelve el maximo. */
function escala(valores: number[], alto: number): { maximo: number; factor: number } {
  const maximo = Math.max(...valores, 0);
  const minimo = Math.min(...valores, 0);
  const rango = maximo - minimo || 1;
  return { maximo, factor: alto / rango };
}

function dibujarBarras(
  ctx: CanvasRenderingContext2D,
  ancho: number,
  alto: number,
  puntos: PuntoGrafico[],
  verticales: boolean,
): void {
  const margenIzq = 52;
  const margenInf = 30;
  const margenSup = 12;
  const margenDer = 12;
  const w = ancho - margenIzq - margenDer;
  const h = alto - margenInf - margenSup;
  if (w <= 10 || h <= 10) return;

  const { factor } = escala(puntos.map((p) => p.valor), h);

  // Rejilla y eje vertical.
  ctx.strokeStyle = REJILLA;
  ctx.lineWidth = 1;
  ctx.fillStyle = EJE;
  ctx.textAlign = "right";
  for (let i = 0; i <= 4; i += 1) {
    const y = margenSup + (h / 4) * i;
    ctx.beginPath();
    ctx.moveTo(margenIzq, y);
    ctx.lineTo(margenIzq + w, y);
    ctx.stroke();
    const v = puntos.reduce((m, p) => Math.max(m, p.valor), 0) * (1 - i / 4);
    ctx.fillText(numero(v, 0), margenIzq - 7, y);
  }

  if (verticales) {
    const anchoBarra = Math.max(3, (w / puntos.length) * 0.66);
    const paso = w / puntos.length;
    puntos.forEach((p, i) => {
      const altura = Math.abs(p.valor) * factor;
      const x = margenIzq + paso * i + (paso - anchoBarra) / 2;
      const y = p.valor >= 0 ? margenSup + h - altura : margenSup + h;
      const grad = ctx.createLinearGradient(0, y, 0, y + altura);
      grad.addColorStop(0, PALETA[i % PALETA.length]);
      grad.addColorStop(1, `${PALETA[i % PALETA.length]}aa`);
      ctx.fillStyle = grad;
      ctx.fillRect(x, y, anchoBarra, altura);

      ctx.save();
      ctx.fillStyle = "#64748b";
      ctx.textAlign = "center";
      ctx.translate(x + anchoBarra / 2, alto - 8);
      const etiqueta = recortar(p.etiqueta, Math.max(5, Math.floor(paso / 6.5)));
      ctx.fillText(etiqueta, 0, 0);
      ctx.restore();
    });
  } else {
    // Serie temporal: se respeta el orden de las etiquetas.
    ctx.strokeStyle = PALETA[0];
    ctx.lineWidth = 2.4;
    ctx.lineJoin = "round";
    ctx.beginPath();
    puntos.forEach((p, i) => {
      const x = margenIzq + (puntos.length === 1 ? w / 2 : (w / (puntos.length - 1)) * i);
      const y = margenSup + h - p.valor * factor;
      if (i === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    });
    ctx.stroke();

    puntos.forEach((p, i) => {
      const x = margenIzq + (puntos.length === 1 ? w / 2 : (w / (puntos.length - 1)) * i);
      const y = margenSup + h - p.valor * factor;
      ctx.fillStyle = "#fff";
      ctx.beginPath();
      ctx.arc(x, y, 4, 0, Math.PI * 2);
      ctx.fill();
      ctx.fillStyle = PALETA[0];
      ctx.beginPath();
      ctx.arc(x, y, 2.4, 0, Math.PI * 2);
      ctx.fill();
    });

    // Solo unas pocas etiquetas para que no se solapen.
    const salto = Math.max(1, Math.ceil(puntos.length / 8));
    ctx.fillStyle = "#64748b";
    ctx.textAlign = "center";
    puntos.forEach((p, i) => {
      if (i % salto !== 0 && i !== puntos.length - 1) return;
      const x = margenIzq + (puntos.length === 1 ? w / 2 : (w / (puntos.length - 1)) * i);
      ctx.fillText(recortar(p.etiqueta, 11), x, alto - 8);
    });
  }

  ctx.strokeStyle = EJE;
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.moveTo(margenIzq, margenSup);
  ctx.lineTo(margenIzq, margenSup + h);
  ctx.lineTo(margenIzq + w, margenSup + h);
  ctx.stroke();
}

function dibujarArea(
  ctx: CanvasRenderingContext2D,
  ancho: number,
  alto: number,
  puntos: PuntoGrafico[],
): void {
  const margenIzq = 52;
  const margenInf = 30;
  const margenSup = 12;
  const margenDer = 12;
  const w = ancho - margenIzq - margenDer;
  const h = alto - margenInf - margenSup;
  if (w <= 10 || h <= 10) return;

  const { factor } = escala(puntos.map((p) => p.valor), h);
  const x = (i: number) =>
    margenIzq + (puntos.length === 1 ? w / 2 : (w / (puntos.length - 1)) * i);
  const y = (v: number) => margenSup + h - v * factor;

  const grad = ctx.createLinearGradient(0, margenSup, 0, margenSup + h);
  grad.addColorStop(0, "#0ea5e9cc");
  grad.addColorStop(1, "#0ea5e900");
  ctx.fillStyle = grad;
  ctx.beginPath();
  ctx.moveTo(x(0), margenSup + h);
  puntos.forEach((p, i) => ctx.lineTo(x(i), y(p.valor)));
  ctx.lineTo(x(puntos.length - 1), margenSup + h);
  ctx.closePath();
  ctx.fill();

  ctx.strokeStyle = PALETA[0];
  ctx.lineWidth = 2.4;
  ctx.beginPath();
  puntos.forEach((p, i) => (i === 0 ? ctx.moveTo(x(i), y(p.valor)) : ctx.lineTo(x(i), y(p.valor))));
  ctx.stroke();

  ctx.strokeStyle = EJE;
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.moveTo(margenIzq, margenSup + h);
  ctx.lineTo(margenIzq + w, margenSup + h);
  ctx.stroke();

  const salto = Math.max(1, Math.ceil(puntos.length / 8));
  ctx.fillStyle = "#64748b";
  ctx.textAlign = "center";
  puntos.forEach((p, i) => {
    if (i % salto !== 0 && i !== puntos.length - 1) return;
    ctx.fillText(recortar(p.etiqueta, 11), x(i), alto - 8);
  });
}

function dibujarTorta(
  ctx: CanvasRenderingContext2D,
  ancho: number,
  alto: number,
  puntos: PuntoGrafico[],
): void {
  const total = puntos.reduce((s, p) => s + Math.abs(p.valor), 0) || 1;
  const conLeyenda = ancho > 330;
  const lado = Math.min(alto - 20, conLeyenda ? ancho - PIE_LEYENDA - 30 : ancho - 20);
  const cx = conLeyenda ? 20 + lado / 2 : ancho / 2;
  const cy = alto / 2;
  const radio = lado / 2;
  const radioInterior = radio * 0.58;

  let angulo = -Math.PI / 2;
  puntos.forEach((p, i) => {
    const barrido = (Math.abs(p.valor) / total) * Math.PI * 2;
    const color = PALETA[i % PALETA.length];
    ctx.fillStyle = color;
    ctx.beginPath();
    ctx.moveTo(cx, cy);
    ctx.arc(cx, cy, radio, angulo, angulo + barrido);
    ctx.closePath();
    ctx.fill();
    ctx.strokeStyle = "#fff";
    ctx.lineWidth = 1.6;
    ctx.stroke();

    // Porcentaje en el sector, si hay sitio.
    if (barrido > 0.28) {
      const medio = angulo + barrido / 2;
      const r = (radio + radioInterior) / 2;
      ctx.fillStyle = "#fff";
      ctx.textAlign = "center";
      ctx.font = '700 11px "Segoe UI", Roboto, Ubuntu, Arial, sans-serif';
      ctx.fillText(`${((p.valor / total) * 100).toFixed(0)}%`, cx + Math.cos(medio) * r, cy + Math.sin(medio) * r);
      ctx.font = '12px "Segoe UI", Roboto, Ubuntu, Arial, sans-serif';
    }
    angulo += barrido;
  });

  // Hueco central con el total.
  ctx.fillStyle = "#fff";
  ctx.beginPath();
  ctx.arc(cx, cy, radioInterior, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillStyle = "#0284c7";
  ctx.textAlign = "center";
  ctx.font = '700 15px "Segoe UI", Roboto, Ubuntu, Arial, sans-serif';
  ctx.fillText(numero(total, 0), cx, cy - 7);
  ctx.fillStyle = "#64748b";
  ctx.font = '10px "Segoe UI", Roboto, Ubuntu, Arial, sans-serif';
  ctx.fillText("total", cx, cy + 9);

  if (conLeyenda) {
    const x0 = cx + radio + 22;
    let y = 14;
    const maximo = Math.min(puntos.length, 9);
    puntos.slice(0, maximo).forEach((p, i) => {
      ctx.fillStyle = PALETA[i % PALETA.length];
      ctx.fillRect(x0, y - 5, 9, 9);
      ctx.fillStyle = TEXTO;
      ctx.textAlign = "left";
      ctx.fillText(recortar(p.etiqueta, 15), x0 + 14, y);
      ctx.fillStyle = "#64748b";
      ctx.fillText(
        `${((Math.abs(p.valor) / total) * 100).toFixed(1)} %`,
        x0 + 14,
        y + 12,
      );
      y += 27;
    });
    if (puntos.length > maximo) {
      ctx.fillStyle = "#94a3b8";
      ctx.fillText(`y ${puntos.length - maximo} mas…`, x0, y);
    }
  }
}

function dibujarIndicador(
  ctx: CanvasRenderingContext2D,
  ancho: number,
  alto: number,
  puntos: PuntoGrafico[],
  o: OpcionesGrafico,
): void {
  const valor = o.valorDestacado ?? puntos[0]?.valor ?? 0;
  const maximo = Math.max(...puntos.map((p) => Math.abs(p.valor)), Math.abs(valor), 1);
  const proporcion = Math.min(1, Math.abs(valor) / maximo);

  const cx = ancho / 2;
  const cy = alto / 2 + 20;
  const radio = Math.min(ancho / 2 - 26, alto / 2 - 30);

  ctx.lineWidth = 16;
  ctx.lineCap = "round";
  ctx.strokeStyle = REJILLA;
  ctx.beginPath();
  ctx.arc(cx, cy, radio, Math.PI * 0.8, Math.PI * 0.2);
  ctx.stroke();

  const grad = ctx.createLinearGradient(cx - radio, 0, cx + radio, 0);
  grad.addColorStop(0, "#7dd3fc");
  grad.addColorStop(1, "#0284c7");
  ctx.strokeStyle = grad;
  ctx.beginPath();
  ctx.arc(cx, cy, radio, Math.PI * 0.8, Math.PI * 0.8 + Math.PI * 0.6 * proporcion);
  ctx.stroke();

  ctx.textAlign = "center";
  ctx.fillStyle = "#0284c7";
  ctx.font = '800 27px "Segoe UI", Roboto, Ubuntu, Arial, sans-serif';
  ctx.fillText(textoValor(valor, o.formato), cx, cy - 4);
  ctx.fillStyle = "#64748b";
  ctx.font = '11px "Segoe UI", Roboto, Ubuntu, Arial, sans-serif';
  ctx.fillText(`${(proporcion * 100).toFixed(0)} % del maximo (${numero(maximo, 0)})`, cx, cy + 22);
}

function dibujarDispersion(
  ctx: CanvasRenderingContext2D,
  ancho: number,
  alto: number,
  puntos: PuntoGrafico[],
): void {
  const margen = 46;
  const w = ancho - margen * 2;
  const h = alto - margen - 22;
  if (w <= 10 || h <= 10) return;

  const ys = puntos.map((p) => p.valor);
  const minY = Math.min(...ys);
  const maxY = Math.max(...ys);
  const rangoY = maxY - minY || 1;

  ctx.strokeStyle = REJILLA;
  ctx.fillStyle = EJE;
  ctx.textAlign = "right";
  for (let i = 0; i <= 4; i += 1) {
    const y = margen + (h / 4) * i;
    ctx.beginPath();
    ctx.moveTo(margen, y);
    ctx.lineTo(margen + w, y);
    ctx.stroke();
    ctx.fillText(numero(maxY - (rangoY / 4) * i, 0), margen - 7, y);
  }

  puntos.forEach((p, i) => {
    const x = margen + (puntos.length === 1 ? w / 2 : (w / (puntos.length - 1)) * i);
    const y = margen + h - ((p.valor - minY) / rangoY) * h;
    ctx.fillStyle = PALETA[i % PALETA.length];
    ctx.globalAlpha = 0.78;
    ctx.beginPath();
    ctx.arc(x, y, 5, 0, Math.PI * 2);
    ctx.fill();
    ctx.globalAlpha = 1;
  });

  ctx.strokeStyle = EJE;
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.moveTo(margen, margen);
  ctx.lineTo(margen, margen + h);
  ctx.lineTo(margen + w, margen + h);
  ctx.stroke();
}

function recortar(texto: string, maximo: number): string {
  const t = texto.length > maximo ? `${texto.slice(0, Math.max(1, maximo - 1))}…` : texto;
  return t;
}

// ---------------------------------------------------------------------
// Exportacion a PNG
// ---------------------------------------------------------------------

/**
 * Devuelve el grafico como PNG en base64, listo para incrustar en el PDF.
 *
 * Se dibuja sobre un canvas temporal con el doble de resolucion para que
 * imprima con nitidez.
 */
export function aPngBase64(o: OpcionesGrafico, escala = 2): string {
  const temporal = document.createElement("canvas");
  temporal.width = 720;
  temporal.style.width = "720px";
  const ctx = temporal.getContext("2d");
  if (!ctx) return "";

  const alto = o.tipo === "kpi" ? 220 : 360;
  temporal.height = alto;
  temporal.style.height = `${alto}px`;

  // `dibujar` calcula el tamano a partir del ancho del canvas, asi que se
  // fuerza un ancho de trabajo conocido.
  Object.defineProperty(temporal, "clientWidth", { value: 720, configurable: true });
  const dpr = escala;
  const original = { w: temporal.width, h: temporal.height };
  temporal.width = 720 * dpr;
  temporal.height = alto * dpr;
  ctx.scale(dpr, dpr);
  dibujarEnContexto(ctx, 720, alto, o);
  temporal.width = original.w;
  temporal.height = original.h;
  return temporal.toDataURL("image/png");
}

/** Dibuja el grafico en un contexto con medidas explicitas. */
function dibujarEnContexto(
  ctx: CanvasRenderingContext2D,
  ancho: number,
  alto: number,
  o: OpcionesGrafico,
): void {
  const lienzoFalso = {
    clientWidth: ancho,
    getContext: () => ctx,
  } as unknown as HTMLCanvasElement;
  const altoOriginal = ctx.canvas.height;
  ctx.canvas.height = alto;
  dibujar(lienzoFalso, o);
  ctx.canvas.height = altoOriginal;
}

/** Lienzo vacio listo para insertar en un widget. */
export function lienzo(): HTMLCanvasElement {
  return document.createElement("canvas");
}

/** Marcado de la leyenda de un grafico de torta, como texto plano. */
export function leyenda(puntos: PuntoGrafico[]): string {
  const total = puntos.reduce((s, p) => s + Math.abs(p.valor), 0) || 1;
  return puntos
    .slice(0, 6)
    .map(
      (p, i) =>
        `<span><i style="background:${PALETA[i % PALETA.length]}"></i>${esc(
          recortar(p.etiqueta, 18),
        )} (${((Math.abs(p.valor) / total) * 100).toFixed(1)} %)</span>`,
    )
    .join("");
}

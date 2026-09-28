//! Exportacion de resultados a Excel, PDF, CSV, JSON, HTML y texto plano.
//!
//! Todas las exportaciones llevan el pie de pagina con el copyright de
//! Jose Manuel Bernabeu Mejias y la licencia MIT.
//!
//! Copyright (c) 2026 Jose Manuel Bernabeu Mejias
//! Licencia MIT

use crate::error::{Error, Resultado};
use crate::models::{
    ArchivoExportado, Documento, FormatoExportacion, ImagenDocumento, SeccionAnalisis,
};
use crate::util::{nombre_fichero_seguro, truncar};
use base64::Engine;

// ======================================================================
// Excel
// ======================================================================

use rust_xlsxwriter::{Color, Format, FormatAlign, FormatBorder, Workbook, XlsxError};

/// Formatos de celda reutilizados en todo el libro.
struct Estilos {
    titulo: Format,
    subtitulo: Format,
    clave: Format,
    dato: Format,
    pie: Format,
    cabecera: Format,
    texto: Format,
    texto_centrado: Format,
    numerico: Format,
    porcentaje: Format,
}

fn crear_estilos() -> Estilos {
    let azul = crate::models::COLOR_AZUL;
    let azul_claro = crate::models::COLOR_AZUL_CLARO;
    let borde = crate::models::COLOR_BORDE;
    Estilos {
        titulo: Format::new()
            .set_bold()
            .set_font_size(20)
            .set_font_color(azul)
            .set_align(FormatAlign::Left),
        subtitulo: Format::new()
            .set_italic()
            .set_font_size(11)
            .set_font_color(azul_claro),
        clave: Format::new().set_bold().set_font_color(azul),
        dato: Format::new(),
        pie: Format::new()
            .set_italic()
            .set_font_size(9)
            .set_font_color(azul_claro),
        cabecera: Format::new()
            .set_bold()
            .set_background_color(azul)
            .set_font_color(Color::White)
            .set_border(FormatBorder::Thin)
            .set_align(FormatAlign::Center)
            .set_text_wrap(),
        texto: Format::new()
            .set_border(FormatBorder::Thin)
            .set_border_color(borde),
        texto_centrado: Format::new()
            .set_border(FormatBorder::Thin)
            .set_border_color(borde)
            .set_align(FormatAlign::Center),
        numerico: Format::new()
            .set_border(FormatBorder::Thin)
            .set_border_color(borde)
            .set_num_format("#,##0.00"),
        porcentaje: Format::new()
            .set_border(FormatBorder::Thin)
            .set_border_color(borde)
            .set_num_format("0.00 \"%\"")
            .set_align(FormatAlign::Right),
    }
}

fn registrar_estilos(libro: &mut Workbook, e: &Estilos) {
    for f in [
        &e.titulo,
        &e.subtitulo,
        &e.clave,
        &e.dato,
        &e.pie,
        &e.cabecera,
        &e.texto,
        &e.texto_centrado,
        &e.numerico,
        &e.porcentaje,
    ] {
        libro.register_format(f);
    }
}

fn error_xlsx(e: XlsxError) -> Error {
    Error::Exportacion(format!("No se pudo generar el Excel: {e}"))
}

/// Genera un libro `.xlsx` con una hoja de portada, una por cada bloque de
/// resultados, una por cada seccion de analisis y una por cada widget.
pub fn a_excel(doc: &Documento) -> Resultado<Vec<u8>> {
    let mut libro = Workbook::new();
    libro.set_properties(
        &doc_props()
            .set_title(doc.titulo.clone())
            .set_subject(format!("{} - {}", crate::NOMBRE_APP, crate::VERSION))
            .set_author(crate::models::AUTOR_NOMBRE)
            .set_manager(crate::models::AUTOR_NOMBRE)
            .set_category("Informe de datos")
            .set_keywords("DataSearch, datos, informe")
            .set_comment(format!(
                "Generado el {} por {}",
                doc.generado_en, doc.generado_por
            )),
    );
    let estilos = crear_estilos();
    registrar_estilos(&mut libro, &estilos);

    escribir_portada(&mut libro, doc, &estilos)?;
    for bloque in &doc.bloques {
        escribir_hoja_resultados(&mut libro, bloque, &estilos)?;
    }
    for seccion in &doc.secciones {
        escribir_hoja_seccion(&mut libro, seccion, &estilos)?;
    }
    for (i, widget) in doc.widgets.iter().enumerate() {
        escribir_hoja_widget(&mut libro, i + 1, widget, &estilos)?;
    }

    libro.save_to_buffer().map_err(error_xlsx)
}

/// Alias corto para no repetir el tipo en las llamadas.
fn doc_props() -> rust_xlsxwriter::DocProperties {
    rust_xlsxwriter::DocProperties::new()
}

fn escribir_portada(libro: &mut Workbook, doc: &Documento, e: &Estilos) -> Resultado<()> {
    let hoja = libro.add_worksheet();
    hoja.set_name("Portada").map_err(error_xlsx)?;
    hoja.set_row_height(0, 34.0).map_err(error_xlsx)?;

    let ancho = 4u16;
    hoja.merge_range(0, 0, 0, ancho - 1, &doc.titulo, &e.titulo)
        .map_err(error_xlsx)?;
    hoja.merge_range(1, 0, 1, ancho - 1, &doc.subtitulo, &e.subtitulo)
        .map_err(error_xlsx)?;

    let mut fila = 3u32;
    let pares: Vec<(&str, String)> = vec![
        ("Generado por", doc.generado_por.clone()),
        ("Fecha de generacion", doc.generado_en.clone()),
        ("Origen de los datos", doc.pie.clone()),
        ("Aplicacion", crate::titulo_con_version()),
        (
            "Licencia",
            format!(
                "{} - Copyright (c) 2026 {}",
                crate::models::LICENCIA,
                crate::models::AUTOR_NOMBRE
            ),
        ),
    ];
    for (clave, valor) in pares {
        hoja.write_with_format(fila, 0, clave, &e.subtitulo)
            .map_err(error_xlsx)?;
        hoja.write_with_format(fila, 1, valor.as_str(), &e.dato)
            .map_err(error_xlsx)?;
        fila += 1;
    }

    fila += 1;
    hoja.write_with_format(fila, 0, "Resumen de los datos", &e.clave)
        .map_err(error_xlsx)?;
    fila += 1;
    hoja.write_with_format(fila, 0, "Fuente", &e.cabecera)
        .map_err(error_xlsx)?;
    hoja.write_with_format(fila, 1, "Esquema", &e.cabecera)
        .map_err(error_xlsx)?;
    hoja.write_with_format(fila, 2, "Filas devueltas", &e.cabecera)
        .map_err(error_xlsx)?;
    hoja.write_with_format(fila, 3, "Total en el origen", &e.cabecera)
        .map_err(error_xlsx)?;
    fila += 1;
    for b in &doc.bloques {
        hoja.write_with_format(fila, 0, b.fuente_nombre.as_str(), &e.texto)
            .map_err(error_xlsx)?;
        hoja.write_with_format(fila, 1, b.esquema.as_str(), &e.texto)
            .map_err(error_xlsx)?;
        hoja.write_with_format(fila, 2, b.filas.len() as f64, &e.numerico)
            .map_err(error_xlsx)?;
        hoja.write_with_format(fila, 3, b.total_filas as f64, &e.numerico)
            .map_err(error_xlsx)?;
        fila += 1;
    }
    if !doc.imagenes.is_empty() {
        fila += 1;
        hoja.write_with_format(fila, 0, "Graficos incrustados", &e.clave)
            .map_err(error_xlsx)?;
        hoja.write_with_format(fila, 1, doc.imagenes.len() as f64, &e.numerico)
            .map_err(error_xlsx)?;
    }

    hoja.merge_range(
        fila + 2,
        0,
        fila + 2,
        ancho - 1,
        &format!(
            "Copyright (c) 2026 {} - Licencia {} - Repositorio: {}",
            crate::models::AUTOR_NOMBRE,
            crate::models::LICENCIA,
            crate::models::REPOSITORIO
        ),
        &e.pie,
    )
    .map_err(error_xlsx)?;

    hoja.set_column_range_width(0, 0, 34.0)
        .map_err(error_xlsx)?;
    hoja.set_column_range_width(1, 3, 24.0)
        .map_err(error_xlsx)?;
    Ok(())
}

/// Vuelca un bloque de resultados en su propia hoja, con autofiltro y anchos
/// calculados a partir del contenido.
fn escribir_hoja_resultados(
    libro: &mut Workbook,
    bloque: &crate::models::BloqueResultados,
    e: &Estilos,
) -> Resultado<()> {
    let nombre = sanitizar_nombre_hoja(&format!("{}_{}", bloque.fuente_nombre, bloque.esquema), 2);
    let hoja = libro.add_worksheet();
    hoja.set_name(nombre).map_err(error_xlsx)?;

    let (columnas, datos) = crate::util::a_matriz(&bloque.filas);
    let ultima_col = (columnas.len() as u16).saturating_sub(1);
    let ancho = columna_maximo(ultima_col, 1);

    hoja.set_row_height(0, 22.0).map_err(error_xlsx)?;
    hoja.merge_range(0, 0, 0, ancho, &bloque.fuente_nombre, &e.titulo)
        .map_err(error_xlsx)?;
    let subtitulo = format!(
        "Esquema: {}   |   Filas devueltas: {}   |   Total estimado: {}   |   Tiempo: {} ms",
        if bloque.esquema.is_empty() {
            "-"
        } else {
            &bloque.esquema
        },
        bloque.filas.len(),
        bloque.total_filas,
        bloque.duracion_ms
    );
    hoja.merge_range(1, 0, 1, ancho, &subtitulo, &e.subtitulo)
        .map_err(error_xlsx)?;

    let cabecera_fila = 3u32;
    hoja.set_row_height(cabecera_fila, 26.0)
        .map_err(error_xlsx)?;
    for (i, c) in columnas.iter().enumerate() {
        hoja.write_with_format(cabecera_fila, i as u16, c.as_str(), &e.cabecera)
            .map_err(error_xlsx)?;
    }

    for (f, valores) in datos.iter().enumerate() {
        let r = cabecera_fila + 1 + f as u32;
        for (i, v) in valores.iter().enumerate() {
            let numerica = bloque
                .columnas
                .iter()
                .any(|c| c.numerica && &c.nombre == columnas.get(i).unwrap_or(&String::new()));
            if numerica {
                match crate::util::a_f64(v) {
                    Some(n) => {
                        hoja.write_with_format(r, i as u16, n, &e.numerico)
                            .map_err(error_xlsx)?;
                    }
                    None => {
                        hoja.write_with_format(r, i as u16, v.as_str(), &e.texto)
                            .map_err(error_xlsx)?;
                    }
                }
            } else {
                hoja.write_with_format(r, i as u16, v.as_str(), &e.texto)
                    .map_err(error_xlsx)?;
            }
        }
    }

    if !columnas.is_empty() && !datos.is_empty() {
        hoja.autofilter(
            cabecera_fila,
            0,
            cabecera_fila + datos.len() as u32,
            ultima_col,
        )
        .map_err(error_xlsx)?;
    }
    for (i, c) in columnas.iter().enumerate() {
        let ancho_col = bloque
            .columnas
            .iter()
            .find(|ci| &ci.nombre == c)
            .and_then(|ci| {
                ci.valores_frecuentes
                    .first()
                    .map(|f| f.valor.chars().count())
            })
            .unwrap_or(c.chars().count())
            .clamp(10, 42) as f64
            + 2.0;
        hoja.set_column_width(i as u16, ancho_col)
            .map_err(error_xlsx)?;
    }
    Ok(())
}

fn escribir_hoja_seccion(
    libro: &mut Workbook,
    seccion: &SeccionAnalisis,
    e: &Estilos,
) -> Resultado<()> {
    let nombre = sanitizar_nombre_hoja(&seccion.titulo, 3);
    let hoja = libro.add_worksheet();
    hoja.set_name(nombre).map_err(error_xlsx)?;

    hoja.write_with_format(0, 0, seccion.titulo.as_str(), &e.titulo)
        .map_err(error_xlsx)?;
    hoja.write_with_format(1, 0, seccion.descripcion.as_str(), &e.subtitulo)
        .map_err(error_xlsx)?;

    let mut fila = 3u32;
    if !seccion.metricas.is_empty() {
        for (i, t) in ["Indicador", "Valor", "Unidad"].iter().enumerate() {
            hoja.write_with_format(fila, i as u16, *t, &e.cabecera)
                .map_err(error_xlsx)?;
        }
        fila += 1;
        for m in &seccion.metricas {
            hoja.write_with_format(fila, 0, m.nombre.as_str(), &e.texto)
                .map_err(error_xlsx)?;
            hoja.write_with_format(fila, 1, m.valor.as_str(), &e.texto)
                .map_err(error_xlsx)?;
            hoja.write_with_format(fila, 2, m.unidad.as_str(), &e.texto)
                .map_err(error_xlsx)?;
            fila += 1;
        }
        fila += 1;
    }
    if !seccion.filas.is_empty() {
        let (columnas, datos) = crate::util::a_matriz(&seccion.filas);
        for (i, c) in columnas.iter().enumerate() {
            hoja.write_with_format(fila, i as u16, c.as_str(), &e.cabecera)
                .map_err(error_xlsx)?;
        }
        fila += 1;
        for valores in datos {
            for (i, v) in valores.iter().enumerate() {
                hoja.write_with_format(fila, i as u16, v.as_str(), &e.texto)
                    .map_err(error_xlsx)?;
            }
            fila += 1;
        }
    }
    hoja.set_column_range_width(0, 3, 30.0)
        .map_err(error_xlsx)?;
    Ok(())
}

fn escribir_hoja_widget(
    libro: &mut Workbook,
    indice: usize,
    widget: &crate::models::WidgetListo,
    e: &Estilos,
) -> Resultado<()> {
    let hoja = libro.add_worksheet();
    hoja.set_name(format!("Grafico_{indice}"))
        .map_err(error_xlsx)?;
    hoja.write_with_format(0, 0, widget.titulo.as_str(), &e.titulo)
        .map_err(error_xlsx)?;
    hoja.write_with_format(1, 0, widget.subtitulo.as_str(), &e.subtitulo)
        .map_err(error_xlsx)?;

    let mut fila = 3u32;
    if !widget.filas.is_empty() {
        let (columnas, datos) = crate::util::a_matriz(&widget.filas);
        for (i, c) in columnas.iter().enumerate() {
            hoja.write_with_format(fila, i as u16, c.as_str(), &e.cabecera)
                .map_err(error_xlsx)?;
        }
        fila += 1;
        for valores in datos {
            for (i, v) in valores.iter().enumerate() {
                match crate::util::a_f64(v) {
                    Some(n) => {
                        hoja.write_with_format(fila, i as u16, n, &e.numerico)
                            .map_err(error_xlsx)?;
                    }
                    None => {
                        hoja.write_with_format(fila, i as u16, v.as_str(), &e.texto)
                            .map_err(error_xlsx)?;
                    }
                }
            }
            fila += 1;
        }
        if !columnas.is_empty() {
            hoja.set_column_range_width(0, columnas.len() as u16 - 1, 24.0)
                .map_err(error_xlsx)?;
        }
    } else {
        hoja.write_with_format(fila, 0, "Etiqueta", &e.cabecera)
            .map_err(error_xlsx)?;
        hoja.write_with_format(fila, 1, "Valor", &e.cabecera)
            .map_err(error_xlsx)?;
        fila += 1;
        for p in &widget.puntos {
            hoja.write_with_format(fila, 0, p.etiqueta.as_str(), &e.texto)
                .map_err(error_xlsx)?;
            hoja.write_with_format(fila, 1, p.valor, &e.numerico)
                .map_err(error_xlsx)?;
            fila += 1;
        }
        hoja.set_column_range_width(0, 0, 34.0)
            .map_err(error_xlsx)?;
        hoja.set_column_range_width(1, 1, 20.0)
            .map_err(error_xlsx)?;
    }
    Ok(())
}

/// Excel limita a 31 caracteres el nombre de una hoja y no admite algunos
/// caracteres, de modo que se sanea el nombre del esquema.
fn sanitizar_nombre_hoja(nombre: &str, indice: u32) -> String {
    let limpio: String = nombre
        .chars()
        .map(|c| match c {
            '[' | ']' | ':' | '*' | '?' | '/' | '\\' => '-',
            otro => otro,
        })
        .collect();
    let base: String = limpio.trim().to_string();
    let sufijo = format!("_{indice}");
    let maximo = 31usize.saturating_sub(sufijo.len());
    if base.chars().count() <= maximo {
        format!("{base}{sufijo}")
    } else {
        format!("{}{sufijo}", base.chars().take(maximo).collect::<String>())
    }
}

fn columna_maximo(ultima: u16, minimo: u16) -> u16 {
    ultima.max(minimo)
}

// ======================================================================
// PDF
// ======================================================================

use pdf_writer::{Content, Filter, Name, Pdf, Rect, Ref, Str};

// Medidas de A4 en puntos.
const A4_ANCHO: f32 = 595.28;
const A4_ALTO: f32 = 841.89;
const MARGEN: f32 = 40.0;
const ESPACIO_PIE: f32 = 34.0;

const FUENTE_NORMAL: Name<'static> = Name(b"F1");
const FUENTE_NEGRITA: Name<'static> = Name(b"F2");
const FUENTE_CURSIVA: Name<'static> = Name(b"F3");

/// Una imagen incrustada, pendiente de registrar en el PDF.
struct ImagenIncrustada {
    ref_id: Ref,
    nombre: Name<'static>,
    datos: Vec<u8>,
    ancho_px: i32,
    alto_px: i32,
}

/// Constructor del PDF.
///
/// Mantiene un flujo de contenido por pagina. Al empezar una pagina nueva
/// cierra la anterior como objeto `stream`, de forma que al final solo hay que
/// enlazar el arbol de paginas.
struct GeneradorPdf {
    pdf: Pdf,
    contenido: Content,
    y: f32,
    paginas: Vec<Ref>,
    imagenes: Vec<ImagenIncrustada>,
    siguiente_ref: i32,
    primera_pagina: bool,
    nombre_xobject: Name<'static>,
}

impl GeneradorPdf {
    fn nuevo() -> Self {
        Self {
            pdf: Pdf::new(),
            contenido: Content::new(),
            y: A4_ALTO - MARGEN,
            paginas: Vec::new(),
            imagenes: Vec::new(),
            siguiente_ref: 100,
            primera_pagina: true,
            nombre_xobject: Name(b"Im1"),
        }
    }

    fn nuevo_ref(&mut self) -> Ref {
        let r = Ref::new(self.siguiente_ref);
        self.siguiente_ref += 1;
        r
    }

    fn ancho_util(&self) -> f32 {
        A4_ANCHO - 2.0 * MARGEN
    }

    /// Cierra la pagina en curso y abre una nueva.
    fn nueva_pagina(&mut self) {
        let id = self.nuevo_ref();
        let buf = std::mem::replace(&mut self.contenido, Content::new()).finish();
        self.pdf.stream(id, &buf);
        self.paginas.push(id);
        self.y = A4_ALTO - MARGEN;
        self.primera_pagina = false;
    }

    /// Abre una pagina nueva si no queda sitio.
    fn asegurar_espacio(&mut self, necesario: f32) {
        if self.primera_pagina {
            return;
        }
        if self.y - necesario < MARGEN + ESPACIO_PIE {
            self.nueva_pagina();
        }
    }

    fn escribir_texto(
        &mut self,
        texto: &str,
        tamano: f32,
        fuente: Name<'static>,
        color: (f32, f32, f32),
    ) {
        self.asegurar_espacio(tamano + 6.0);
        self.y -= tamano + 4.0;
        let (r, g, b) = color;
        self.contenido.begin_text();
        self.contenido.set_font(fuente, tamano);
        self.contenido.set_fill_rgb(r, g, b);
        self.contenido.next_line(MARGEN, self.y);
        self.contenido.show(Str(latin1(texto).as_bytes()));
        self.contenido.end_text();
    }

    /// Titulo principal de la portada.
    fn escribir_titulo(&mut self, texto: &str) {
        let (r, g, b) = crate::models::color_rgb(crate::models::COLOR_AZUL);
        self.y -= 26.0;
        self.contenido.begin_text();
        self.contenido.set_font(FUENTE_NEGRITA, 18.0);
        self.contenido.set_fill_rgb(r, g, b);
        self.contenido.next_line(MARGEN, self.y);
        self.contenido.show(Str(latin1(texto).as_bytes()));
        self.contenido.end_text();
        self.y -= 8.0;
    }

    /// Titulo de seccion con una linea de acento azul.
    fn escribir_seccion(&mut self, texto: &str) {
        self.asegurar_espacio(48.0);
        self.y -= 10.0;
        let (r, g, b) = crate::models::color_rgb(crate::models::COLOR_AZUL);
        let (r2, g2, b2) = crate::models::color_rgb(crate::models::COLOR_AZUL_CLARO);
        self.contenido.set_fill_rgb(r2, g2, b2);
        self.contenido
            .rect(MARGEN, self.y - 4.0, self.ancho_util(), 1.5);
        self.contenido.fill_nonzero();

        self.y -= 8.0;
        self.contenido.begin_text();
        self.contenido.set_font(FUENTE_NEGRITA, 12.0);
        self.contenido.set_fill_rgb(r, g, b);
        self.contenido.next_line(MARGEN, self.y);
        self.contenido.show(Str(latin1(texto).as_bytes()));
        self.contenido.end_text();
        self.y -= 14.0;
    }

    /// Tabla sencilla: cabecera azul y rejilla tenue.
    fn escribir_tabla(&mut self, columnas: &[String], datos: &[Vec<String>]) -> Resultado<()> {
        if columnas.is_empty() {
            return Ok(());
        }
        let mut anchos = anchos_de_columnas(columnas, datos, self.ancho_util());
        // El ancho minimo evita columnas invisibles con contenido largo.
        let total: f32 = anchos.iter().sum();
        if total < self.ancho_util() {
            let extra = (self.ancho_util() - total) / anchos.len() as f32;
            for a in anchos.iter_mut() {
                *a += extra;
            }
        }
        let _ = &mut anchos;

        // Cabecera.
        self.asegurar_espacio(50.0);
        const ALTO: f32 = 15.0;
        let (ra, ga, ba) = crate::models::color_rgb(crate::models::COLOR_AZUL);
        self.contenido.set_fill_rgb(ra, ga, ba);
        self.contenido
            .rect(MARGEN, self.y - ALTO, self.ancho_util(), ALTO);
        self.contenido.fill_nonzero();
        self.contenido.set_fill_rgb(1.0, 1.0, 1.0);
        let mut x = MARGEN + 3.0;
        for (i, c) in columnas.iter().enumerate() {
            self.contenido.begin_text();
            self.contenido.set_font(FUENTE_NEGRITA, 7.5);
            self.contenido.next_line(x, self.y - ALTO + 4.5);
            let maximo = ((anchos[i] - 6.0) / 3.75) as usize;
            self.contenido
                .show(Str(latin1(&truncar(c, maximo.max(2))).as_bytes()));
            self.contenido.end_text();
            x += anchos[i];
        }
        self.y -= ALTO;

        // Filas.
        let (rb, gb, bb) = crate::models::color_rgb(crate::models::COLOR_TEXTO);
        let (rl, gl, bl) = crate::models::color_rgb(crate::models::COLOR_BORDE);
        for fila in datos {
            if self.y - 12.0 < MARGEN + ESPACIO_PIE {
                self.nueva_pagina();
            }
            const ALTO_FILA: f32 = 12.0;
            self.contenido.set_stroke_rgb(rl, gl, bl);
            self.contenido.set_line_width(0.3);
            let mut x = MARGEN;
            for a in &anchos {
                self.contenido.move_to(x, self.y - ALTO_FILA);
                self.contenido.line_to(x, self.y);
                x += a;
            }
            self.contenido.stroke();

            self.contenido.begin_text();
            self.contenido.set_font(FUENTE_NORMAL, 7.5);
            self.contenido.set_fill_rgb(rb, gb, bb);
            let mut x = MARGEN + 3.0;
            for (i, ancho) in anchos.iter().enumerate() {
                let texto = fila.get(i).cloned().unwrap_or_default();
                let maximo = ((ancho - 6.0) / 3.75) as usize;
                self.contenido.next_line(x, self.y - 9.0);
                self.contenido
                    .show(Str(latin1(&truncar(&texto, maximo.max(2))).as_bytes()));
                x += ancho;
            }
            self.contenido.end_text();
            self.y -= ALTO_FILA;
        }
        self.y -= 12.0;
        Ok(())
    }

    /// Incrusta una imagen PNG ajustada al ancho util.
    fn escribir_imagen(&mut self, bytes_png: &[u8]) -> Resultado<()> {
        let imagen = image::load_from_memory_with_format(bytes_png, image::ImageFormat::Png)
            .map_err(|e| Error::Exportacion(format!("Grafico no valido: {e}")))?;
        let rgb = imagen.to_rgb8();
        let (ancho_px, alto_px) = (
            i32::try_from(rgb.width()).unwrap_or(0),
            i32::try_from(rgb.height()).unwrap_or(0),
        );
        if ancho_px <= 0 || alto_px <= 0 {
            return Err(Error::Exportacion("El grafico esta vacio".into()));
        }
        let comprimido = comprimir(rgb.as_raw())?;
        let ref_id = self.nuevo_ref();
        let nombre = self.nombre_xobject;
        self.imagenes.push(ImagenIncrustada {
            ref_id,
            nombre,
            datos: comprimido,
            ancho_px,
            alto_px,
        });
        Ok(())
    }

    /// Dibuja en la pagina la imagen registrada con indice `indice`.
    fn dibujar_imagen(&mut self, indice: usize) {
        let (ref_id, ancho_px, alto_px) = match self.imagenes.get(indice) {
            Some(i) => (i.ref_id, i.ancho_px, i.alto_px),
            None => return,
        };
        let nombre = self.nombre_xobject;
        let max_alto = A4_ALTO - MARGEN * 2.0 - 80.0;
        let escala = (self.ancho_util() / ancho_px as f32).min(max_alto / alto_px as f32);
        let ancho = ancho_px as f32 * escala;
        let alto = alto_px as f32 * escala;
        self.asegurar_espacio(alto + 20.0);
        let y = self.y - alto;
        self.contenido.save_state();
        self.contenido.transform([ancho, 0.0, 0.0, alto, MARGEN, y]);
        let _ = ref_id;
        self.contenido.x_object(nombre);
        self.contenido.restore_state();
        self.y -= alto + 16.0;
    }

    /// Cierra el documento y devuelve los bytes del PDF.
    fn finalizar(mut self) -> Vec<u8> {
        if self.primera_pagina {
            self.nueva_pagina();
        } else {
            let id = self.nuevo_ref();
            let buf = std::mem::replace(&mut self.contenido, Content::new()).finish();
            self.pdf.stream(id, &buf);
            self.paginas.push(id);
        }
        let total = self.paginas.len() as u32;

        // Pie de pagina en todas las paginas.
        for numero in 1..=total {
            let (r, g, b) = crate::models::color_rgb(crate::models::COLOR_AZUL_CLARO);
            let copyright = format!(
                "Copyright (c) 2026 {} - Licencia {} - {} v{}",
                crate::models::AUTOR_NOMBRE,
                crate::models::LICENCIA,
                crate::NOMBRE_APP,
                crate::VERSION
            );
            self.contenido.begin_text();
            self.contenido.set_font(FUENTE_CURSIVA, 7.5);
            self.contenido.set_fill_rgb(r, g, b);
            self.contenido.next_line(MARGEN, 22.0);
            self.contenido.show(Str(latin1(&copyright).as_bytes()));
            self.contenido.end_text();

            let etiqueta = format!("Pagina {numero} de {total}");
            let ancho = etiqueta.chars().count() as f32 * 3.8;
            self.contenido.begin_text();
            self.contenido.set_font(FUENTE_NORMAL, 7.5);
            self.contenido.set_fill_rgb(r, g, b);
            self.contenido.next_line(A4_ANCHO - MARGEN - ancho, 22.0);
            self.contenido.show(Str(latin1(&etiqueta).as_bytes()));
            self.contenido.end_text();

            let id = self.nuevo_ref();
            let buf = std::mem::replace(&mut self.contenido, Content::new()).finish();
            self.pdf.stream(id, &buf);
            self.paginas.push(id);
        }
        // Cada pagina lleva su propio stream de contenido, ya generado.
        let contenidos = std::mem::take(&mut self.paginas);
        let mut paginas_reales = Vec::with_capacity(contenidos.len());

        // Fuentes base: no hace falta incrustar los datos.
        let ref_normal = self.nuevo_ref();
        self.pdf
            .type1_font(ref_normal)
            .base_font(Name(b"Helvetica"));
        let ref_negrita = self.nuevo_ref();
        self.pdf
            .type1_font(ref_negrita)
            .base_font(Name(b"Helvetica-Bold"));
        let ref_cursiva = self.nuevo_ref();
        self.pdf
            .type1_font(ref_cursiva)
            .base_font(Name(b"Helvetica-Oblique"));

        // Objetos de imagen.
        for img in std::mem::take(&mut self.imagenes) {
            let mut xobj = self.pdf.image_xobject(img.ref_id, &img.datos);
            xobj.filter(Filter::FlateDecode);
            xobj.width(img.ancho_px);
            xobj.height(img.alto_px);
            xobj.color_space().device_rgb();
            xobj.bits_per_component(8);
        }

        let ref_arbol = self.nuevo_ref();
        let ref_catalogo = self.nuevo_ref();
        let a4 = Rect::new(0.0, 0.0, A4_ANCHO, A4_ALTO);

        for contenido_id in contenidos {
            let ref_pagina = self.nuevo_ref();
            let mut pagina = self.pdf.page(ref_pagina);
            pagina.media_box(a4);
            pagina.parent(ref_arbol);
            pagina.contents(contenido_id);
            {
                let mut recursos = pagina.resources();
                {
                    let mut fuentes = recursos.fonts();
                    fuentes.pair(FUENTE_NORMAL, ref_normal);
                    fuentes.pair(FUENTE_NEGRITA, ref_negrita);
                    fuentes.pair(FUENTE_CURSIVA, ref_cursiva);
                }
                if !self.imagenes.is_empty() {
                    let mut x = recursos.x_objects();
                    for img in &self.imagenes {
                        x.pair(img.nombre, img.ref_id);
                    }
                }
            }
            paginas_reales.push(ref_pagina);
        }

        self.pdf.catalog(ref_catalogo).pages(ref_arbol);
        self.pdf
            .pages(ref_arbol)
            .kids(paginas_reales.iter().copied())
            .count(paginas_reales.len() as i32);

        self.pdf.finish()
    }
}

/// Calcula el ancho de cada columna a partir del contenido mas largo.
fn anchos_de_columnas(columnas: &[String], datos: &[Vec<String>], ancho_total: f32) -> Vec<f32> {
    let mut anchos: Vec<f32> = columnas
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let mut max = c.chars().count() as f32;
            for fila in datos.iter().take(80) {
                if let Some(v) = fila.get(i) {
                    max = max.max(v.chars().count() as f32);
                }
            }
            (max * 0.48).clamp(30.0, 150.0)
        })
        .collect();
    let total: f32 = anchos.iter().sum();
    if total > ancho_total {
        let factor = ancho_total / total;
        for a in anchos.iter_mut() {
            *a *= factor;
        }
    }
    anchos
}

/// Comprime con zlib (el filtro `FlateDecode` que entiende PDF).
pub fn comprimir(datos: &[u8]) -> Resultado<Vec<u8>> {
    use std::io::Write;
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    encoder
        .write_all(datos)
        .map_err(|e| Error::Exportacion(format!("No se pudo comprimir el grafico: {e}")))?;
    encoder
        .finish()
        .map_err(|e| Error::Exportacion(format!("No se pudo comprimir el grafico: {e}")))
}

/// Genera un PDF A4 con los datos, los graficos y las secciones de analisis.
pub fn a_pdf(doc: &Documento) -> Resultado<Vec<u8>> {
    let mut g = GeneradorPdf::nuevo();

    g.escribir_titulo(&doc.titulo);
    if !doc.subtitulo.is_empty() {
        g.escribir_texto(&doc.subtitulo, 11.0, FUENTE_CURSIVA, color_texto());
    }
    g.escribir_texto(
        &format!("Generado por {}", doc.generado_por),
        8.5,
        FUENTE_NORMAL,
        color_texto(),
    );
    g.escribir_texto(
        &format!("Fecha: {}", doc.generado_en),
        8.5,
        FUENTE_NORMAL,
        color_texto(),
    );
    g.escribir_texto(
        &format!("{} v{}", crate::NOMBRE_APP, crate::VERSION),
        8.5,
        FUENTE_NORMAL,
        color_texto(),
    );
    if !doc.pie.is_empty() {
        g.escribir_texto(&doc.pie, 8.5, FUENTE_CURSIVA, color_texto());
    }

    for bloque in &doc.bloques {
        g.escribir_seccion(&format!("Fuente: {}", bloque.fuente_nombre));
        g.escribir_texto(
            &format!(
                "Esquema {}   -   {} filas devueltas   -   {} filas en el origen",
                if bloque.esquema.is_empty() {
                    "-"
                } else {
                    &bloque.esquema
                },
                bloque.filas.len(),
                bloque.total_filas
            ),
            8.5,
            FUENTE_CURSIVA,
            color_texto(),
        );
        if !bloque.mensaje.is_empty() {
            g.escribir_texto(&bloque.mensaje, 8.0, FUENTE_CURSIVA, color_texto());
        }
        if bloque.filas.is_empty() {
            g.escribir_texto(
                "No hay filas que cumplan los criterios indicados.",
                9.0,
                FUENTE_CURSIVA,
                color_texto(),
            );
            continue;
        }
        let (columnas, datos) = crate::util::a_matriz(&bloque.filas);
        g.escribir_tabla(&columnas, &datos)?;
    }

    // Graficos: primero se registran todos y luego se dibujan.
    let validas: Vec<usize> = doc
        .imagenes
        .iter()
        .enumerate()
        .filter_map(|(i, im)| decodificar_base64(&im.base64).map(|b| (i, b)))
        .map(|(_, b)| b)
        .collect::<Vec<_>>()
        .iter()
        .map(|_| 0)
        .collect();
    let _ = validas;
    let mut indice_registrado = 0usize;
    for imagen in &doc.imagenes {
        let Some(bytes) = decodificar_base64(&imagen.base64) else {
            continue;
        };
        if g.escribir_imagen(&bytes).is_ok() {
            if !imagen.titulo.is_empty() {
                g.escribir_seccion(&imagen.titulo);
            } else if indice_registrado == 0 {
                g.escribir_seccion("Graficos del tablero");
            }
            g.dibujar_imagen(indice_registrado);
            indice_registrado += 1;
        }
    }

    for seccion in &doc.secciones {
        g.escribir_seccion(&seccion.titulo);
        g.escribir_texto(&seccion.descripcion, 8.5, FUENTE_CURSIVA, color_texto());
        if !seccion.metricas.is_empty() {
            let filas: Vec<crate::models::Fila> = seccion
                .metricas
                .iter()
                .map(|m| crate::models::Fila {
                    valores: [
                        ("Indicador".to_string(), Some(m.nombre.clone())),
                        (
                            "Valor".to_string(),
                            Some(if m.unidad.is_empty() {
                                m.valor.clone()
                            } else {
                                format!("{} {}", m.valor, m.unidad)
                            }),
                        ),
                    ]
                    .into_iter()
                    .collect(),
                })
                .collect();
            let (columnas, datos) = crate::util::a_matriz(&filas);
            g.escribir_tabla(&columnas, &datos)?;
        }
        if !seccion.filas.is_empty() {
            let (columnas, datos) = crate::util::a_matriz(&seccion.filas);
            g.escribir_tabla(&columnas, &datos)?;
        }
    }

    // Widgets sin imagen asociada.
    for widget in &doc.widgets {
        if doc.imagenes.iter().any(|i| i.titulo == widget.titulo) {
            continue;
        }
        g.escribir_seccion(&widget.titulo);
        g.escribir_texto(&widget.subtitulo, 8.5, FUENTE_CURSIVA, color_texto());
        if !widget.puntos.is_empty() {
            let filas: Vec<crate::models::Fila> = widget
                .puntos
                .iter()
                .map(|p| crate::models::Fila {
                    valores: [
                        ("Etiqueta".to_string(), Some(p.etiqueta.clone())),
                        (
                            "Valor".to_string(),
                            Some(crate::util::formatear_numero(p.valor, 2)),
                        ),
                    ]
                    .into_iter()
                    .collect(),
                })
                .collect();
            let (columnas, datos) = crate::util::a_matriz(&filas);
            g.escribir_tabla(&columnas, &datos)?;
        }
    }

    Ok(g.finalizar())
}

fn color_texto() -> (f32, f32, f32) {
    crate::models::color_rgb(crate::models::COLOR_TEXTO)
}

/// Las fuentes base de PDF solo cubren latin-1: se sustituyen los caracteres
/// fuera de rango por su equivalente ASCII mas proximo.
fn latin1(texto: &str) -> String {
    texto
        .chars()
        .map(|c| {
            let c = match c {
                'á' | 'Á' => 'a',
                'é' | 'É' => 'e',
                'í' | 'Í' => 'i',
                'ó' | 'Ó' => 'o',
                'ú' | 'Ú' | 'ü' | 'Ü' => 'u',
                'ñ' | 'Ñ' => 'n',
                'ç' | 'Ç' => 'c',
                'à' | 'è' | 'ì' | 'ò' | 'ù' => match c {
                    'à' => 'a',
                    'è' => 'e',
                    'ì' => 'i',
                    'ò' => 'o',
                    _ => 'u',
                },
                '«' | '»' => '"',
                '€' => 'E',
                other => other,
            };
            if (c as u32) < 256 {
                c
            } else {
                '?'
            }
        })
        .collect()
}

// ======================================================================
// CSV, JSON, HTML y texto
// ======================================================================

fn a_csv_interno(columnas: &[String], datos: &[Vec<String>]) -> String {
    let mut salida = String::new();
    salida.push_str(
        &columnas
            .iter()
            .map(|c| escapar_csv(c))
            .collect::<Vec<_>>()
            .join(";"),
    );
    salida.push_str("\r\n");
    for fila in datos {
        salida.push_str(
            &fila
                .iter()
                .map(|v| escapar_csv(v))
                .collect::<Vec<_>>()
                .join(";"),
        );
        salida.push_str("\r\n");
    }
    salida
}

fn escapar_csv(valor: &str) -> String {
    let v = valor.replace('\r', " ").replace('\n', " ");
    if v.contains(';') || v.contains('"') {
        format!("\"{}\"", v.replace('"', "\"\""))
    } else {
        v
    }
}

pub fn a_csv(doc: &Documento) -> Resultado<Vec<u8>> {
    let mut salida = String::new();
    for bloque in &doc.bloques {
        let (columnas, datos) = crate::util::a_matriz(&bloque.filas);
        if columnas.is_empty() {
            continue;
        }
        salida.push_str(&format!(
            "# {} - {} - {} filas\r\n",
            bloque.fuente_nombre,
            bloque.esquema,
            bloque.filas.len()
        ));
        salida.push_str(&a_csv_interno(&columnas, &datos));
        salida.push_str("\r\n");
    }
    salida.push_str(&format!(
        "# Copyright (c) 2026 {} - Licencia {}\r\n",
        crate::models::AUTOR_NOMBRE,
        crate::models::LICENCIA
    ));
    Ok(salida.into_bytes())
}

pub fn a_json(doc: &Documento) -> Resultado<Vec<u8>> {
    let valor = serde_json::json!({
        "titulo": doc.titulo,
        "subtitulo": doc.subtitulo,
        "generadoPor": doc.generado_por,
        "generadoEn": doc.generado_en,
        "licencia": crate::models::LICENCIA,
        "copyright": format!("Copyright (c) 2026 {}", crate::models::AUTOR_NOMBRE),
        "bloques": doc.bloques,
        "analisis": doc.secciones,
        "widgets": doc.widgets.iter().map(|w| serde_json::json!({
            "titulo": w.titulo,
            "tipo": w.tipo,
            "subtitulo": w.subtitulo,
            "columnaGrupo": w.columna_grupo,
            "columnaValor": w.columna_valor,
            "agregacion": w.agregacion,
            "puntos": w.puntos,
        })).collect::<Vec<_>>(),
    });
    let texto = serde_json::to_string_pretty(&valor)
        .map_err(|e| Error::Exportacion(format!("No se pudo generar el JSON: {e}")))?;
    Ok(texto.into_bytes())
}

pub fn a_html(doc: &Documento) -> Resultado<Vec<u8>> {
    let texto = crate::util::nombre_fichero_seguro(&doc.titulo, "html");
    let _ = texto;
    let mut h = String::new();
    h.push_str("<!DOCTYPE html>\n<html lang=\"es\">\n<head>\n<meta charset=\"utf-8\">\n");
    h.push_str(&format!("<title>{}</title>\n", escapar_html(&doc.titulo)));
    h.push_str(
        "<style>body{font-family:Segoe UI,Roboto,Arial,sans-serif;background:#f0f9ff;color:#0f172a;margin:0;padding:24px}\
h1{color:#0284c7}table{border-collapse:collapse;margin:12px 0 28px;background:#fff;box-shadow:0 1px 4px rgba(2,132,199,.15)}\
th{background:#0ea5e9;color:#fff;padding:6px 10px;text-align:left}td{padding:5px 10px;border:1px solid #e0f2fe}\
footer{margin-top:32px;color:#0369a1;font-size:12px;border-top:1px solid #bae6fd;padding-top:10px}\
h2{color:#0284c7}</style>\n</head>\n<body>\n",
    );
    h.push_str(&format!(
        "<h1>{}</h1>\n<p>{}</p>\n",
        escapar_html(&doc.titulo),
        escapar_html(&doc.subtitulo)
    ));
    for bloque in &doc.bloques {
        h.push_str(&format!(
            "<h2>{}</h2>\n",
            escapar_html(&bloque.fuente_nombre)
        ));
        h.push_str(&format!(
            "<p>Esquema {} - {} filas</p>\n",
            escapar_html(&bloque.esquema),
            bloque.filas.len()
        ));
        let (columnas, datos) = crate::util::a_matriz(&bloque.filas);
        h.push_str("<table>\n<thead><tr>");
        for c in &columnas {
            h.push_str(&format!("<th>{}</th>", escapar_html(c)));
        }
        h.push_str("</tr></thead>\n<tbody>\n");
        for fila in &datos {
            h.push_str("<tr>");
            for v in fila {
                h.push_str(&format!("<td>{}</td>", escapar_html(v)));
            }
            h.push_str("</tr>\n");
        }
        h.push_str("</tbody>\n</table>\n");
    }
    for imagen in &doc.imagenes {
        h.push_str(&format!(
            "<h3>{}</h3><img alt=\"{}\" src=\"data:image/png;base64,{}\" style=\"max-width:100%\">\n",
            escapar_html(&imagen.titulo),
            escapar_html(&imagen.titulo),
            imagen.base64
        ));
    }
    for seccion in &doc.secciones {
        h.push_str(&format!(
            "<h2>{}</h2>\n<p>{}</p>\n",
            escapar_html(&seccion.titulo),
            escapar_html(&seccion.descripcion)
        ));
        let (columnas, datos) = crate::util::a_matriz(&seccion.filas);
        if !columnas.is_empty() {
            h.push_str("<table>\n<thead><tr>");
            for c in &columnas {
                h.push_str(&format!("<th>{}</th>", escapar_html(c)));
            }
            h.push_str("</tr></thead>\n<tbody>\n");
            for fila in &datos {
                h.push_str("<tr>");
                for v in fila {
                    h.push_str(&format!("<td>{}</td>", escapar_html(v)));
                }
                h.push_str("</tr>\n");
            }
            h.push_str("</tbody>\n</table>\n");
        }
    }
    h.push_str(&format!(
        "<footer>Copyright (c) 2026 {} - Licencia {} - {} v{}<br>Generado por {} el {}</footer>\n",
        crate::models::AUTOR_NOMBRE,
        crate::models::LICENCIA,
        crate::NOMBRE_APP,
        crate::VERSION,
        escapar_html(&doc.generado_por),
        escapar_html(&doc.generado_en)
    ));
    h.push_str("</body>\n</html>\n");
    Ok(h.into_bytes())
}

pub fn a_texto(doc: &Documento) -> Resultado<Vec<u8>> {
    let mut t = String::new();
    t.push_str(&format!(
        "{}\n{}\n{}\n",
        "=".repeat(78),
        doc.titulo,
        "=".repeat(78)
    ));
    if !doc.subtitulo.is_empty() {
        t.push_str(&format!("{}\n", doc.subtitulo));
    }
    t.push_str(&format!(
        "Generado por {} el {}\n\n",
        doc.generado_por, doc.generado_en
    ));
    for bloque in &doc.bloques {
        t.push_str(&format!(
            "FUENTE: {}  ESQUEMA: {}  FILAS: {}\n",
            bloque.fuente_nombre,
            bloque.esquema,
            bloque.filas.len()
        ));
        let (columnas, datos) = crate::util::a_matriz(&bloque.filas);
        t.push_str(&ancho_fijo(&columnas, &datos));
        t.push('\n');
    }
    for seccion in &doc.secciones {
        t.push_str(&format!("{}\n{}\n", seccion.titulo, seccion.descripcion));
        for m in &seccion.metricas {
            t.push_str(&format!("  {:<40} {}\n", m.nombre, m.valor));
        }
        let (columnas, datos) = crate::util::a_matriz(&seccion.filas);
        t.push_str(&ancho_fijo(&columnas, &datos));
        t.push('\n');
    }
    t.push_str(&format!(
        "Copyright (c) 2026 {} - Licencia {}\n",
        crate::models::AUTOR_NOMBRE,
        crate::models::LICENCIA
    ));
    Ok(t.into_bytes())
}

fn ancho_fijo(columnas: &[String], datos: &[Vec<String>]) -> String {
    let mut anchos: Vec<usize> = columnas
        .iter()
        .map(|c| c.chars().count().clamp(4, 30))
        .collect();
    for fila in datos.iter().take(200) {
        for (i, v) in fila.iter().enumerate() {
            if let Some(a) = anchos.get_mut(i) {
                *a = (*a).max(v.chars().count().clamp(4, 30));
            }
        }
    }
    let separador = anchos
        .iter()
        .map(|a| "-".repeat(*a))
        .collect::<Vec<_>>()
        .join("-+-");
    let mut salida = String::new();
    let linea = |s: &[String], anchos: &[usize]| -> String {
        s.iter()
            .zip(anchos)
            .map(|(v, a)| format!("{:<a$}", truncar(v, *a)))
            .collect::<Vec<_>>()
            .join(" | ")
    };
    salida.push_str(&linea(columnas, &anchos));
    salida.push('\n');
    salida.push_str(&separador);
    salida.push('\n');
    for fila in datos {
        salida.push_str(&linea(fila, &anchos));
        salida.push('\n');
    }
    salida
}

fn escapar_html(texto: &str) -> String {
    texto
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// ======================================================================
// Punto de entrada
// ======================================================================

fn decodificar_base64(texto: &str) -> Option<Vec<u8>> {
    let limpio = texto.split(',').next_back().unwrap_or(texto);
    base64::engine::general_purpose::STANDARD
        .decode(limpio.trim())
        .ok()
}

/// Exporta el documento en el formato pedido.
pub fn exportar(doc: &Documento, formato: &str) -> Resultado<ArchivoExportado> {
    let f = FormatoExportacion::desde_txt(formato)?;
    let (bytes, extension, mime): (Vec<u8>, &str, &str) = match f {
        FormatoExportacion::Excel => (
            a_excel(doc)?,
            "xlsx",
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        ),
        FormatoExportacion::Pdf => (a_pdf(doc)?, "pdf", "application/pdf"),
        FormatoExportacion::Csv => (a_csv(doc)?, "csv", "text/csv"),
        FormatoExportacion::Json => (a_json(doc)?, "json", "application/json"),
        FormatoExportacion::Html => (a_html(doc)?, "html", "text/html"),
        FormatoExportacion::Texto => (a_texto(doc)?, "txt", "text/plain"),
    };
    let _ = extension;
    Ok(ArchivoExportado {
        nombre: nombre_fichero_seguro(&doc.titulo, mime_extension(mime)),
        tamano_bytes: bytes.len() as u64,
        formato: format!("{f:?}").to_lowercase(),
        mime: mime.to_string(),
        contenido_base64: base64::engine::general_purpose::STANDARD.encode(&bytes),
    })
}

fn mime_extension(mime: &str) -> &'static str {
    match mime {
        "application/pdf" => "pdf",
        "application/json" => "json",
        "text/html" => "html",
        "text/csv" => "csv",
        _ => "xlsx",
    }
}

/// Bytes del PDF de un documento, util para la vista previa.
pub fn pdf_de(doc: &Documento) -> Resultado<Vec<u8>> {
    a_pdf(doc)
}

/// Nombres de los formatos que la interfaz debe ofrecer.
pub fn formatos_disponibles() -> Vec<(String, String)> {
    vec![
        ("excel".into(), "Excel (.xlsx)".into()),
        ("pdf".into(), "PDF (.pdf)".into()),
        ("csv".into(), "CSV (.csv)".into()),
        ("json".into(), "JSON (.json)".into()),
        ("html".into(), "HTML (.html)".into()),
        ("texto".into(), "Texto plano (.txt)".into()),
    ]
}

/// Comprueba que un documento tiene contenido exportable.
pub fn documento_vacio(doc: &Documento) -> bool {
    doc.bloques.iter().all(|b| b.filas.is_empty())
        && doc.secciones.is_empty()
        && doc.widgets.is_empty()
}

/// Prepara una imagen PNG y sus dimensiones para incrustarla.
pub fn preparar_imagen(base64_png: &str, titulo: &str) -> Option<ImagenDocumento> {
    let bytes = decodificar_base64(base64_png)?;
    let imagen = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png).ok()?;
    let (ancho, alto) = (
        u32::try_from(imagen.width()).unwrap_or(0),
        u32::try_from(imagen.height()).unwrap_or(0),
    );
    Some(ImagenDocumento {
        titulo: titulo.to_string(),
        base64: base64_png.to_string(),
        ancho_px: ancho,
        alto_px: alto,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{BloqueResultados, ColumnaInfo, Fila, TipoFuente};
    use std::collections::BTreeMap;

    fn documento() -> Documento {
        let filas = vec![Fila {
            valores: BTreeMap::from([
                ("importe".to_string(), Some("1234,50".to_string())),
                ("zona".to_string(), Some("Nordeste".to_string())),
            ]),
        }];
        let columnas = vec![ColumnaInfo {
            nombre: "importe".into(),
            etiqueta: "importe".into(),
            tipo: "numero".into(),
            numerica: true,
            ..Default::default()
        }];
        Documento {
            titulo: "Informe de ventas".into(),
            subtitulo: "Primer trimestre".into(),
            pie: "Base de datos de produccion".into(),
            bloques: vec![BloqueResultados {
                fuente_id: 1,
                fuente_nombre: "Ventas".into(),
                tipo_fuente: TipoFuente::Sqlite,
                esquema: "ventas".into(),
                filas,
                columnas,
                total_filas: 1,
                filas_omitidas: 0,
                duracion_ms: 3,
                mensaje: String::new(),
            }],
            secciones: vec![SeccionAnalisis {
                titulo: "Resumen".into(),
                tipo: "resumen".into(),
                descripcion: "Resumen de las ventas".into(),
                filas: Vec::new(),
                clave: BTreeMap::new(),
                metricas: vec![crate::models::Metrica {
                    nombre: "Total".into(),
                    valor: "1.234,50".into(),
                    unidad: "EUR".into(),
                }],
            }],
            widgets: Vec::new(),
            imagenes: Vec::new(),
            generado_por: "JMBernabeu".into(),
            generado_en: "2026-09-28T10:00:00Z".into(),
            duracion_ms: 12,
        }
    }

    #[test]
    fn genera_excel_valido() {
        let bytes = a_excel(&documento()).unwrap();
        // Un .xlsx es un ZIP: empieza con la firma PK.
        assert_eq!(&bytes[0..2], b"PK");
        assert!(bytes.len() > 1000);
    }

    #[test]
    fn genera_pdf_valido() {
        let bytes = a_pdf(&documento()).unwrap();
        assert_eq!(&bytes[0..5], b"%PDF-");
        assert!(bytes.len() > 800);
    }

    #[test]
    fn genera_csv_y_html() {
        let csv = String::from_utf8(a_csv(&documento()).unwrap()).unwrap();
        assert!(csv.contains("importe"));
        assert!(csv.contains("Jose Manuel Bernabeu Mejias"));
        let html = String::from_utf8(a_html(&documento()).unwrap()).unwrap();
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("Copyright (c) 2026 Jose Manuel Bernabeu Mejias"));
    }

    #[test]
    fn genera_json_estructurado() {
        let texto = String::from_utf8(a_json(&documento()).unwrap()).unwrap();
        let v: serde_json::Value = serde_json::from_str(&texto).unwrap();
        assert_eq!(v["titulo"], "Informe de ventas");
        assert_eq!(v["licencia"], "MIT");
    }

    #[test]
    fn genera_txt_legible() {
        let t = String::from_utf8(a_texto(&documento()).unwrap()).unwrap();
        assert!(t.contains("Informe de ventas"));
        assert!(t.contains("1.234,50"));
    }

    #[test]
    fn exporta_en_cada_formato() {
        for (f, _) in formatos_disponibles() {
            let r = exportar(&documento(), &f);
            assert!(r.is_ok(), "fallo el formato {f}");
            if let Ok(r) = r {
                assert!(!r.contenido_base64.is_empty());
            }
        }
    }

    #[test]
    fn rechaza_formato_desconocido() {
        assert!(exportar(&documento(), "docx").is_err());
    }

    #[test]
    fn escapa_caracteres_en_html() {
        assert_eq!(escapar_html("<b>a & b</b>"), "&lt;b&gt;a &amp; b&lt;/b&gt;");
    }

    #[test]
    fn genera_texto_latin1() {
        assert_eq!(latin1("José Muñoz ©"), "Jose Munoz ©");
        assert_eq!(latin1("año 2026 → 100%"), "ano 2026 ? 100%");
        assert_eq!(latin1("Cambios €"), "Cambios E");
    }
}

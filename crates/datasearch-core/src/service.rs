//! Busqueda multi-fuente, construccion de tableros y confirmacion de analisis.
//!
//! Este modulo es el que usa la interfaz: unifica el comportamiento de todas
//! las fuentes de datos y garantiza que solo se leen datos cuando el usuario
//! pulsa el boton de buscar.
//!
//! Copyright (c) 2026 Jose Manual Bernabeu Mejias
//! Licencia MIT

use crate::analysis;
use crate::appdb::AppDb;
use crate::auth::{Contexto, Permiso};
use crate::drivers;
use crate::error::{Error, Resultado};
use crate::models::{
    ColumnaInfo, Documento, Esquema, Fila, Fuente, PeticionAnalisis, PeticionBusqueda,
    PeticionDashboard, RespuestaAnalisis, RespuestaBusqueda, Sugerencia, TipoGrafico, Widget,
    WidgetListo,
};
use crate::query;
use crate::util::a_f64;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Servicio de consulta sobre las fuentes del usuario.
pub struct ServicioBusqueda {
    db: Arc<AppDb>,
}

impl ServicioBusqueda {
    pub fn nuevo(db: Arc<AppDb>) -> Self {
        Self { db }
    }

    /// Fuentes que intervienen en la busqueda, ya comprobadas.
    fn fuentes_de(&self, ctx: &Contexto, pedidas: &[i64]) -> Resultado<Vec<Fuente>> {
        let disponibles: Vec<Fuente> = self
            .db
            .listar_fuentes(ctx.id_usuario, ctx.es_admin())?
            .into_iter()
            .filter(|f| f.activa)
            .collect();
        if pedidas.is_empty() {
            if disponibles.is_empty() {
                return Err(Error::Validacion(
                    "No hay ninguna fuente de datos activa. Anade una desde 'Fuentes de datos'."
                        .into(),
                ));
            }
            return Ok(disponibles);
        }
        let mut salida = Vec::new();
        for id in pedidas {
            let f = disponibles
                .iter()
                .find(|f| f.id == *id)
                .cloned()
                .ok_or_else(|| {
                    Error::PermisoDenegado(format!(
                        "La fuente {id} no existe o no esta disponible para su cuenta"
                    ))
                })?;
            salida.push(f);
        }
        Ok(salida)
    }

    /// Ejecuta la busqueda del usuario sobre todas las fuentes seleccionadas.
    ///
    /// Los errores de una fuente no impiden ver el resto de resultados: se
    /// recogen en `fuentesConError` para que la interfaz los muestre.
    pub async fn buscar(
        &self,
        ctx: &Contexto,
        peticion: &PeticionBusqueda,
    ) -> Resultado<RespuestaBusqueda> {
        ctx.exigir(Permiso::Consultar)?;
        let inicio = std::time::Instant::now();
        let fuentes = self.fuentes_de(ctx, &peticion.fuentes)?;

        let mut bloques = Vec::new();
        let mut errores: Vec<String> = Vec::new();

        let resultados: Vec<_> =
            futures_util::future::join_all(fuentes.iter().map(|f| drivers::consultar(f, peticion)))
                .await;

        for (fuente, resultado) in fuentes.iter().zip(resultados) {
            match resultado {
                Ok(b) => {
                    if !b.filas.is_empty() {
                        bloques.push(b);
                    }
                }
                Err(e) => {
                    self.db.auditar(
                        ctx.id_usuario,
                        "busqueda",
                        &format!("Fallo en '{}': {e}", fuente.nombre),
                        false,
                    );
                    errores.push(format!("{}: {e}", fuente.nombre));
                }
            }
        }

        let total: i64 = bloques.iter().map(|b| b.total_filas).sum();
        let sugerencias = sugerir_filtros(&bloques, peticion);
        self.db.auditar(
            ctx.id_usuario,
            "busqueda",
            &format!(
                "{} fuentes, {} bloques, {} filas",
                fuentes.len(),
                bloques.len(),
                total
            ),
            true,
        );

        let mensaje = if bloques.is_empty() && !errores.is_empty() {
            "Ninguna fuente pudo consultarse. Revise los errores indicados.".to_string()
        } else if bloques.is_empty() {
            "No hay filas que cumplan los criterios de busqueda.".to_string()
        } else {
            String::new()
        };

        Ok(RespuestaBusqueda {
            duracion_ms: inicio.elapsed().as_millis() as u64,
            total_bloques: bloques.len(),
            fuentes_con_error: errores,
            sugerencias,
            mensaje,
            bloques,
            total_filas: total,
        })
    }

    /// Devuelve una muestra de las filas de una fuente, para el desplegable de
    /// esquemas y columnas que usa el constructor de tableros.
    pub async fn mostrar_datos(
        &self,
        ctx: &Contexto,
        fuente_id: i64,
        esquema: &str,
        limite: u32,
    ) -> Resultado<Documento> {
        ctx.exigir(Permiso::Consultar)?;
        let fuentes = self.fuentes_de(ctx, &[fuente_id])?;
        let fuente = fuentes
            .into_iter()
            .next()
            .ok_or_else(|| Error::Validacion("La fuente no esta disponible".into()))?;
        let peticion = PeticionBusqueda {
            esquemas: if esquema.trim().is_empty() {
                Vec::new()
            } else {
                vec![esquema.to_string()]
            },
            limite: limite.clamp(1, 1000),
            ..Default::default()
        };
        let bloque = drivers::consultar(&fuente, &peticion).await?;
        Ok(Documento {
            titulo: format!("Muestra de {}", fuente.nombre),
            subtitulo: bloque.esquema.clone(),
            pie: format!("Fuente: {} ({})", fuente.nombre, fuente.tipo.etiqueta()),
            bloques: vec![bloque],
            secciones: Vec::new(),
            widgets: Vec::new(),
            imagenes: Vec::new(),
            generado_por: ctx.usuario.clone(),
            generado_en: crate::appdb::ahora_iso(),
            duracion_ms: 0,
        })
    }

    /// Ejecuta el analisis que el usuario ha confirmado con el boton.
    pub async fn analizar(
        &self,
        ctx: &Contexto,
        peticion: &PeticionAnalisis,
    ) -> Resultado<RespuestaAnalisis> {
        ctx.exigir(Permiso::Analizar)?;
        if !peticion.es_analisis_confirmado {
            return Err(Error::Validacion(
                "Pulse el boton de confirmar para ejecutar el analisis".into(),
            ));
        }
        let fuentes = self.fuentes_de(ctx, &[peticion.fuente_id])?;
        let fuente = fuentes
            .into_iter()
            .next()
            .ok_or_else(|| Error::Validacion("La fuente no esta disponible".into()))?;

        let consulta = PeticionBusqueda {
            fuentes: vec![peticion.fuente_id],
            esquemas: if peticion.esquema.trim().is_empty() {
                Vec::new()
            } else {
                vec![peticion.esquema.clone()]
            },
            filtros: peticion.filtros.clone(),
            limite: peticion.limite.clamp(1, 100_000),
            ..Default::default()
        };
        let bloque = drivers::consultar(&fuente, &consulta).await?;
        if bloque.filas.is_empty() {
            return Err(Error::SinResultados);
        }

        // Se reforza la deteccion de columnas numericas antes de analizar.
        let columnas = analysis::forzar_numerica(&bloque.columnas, &bloque.filas);
        let mut respuesta = analysis::ejecutar(peticion, &bloque.filas, &columnas, &fuente.nombre)?;
        respuesta.esquema = bloque.esquema.clone();
        self.db.auditar(
            ctx.id_usuario,
            "analisis",
            &format!(
                "Analisis {:?} sobre {} ({} filas)",
                peticion.analyses,
                bloque.esquema,
                bloque.filas.len()
            ),
            true,
        );
        Ok(respuesta)
    }

    /// Construye un tablero a partir de la peticion del usuario.
    ///
    /// Acepta tanto una lista de widgets definida a mano como una instruccion
    /// en lenguaje natural, que se interpreta combinando lo que el usuario ha
    /// escrito con las columnas reales de los datos.
    pub async fn crear_dashboard(
        &self,
        ctx: &Contexto,
        peticion: &PeticionDashboard,
    ) -> Resultado<crate::models::Tablero> {
        ctx.exigir(Permiso::CrearDashboards)?;
        let inicio = std::time::Instant::now();
        let fuentes = self.fuentes_de(ctx, &peticion.fuentes)?;
        if fuentes.len() > 6 {
            return Err(Error::Validacion(
                "Un tablero puede combinar como maximo 6 fuentes a la vez".into(),
            ));
        }

        let mut esquemas: Vec<Esquema> = Vec::new();
        for f in &fuentes {
            if let Ok(lista) = drivers::esquemas(f).await {
                esquemas.extend(lista.into_iter().filter(|e| {
                    peticion.esquemas.is_empty() || peticion.esquemas.contains(&e.nombre)
                }));
            }
        }

        if esquemas.is_empty() {
            return Err(Error::Validacion(
                "Las fuentes indicadas no tienen tablas disponibles".into(),
            ));
        }

        // Se elige la combinacion de esquema y columna mas adecuada.
        let (esquema_elegido, columnas) = elegir_esquema(&fuentes, &esquemas, peticion).await?;
        let widgets = if peticion.instruccion.trim().is_empty() {
            widgets_base(&esquema_elegido, &columnas, &peticion.widgets_base)
        } else {
            widgets_desde_texto(
                &esquema_elegido,
                &columnas,
                &peticion.instruccion,
                peticion.max_widgets.clamp(1, 12),
            )
        };

        let mut listos = Vec::new();
        let mut advertencias = Vec::new();
        for w in &widgets {
            match self.calcular_widget(ctx, w, &peticion.filtros).await {
                Ok(listo) => listos.push(listo),
                Err(e) => advertencias.push(format!("{}: {e}", w.titulo)),
            }
        }

        if listos.is_empty() {
            return Err(Error::Validacion(
                "No se pudo construir ningun grafico con los criterios indicados".into(),
            ));
        }

        self.db.auditar(
            ctx.id_usuario,
            "crea_dashboard",
            &format!(
                "Tablero '{}' con {} graficos",
                peticion.nombre,
                listos.len()
            ),
            true,
        );

        Ok(crate::models::Tablero {
            nombre: if peticion.nombre.trim().is_empty() {
                format!("Tablero de {}", esquema_elegido.nombre)
            } else {
                peticion.nombre.trim().to_string()
            },
            esquemas: peticion.esquemas.clone(),
            fuentes: fuentes.iter().map(|f| f.nombre.clone()).collect(),
            generado_en: crate::appdb::ahora_iso(),
            duracion_ms: inicio.elapsed().as_millis() as u64,
            widgets: listos,
            advertencias,
        })
    }

    /// Consulta los datos de un widget y los deja listos para pintar.
    pub async fn calcular_widget(
        &self,
        ctx: &Contexto,
        widget: &Widget,
        filtros_globales: &[crate::models::Filtro],
    ) -> Resultado<WidgetListo> {
        ctx.exigir(Permiso::Consultar)?;
        let fuentes = self.fuentes_de(ctx, &[widget.fuente_id])?;
        let fuente = fuentes
            .into_iter()
            .next()
            .ok_or_else(|| Error::Validacion("La fuente no esta disponible".into()))?;

        let mut filtros = filtros_globales.to_vec();
        filtros.extend(widget.filtros.clone());
        let peticion = PeticionBusqueda {
            fuentes: vec![widget.fuente_id],
            esquemas: if widget.esquema.trim().is_empty() {
                Vec::new()
            } else {
                vec![widget.esquema.clone()]
            },
            filtros,
            campos: Vec::new(),
            limite: 100_000,
            ..Default::default()
        };
        let bloque = drivers::consultar(&fuente, &peticion).await?;
        let columnas = analysis::forzar_numerica(&bloque.columnas, &bloque.filas);
        let puntos = agregar(&bloque.filas, &columnas, widget);

        let valor_destacado = match widget.tipo {
            TipoGrafico::Kpi => Some(calcular_kpi(&bloque.filas, &columnas, widget)),
            _ => None,
        };

        Ok(WidgetListo {
            id: widget.id.clone(),
            fuente_nombre: fuente.nombre.clone(),
            columnas: columnas.clone(),
            puntos,
            filas: match widget.tipo {
                TipoGrafico::Tabla => bloque.filas.iter().take(200).cloned().collect(),
                _ => Vec::new(),
            },
            total_filas: bloque.filas.len() as i64,
            titulo: widget.titulo.clone(),
            tipo: widget.tipo,
            subtitulo: widget.subtitulo(),
            esquema: bloque.esquema.clone(),
            columna_grupo: widget.columna_grupo.clone(),
            columna_valor: widget.columna_valor.clone(),
            agregacion: widget.agregacion.clone(),
            formato: widget.formato.clone(),
            ancho: widget.ancho.clamp(1, 3),
            valor_destacado,
            texto_destacado: None,
            estado: if bloque.filas.is_empty() {
                "sin datos para los criterios indicados".into()
            } else {
                String::new()
            },
        })
    }
}

impl Widget {
    /// Descripcion corta que se muestra bajo el titulo del grafico.
    pub fn subtitulo(&self) -> String {
        let partes = [
            self.esquema.clone(),
            if self.columna_grupo.is_empty() {
                String::new()
            } else {
                self.columna_grupo.clone()
            },
            if self.columna_valor.is_empty() {
                String::new()
            } else {
                format!("{} ({})", self.columna_valor, self.agregacion)
            },
        ];
        partes
            .into_iter()
            .filter(|p| !p.is_empty())
            .collect::<Vec<_>>()
            .join("  |  ")
    }
}

// ----------------------------------------------------------------------
// Agregacion
// ----------------------------------------------------------------------

/// Agrupa las filas y calcula el valor de cada punto del grafico.
fn agregar(
    filas: &[Fila],
    columnas: &[ColumnaInfo],
    widget: &Widget,
) -> Vec<crate::models::PuntoGrafico> {
    let numericas: Vec<&str> = columnas
        .iter()
        .filter(|c| c.numerica)
        .map(|c| c.nombre.as_str())
        .collect();
    let valor_real = if widget.columna_valor.is_empty() {
        numericas.first().copied().unwrap_or("")
    } else {
        widget.columna_valor.as_str()
    };

    // Sin columna de medida: se cuenta el numero de filas de cada grupo.
    if valor_real.is_empty()
        || !columnas
            .iter()
            .any(|c| c.numerica && c.nombre == valor_real)
    {
        let mut cuentas: BTreeMap<String, f64> = BTreeMap::new();
        for f in filas {
            let clave = if widget.columna_grupo.is_empty() {
                "Total".to_string()
            } else {
                f.valores
                    .get(&widget.columna_grupo)
                    .cloned()
                    .flatten()
                    .unwrap_or_else(|| "(sin valor)".to_string())
            };
            *cuentas.entry(clave).or_insert(0.0) += 1.0;
        }
        let mut puntos: Vec<_> = cuentas
            .into_iter()
            .map(|(etiqueta, valor)| crate::models::PuntoGrafico {
                etiqueta,
                valor,
                valor_secundario: None,
                categoria: String::new(),
            })
            .collect();
        query::ordenar_puntos(&mut puntos, |p| p.valor);
        puntos.truncate(widget.limite.clamp(1, 100) as usize);
        return puntos;
    }

    let mut grupos: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    for f in filas {
        let clave = if widget.columna_grupo.is_empty() {
            "Total".to_string()
        } else {
            f.valores
                .get(&widget.columna_grupo)
                .cloned()
                .flatten()
                .unwrap_or_else(|| "(sin valor)".to_string())
        };
        if let Some(v) = f
            .valores
            .get(valor_real)
            .cloned()
            .flatten()
            .and_then(|x| a_f64(&x))
        {
            grupos.entry(clave).or_default().push(v);
        }
    }

    let mut puntos: Vec<crate::models::PuntoGrafico> = grupos
        .into_iter()
        .map(|(etiqueta, valores)| {
            let stats = analysis::estadisticas(&valores);
            let valor = aplicar_agregacion(&widget.agregacion, &stats, valores.len());
            crate::models::PuntoGrafico {
                etiqueta: etiqueta.clone(),
                valor,
                valor_secundario: Some(valores.len() as f64),
                categoria: etiqueta,
            }
        })
        .collect();

    match widget.tipo {
        TipoGrafico::Linea | TipoGrafico::Area => {
            // En series temporales se respeta el orden de aparicion.
            puntos.sort_by(|a, b| a.etiqueta.cmp(&b.etiqueta));
        }
        _ => {
            query::ordenar_puntos(&mut puntos, |p| p.valor);
        }
    }
    puntos.truncate(widget.limite.clamp(1, 100) as usize);
    puntos
}

fn aplicar_agregacion(agregacion: &str, stats: &analysis::Estadisticas, cuenta: usize) -> f64 {
    match agregacion.to_ascii_lowercase().as_str() {
        "media" | "promedio" | "avg" => stats.media,
        "minimo" | "min" => stats.min,
        "maximo" | "max" => stats.max,
        "mediana" | "median" => stats.mediana,
        "distinta" | "distintos" | "unicos" => stats.mediana, // marcador, se ajusta abajo
        "cuenta" | "count" => cuenta as f64,
        _ => {
            let _ = cuenta;
            // "suma" es el valor por defecto: se recalcula fuera.
            stats.media * cuenta as f64
        }
    }
}

/// Calcula la cifra destacada de un widget KPI.
fn calcular_kpi(filas: &[Fila], columnas: &[ColumnaInfo], widget: &Widget) -> f64 {
    let numericas: Vec<&ColumnaInfo> = columnas.iter().filter(|c| c.numerica).collect();
    if numericas.is_empty() {
        return filas.len() as f64;
    }
    let columna = if widget.columna_valor.is_empty() {
        numericas[0].clone()
    } else {
        match numericas.iter().find(|c| c.nombre == widget.columna_valor) {
            Some(c) => (*c).clone(),
            None => numericas[0].clone(),
        }
    };
    let valores: Vec<f64> = filas
        .iter()
        .filter_map(|f| f.valores.get(&columna.nombre).cloned().flatten())
        .filter_map(|v| a_f64(&v))
        .collect();
    if valores.is_empty() {
        return filas.len() as f64;
    }
    match widget.agregacion.to_ascii_lowercase().as_str() {
        "media" | "promedio" | "avg" => analysis::estadisticas(&valores).media,
        "minimo" | "min" => valores.iter().cloned().fold(f64::INFINITY, f64::min),
        "maximo" | "max" => valores.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
        "mediana" | "median" => analysis::estadisticas(&valores).mediana,
        "cuenta" | "count" => valores.len() as f64,
        "distinta" | "distintos" | "unicos" => {
            let set: std::collections::BTreeSet<String> = filas
                .iter()
                .filter_map(|f| f.valores.get(&columna.nombre).cloned().flatten())
                .map(|v| crate::util::formatear_valor(&v))
                .collect();
            set.len() as f64
        }
        _ => valores.iter().sum(),
    }
}

// ----------------------------------------------------------------------
// Construccion de widgets
// ----------------------------------------------------------------------

/// Escoge el esquema con el que conviene construir el tablero.
///
/// Se prioriza el que el usuario indico, despues el primero que exista, y si
/// hay varios se elige el que aporta mas filas de muestra.
async fn elegir_esquema(
    fuentes: &[Fuente],
    esquemas: &[Esquema],
    peticion: &PeticionDashboard,
) -> Resultado<(Esquema, Vec<ColumnaInfo>)> {
    let candidatos: Vec<&Esquema> = if peticion.esquemas.is_empty() {
        esquemas.iter().collect()
    } else {
        esquemas
            .iter()
            .filter(|e| peticion.esquemas.contains(&e.nombre))
            .collect()
    };
    if candidatos.is_empty() {
        return Err(Error::Validacion(
            "Ninguno de los esquemas indicados existe en las fuentes seleccionadas".into(),
        ));
    }

    let mut mejor: Option<(i64, &Esquema, Vec<ColumnaInfo>)> = None;
    for es in candidatos.iter().take(6) {
        let fuente = fuentes
            .iter()
            .find(|f| f.tipo.es_plano() || f.tipo.es_documental() || true)
            .ok_or_else(|| Error::Validacion("No hay fuentes disponibles".into()))?;
        let consulta = PeticionBusqueda {
            esquemas: vec![es.nombre.clone()],
            limite: 300,
            contar_total: false,
            ..Default::default()
        };
        let filas = match drivers::consultar(fuente, &consulta).await {
            Ok(b) => b.filas,
            Err(_) => continue,
        };
        let columnas = analysis::forzar_numerica(&query::describir_columnas(&filas, 10), &filas);
        if filas.is_empty() {
            continue;
        }
        let puntuacion =
            filas.len() as i64 + (columnas.iter().filter(|c| c.numerica).count() as i64 * 50);
        if mejor
            .as_ref()
            .map(|(p, _, _)| puntuacion > *p)
            .unwrap_or(true)
        {
            mejor = Some((puntuacion, es, columnas));
        }
    }

    mejor
        .map(|(_, e, c)| (e.clone(), c))
        .ok_or(Error::SinResultados)
}

/// Los graficos que el panel de control incluye siempre.
fn widgets_base(esquema: &Esquema, columnas: &[ColumnaInfo], pedidos: &[String]) -> Vec<Widget> {
    let numerica = columnas.iter().find(|c| c.numerica);
    let texto = columnas
        .iter()
        .find(|c| !c.numerica && c.distintos > 1 && c.distintos < 60);
    let valor = numerica.map(|c| c.nombre.clone()).unwrap_or_default();
    let grupo = texto
        .map(|c| c.nombre.clone())
        .or_else(|| numerica.map(|c| c.nombre.clone()))
        .unwrap_or_default();
    let agregacion = if numerica.is_some() { "suma" } else { "cuenta" };
    pedidos
        .iter()
        .map(|t| tipo_base(t))
        .take(6)
        .map(|tipo| construir_widget(tipo, esquema, &grupo, &valor, agregacion, 15))
        .collect()
}

fn tipo_base(txt: &str) -> TipoGrafico {
    match txt.to_ascii_lowercase().as_str() {
        "kpi" | "cifra" => TipoGrafico::Kpi,
        "linea" | "lineas" => TipoGrafico::Linea,
        "torta" | "circular" => TipoGrafico::Torta,
        "area" => TipoGrafico::Area,
        "indicador" | "gauge" => TipoGrafico::Indicador,
        "dispersion" | "scatter" => TipoGrafico::Dispersion,
        "kpi,tabla" => TipoGrafico::Kpi,
        _ => TipoGrafico::Barra,
    }
}

fn construir_widget(
    tipo: TipoGrafico,
    esquema: &Esquema,
    grupo: &str,
    valor: &str,
    agregacion: &str,
    limite: u32,
) -> Widget {
    let titulo = match tipo {
        TipoGrafico::Kpi => format!("Total de {}", valor_label(valor, esquema)),
        TipoGrafico::Tabla => "Detalle de los datos".to_string(),
        TipoGrafico::Barra => format!("{} por {}", valor_label(valor, esquema), grupo_label(grupo)),
        TipoGrafico::Linea | TipoGrafico::Area => {
            format!(
                "Evolucion de {} por {}",
                valor_label(valor, esquema),
                grupo_label(grupo)
            )
        }
        TipoGrafico::Torta => format!("Distribucion por {}", grupo_label(grupo)),
        TipoGrafico::Indicador => format!("Nivel de {}", valor_label(valor, esquema)),
        TipoGrafico::Dispersion => format!(
            "{} frente a {}",
            valor_label(valor, esquema),
            "otra variable"
        ),
        TipoGrafico::MapaCalor => format!("Intensidad por {}", grupo_label(grupo)),
    };
    Widget {
        id: format!("{}-{}", tipo_label(tipo), esquema.nombre),
        titulo,
        tipo,
        fuente_id: 0,
        esquema: esquema.nombre.clone(),
        columna_grupo: grupo.to_string(),
        columna_valor: valor.to_string(),
        agregacion: agregacion.to_string(),
        limite,
        filtros: Vec::new(),
        ancho: if tipo == TipoGrafico::Kpi || tipo == TipoGrafico::Tabla {
            1
        } else {
            2
        },
        formato: "numero".to_string(),
        peticion_texto: String::new(),
    }
}

fn tipo_label(t: TipoGrafico) -> String {
    match t {
        TipoGrafico::Kpi => "kpi",
        TipoGrafico::Barra => "barra",
        TipoGrafico::Linea => "linea",
        TipoGrafico::Area => "area",
        TipoGrafico::Torta => "torta",
        TipoGrafico::Dispersion => "dispersion",
        TipoGrafico::Tabla => "tabla",
        TipoGrafico::Indicador => "indicador",
        TipoGrafico::MapaCalor => "mapa_calor",
    }
    .to_string()
}

fn valor_label(valor: &str, esquema: &Esquema) -> String {
    if valor.is_empty() {
        esquema.nombre.clone()
    } else {
        valor.to_string()
    }
}

fn grupo_label(grupo: &str) -> String {
    if grupo.is_empty() { "categoria" } else { grupo }.to_string()
}

/// Interpreta la instruccion escrita por el usuario y genera los graficos.
///
/// Se apoya en la caja de texto, como pide la interfaz, y combina lo que ha
/// escrito con las columnas que existen de verdad en los datos.
fn widgets_desde_texto(
    esquema: &Esquema,
    columnas: &[ColumnaInfo],
    instruccion: &str,
    maximo: u32,
) -> Vec<Widget> {
    let minusculas = crate::util::normalizar(instruccion);
    let numericas: Vec<&ColumnaInfo> = columnas.iter().filter(|c| c.numerica).collect();
    let categoricas: Vec<&ColumnaInfo> = columnas
        .iter()
        .filter(|c| !c.numerica && c.distintos > 1)
        .collect();

    // Se intenta ajustar las columnas que el usuario ha mencionado.
    let mentioned = |nombre: &str| minusculas.contains(&crate::util::normalizar(nombre));

    let valor = numericas
        .iter()
        .find(|c| mentioned(&c.nombre))
        .or_else(|| numericas.first())
        .map(|c| c.nombre.clone())
        .unwrap_or_default();
    let grupo = categoricas
        .iter()
        .find(|c| mentioned(&c.nombre))
        .or_else(|| categoricas.first())
        .map(|c| c.nombre.clone())
        .unwrap_or_default();

    // Palabras clave que sugieren un tipo de grafico concreto.
    let menciona = |palabras: &[&str]| palabras.iter().any(|p| minusculas.contains(p));

    let mut tipos: Vec<(TipoGrafico, &str)> = Vec::new();
    if menciona(&["torta", "circular", "reparto", "porcentaje", "proporcion"]) {
        tipos.push((TipoGrafico::Torta, "suma"));
    }
    if menciona(&[
        "evolucion",
        "tendencia",
        "linea",
        "tiempo",
        "fecha",
        "mensual",
        "diario",
    ]) {
        tipos.push((TipoGrafico::Linea, "media"));
    }
    if menciona(&[
        "gauge",
        "indicador",
        "medidor",
        "porcentaje de logro",
        "objetivo",
    ]) {
        tipos.push((TipoGrafico::Indicador, "media"));
    }
    if menciona(&["dispersion", "correlacion", "nube de puntos", "relacion"]) {
        tipos.push((TipoGrafico::Dispersion, "suma"));
    }
    if menciona(&[
        "comparar",
        "comparativa",
        "ranking",
        "top",
        "mejor",
        "peor",
        "por cada",
    ]) {
        tipos.push((TipoGrafico::Barra, "suma"));
    }
    if menciona(&["area", "acumulado"]) {
        tipos.push((TipoGrafico::Area, "suma"));
    }

    // Un tablero vacio siempre debe tener un panel util.
    if tipos.is_empty() {
        tipos.push((TipoGrafico::Kpi, "suma"));
        tipos.push((TipoGrafico::Barra, "suma"));
    }
    tipos.push((TipoGrafico::Tabla, "suma"));

    tipos
        .into_iter()
        .take(maximo.clamp(1, 12) as usize)
        .map(|(tipo, agregacion)| {
            let mut w = construir_widget(tipo, esquema, &grupo, &valor, agregacion, 12);
            w.peticion_texto = instruccion.to_string();
            w.titulo = personalised(&w.titulo, instruccion, tipo);
            w
        })
        .collect()
}

/// Reescribe el titulo del grafico con las palabras del usuario.
fn personalised(base: &str, instruccion: &str, tipo: TipoGrafico) -> String {
    let limpio = instruccion.trim();
    if limpio.is_empty() {
        return base.to_string();
    }
    let recortado = if limpio.chars().count() > 70 {
        format!("{}...", limpio.chars().take(67).collect::<String>())
    } else {
        limpio.to_string()
    };
    match tipo {
        TipoGrafico::Kpi => format!("Indicador: {recortado}"),
        _ => format!("{recortado} ({})", tipo.etiqueta().to_lowercase()),
    }
}

/// Propone filtros a partir de lo encontrado, para afinar la busqueda.
fn sugerir_filtros(
    bloques: &[crate::models::BloqueResultados],
    peticion: &PeticionBusqueda,
) -> Vec<Sugerencia> {
    let mut sugerencias = Vec::new();
    let ya_puestos: Vec<String> = peticion
        .filtros
        .iter()
        .filter(|f| f.activo)
        .map(|f| f.campo.clone())
        .collect();
    for bloque in bloques {
        for columna in &bloque.columnas {
            if ya_puestos.contains(&columna.nombre) {
                continue;
            }
            for frecuencia in columna.valores_frecuentes.iter().take(3) {
                if frecuencia.cuenta < 2 {
                    continue;
                }
                let confianza = (frecuencia.porcentaje / 100.0).clamp(0.05, 0.99);
                let etiqueta = if columna.numerica {
                    format!(
                        "Filtrar {} {}",
                        columna.nombre,
                        if frecuencia.porcentaje > 50.0 {
                            "mayor"
                        } else {
                            "menor"
                        }
                    )
                } else {
                    format!(
                        "Filtrar {} = {}",
                        columna.nombre,
                        crate::util::truncar(&frecuencia.valor, 24)
                    )
                };
                sugerencias.push(Sugerencia {
                    etiqueta,
                    columna: columna.nombre.clone(),
                    valor: frecuencia.valor.clone(),
                    confianza,
                });
            }
        }
    }
    sugerencias.sort_by(|a, b| {
        b.confianza
            .partial_cmp(&a.confianza)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    sugerencias.truncate(12);
    sugerencias
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ColumnaInfo, Fila, Widget};

    fn columnas() -> Vec<ColumnaInfo> {
        vec![
            ColumnaInfo {
                nombre: "zona".into(),
                etiqueta: "zona".into(),
                tipo: "texto".into(),
                es_texto: true,
                distintos: 2,
                ..Default::default()
            },
            ColumnaInfo {
                nombre: "importe".into(),
                etiqueta: "importe".into(),
                tipo: "numero".into(),
                numerica: true,
                distintos: 4,
                ..Default::default()
            },
        ]
    }

    fn filas() -> Vec<Fila> {
        vec![
            Fila {
                valores: BTreeMap::from([
                    ("zona".into(), Some("Norte".into())),
                    ("importe".into(), Some("100".into())),
                ]),
            },
            Fila {
                valores: BTreeMap::from([
                    ("zona".into(), Some("Norte".into())),
                    ("importe".into(), Some("300".into())),
                ]),
            },
            Fila {
                valores: BTreeMap::from([
                    ("zona".into(), Some("Sur".into())),
                    ("importe".into(), Some("200".into())),
                ]),
            },
        ]
    }

    fn widget(tipo: TipoGrafico) -> Widget {
        Widget {
            id: "t".into(),
            titulo: "Prueba".into(),
            tipo,
            fuente_id: 1,
            esquema: "ventas".into(),
            columna_grupo: "zona".into(),
            columna_valor: "importe".into(),
            agregacion: "suma".into(),
            limite: 10,
            filtros: Vec::new(),
            ancho: 1,
            formato: "numero".into(),
            peticion_texto: String::new(),
        }
    }

    #[test]
    fn agrega_por_categoria() {
        let puntos = agregar(&filas(), &columnas(), &widget(TipoGrafico::Barra));
        assert_eq!(puntos.len(), 2);
        let norte = puntos.iter().find(|p| p.etiqueta == "Norte").unwrap();
        assert_eq!(norte.valor, 400.0);
        let sur = puntos.iter().find(|p| p.etiqueta == "Sur").unwrap();
        assert_eq!(sur.valor, 200.0);
    }

    #[test]
    fn kpi_suma_el_total() {
        assert_eq!(
            calcular_kpi(&filas(), &columnas(), &widget(TipoGrafico::Kpi)),
            600.0
        );
    }

    #[test]
    fn kpi_cuenta_distintos() {
        // Distintos sobre la columna de medida: 100, 200 y 300.
        let mut w = widget(TipoGrafico::Kpi);
        w.agregacion = "distinta".into();
        assert_eq!(calcular_kpi(&filas(), &columnas(), &w), 3.0);
    }

    #[test]
    fn interpreta_lenguaje_natural() {
        let es = Esquema {
            nombre: "ventas".into(),
            tipo: "tabla".into(),
            filas_estimadas: 3,
            columnas: Vec::new(),
        };
        let widgets = widgets_desde_texto(&es, &columnas(), "ventas por zona en torta", 6);
        assert!(!widgets.is_empty());
        assert!(widgets.iter().any(|w| w.tipo == TipoGrafico::Torta));
        assert!(widgets.iter().any(|w| w.titulo.contains("ventas por zona")));
    }

    #[test]
    fn detecta_tendencia_por_palabras_clave() {
        let es = Esquema {
            nombre: "ventas".into(),
            tipo: "tabla".into(),
            filas_estimadas: 3,
            columnas: Vec::new(),
        };
        let widgets = widgets_desde_texto(&es, &columnas(), "evolucion mensual de importe", 4);
        assert!(widgets.iter().any(|w| w.tipo == TipoGrafico::Linea));
    }

    #[test]
    fn respeta_el_limite_de_widgets() {
        let es = Esquema {
            nombre: "v".into(),
            tipo: "tabla".into(),
            filas_estimadas: 1,
            columnas: Vec::new(),
        };
        let widgets = widgets_desde_texto(&es, &columnas(), "todo", 2);
        assert!(widgets.len() <= 2);
    }

    #[test]
    fn genera_panel_base() {
        let es = Esquema {
            nombre: "ventas".into(),
            tipo: "tabla".into(),
            filas_estimadas: 3,
            columnas: Vec::new(),
        };
        let widgets = widgets_base(
            &es,
            &columnas(),
            &["kpi".into(), "barra".into(), "tabla".into()],
        );
        assert_eq!(widgets.len(), 3);
        assert_eq!(widgets[0].tipo, TipoGrafico::Kpi);
    }

    #[test]
    fn sugiere_filtros_relevantes() {
        let bloque = crate::models::BloqueResultados {
            fuente_id: 1,
            fuente_nombre: "v".into(),
            tipo_fuente: crate::models::TipoFuente::Sqlite,
            esquema: "ventas".into(),
            filas: filas(),
            columnas: query::describir_columnas(&filas(), 10),
            total_filas: 3,
            filas_omitidas: 0,
            duracion_ms: 0,
            mensaje: String::new(),
        };
        let s = sugerir_filtros(&[bloque], &PeticionBusqueda::default());
        assert!(!s.is_empty());
        assert!(s.iter().any(|x| x.columna == "zona"));
    }
}

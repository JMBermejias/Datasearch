//! Analisis estadistico de los datos.
//!
//! El usuario elige que analisis ejecutar y lo confirma con un boton. El
//! motor calcula los indicadores sobre los datos ya filtrados y devuelve
//! secciones legibles, pensadas para mostrarse en la interfaz y para las
//! exportaciones.
//!
//! Copyright (c) 2026 Jose Manuel Bernabeu Mejias
//! Licencia MIT

use crate::error::{Error, Resultado};
use crate::models::{ColumnaInfo, Fila, Metrica, PeticionAnalisis, SeccionAnalisis, TipoAnalisis};
use crate::util::{a_f64, formatear_numero, Familia};
use std::collections::BTreeMap;

/// Normaliza los nombres de analisis y deja el conjunto deduplicado.
pub fn analyses_pedidas(p: &PeticionAnalisis) -> Vec<TipoAnalisis> {
    if p.analyses.is_empty() {
        return vec![TipoAnalisis::Resumen];
    }
    let mut salida: Vec<TipoAnalisis> = Vec::new();
    for a in &p.analyses {
        let t = TipoAnalisis::desde_txt(a);
        if !salida.contains(&t) {
            salida.push(t);
        }
    }
    salida
}

/// Ejecuta el analisis pedido.
///
/// `p.esAnalisisConfirmado` debe ser `true`: el usuario tiene que pulsar el
/// boton de confirmacion, tal y como se exige en la interfaz.
pub fn ejecutar(
    peticion: &PeticionAnalisis,
    filas: &[Fila],
    columnas: &[ColumnaInfo],
    nombre_fuente: &str,
) -> Resultado<crate::models::RespuestaAnalisis> {
    if !peticion.es_analisis_confirmado {
        return Err(Error::Validacion(
            "Debe confirmar el analisis antes de ejecutarlo".into(),
        ));
    }
    if filas.is_empty() {
        return Err(Error::SinResultados);
    }
    let inicio = std::time::Instant::now();

    let columnas_interes: Vec<ColumnaInfo> = if peticion.columnas.is_empty() {
        columnas.to_vec()
    } else {
        columnas
            .iter()
            .filter(|c| peticion.columnas.contains(&c.nombre))
            .cloned()
            .collect()
    };
    if columnas_interes.is_empty() {
        return Err(Error::Validacion(
            "Las columnas indicadas no existen en los datos".into(),
        ));
    }

    let tipos = analyses_pedidas(peticion);
    let mut secciones: Vec<SeccionAnalisis> = Vec::new();
    let mut advertencias: Vec<String> = Vec::new();

    for t in &tipos {
        match t {
            TipoAnalisis::Resumen => {
                secciones.push(seccion_describir(&columnas_interes, filas));
                secciones.push(seccion_nulos(&columnas_interes, filas));
                secciones.push(seccion_frecuencias(&columnas_interes, filas));
            }
            TipoAnalisis::Describir => secciones.push(seccion_describir(&columnas_interes, filas)),
            TipoAnalisis::Nulos => secciones.push(seccion_nulos(&columnas_interes, filas)),
            TipoAnalisis::Distintos => secciones.push(seccion_distintos(&columnas_interes, filas)),
            TipoAnalisis::Frecuencias => {
                secciones.push(seccion_frecuencias(&columnas_interes, filas))
            }
            TipoAnalisis::Histograma => match seccion_histograma(&columnas_interes, filas) {
                Ok(s) => secciones.push(s),
                Err(e) => advertencias.push(e.to_string()),
            },
            TipoAnalisis::Correlacion => match seccion_correlacion(&columnas_interes, filas) {
                Ok(s) => secciones.push(s),
                Err(e) => advertencias.push(e.to_string()),
            },
            TipoAnalisis::Outliers => match seccion_atipicos(&columnas_interes, filas) {
                Ok(s) => secciones.push(s),
                Err(e) => advertencias.push(e.to_string()),
            },
            TipoAnalisis::Tendencia => match seccion_tendencia(&columnas_interes, filas) {
                Ok(s) => secciones.push(s),
                Err(e) => advertencias.push(e.to_string()),
            },
            TipoAnalisis::Calidad => secciones.push(seccion_calidad(&columnas_interes, filas)),
            TipoAnalisis::Duplicados => secciones.push(seccion_duplicados(filas)),
        }
    }

    let resumen = construir_resumen(&secciones, filas.len(), nombre_fuente);
    Ok(crate::models::RespuestaAnalisis {
        fuente_nombre: nombre_fuente.to_string(),
        esquema: String::new(),
        secciones,
        filas_analizadas: filas.len() as i64,
        columnas_analizadas: columnas_interes.iter().map(|c| c.nombre.clone()).collect(),
        duracion_ms: inicio.elapsed().as_millis() as u64,
        resumen,
        advertencias,
    })
}

// ----------------------------------------------------------------------
// Secciones
// ----------------------------------------------------------------------

fn valores_de(filas: &[Fila], columna: &str) -> Vec<String> {
    filas
        .iter()
        .filter_map(|f| f.valores.get(columna).cloned().flatten())
        .collect()
}

fn numeros_de(filas: &[Fila], columna: &str) -> Vec<f64> {
    filas
        .iter()
        .filter_map(|f| f.valores.get(columna).cloned().flatten())
        .filter_map(|v| a_f64(&v))
        .collect()
}

fn seccion_describir(columnas: &[ColumnaInfo], filas: &[Fila]) -> SeccionAnalisis {
    let total = filas.len() as i64;
    let mut f = Vec::new();
    let mut metricas: Vec<Metrica> = Vec::new();

    for c in columnas {
        let presentes = valores_de(filas, &c.nombre).len() as i64;
        let nulos = total - presentes;
        let completitud = if total == 0 {
            0.0
        } else {
            (presentes as f64 / total as f64) * 100.0
        };

        let mut mapa: BTreeMap<String, crate::models::ValorCelda> = BTreeMap::from([
            ("Columna".to_string(), Some(c.nombre.clone())),
            ("Tipo".to_string(), Some(c.tipo.clone())),
            (
                "Completitud".to_string(),
                Some(format!("{completitud:.2} %")),
            ),
            ("Nulos".to_string(), Some(nulos.to_string())),
            ("Distintos".to_string(), Some(c.distintos.to_string())),
        ]);

        metricas.push(Metrica {
            nombre: format!("{} - filas con dato", c.nombre),
            valor: presentes.to_string(),
            unidad: String::new(),
        });

        let valores = numeros_de(filas, &c.nombre);
        if !valores.is_empty() {
            let s = estadisticas(&valores);
            mapa.insert("Minimo".to_string(), Some(formatear_numero(s.min, 2)));
            mapa.insert("Maximo".to_string(), Some(formatear_numero(s.max, 2)));
            mapa.insert("Media".to_string(), Some(formatear_numero(s.media, 4)));
            mapa.insert("Mediana".to_string(), Some(formatear_numero(s.mediana, 4)));
            mapa.insert(
                "Desviacion".to_string(),
                Some(formatear_numero(s.desviacion, 4)),
            );
            mapa.insert("Cuartil 25".to_string(), Some(formatear_numero(s.q1, 4)));
            mapa.insert("Cuartil 75".to_string(), Some(formatear_numero(s.q3, 4)));
            mapa.insert(
                "Asimetria".to_string(),
                Some(formatear_numero(s.asimetria, 4)),
            );
            mapa.insert(
                "Coef. variacion".to_string(),
                Some(formatear_numero(
                    if s.media.abs() > f64::EPSILON {
                        s.desviacion / s.media.abs() * 100.0
                    } else {
                        0.0
                    },
                    2,
                )),
            );
            for (nombre, valor) in [
                ("Media", s.media),
                ("Mediana", s.mediana),
                ("Desviacion tipica", s.desviacion),
            ] {
                metricas.push(Metrica {
                    nombre: format!("{} - {nombre}", c.nombre),
                    valor: formatear_numero(valor, 4),
                    unidad: String::new(),
                });
            }
        }
        f.push(Fila { valores: mapa });
    }

    SeccionAnalisis {
        titulo: "Descripcion estadistica".into(),
        tipo: TipoAnalisis::Describir.etiqueta().to_string(),
        descripcion: format!(
            "Indicadores de las {} columnas sobre {total} filas.",
            columnas.len()
        ),
        filas: f,
        clave: BTreeMap::new(),
        metricas,
    }
}

fn seccion_nulos(columnas: &[ColumnaInfo], filas: &[Fila]) -> SeccionAnalisis {
    let total = filas.len() as i64;
    let mut f = Vec::new();
    for c in columnas {
        let nulos = total - valores_de(filas, &c.nombre).len() as i64;
        let pct = if total == 0 {
            0.0
        } else {
            (nulos as f64 / total as f64) * 100.0
        };
        f.push(Fila {
            valores: BTreeMap::from([
                ("Columna".to_string(), Some(c.nombre.clone())),
                ("Tipo".to_string(), Some(c.tipo.clone())),
                ("Nulos".to_string(), Some(nulos.to_string())),
                ("Porcentaje".to_string(), Some(formatear_numero(pct, 2))),
                (
                    "Completitud".to_string(),
                    Some(formatear_numero(100.0 - pct, 2)),
                ),
            ]),
        });
    }
    SeccionAnalisis {
        titulo: "Analisis de valores nulos".into(),
        tipo: TipoAnalisis::Nulos.etiqueta().to_string(),
        descripcion: format!("Reparto de valores ausentes por columna sobre {total} filas."),
        filas: f,
        clave: BTreeMap::new(),
        metricas: Vec::new(),
    }
}

fn seccion_distintos(columnas: &[ColumnaInfo], filas: &[Fila]) -> SeccionAnalisis {
    let total = filas.len() as i64;
    let mut f_distintos = Vec::new();
    for c in columnas {
        let distintos: BTreeMap<String, ()> = valores_de(filas, &c.nombre)
            .into_iter()
            .map(|v| (v, ()))
            .collect();
        let n = distintos.len() as i64;
        f_distintos.push(Fila {
            valores: BTreeMap::from([
                ("Columna".to_string(), Some(c.nombre.clone())),
                ("Distintos".to_string(), Some(n.to_string())),
                (
                    "Porcentaje sobre el total".to_string(),
                    Some(formatear_numero(
                        if total == 0 {
                            0.0
                        } else {
                            n as f64 / total as f64 * 100.0
                        },
                        2,
                    )),
                ),
                (
                    "Es identificador".to_string(),
                    Some(if n == total { "si" } else { "no" }.to_string()),
                ),
            ]),
        });
    }
    SeccionAnalisis {
        titulo: "Valores distintos".into(),
        tipo: TipoAnalisis::Distintos.etiqueta().to_string(),
        descripcion: "Cuantas combinaciones diferentes contiene cada columna.".into(),
        filas: f_distintos,
        clave: BTreeMap::new(),
        metricas: Vec::new(),
    }
}

fn seccion_frecuencias(columnas: &[ColumnaInfo], filas: &[Fila]) -> SeccionAnalisis {
    let _ = filas.len();
    let mut f = Vec::new();
    for c in columnas {
        if c.numerica {
            continue;
        }
        for frec in c.valores_frecuentes.iter().take(15) {
            f.push(Fila {
                valores: BTreeMap::from([
                    ("Columna".to_string(), Some(c.nombre.clone())),
                    ("Valor".to_string(), Some(frec.valor.clone())),
                    ("Apariciones".to_string(), Some(frec.cuenta.to_string())),
                    (
                        "Porcentaje".to_string(),
                        Some(formatear_numero(frec.porcentaje, 2)),
                    ),
                ]),
            });
        }
    }
    SeccionAnalisis {
        titulo: "Valores mas frecuentes".into(),
        tipo: TipoAnalisis::Frecuencias.etiqueta().to_string(),
        descripcion: "Los 15 valores mas repetidos de cada columna de texto.".into(),
        filas: f,
        clave: BTreeMap::new(),
        metricas: Vec::new(),
    }
}

fn seccion_histograma(columnas: &[ColumnaInfo], filas: &[Fila]) -> Resultado<SeccionAnalisis> {
    let numericas: Vec<&ColumnaInfo> = columnas.iter().filter(|c| c.numerica).collect();
    if numericas.is_empty() {
        return Err(Error::Validacion(
            "No hay columnas numericas para construir un histograma".into(),
        ));
    }
    let mut f = Vec::new();
    for c in numericas {
        let valores = numeros_de(filas, &c.nombre);
        if valores.is_empty() {
            continue;
        }
        let stats = estadisticas(&valores);
        let cortes = vec![
            ("Menor o igual que", stats.min),
            ("Primer cuartil", stats.q1),
            ("Mediana", stats.mediana),
            ("Tercer cuartil", stats.q3),
            ("Mayor o igual que", stats.max),
        ];
        for (i, (_etiqueta, limite)) in cortes.iter().enumerate() {
            let (inferior, superior) = if i == 0 {
                (None, Some(*limite))
            } else if i == cortes.len() - 1 {
                (Some(*limite), None)
            } else {
                (Some(*limite), Some(cortes[i + 1].1))
            };
            let cuenta = valores
                .iter()
                .filter(|v| {
                    inferior.map(|l| **v >= l).unwrap_or(true)
                        && superior.map(|s| **v < s).unwrap_or(true)
                })
                .count();
            f.push(Fila {
                valores: BTreeMap::from([
                    ("Columna".to_string(), Some(c.nombre.clone())),
                    (
                        "Intervalo".to_string(),
                        Some(format_intervalo(inferior, superior)),
                    ),
                    ("Frecuencia".to_string(), Some(cuenta.to_string())),
                    (
                        "Porcentaje".to_string(),
                        Some(formatear_numero(
                            if valores.is_empty() {
                                0.0
                            } else {
                                cuenta as f64 / valores.len() as f64 * 100.0
                            },
                            2,
                        )),
                    ),
                ]),
            });
        }
    }
    Ok(SeccionAnalisis {
        titulo: "Histograma".into(),
        tipo: TipoAnalisis::Histograma.etiqueta().to_string(),
        descripcion: "Reparto por cuartiles de cada columna numerica.".into(),
        filas: f,
        clave: BTreeMap::new(),
        metricas: Vec::new(),
    })
}

fn format_intervalo(inferior: Option<f64>, superior: Option<f64>) -> String {
    match (inferior, superior) {
        (None, Some(s)) => format!("<= {}", formatear_numero(s, 2)),
        (Some(i), None) => format!(">= {}", formatear_numero(i, 2)),
        (Some(i), Some(s)) => format!("{} - {}", formatear_numero(i, 2), formatear_numero(s, 2)),
        (None, None) => "(sin limites)".into(),
    }
}

fn seccion_correlacion(columnas: &[ColumnaInfo], filas: &[Fila]) -> Resultado<SeccionAnalisis> {
    let numericas: Vec<&ColumnaInfo> = columnas.iter().filter(|c| c.numerica).collect();
    if numericas.len() < 2 {
        return Err(Error::Validacion(
            "Se necesitan al menos dos columnas numericas".into(),
        ));
    }
    let series: Vec<(&str, Vec<f64>)> = numericas
        .iter()
        .map(|c| (c.nombre.as_str(), numeros_de(filas, &c.nombre)))
        .collect();

    let mut f = Vec::new();
    for (nombre_a, a) in &series {
        for (nombre_b, b) in &series {
            if a.len() < 3 || b.len() < 3 {
                continue;
            }
            let r = pearson(a, b);
            let fuerza = fuerza_correlacion(r);
            f.push(Fila {
                valores: BTreeMap::from([
                    ("Columna A".to_string(), Some((*nombre_a).to_string())),
                    ("Columna B".to_string(), Some((*nombre_b).to_string())),
                    (
                        "Correlacion de Pearson".to_string(),
                        Some(formatear_numero(r, 4)),
                    ),
                    ("Fuerza".to_string(), Some(fuerza.to_string())),
                ]),
            });
        }
    }
    Ok(SeccionAnalisis {
        titulo: "Matriz de correlacion".into(),
        tipo: TipoAnalisis::Correlacion.etiqueta().to_string(),
        descripcion:
            "Correlacion de Pearson entre columnas numericas. Valores próximos a 1 o -1 indican relacion fuerte."
                .into(),
        filas: f,
        clave: BTreeMap::new(),
        metricas: Vec::new(),
    })
}

fn seccion_atipicos(columnas: &[ColumnaInfo], filas: &[Fila]) -> Resultado<SeccionAnalisis> {
    let numericas: Vec<&ColumnaInfo> = columnas.iter().filter(|c| c.numerica).collect();
    if numericas.is_empty() {
        return Err(Error::Validacion(
            "No hay columnas numericas donde buscar valores atipicos".into(),
        ));
    }
    let mut f = Vec::new();
    for c in numericas {
        let valores = numeros_de(filas, &c.nombre);
        if valores.len() < 4 {
            continue;
        }
        let stats = estadisticas(&valores);
        let limite_inf = stats.media - 3.0 * stats.desviacion;
        let limite_sup = stats.media + 3.0 * stats.desviacion;
        for (i, fila) in filas.iter().enumerate() {
            let valor = fila.valores.get(&c.nombre).cloned().flatten();
            if let Some(v) = valor.as_deref().and_then(a_f64) {
                if v < limite_inf || v > limite_sup {
                    f.push(Fila {
                        valores: BTreeMap::from([
                            ("Fila".to_string(), Some((i + 1).to_string())),
                            ("Columna".to_string(), Some(c.nombre.clone())),
                            ("Valor".to_string(), Some(formatear_numero(v, 2))),
                            (
                                "Limite inferior".to_string(),
                                Some(formatear_numero(limite_inf, 2)),
                            ),
                            (
                                "Limite superior".to_string(),
                                Some(formatear_numero(limite_sup, 2)),
                            ),
                            (
                                "Desviacion".to_string(),
                                Some(if stats.desviacion > 0.0 {
                                    formatear_numero((v - stats.media) / stats.desviacion, 2)
                                } else {
                                    "0".into()
                                }),
                            ),
                        ]),
                    });
                }
            }
        }
    }
    Ok(SeccionAnalisis {
        titulo: "Valores atipicos (outliers)".into(),
        tipo: TipoAnalisis::Outliers.etiqueta().to_string(),
        descripcion: format!(
            "Valores que se apartan mas de 3 desviaciones tipicas de la media. {} detectados.",
            f.len()
        ),
        filas: f,
        clave: BTreeMap::new(),
        metricas: Vec::new(),
    })
}

fn seccion_tendencia(columnas: &[ColumnaInfo], filas: &[Fila]) -> Resultado<SeccionAnalisis> {
    let numericas: Vec<&ColumnaInfo> = columnas.iter().filter(|c| c.numerica).collect();
    if numericas.is_empty() {
        return Err(Error::Validacion(
            "No hay columnas numericas para analizar la tendencia".into(),
        ));
    }
    let mut f = Vec::new();
    for c in numericas {
        let valores = numeros_de(filas, &c.nombre);
        if valores.len() < 3 {
            continue;
        }
        // Regresion lineal simple: y = a + b * x
        let (a, b) = regresion_lineal(&valores);
        let inicio = valores[0];
        let fin = *valores.last().unwrap();
        let direccion = if b > 0.0001 {
            "creciente"
        } else if b < -0.0001 {
            "decreciente"
        } else {
            "estable"
        };
        f.push(Fila {
            valores: BTreeMap::from([
                ("Columna".to_string(), Some(c.nombre.clone())),
                ("Pendiente".to_string(), Some(formatear_numero(b, 4))),
                ("Intercepto".to_string(), Some(formatear_numero(a, 4))),
                ("Direccion".to_string(), Some(direccion.to_string())),
                (
                    "Valor inicial".to_string(),
                    Some(formatear_numero(inicio, 2)),
                ),
                ("Valor final".to_string(), Some(formatear_numero(fin, 2))),
                (
                    "Variacion total".to_string(),
                    Some(formatear_numero(fin - inicio, 2)),
                ),
            ]),
        });
    }
    Ok(SeccionAnalisis {
        titulo: "Tendencia de la serie".into(),
        tipo: TipoAnalisis::Tendencia.etiqueta().to_string(),
        descripcion: "Regresion lineal sobre el orden de las filas.".into(),
        filas: f,
        clave: BTreeMap::new(),
        metricas: Vec::new(),
    })
}

fn seccion_calidad(columnas: &[ColumnaInfo], filas: &[Fila]) -> SeccionAnalisis {
    let total = filas.len() as f64;
    let mut metricas = vec![Metrica {
        nombre: "Filas analizadas".into(),
        valor: filas.len().to_string(),
        unidad: String::new(),
    }];
    let mut puntuacion = 100.0f64;
    let total_columnas = columnas.len().max(1) as f64;
    let mut f = Vec::new();

    for c in columnas {
        let nulos = filas.len() as i64 - valores_de(filas, &c.nombre).len() as i64;
        let pct_nulos = if total == 0.0 {
            0.0
        } else {
            nulos as f64 / total * 100.0
        };
        // Los valores vacios "", "NULL", "N/A" y "-" se consideran falta de dato.
        let ios = valores_de(filas, &c.nombre)
            .iter()
            .filter(|v| {
                let n = crate::util::normalizar(v);
                matches!(
                    n.as_str(),
                    "" | "null" | "n/a" | "na" | "-" | "?" | "sin datos"
                )
            })
            .count();
        let pct_ios = if total == 0.0 {
            0.0
        } else {
            ios as f64 / total * 100.0
        };
        puntuacion -= (pct_nulos + pct_ios) / total_columnas;
        f.push(Fila {
            valores: BTreeMap::from([
                ("Columna".to_string(), Some(c.nombre.clone())),
                ("Tipo".to_string(), Some(c.tipo.clone())),
                ("Nulos".to_string(), Some(nulos.to_string())),
                (
                    "Porc. nulos".to_string(),
                    Some(formatear_numero(pct_nulos, 2)),
                ),
                ("Valores sin dato".to_string(), Some(ios.to_string())),
                (
                    "Porc. sin dato".to_string(),
                    Some(formatear_numero(pct_ios, 2)),
                ),
                ("Distintos".to_string(), Some(c.distintos.to_string())),
            ]),
        });
    }
    metricas.push(Metrica {
        nombre: "Puntuacion de calidad".into(),
        valor: formatear_numero(puntuacion.clamp(0.0, 100.0), 1),
        unidad: "/100".into(),
    });

    SeccionAnalisis {
        titulo: "Calidad de los datos".into(),
        tipo: TipoAnalisis::Calidad.etiqueta().to_string(),
        descripcion: "Puntuccion estimada a partir de los valores ausentes y vacios.".into(),
        filas: f,
        clave: BTreeMap::new(),
        metricas,
    }
}

fn seccion_duplicados(filas: &[Fila]) -> SeccionAnalisis {
    let mut cuenta: BTreeMap<String, usize> = BTreeMap::new();
    for fila in filas {
        let clave: Vec<String> = fila
            .valores
            .values()
            .map(|v| v.clone().unwrap_or_default())
            .collect();
        *cuenta.entry(clave.join("\u{1}")).or_insert(0) += 1;
    }
    let mut duplicados: Vec<(&String, &usize)> = cuenta.iter().filter(|(_, c)| **c > 1).collect();
    duplicados.sort_by(|a, b| b.1.cmp(a.1));

    let f = duplicados
        .iter()
        .take(200)
        .map(|(clave, c)| {
            let apariciones = **c;
            let valores: Vec<&str> = clave.split('\u{1}').collect();
            Fila {
                valores: valores
                    .iter()
                    .enumerate()
                    .map(|(i, v)| (format!("valor_{}", i + 1), Some(v.to_string())))
                    .collect(),
            }
            .con_repeticiones(apariciones)
        })
        .collect();

    SeccionAnalisis {
        titulo: "Filas duplicadas".into(),
        tipo: TipoAnalisis::Duplicados.etiqueta().to_string(),
        descripcion: format!(
            "{} combinaciones de valores aparecen mas de una vez.",
            duplicados.len()
        ),
        filas: f,
        clave: BTreeMap::new(),
        metricas: vec![Metrica {
            nombre: "Grupos duplicados".into(),
            valor: duplicados.len().to_string(),
            unidad: String::new(),
        }],
    }
}

trait ConRepeticiones {
    fn con_repeticiones(self, n: usize) -> Fila;
}
impl ConRepeticiones for Fila {
    fn con_repeticiones(mut self, n: usize) -> Fila {
        self.valores
            .insert("Apariciones".to_string(), Some(n.to_string()));
        self
    }
}

// ----------------------------------------------------------------------
// Estadistica
// ----------------------------------------------------------------------

/// Indicadores de un conjunto numerico.
pub struct Estadisticas {
    pub min: f64,
    pub max: f64,
    pub media: f64,
    pub mediana: f64,
    pub desviacion: f64,
    pub q1: f64,
    pub q3: f64,
    pub asimetria: f64,
}

/// Calcula los indicadores estadisticos basicos de una serie.
pub fn estadisticas(valores: &[f64]) -> Estadisticas {
    if valores.is_empty() {
        return Estadisticas {
            min: 0.0,
            max: 0.0,
            media: 0.0,
            mediana: 0.0,
            desviacion: 0.0,
            q1: 0.0,
            q3: 0.0,
            asimetria: 0.0,
        };
    }
    let mut ordenados = valores.to_vec();
    ordenados.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = ordenados.len() as f64;
    let media = ordenados.iter().sum::<f64>() / n;
    let varianza = ordenados.iter().map(|v| (v - media).powi(2)).sum::<f64>() / n;
    let desviacion = varianza.sqrt();
    let mediana = percentil(&ordenados, 50.0);
    let asimetria = if desviacion > f64::EPSILON {
        ordenados
            .iter()
            .map(|v| ((v - media) / desviacion).powi(3))
            .sum::<f64>()
            / n
    } else {
        0.0
    };
    Estadisticas {
        min: ordenados[0],
        max: *ordenados.last().unwrap(),
        media,
        mediana,
        desviacion,
        q1: percentil(&ordenados, 25.0),
        q3: percentil(&ordenados, 75.0),
        asimetria,
    }
}

/// Percentil por interpolacion lineal sobre una serie ya ordenada.
pub fn percentil(ordenados: &[f64], p: f64) -> f64 {
    if ordenados.is_empty() {
        return 0.0;
    }
    if ordenados.len() == 1 {
        return ordenados[0];
    }
    let posicion = (p / 100.0) * (ordenados.len() as f64 - 1.0);
    let inferior = posicion.floor() as usize;
    let superior = posicion.ceil() as usize;
    if inferior == superior {
        return ordenados[inferior];
    }
    let peso = posicion - inferior as f64;
    ordenados[inferior] * (1.0 - peso) + ordenados[superior] * peso
}

/// Correlacion de Pearson entre dos series de la misma longitud.
pub fn pearson(a: &[f64], b: &[f64]) -> f64 {
    let n = a.len().min(b.len());
    if n < 2 {
        return 0.0;
    }
    let ma = a[..n].iter().sum::<f64>() / n as f64;
    let mb = b[..n].iter().sum::<f64>() / n as f64;
    let mut num = 0.0;
    let mut da = 0.0;
    let mut db = 0.0;
    for i in 0..n {
        let x = a[i] - ma;
        let y = b[i] - mb;
        num += x * y;
        da += x * x;
        db += y * y;
    }
    let den = (da * db).sqrt();
    if den.abs() < f64::EPSILON {
        0.0
    } else {
        num / den
    }
}

/// Regresion lineal simple sobre el indice de la serie.
pub fn regresion_lineal(valores: &[f64]) -> (f64, f64) {
    let n = valores.len() as f64;
    if n < 2.0 {
        return (valores.first().copied().unwrap_or(0.0), 0.0);
    }
    let media_x = (0..valores.len()).map(|i| i as f64).sum::<f64>() / n;
    let media_y = valores.iter().sum::<f64>() / n;
    let mut num = 0.0;
    let mut den = 0.0;
    for (i, y) in valores.iter().enumerate() {
        let x = i as f64;
        num += (x - media_x) * (y - media_y);
        den += (x - media_x).powi(2);
    }
    if den.abs() < f64::EPSILON {
        (media_y, 0.0)
    } else {
        let b = num / den;
        (media_y - b * media_x, b)
    }
}

fn fuerza_correlacion(r: f64) -> &'static str {
    let a = r.abs();
    if a >= 0.9 {
        "muy fuerte"
    } else if a >= 0.7 {
        "fuerte"
    } else if a >= 0.4 {
        "moderada"
    } else if a >= 0.2 {
        "debil"
    } else {
        "nula"
    }
}

fn construir_resumen(secciones: &[SeccionAnalisis], filas: usize, fuente: &str) -> String {
    let titulos: Vec<&str> = secciones.iter().map(|s| s.titulo.as_str()).collect();
    format!(
        "Analisis completado sobre {filas} filas de '{fuente}'. Secciones generadas: {}.",
        if titulos.is_empty() {
            "ninguna".to_string()
        } else {
            titulos.join(", ")
        }
    )
}

/// Lista de analisis disponibles con su etiqueta, para el desplegable.
pub fn catalogo_analisis() -> Vec<(String, String)> {
    use TipoAnalisis::*;
    [
        Resumen,
        Describir,
        Nulos,
        Distintos,
        Frecuencias,
        Histograma,
        Correlacion,
        Outliers,
        Tendencia,
        Calidad,
        Duplicados,
    ]
    .iter()
    .map(|a| (format!("{a:?}").to_lowercase(), a.etiqueta().to_string()))
    .collect()
}

/// Indica si una columna parece numerica aunque la tipologia no lo detectara.
pub fn forzar_numerica(columnas: &[ColumnaInfo], filas: &[Fila]) -> Vec<ColumnaInfo> {
    columnas
        .iter()
        .map(|c| {
            if c.numerica {
                return c.clone();
            }
            let valores = valores_de(filas, &c.nombre);
            if valores.len() < 3 {
                return c.clone();
            }
            let numericos = valores
                .iter()
                .filter(|v| !v.trim().is_empty() && a_f64(v).is_some())
                .count();
            let mut copia = c.clone();
            if numericos * 10 >= valores.len() * 9 {
                copia.numerica = true;
                copia.tipo = Familia::Numero.etiqueta().to_string();
                copia.es_texto = false;
            }
            copia
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peticion(analyses: &[&str]) -> PeticionAnalisis {
        PeticionAnalisis {
            fuente_id: 1,
            esquema: "ventas".into(),
            columnas: Vec::new(),
            analyses: analyses.iter().map(|s| s.to_string()).collect(),
            es_analisis_confirmado: true,
            filtros: Vec::new(),
            limite: 1000,
            comentario: String::new(),
        }
    }

    fn datos() -> (Vec<Fila>, Vec<ColumnaInfo>) {
        let filas: Vec<Fila> = vec![
            Fila {
                valores: BTreeMap::from([
                    ("importe".to_string(), Some("100".into())),
                    ("unidades".to_string(), Some("2".into())),
                    ("zona".to_string(), Some("Nordeste".into())),
                ]),
            },
            Fila {
                valores: BTreeMap::from([
                    ("importe".to_string(), Some("200".into())),
                    ("unidades".to_string(), Some("4".into())),
                    ("zona".to_string(), Some("Sur".into())),
                ]),
            },
            Fila {
                valores: BTreeMap::from([
                    ("importe".to_string(), Some("300".into())),
                    ("unidades".to_string(), Some("6".into())),
                    ("zona".to_string(), Some("Nordeste".into())),
                ]),
            },
            Fila {
                valores: BTreeMap::from([
                    ("importe".to_string(), Some("4000".into())),
                    ("unidades".to_string(), None),
                    ("zona".to_string(), Some("Sur".into())),
                ]),
            },
        ];
        let columnas = crate::query::describir_columnas(&filas, 10);
        (filas, columnas)
    }

    #[test]
    fn exige_confirmacion_explicita() {
        let (filas, columnas) = datos();
        let mut p = peticion(&["resumen"]);
        p.es_analisis_confirmado = false;
        let r = ejecutar(&p, &filas, &columnas, "ventas");
        assert!(r.is_err());
    }

    #[test]
    fn calcula_descripcion() {
        let (filas, columnas) = datos();
        let r = ejecutar(&peticion(&["describir"]), &filas, &columnas, "ventas").unwrap();
        assert_eq!(r.filas_analizadas, 4);
        let s = &r.secciones[0];
        assert!(s.metricas.iter().any(|m| m.nombre.contains("Media")));
    }

    #[test]
    fn detecta_nulos() {
        let (filas, columnas) = datos();
        let r = ejecutar(&peticion(&["nulos"]), &filas, &columnas, "ventas").unwrap();
        let unidades = r.secciones[0]
            .filas
            .iter()
            .find(|f| f.valores.get("Columna").and_then(|v| v.as_deref()) == Some("unidades"))
            .unwrap();
        assert_eq!(
            unidades.valores.get("Nulos").and_then(|v| v.as_deref()),
            Some("1")
        );
    }

    #[test]
    fn encuentra_atipicos() {
        // Veinte valores iguales y uno muy extremo: la regla de 3 desviaciones
        // tiene que senalar el extremo.
        let mut filas: Vec<Fila> = (0..20)
            .map(|_| Fila {
                valores: BTreeMap::from([("importe".to_string(), Some("100".into()))]),
            })
            .collect();
        filas.push(Fila {
            valores: BTreeMap::from([("importe".to_string(), Some("100000".into()))]),
        });
        let columnas = crate::query::describir_columnas(&filas, 5);
        let r = ejecutar(&peticion(&["outliers"]), &filas, &columnas, "ventas").unwrap();
        assert_eq!(r.secciones[0].filas.len(), 1);
        assert_eq!(
            r.secciones[0].filas[0]
                .valores
                .get("Valor")
                .and_then(|v| v.as_deref()),
            Some("100.000,00")
        );
    }

    #[test]
    fn correlacion_positiva() {
        let (filas, columnas) = datos();
        let r = ejecutar(&peticion(&["correlacion"]), &filas, &columnas, "ventas").unwrap();
        assert!(!r.secciones[0].filas.is_empty());
    }

    #[test]
    fn estadisticas_basicas() {
        let s = estadisticas(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(s.media, 3.0);
        assert_eq!(s.mediana, 3.0);
        assert_eq!(s.min, 1.0);
        assert_eq!(s.max, 5.0);
    }

    #[test]
    fn percentil_interpola() {
        let v = [1.0, 2.0, 3.0, 4.0];
        assert_eq!(percentil(&v, 50.0), 2.5);
        assert_eq!(percentil(&v, 0.0), 1.0);
        assert_eq!(percentil(&v, 100.0), 4.0);
    }

    #[test]
    fn pearson_detecta_relacion() {
        assert!((pearson(&[1.0, 2.0, 3.0], &[2.0, 4.0, 6.0]) - 1.0).abs() < 1e-9);
        assert!((pearson(&[1.0, 2.0, 3.0], &[6.0, 4.0, 2.0]) + 1.0).abs() < 1e-9);
        assert!(pearson(&[1.0, 1.0, 1.0], &[1.0, 2.0, 3.0]).abs() < 1e-9);
    }

    #[test]
    fn regresion_detecta_pendiente() {
        let (_, b) = regresion_lineal(&[1.0, 3.0, 5.0, 7.0]);
        assert!((b - 2.0).abs() < 1e-9);
    }

    #[test]
    fn calidad_puntua_datos_completos() {
        let filas: Vec<Fila> = vec![Fila {
            valores: BTreeMap::from([("a".to_string(), Some("1".into()))]),
        }];
        let columnas = crate::query::describir_columnas(&filas, 5);
        let r = ejecutar(&peticion(&["calidad"]), &filas, &columnas, "t").unwrap();
        let puntuacion = r.secciones[0]
            .metricas
            .iter()
            .find(|m| m.nombre == "Puntuacion de calidad")
            .unwrap();
        assert!(puntuacion.valor.starts_with("100"));
    }
}

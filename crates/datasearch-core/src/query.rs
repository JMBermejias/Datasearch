//! Motor de consultas: construccion de SQL y filtrado en memoria.
//!
//! El usuario describe lo que busca mediante filtros (`Filtro`), texto libre
//! y un modo AND/OR. Este modulo traduce esa descripcion a SQL para las bases
//! de datos relacionales y a predicados en memoria para MongoDB, ficheros y
//! recursos web, de modo que el comportamiento sea identico en todos los casos.
//!
//! Copyright (c) 2026 Jose Manuel Bernabeu Mejias
//! Licencia MIT

use crate::error::{Error, Resultado};
use crate::models::{Conector, Fila, Filtro, Operador, PeticionBusqueda};
use crate::util::{a_f64, familia_de, normalizar, Familia};

/// Reexportado por comodidad: los constructores de SQL lo usan a menudo.
pub use crate::util::escapar_identificador;
use std::collections::{BTreeMap, BTreeSet};

/// Dialecto SQL de la fuente de datos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialecto {
    Sqlite,
    Postgres,
    MySql,
    SqlServer,
}

impl Dialecto {
    /// Cadena de parametros preparada: `?` en SQLite/PostgreSQL, `?` en MySQL
    /// y SQL Server (todos aceptan marcadores posicionales en sus crates).
    pub fn marcador(&self) -> &'static str {
        "?"
    }

    pub fn limitar(&self, sql: &str, limite: u32) -> String {
        format!("{sql} LIMIT {limite}")
    }

    pub fn obtener_limite(&self, sql: &str, limite: u32, desplazamiento: u32) -> String {
        match self {
            Dialecto::SqlServer => {
                // OFFSET/FETCH exige ORDER BY en T-SQL.
                let con_orden = if sql.to_ascii_uppercase().contains("ORDER BY") {
                    sql.to_string()
                } else {
                    format!("{sql} ORDER BY (SELECT NULL)")
                };
                format!("{con_orden} OFFSET {desplazamiento} ROWS FETCH NEXT {limite} ROWS ONLY")
            }
            _ => format!("{sql} LIMIT {limite} OFFSET {desplazamiento}"),
        }
    }

    /// Como se引用 un identificador.
    pub fn identificar(&self, nombre: &str) -> String {
        match self {
            Dialecto::MySql => format!("`{}`", nombre.replace('`', "``")),
            Dialecto::SqlServer => format!("[{}]", nombre.replace(']', "]]")),
            _ => format!("\"{}\"", nombre.replace('"', "\"\"")),
        }
    }
}

/// Predicado ya normalizado, independiente del motor de datos.
#[derive(Debug, Clone)]
pub enum Predicado {
    /// Comparacion sobre una columna concreta.
    Columna {
        columna: String,
        op: Operador,
        valor: String,
        valor2: String,
    },
    /// El valor debe aparecer en alguna columna de la fila.
    TextoLibre(String),
}

/// Consulta preparada para ejecutarse contra cualquier fuente.
#[derive(Debug, Clone, Default)]
pub struct Consulta {
    pub esquema: String,
    pub columnas: Vec<String>,
    pub orden_por: String,
    pub orden_desc: bool,
    pub limite: u32,
    pub desplazamiento: u32,
    /// Solo para el modo relacional.
    pub texto_relacional: String,
    /// Modo AND/OR entre los filtros.
    pub modo: Conector,
    /// Valor de busqueda en texto libre.
    pub texto: String,
}

impl Consulta {
    pub fn desde_peticion(p: &PeticionBusqueda, esquema: &str) -> Self {
        Self {
            esquema: esquema.to_string(),
            columnas: p.campos.clone(),
            orden_por: p.ordenar_por.clone(),
            orden_desc: p.orden_desc,
            limite: p.limite.clamp(1, 100_000),
            desplazamiento: p.desplazamiento,
            texto_relacional: String::new(),
            modo: Conector::desde_txt(&p.modo),
            texto: p.texto.trim().to_string(),
        }
    }
}

/// Convierte los filtros del usuario en predicados, validando cada uno.
///
/// Devuelve un error claro si un filtro esta mal formado, de modo que la
/// interfaz pueda senalar el campo problematico.
pub fn predicados(filtros: &[Filtro]) -> Resultado<Vec<(Predicado, Conector)>> {
    let mut salida = Vec::new();
    for f in filtros.iter().filter(|f| f.activo) {
        let op = f.op()?;
        if f.campo.trim().is_empty() {
            return Err(Error::FiltroInvalido(
                "indique la columna sobre la que aplicar el filtro".into(),
            ));
        }
        if !op.es_unario() {
            if f.valor.is_empty() {
                return Err(Error::FiltroInvalido(format!(
                    "el filtro '{}' necesita un valor",
                    op.etiqueta()
                )));
            }
            if op.necesita_segundo_valor() && f.valor2.is_empty() {
                return Err(Error::FiltroInvalido(
                    "el filtro 'esta entre' necesita los dos limites".into(),
                ));
            }
        }
        let conector = f.con();
        salida.push((
            Predicado::Columna {
                columna: f.campo.trim().to_string(),
                op,
                valor: f.valor.clone(),
                valor2: f.valor2.clone(),
            },
            conector,
        ));
    }
    Ok(salida)
}

/// Construye el fragmento `WHERE` de una consulta SQL relacional.
///
/// Devuelve el SQL y los parametros, en el orden en que aparecen, para que
/// se puedan enlazar sin inyeccion.
pub fn construir_where(
    filtros: &[Filtro],
    texto: &str,
    modo: Conector,
    dialecto: Dialecto,
    columnas_textuales: &[String],
) -> Resultado<(String, Vec<String>)> {
    let mut params: Vec<String> = Vec::new();
    let mut partes: Vec<String> = Vec::new();

    for (pred, conector) in predicados(filtros)? {
        let gl = if modo == Conector::O { "OR" } else { "AND" };
        let (sql, mut p) = predicado_a_sql(&pred, dialecto)?;
        params.append(&mut p);
        if partes.is_empty() {
            partes.push(sql);
        } else if conector == Conector::Y {
            // En modo AND cada condicion se envuelve para respetar la precedencia.
            partes.push(format!("({gl} {sql})"));
        } else {
            partes.push(format!("OR {sql}"));
        }
    }

    let t = texto.trim();
    if !t.is_empty() && !columnas_textuales.is_empty() {
        let patron = format!("%{}%", t);
        let campos = columnas_textuales
            .iter()
            .map(|c| format!("CAST({} AS TEXT) LIKE ?", dialecto.identificar(c)))
            .collect::<Vec<_>>()
            .join(" OR ");
        if partes.is_empty() {
            partes.push(format!("({campos})"));
        } else if modo == Conector::O {
            partes.push(format!("OR ({campos})"));
        } else {
            partes.push(format!("AND ({campos})"));
        }
        params.push(patron);
    }

    if partes.is_empty() {
        return Ok((String::new(), params));
    }
    Ok((format!("WHERE {}", partes.join(" ")), params))
}

/// Traduce un predicado a SQL con sus parametros.
fn predicado_a_sql(pred: &Predicado, d: Dialecto) -> Resultado<(String, Vec<String>)> {
    use Operador::*;
    let Predicado::Columna {
        columna,
        op,
        valor,
        valor2,
    } = pred
    else {
        return Err(Error::FiltroInvalido(
            "el texto libre no puede convertirse en SQL".into(),
        ));
    };
    let col = d.identificar(columna);
    let v = valor.as_str();

    Ok(match op {
        Contiene => (
            format!("CAST({col} AS TEXT) LIKE ?"),
            vec![format!("%{v}%")],
        ),
        NoContiene => (
            format!("CAST({col} AS TEXT) NOT LIKE ?"),
            vec![format!("%{v}%")],
        ),
        EmpiezaCon => (format!("CAST({col} AS TEXT) LIKE ?"), vec![format!("{v}%")]),
        TerminaCon => (format!("CAST({col} AS TEXT) LIKE ?"), vec![format!("%{v}")]),
        Igual => {
            if let Some(n) = a_f64(v) {
                (format!("{col} = ?"), vec![n.to_string()])
            } else {
                (format!("{col} = ?"), vec![v.to_string()])
            }
        }
        Distinto => {
            if let Some(n) = a_f64(v) {
                (format!("{col} <> ?"), vec![n.to_string()])
            } else {
                (format!("{col} <> ?"), vec![v.to_string()])
            }
        }
        Mayor => (format!("{col} > ?"), vec![v.to_string()]),
        MayorIgual => (format!("{col} >= ?"), vec![v.to_string()]),
        Menor => (format!("{col} < ?"), vec![v.to_string()]),
        MenorIgual => (format!("{col} <= ?"), vec![v.to_string()]),
        Entre => (
            format!("{col} BETWEEN ? AND ?"),
            vec![valor.clone(), valor2.clone()],
        ),
        EnLista => {
            let lista: Vec<String> = valor
                .split(&['\u{1F}', ','][..])
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if lista.is_empty() {
                return Err(Error::FiltroInvalido(
                    "la lista de valores esta vacia".into(),
                ));
            }
            let huecos = vec!["?"; lista.len()].join(", ");
            (format!("{col} IN ({huecos})"), lista)
        }
        NoEnLista => {
            let lista: Vec<String> = valor
                .split(&['\u{1F}', ','][..])
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if lista.is_empty() {
                return Err(Error::FiltroInvalido(
                    "la lista de valores esta vacia".into(),
                ));
            }
            let huecos = vec!["?"; lista.len()].join(", ");
            (format!("{col} NOT IN ({huecos})"), lista)
        }
        EsNulo => (format!("{col} IS NULL"), vec![]),
        NoEsNulo => (format!("{col} IS NOT NULL"), vec![]),
        Regex => {
            // Se traduce a LIKE cuando es posible; si no, se avisa al usuario.
            if let Some(re) = crate::util::compilar_regex(v) {
                let texto = re.as_str();
                let like = texto.replace("(.*)", "%").replace("(.*?)", "%");
                (format!("CAST({col} AS TEXT) LIKE ?"), vec![like])
            } else {
                return Err(Error::FiltroInvalido(
                    "la expresion regular no es valida".into(),
                ));
            }
        }
        Vacio => (format!("TRIM(CAST({col} AS TEXT)) = ''"), vec![]),
        NoVacio => (format!("TRIM(CAST({col} AS TEXT)) <> ''"), vec![]),
        EsVerdadero => (
            format!("CAST({col} AS TEXT) IN ('1','true','True','T')"),
            vec![],
        ),
        EsFalso => (
            format!("CAST({col} AS TEXT) IN ('0','false','False','F')"),
            vec![],
        ),
    })
}

/// Evalua un predicado sobre una fila en memoria.
///
/// Se usa para MongoDB, ficheros CSV/JSON/XML/Excel y recursos web, de modo
/// que el filtrado sea equivalente al de las bases relacionales.
pub fn fila_cumple(
    fila: &Fila,
    preds: &[(Predicado, Conector)],
    modo: Conector,
    texto: &str,
) -> bool {
    if preds.is_empty() && texto.trim().is_empty() {
        return true;
    }
    if !texto.trim().is_empty() && !coincide_texto(fila, texto) {
        return false;
    }
    if preds.is_empty() {
        return true;
    }

    if modo == Conector::O {
        // Basta con que se cumpla uno.
        preds.iter().any(|(p, _)| predicado_cumple(fila, p))
    } else {
        // En modo AND, un filtro marcado como "o" libera al grupo.
        let hay_alguno_or = preds.iter().any(|(_, c)| *c == Conector::O);
        if !hay_alguno_or {
            return preds.iter().all(|(p, _)| predicado_cumple(fila, p));
        }
        // Se evalua como: (A AND B) OR C, es decir, se corta en el primer "o".
        let mut acumulado = true;
        for (p, c) in preds {
            match c {
                Conector::Y => {
                    if !predicado_cumple(fila, p) {
                        return false;
                    }
                }
                Conector::O => {
                    acumulado = acumulado || predicado_cumple(fila, p);
                }
            }
        }
        acumulado
    }
}

fn predicado_cumple(fila: &Fila, pred: &Predicado) -> bool {
    match pred {
        Predicado::TextoLibre(t) => coincide_texto(fila, t),
        Predicado::Columna {
            columna,
            op,
            valor,
            valor2,
        } => {
            let actual = fila
                .valores
                .get(columna)
                .cloned()
                .flatten()
                .unwrap_or_default();
            let objetivo = valor.as_str();
            let num_actual = a_f64(&actual);
            let num_valor = a_f64(objetivo);

            match op {
                Operador::Contiene => normalizar(&actual).contains(&normalizar(objetivo)),
                Operador::NoContiene => !normalizar(&actual).contains(&normalizar(objetivo)),
                Operador::EmpiezaCon => normalizar(&actual).starts_with(&normalizar(objetivo)),
                Operador::TerminaCon => normalizar(&actual).ends_with(&normalizar(objetivo)),
                Operador::Igual => {
                    comparar_texto(&actual, objetivo, num_actual, num_valor, |a, b| a == b)
                }
                Operador::Distinto => {
                    !comparar_texto(&actual, objetivo, num_actual, num_valor, |a, b| a == b)
                }
                Operador::Mayor => {
                    comparar_orden(&actual, objetivo, num_actual, num_valor, |o| o.is_gt())
                }
                Operador::MayorIgual => {
                    comparar_orden(&actual, objetivo, num_actual, num_valor, |o| o.is_ge())
                }
                Operador::Menor => {
                    comparar_orden(&actual, objetivo, num_actual, num_valor, |o| o.is_lt())
                }
                Operador::MenorIgual => {
                    comparar_orden(&actual, objetivo, num_actual, num_valor, |o| o.is_le())
                }
                Operador::Entre => {
                    let n2 = a_f64(valor2);
                    comparar_orden(&actual, objetivo, num_actual, num_valor, |o| o.is_ge())
                        && comparar_orden(&actual, valor2, num_actual, n2, |o| o.is_le())
                }
                Operador::EnLista => lista_filtro(valor)
                    .iter()
                    .any(|v| normalizar(&actual).contains(&normalizar(v))),
                Operador::NoEnLista => !lista_filtro(valor)
                    .iter()
                    .any(|v| normalizar(&actual).contains(&normalizar(v))),
                Operador::EsNulo => actual.trim().is_empty(),
                Operador::NoEsNulo => !actual.trim().is_empty(),
                Operador::Regex => match crate::util::compilar_regex(objetivo) {
                    Some(re) => re.is_match(&actual),
                    None => false,
                },
                Operador::Vacio => actual.trim().is_empty(),
                Operador::NoVacio => !actual.trim().is_empty(),
                Operador::EsVerdadero => es_booleano(&actual, true),
                Operador::EsFalso => es_booleano(&actual, false),
            }
        }
    }
}

fn lista_filtro(valor: &str) -> Vec<String> {
    valor
        .split(&['\u{1F}', ',', '\n'][..])
        .map(|s| s.trim().trim_matches(['\'', '"']).to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn coincide_texto(fila: &Fila, texto: &str) -> bool {
    let busqueda = normalizar(texto);
    if busqueda.is_empty() {
        return true;
    }
    fila.valores.values().any(|v| {
        v.as_deref()
            .map(|s| normalizar(s).contains(&busqueda))
            .unwrap_or(false)
    })
}

/// Compara como texto o como numero, segun lo que resulte natural.
fn comparar_texto(
    actual: &str,
    objetivo: &str,
    num_actual: Option<f64>,
    num_objetivo: Option<f64>,
    cmp: impl Fn(&str, &str) -> bool,
) -> bool {
    match (num_actual, num_objetivo) {
        (Some(a), Some(b)) => (a - b).abs() < f64::EPSILON * a.abs().max(1.0),
        _ => cmp(&normalizar(actual), &normalizar(objetivo)),
    }
}

fn comparar_orden(
    actual: &str,
    objetivo: &str,
    num_actual: Option<f64>,
    num_objetivo: Option<f64>,
    cmp: impl Fn(std::cmp::Ordering) -> bool,
) -> bool {
    match (num_actual, num_objetivo) {
        (Some(a), Some(b)) => cmp(a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal)),
        _ => {
            if actual.trim().is_empty() {
                return false;
            }
            cmp(normalizar(actual).cmp(&normalizar(objetivo)))
        }
    }
}

fn es_booleano(valor: &str, esperado: bool) -> bool {
    let v = valor.trim().to_ascii_lowercase();
    if esperado {
        matches!(
            v.as_str(),
            "1" | "true" | "t" | "si" | "sí" | "yes" | "verdadero"
        )
    } else {
        matches!(v.as_str(), "0" | "false" | "f" | "no" | "falso")
    }
}

/// Filtra, ordena y pagina un conjunto de filas en memoria.
/// Opciones de presentacion al filtrar en memoria.
#[derive(Debug, Clone, Default)]
pub struct Vista {
    pub orden_por: String,
    pub orden_desc: bool,
    pub desplazamiento: u32,
    pub limite: u32,
}

pub fn aplicar_en_memoria(
    filas: Vec<Fila>,
    preds: &[(Predicado, Conector)],
    modo: Conector,
    texto: &str,
    vista: &Vista,
) -> Vec<Fila> {
    let (orden_por, orden_desc, desplazamiento, limite) = (
        vista.orden_por.as_str(),
        vista.orden_desc,
        vista.desplazamiento,
        vista.limite,
    );
    let mut filtradas: Vec<Fila> = filas
        .into_iter()
        .filter(|f| fila_cumple(f, preds, modo, texto))
        .collect();

    if !orden_por.trim().is_empty() {
        let clave = orden_por.to_string();
        filtradas.sort_by(|a, b| {
            let va = a.valores.get(&clave).cloned().flatten().unwrap_or_default();
            let vb = b.valores.get(&clave).cloned().flatten().unwrap_or_default();
            // Los numeros se ordenan numericamente, el resto como texto natural.
            match (a_f64(&va), a_f64(&vb)) {
                (Some(x), Some(y)) => x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal),
                _ => normalizar(&va).cmp(&normalizar(&vb)),
            }
        });
        if orden_desc {
            filtradas.reverse();
        }
    }

    filtradas
        .into_iter()
        .skip(desplazamiento as usize)
        .take(limite as usize)
        .collect()
}

/// Calcula la tipologia de cada columna a partir de las filas de muestra.
pub fn describir_columnas(
    filas: &[Fila],
    maximo_valores: usize,
) -> Vec<crate::models::ColumnaInfo> {
    let mut cuentas: BTreeMap<
        String,
        (i64, i64, BTreeSet<String>, Vec<crate::models::Frecuencia>),
    > = BTreeMap::new();
    let total = filas.len() as i64;

    for fila in filas {
        for (nombre, valor) in &fila.valores {
            let entrada = cuentas
                .entry(nombre.clone())
                .or_insert_with(|| (0, 0, BTreeSet::new(), Vec::new()));
            entrada.0 += 1;
            match valor {
                Some(v) if !v.trim().is_empty() => {
                    entrada.1 += 1;
                    entrada.2.insert(v.clone());
                    let limite_alcanzado = entrada.3.len() >= maximo_valores;
                    let ya_contado = match entrada.3.iter_mut().find(|f| f.valor == *v) {
                        Some(f) => {
                            f.cuenta += 1;
                            true
                        }
                        None => false,
                    };
                    if !ya_contado && !limite_alcanzado {
                        entrada.3.push(crate::models::Frecuencia {
                            valor: v.clone(),
                            cuenta: 1,
                            porcentaje: 0.0,
                        });
                    }
                }
                _ => {}
            }
        }
    }

    cuentas
        .into_iter()
        .map(|(nombre, (_vistos, no_nulos, distintos, mut frecuentes))| {
            let nulos = total - no_nulos;
            for f in frecuentes.iter_mut() {
                f.porcentaje = if no_nulos > 0 {
                    (f.cuenta as f64 / no_nulos as f64) * 100.0
                } else {
                    0.0
                };
            }
            frecuentes.sort_by_key(|f| std::cmp::Reverse(f.cuenta));

            // Tipo mayoritario entre los valores presentes.
            let muestras: Vec<String> = filas
                .iter()
                .filter_map(|f| f.valores.get(&nombre).cloned().flatten())
                .filter(|v| !v.trim().is_empty())
                .take(500)
                .collect();
            let tipo = if muestras.is_empty() {
                Familia::Vacio
            } else {
                let cuenta = |f: Familia| muestras.iter().filter(|v| familia_de(v) == f).count();
                let candidatos = [
                    (Familia::Numero, cuenta(Familia::Numero)),
                    (Familia::Fecha, cuenta(Familia::Fecha)),
                    (Familia::Booleano, cuenta(Familia::Booleano)),
                    (Familia::Texto, cuenta(Familia::Texto)),
                ];
                let mejor = candidatos.iter().max_by_key(|(_, c)| *c).copied();
                match mejor {
                    Some((f, c)) if c * 2 > muestras.len() => f,
                    _ => Familia::Texto,
                }
            };

            crate::models::ColumnaInfo {
                etiqueta: nombre.clone(),
                nombre,
                tipo: tipo.etiqueta().to_string(),
                numerica: tipo == Familia::Numero,
                es_texto: tipo == Familia::Texto,
                es_fecha: tipo == Familia::Fecha,
                es_booleano: tipo == Familia::Booleano,
                nulos,
                distintos: distintos.len() as i64,
                valores_frecuentes: frecuentes,
            }
        })
        .collect()
}

/// Nombres de todas las columnas presentes en un conjunto de filas.
pub fn columnas_de(filas: &[Fila]) -> Vec<String> {
    let mut set = BTreeSet::new();
    for f in filas {
        for k in f.valores.keys() {
            set.insert(k.clone());
        }
    }
    set.into_iter().collect()
}

/// Convierte un mapa de valores en una fila. Los valores vacios se guardan
/// como `None` para distinguirlos de un dato real.
pub fn fila_de(mapa: BTreeMap<String, String>) -> Fila {
    Fila {
        valores: mapa
            .into_iter()
            .map(|(k, v)| {
                if v.is_empty() {
                    (k, None)
                } else {
                    (k, Some(v))
                }
            })
            .collect(),
    }
}

/// Normaliza el texto que el usuario escribe en el buscador.
///
/// Admite comillas para frases exactas y el prefijo `campo:` para acotar la
/// busqueda a una columna concreta. Ejemplo: `pais:ES "San Javier"`.
pub fn analizar_texto_busqueda(texto: &str) -> Vec<(Option<String>, String)> {
    let mut partes = Vec::new();
    let mut buffer = String::new();
    let mut en_comillas = false;
    let mut actual: Vec<char> = Vec::new();

    let cerrar = |buffer: &mut String,
                  actual: &mut Vec<char>,
                  en_comillas: &mut bool,
                  partes: &mut Vec<(Option<String>, String)>| {
        if !actual.is_empty() {
            buffer.push_str(&actual.iter().collect::<String>());
            actual.clear();
        }
        let bruto = buffer.trim().to_string();
        *en_comillas = false;
        buffer.clear();
        if bruto.is_empty() {
            return;
        }
        match bruto.split_once(':') {
            Some((campo, resto))
                if !campo.is_empty()
                    && !campo.contains(' ')
                    && campo.len() <= 64
                    && !resto.trim().is_empty() =>
            {
                partes.push((Some(campo.trim().to_string()), resto.trim().to_string()));
            }
            _ => partes.push((None, bruto)),
        }
    };

    for c in texto.chars() {
        match c {
            '"' => {
                if en_comillas {
                    cerrar(&mut buffer, &mut actual, &mut en_comillas, &mut partes);
                } else {
                    if !actual.is_empty() {
                        buffer.push_str(&actual.iter().collect::<String>());
                        actual.clear();
                    }
                    en_comillas = true;
                }
            }
            c if c.is_whitespace() && !en_comillas => {
                cerrar(&mut buffer, &mut actual, &mut en_comillas, &mut partes);
            }
            c => {
                if en_comillas {
                    buffer.push(c);
                } else {
                    actual.push(c);
                }
            }
        }
    }
    cerrar(&mut buffer, &mut actual, &mut en_comillas, &mut partes);
    partes
}

/// Verifica que los filtros del usuario son coherentes con las columnas
/// disponibles. Devuelve un aviso legible si algo no cuadra.
pub fn validar_contra_columnas(filtros: &[Filtro], columnas: &[String]) -> Vec<String> {
    let mut avisos = Vec::new();
    for f in filtros.iter().filter(|f| f.activo) {
        if !columnas.iter().any(|c| c == &f.campo) {
            avisos.push(format!(
                "La columna '{}' no existe en los datos de la fuente '{}'",
                f.campo, "(seleccionada)"
            ));
        }
    }
    avisos
}

/// Camino comodo: filtra filas de un origen documental o plano.
pub fn filtrar_filas(
    filas: Vec<Fila>,
    peticion: &PeticionBusqueda,
    _esquema: &str,
) -> Resultado<Vec<Fila>> {
    let preds = predicados(&peticion.filtros)?;
    let modo = Conector::desde_txt(&peticion.modo);
    let texto = peticion.texto.trim().to_string();
    // En modo documental cada termino se comprueba por separado.
    let terminos = if texto.is_empty() {
        Vec::new()
    } else {
        analizar_texto_busqueda(&texto)
            .into_iter()
            .map(|(_, t)| t)
            .collect::<Vec<String>>()
    };

    let filtradas: Vec<Fila> = filas
        .into_iter()
        .filter(|f| {
            let coincide_filtros = fila_cumple(f, &preds, modo, "");
            let coincide_texto = terminos.is_empty()
                || terminos.iter().any(|t| {
                    let textos = analizar_texto_busqueda(t);
                    textos.iter().all(|(campo, valor)| match campo {
                        Some(c) => f
                            .valores
                            .get(c.as_str())
                            .cloned()
                            .flatten()
                            .map(|v| normalizar(&v).contains(&normalizar(valor)))
                            .unwrap_or(false),
                        None => coincide_texto(f, valor),
                    })
                });
            coincide_filtros && coincide_texto
        })
        .collect();

    Ok(aplicar_en_memoria(
        filtradas,
        &preds,
        modo,
        "",
        &Vista {
            orden_por: peticion.ordenar_por.clone(),
            orden_desc: peticion.orden_desc,
            desplazamiento: peticion.desplazamiento,
            limite: peticion.limite.clamp(1, 100_000),
        },
    ))
}

/// Ordena los puntos de un grafico por valor descendente.
pub fn ordenar_puntos<T, F>(puntos: &mut [T], valor: F)
where
    F: Fn(&T) -> f64,
{
    puntos.sort_by(|a, b| {
        valor(b)
            .partial_cmp(&valor(a))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Filtro;

    fn fila() -> Fila {
        let mut v = BTreeMap::new();
        v.insert("nombre".to_string(), Some("José Cañizares".to_string()));
        v.insert("pais".to_string(), Some("ES".to_string()));
        v.insert("importe".to_string(), Some("1.500,50".to_string()));
        v.insert("notas".to_string(), Some("9".to_string()));
        Fila { valores: v }
    }

    fn fl(campo: &str, op: &str, valor: &str) -> Filtro {
        Filtro {
            campo: campo.into(),
            operador: op.into(),
            valor: valor.into(),
            valor2: String::new(),
            conector: "y".into(),
            activo: true,
        }
    }

    #[test]
    fn filtra_por_texto_sin_acentos() {
        let f = fila();
        let preds = [(
            Predicado::Columna {
                columna: "nombre".into(),
                op: Operador::Contiene,
                valor: "jose".into(),
                valor2: String::new(),
            },
            Conector::Y,
        )];
        assert!(predicado_cumple(&f, &preds[0].0));
    }

    #[test]
    fn filtra_numeros_en_formato_espanol() {
        let f = fila();
        let predicado = Predicado::Columna {
            columna: "importe".into(),
            op: Operador::Mayor,
            valor: "1000".into(),
            valor2: String::new(),
        };
        assert!(predicado_cumple(&f, &predicado));
    }

    #[test]
    fn filtra_entre() {
        let f = fila();
        let preds = predicados(&[Filtro {
            valor2: "10".into(),
            ..fl("notas", "entre", "5")
        }])
        .unwrap();
        assert!(fila_cumple(&f, &preds, Conector::Y, ""));
    }

    #[test]
    fn modo_o_devuelve_coincidencia() {
        let f = fila();
        let preds = predicados(&[
            Filtro {
                conector: "o".into(),
                ..fl("pais", "igual", "FR")
            },
            Filtro {
                conector: "y".into(),
                ..fl("notas", "igual", "9")
            },
        ])
        .unwrap();
        assert!(fila_cumple(&f, &preds, Conector::Y, ""));
    }

    #[test]
    fn modo_y_exige_todas() {
        let f = fila();
        let preds = predicados(&[fl("pais", "igual", "FR"), fl("notas", "igual", "9")]).unwrap();
        assert!(!fila_cumple(&f, &preds, Conector::Y, ""));
    }

    #[test]
    fn sql_para_postgres() {
        let (where_, params) = construir_where(
            &[fl("pais", "igual", "ES")],
            "jose",
            Conector::Y,
            Dialecto::Postgres,
            &["nombre".to_string(), "pais".to_string()],
        )
        .unwrap();
        assert!(where_.contains("WHERE"));
        assert!(where_.contains("LIKE"));
        assert_eq!(params, vec!["ES".to_string(), "%jose%".to_string()]);
    }

    #[test]
    fn sql_server_usa_offset_fetch() {
        let sql = Dialecto::SqlServer.obtener_limite("SELECT * FROM t", 10, 20);
        assert!(sql.contains("OFFSET 20 ROWS FETCH NEXT 10 ROWS ONLY"));
    }

    #[test]
    fn analisis_de_texto_con_comillas() {
        let partes = analizar_texto_busqueda("pais:ES \"San Javier\" datos");
        assert_eq!(partes.len(), 3);
        assert_eq!(partes[0].0.as_deref(), Some("pais"));
        assert_eq!(partes[1], (None, "San Javier".to_string()));
    }

    #[test]
    fn filtro_invalido_avisa() {
        let r = predicados(&[fl("", "igual", "x")]);
        assert!(r.is_err());
    }

    #[test]
    fn describe_columnas() {
        let filas = vec![
            fila(),
            Fila {
                valores: BTreeMap::from([
                    ("nombre".to_string(), Some("Ana".to_string())),
                    ("pais".to_string(), Some("FR".to_string())),
                    ("importe".to_string(), Some("200".to_string())),
                    ("notas".to_string(), None),
                ]),
            },
        ];
        let cols = describir_columnas(&filas, 5);
        let pais = cols.iter().find(|c| c.nombre == "pais").unwrap();
        assert_eq!(pais.distintos, 2);
        let notas = cols.iter().find(|c| c.nombre == "notas").unwrap();
        assert_eq!(notas.nulos, 1);
    }

    #[test]
    fn paginacion_en_memoria() {
        let filas: Vec<Fila> = (0..50)
            .map(|i| Fila {
                valores: BTreeMap::from([("n".to_string(), Some(i.to_string()))]),
            })
            .collect();
        let salida = aplicar_en_memoria(
            filas,
            &[],
            Conector::Y,
            "",
            &Vista {
                desplazamiento: 10,
                limite: 5,
                ..Vista::default()
            },
        );
        assert_eq!(salida.len(), 5);
        assert_eq!(salida[0].valores.get("n").unwrap().as_deref(), Some("10"));
    }
}

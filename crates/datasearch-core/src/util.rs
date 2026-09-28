//! Utilidades comunes: inferencia de tipos, formato de valores y helpers SQL.
//!
//! Copyright (c) 2026 Jose Manuel Bernabeu Mejias
//! Licencia MIT

use regex::Regex;
use std::collections::BTreeMap;
use std::sync::OnceLock;

/// Familia de dato deducida de un valor textual.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Familia {
    Vacio,
    Numero,
    Fecha,
    Booleano,
    Texto,
    Json,
}

impl Familia {
    pub fn etiqueta(&self) -> &'static str {
        match self {
            Familia::Vacio => "vacio",
            Familia::Numero => "numero",
            Familia::Fecha => "fecha",
            Familia::Booleano => "booleano",
            Familia::Texto => "texto",
            Familia::Json => "json",
        }
    }
}

/// Deduce la familia de dato de un valor.
pub fn familia_de(valor: &str) -> Familia {
    let v = valor.trim();
    if v.is_empty() || v == "NULL" || v.eq_ignore_ascii_case("null") {
        return Familia::Vacio;
    }
    if v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("false") || v == "0" || v == "1" {
        // 0 y 1 pueden ser numeros; se decide mas abajo si son coerentes.
        if v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("false") {
            return Familia::Booleano;
        }
    }
    // Las fechas se comprueban antes que los numeros: si no, "2026-09-28" se
    // detectaria como el entero 20260928 al quitar los separadores.
    if es_fecha(v) {
        return Familia::Fecha;
    }
    if a_f64(v).is_some() {
        return Familia::Numero;
    }
    if v.starts_with('{') || v.starts_with('[') {
        return Familia::Json;
    }
    Familia::Texto
}

/// Convierte texto a numero. Acepta el formato español (`1.234,56`) y el
/// anglosajon (`1,234.56`), porcentajes (`25%`) y signos.
pub fn a_f64(valor: &str) -> Option<f64> {
    let v = valor.trim();
    if v.is_empty() {
        return None;
    }
    if let Ok(n) = v.parse::<f64>() {
        return if n.is_finite() { Some(n) } else { None };
    }

    let negativo = v.starts_with('-');
    let limpio: String = v
        .trim_start_matches(['-', '+', '$', '€', '£'])
        .trim_end_matches(['%', '€', '$', '£'])
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.' || *c == ',' || *c == '_' || *c == ' ')
        .collect();
    if limpio.is_empty() {
        return None;
    }

    let ultima_coma = limpio.rfind(',');
    let ultimo_punto = limpio.rfind('.');
    let normalizado = match (ultima_coma, ultimo_punto) {
        // Hay separador de miles: el ultimo es decimal.
        (Some(c), Some(p)) => {
            if c > p {
                limpio.replace('.', "").replace(',', ".")
            } else {
                limpio.replace(',', "")
            }
        }
        (Some(_), None) => limpio.replace(',', "."),
        (None, Some(_)) => limpio.replace(',', ""),
        (None, None) => limpio.replace(['_', ' '], ""),
    };

    let n: f64 = normalizado.parse().ok()?;
    if !n.is_finite() {
        return None;
    }
    Some(if negativo { -n } else { n })
}

/// Formatea un numero con separadores de miles. `decimales` fija el precision.
pub fn formatear_numero(n: f64, decimales: usize) -> String {
    let negativo = n < 0.0;
    let abs = n.abs();
    let texto = format!("{abs:.decimales$}", decimales = decimales);
    let mut partes = texto.splitn(2, '.');
    let entero = partes.next().unwrap_or("0").to_string();
    let decimal = partes.next().map(|d| format!(",{d}")).unwrap_or_default();

    let mut con_separadores = String::new();
    for (i, c) in entero.chars().enumerate() {
        if i > 0 && (entero.len() - i) % 3 == 0 {
            con_separadores.push('.');
        }
        con_separadores.push(c);
    }
    if negativo {
        format!("-{con_separadores}{decimal}")
    } else {
        format!("{con_separadores}{decimal}")
    }
}

/// Formatea un valor con la familia deducida, listo para la interfaz.
pub fn formatear_valor(valor: &str) -> String {
    if valor.is_empty() {
        return String::new();
    }
    match familia_de(valor) {
        Familia::Numero => {
            if valor.contains(',') || valor.contains('%') {
                valor.to_string()
            } else {
                match a_f64(valor) {
                    Some(n) if n.fract() == 0.0 && n.abs() < 1e15 => formatear_numero(n, 0),
                    Some(n) => formatear_numero(n, 2),
                    None => valor.to_string(),
                }
            }
        }
        _ => valor.to_string(),
    }
}

fn regex_fecha() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"^(\d{4})-(\d{2})-(\d{2})([T ]\d{2}:\d{2}(:\d{2})?)?|\d{2}/\d{2}/\d{4}([ T]\d{2}:\d{2}(:\d{2})?)?$",
        )
        .expect("regex de fecha valido")
    })
}

/// Indica si el texto parece una fecha.
pub fn es_fecha(valor: &str) -> bool {
    regex_fecha().is_match(valor.trim())
}

/// Expresion regular reutilizable (cacheada) para el operador `Regex`.
pub fn compilar_regex(patron: &str) -> Option<Regex> {
    Regex::new(patron).ok()
}

/// Escapa una cadena para insertarla como literal SQL entre comillas simples.
pub fn escapar_sql(texto: &str) -> String {
    texto.replace('\'', "''")
}

/// Escapa un identificador (columna, tabla) entre comillas dobles.
pub fn escapar_identificador(texto: &str) -> String {
    format!("\"{}\"", texto.replace('"', "\"\""))
}

/// Convierte una clave de objeto JSON en un nombre de columna utilizable.
pub fn limpiar_nombre_columna(nombre: &str) -> String {
    let s = nombre.trim();
    if s.is_empty() {
        return "columna".to_string();
    }
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '.' || c == '-' || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Normaliza un texto para busquedas: minusculas y sin acentos.
pub fn normalizar(texto: &str) -> String {
    let minusculas = texto.to_lowercase();
    let mut salida = String::with_capacity(minusculas.len());
    for c in minusculas.chars() {
        match c {
            'á' | 'à' | 'ä' | 'â' => salida.push('a'),
            'é' | 'è' | 'ë' | 'ê' => salida.push('e'),
            'í' | 'ì' | 'ï' | 'î' => salida.push('i'),
            'ó' | 'ò' | 'ö' | 'ô' => salida.push('o'),
            'ú' | 'ù' | 'ü' | 'û' => salida.push('u'),
            'ñ' => salida.push('n'),
            'ç' => salida.push('c'),
            other => salida.push(other),
        }
    }
    salida
}

/// Une las partes de una ruta JSON (`data.items`).
pub fn partir_ruta_json(ruta: &str) -> Vec<String> {
    ruta.split(['.', '/'])
        .map(|s| s.trim().trim_matches(['[', ']', '"', '\'']))
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

/// Trunca un texto para mostrarlo en tablas, marcando el corte.
pub fn truncar(texto: &str, maximo: usize) -> String {
    if texto.chars().count() <= maximo {
        return texto.to_string();
    }
    let corte: String = texto.chars().take(maximo.saturating_sub(1)).collect();
    format!("{corte}\u{2026}")
}

/// Aplana un `serde_json::Value` en un mapa `columna -> valor legible`.
///
/// Los objetos anidados se convierten en columnas `padre.hijo` y los arrays
/// en indices `padre[0]`, `padre[1]`, ...
pub fn aplanar_json(valor: &serde_json::Value, prefijo: &str) -> BTreeMap<String, String> {
    let mut salida = BTreeMap::new();
    aplanar_json_recursivo(valor, prefijo, &mut salida, 0);
    salida
}

fn aplanar_json_recursivo(
    valor: &serde_json::Value,
    prefijo: &str,
    salida: &mut BTreeMap<String, String>,
    profundidad: usize,
) {
    const MAX_PROFUNDIDAD: usize = 6;
    match valor {
        serde_json::Value::Object(mapa) => {
            if mapa.is_empty() {
                insertar(salida, prefijo, "{}");
                return;
            }
            if profundidad >= MAX_PROFUNDIDAD {
                insertar(salida, prefijo, &truncar(&valor.to_string(), 2000));
                return;
            }
            for (clave, hijo) in mapa {
                let nombre = if prefijo.is_empty() {
                    limpiar_nombre_columna(clave)
                } else {
                    format!("{prefijo}.{}", limpiar_nombre_columna(clave))
                };
                aplanar_json_recursivo(hijo, &nombre, salida, profundidad + 1);
            }
        }
        serde_json::Value::Array(lista) => {
            if lista.is_empty() {
                insertar(salida, prefijo, "[]");
                return;
            }
            if profundidad >= MAX_PROFUNDIDAD {
                insertar(salida, prefijo, &truncar(&valor.to_string(), 2000));
                return;
            }
            for (i, hijo) in lista.iter().enumerate().take(200) {
                let nombre = format!("{prefijo}[{i}]");
                aplanar_json_recursivo(hijo, &nombre, salida, profundidad + 1);
            }
            if lista.len() > 200 {
                insertar(salida, &format!("{prefijo}[..]"), "resto de elementos");
            }
        }
        serde_json::Value::Null => {
            insertar(salida, prefijo, "");
        }
        serde_json::Value::String(s) => {
            insertar(salida, prefijo, s);
        }
        otro => {
            insertar(salida, prefijo, &otro.to_string());
        }
    }
}

fn insertar(salida: &mut BTreeMap<String, String>, clave: &str, valor: &str) {
    let k = if clave.is_empty() {
        "valor".to_string()
    } else {
        clave.to_string()
    };
    salida.insert(k, valor.to_string());
}

/// Convierte un `serde_json::Value` a texto plano (para XML o CSV).
pub fn json_a_texto(valor: &serde_json::Value) -> String {
    match valor {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => String::new(),
        otro => otro.to_string(),
    }
}

/// Construye un nombre de fichero seguro a partir de un titulo.
pub fn nombre_fichero_seguro(titulo: &str, extension: &str) -> String {
    let base: String = titulo
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let limpio = base
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("_")
        .trim_matches('-')
        .to_lowercase();
    let final_txt = if limpio.is_empty() {
        "datasearch".to_string()
    } else {
        limpio
    };
    format!("{final_txt}.{extension}")
}

/// SHA-256 en hexadecimal. Se usa para firmar la descarga de actualizaciones.
pub fn sha256_hex(datos: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(datos);
    hex::encode(h.finalize())
}

/// Divide una lista de cadenas en trozos de `tamano`.
pub fn trozos<T: Clone>(lista: &[T], tamano: usize) -> Vec<Vec<T>> {
    if tamano == 0 {
        return vec![lista.to_vec()];
    }
    lista.chunks(tamano).map(|c| c.to_vec()).collect()
}

/// Une columnas y filas en una matriz de texto, para exportaciones.
pub fn a_matriz(filas: &[crate::models::Fila]) -> (Vec<String>, Vec<Vec<String>>) {
    let mut columnas: Vec<String> = Vec::new();
    let mut vistas = std::collections::BTreeSet::new();
    for f in filas {
        for k in f.valores.keys() {
            if vistas.insert(k.clone()) {
                columnas.push(k.clone());
            }
        }
    }
    let datos = filas
        .iter()
        .map(|f| {
            columnas
                .iter()
                .map(|c| f.valores.get(c).cloned().flatten().unwrap_or_default())
                .collect::<Vec<String>>()
        })
        .collect();
    (columnas, datos)
}

/// Percent-encodifica un componente de URL (usuario, clave, host, base de datos).
pub fn escapar_url(texto: &str) -> String {
    let mut salida = String::with_capacity(texto.len());
    for b in texto.as_bytes() {
        let c = *b;
        let seguro = c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b'.' | b'~');
        if seguro {
            salida.push(c as char);
        } else {
            salida.push_str(&format!("%{c:02X}"));
        }
    }
    salida
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeros_en_ambos_formatos() {
        assert_eq!(a_f64("1234.56"), Some(1234.56));
        assert_eq!(a_f64("1.234,56"), Some(1234.56));
        assert_eq!(a_f64("1,234.56"), Some(1234.56));
        assert_eq!(a_f64("25%"), Some(25.0));
        assert_eq!(a_f64("-42"), Some(-42.0));
        assert_eq!(a_f64("no es un numero"), None);
        assert_eq!(a_f64(""), None);
    }

    #[test]
    fn formato_de_miles() {
        assert_eq!(formatear_numero(1234567.0, 0), "1.234.567");
        assert_eq!(formatear_numero(1234.5, 2), "1.234,50");
        assert_eq!(formatear_numero(-99.0, 0), "-99");
    }

    #[test]
    fn deteccion_de_familias() {
        assert_eq!(familia_de("42"), Familia::Numero);
        assert_eq!(familia_de("2026-09-28"), Familia::Fecha);
        assert_eq!(familia_de("28/09/2026"), Familia::Fecha);
        assert_eq!(familia_de("hola"), Familia::Texto);
        assert_eq!(familia_de(""), Familia::Vacio);
        assert_eq!(familia_de("true"), Familia::Booleano);
    }

    #[test]
    fn normalizacion_de_acentos() {
        assert_eq!(normalizar("Provincia de Alicante"), "provincia de alicante");
        assert_eq!(normalizar("MUÑOZ"), "munoz");
    }

    #[test]
    fn aplanado_de_json() {
        let v: serde_json::Value = serde_json::json!({
            "id": 1,
            "cliente": {"nombre": "Ana", "pais": "ES"},
            "lineas": [{"sku": "A1", "cant": 2}, {"sku": "B2", "cant": 5}]
        });
        let plano = aplanar_json(&v, "");
        assert_eq!(plano.get("id").map(String::as_str), Some("1"));
        assert_eq!(plano.get("cliente.nombre").map(String::as_str), Some("Ana"));
        assert_eq!(plano.get("lineas[1].sku").map(String::as_str), Some("B2"));
    }

    #[test]
    fn escapado_sql() {
        assert_eq!(escapar_sql("O'Brien"), "O''Brien");
        assert_eq!(escapar_identificador("mi columna"), "\"mi columna\"");
    }
}

//! Modelos de dominio compartidos entre el nucleo y la interfaz.
//!
//! Todos los structs derivan `Serialize`/`Deserialize` en `camelCase` porque son
//! consumidos directamente por TypeScript mediante los comandos IPC de Tauri.
//!
//! Copyright (c) 2026 Jose Manuel Bernabeu Mejias
//! Licencia MIT

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Rol de un usuario registrado.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Rol {
    Usuario,
    Administrador,
}

impl Rol {
    pub fn desde_txt(txt: &str) -> Self {
        match txt.to_ascii_lowercase().as_str() {
            "admin" | "administrador" => Rol::Administrador,
            _ => Rol::Usuario,
        }
    }
    pub fn es_admin(&self) -> bool {
        matches!(self, Rol::Administrador)
    }
}

/// Estado de verificacion de una cuenta.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Estado {
    /// Registrada, esperando validacion del administrador.
    Pendiente,
    /// Validada por el administrador; puede iniciar sesion.
    Activo,
    /// Bloqueada por el administrador.
    Suspendido,
}

impl Estado {
    pub fn desde_txt(txt: &str) -> Self {
        match txt.to_ascii_lowercase().as_str() {
            "activo" | "active" => Estado::Activo,
            "suspendido" | "suspend" => Estado::Suspendido,
            _ => Estado::Pendiente,
        }
    }
    pub fn puede_entrar(&self) -> bool {
        matches!(self, Estado::Activo)
    }
}

/// Permisos asignables por el administrador.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Permisos {
    pub consultar: bool,
    pub crear_fuentes: bool,
    pub modificar_fuentes: bool,
    pub eliminar_fuentes: bool,
    pub exportar: bool,
    pub imprimir: bool,
    pub analizar: bool,
    pub crear_dashboards: bool,
    pub ver_auditoria: bool,
}

impl Default for Permisos {
    fn default() -> Self {
        Self {
            consultar: true,
            crear_fuentes: true,
            modificar_fuentes: true,
            eliminar_fuentes: true,
            exportar: true,
            imprimir: true,
            analizar: true,
            crear_dashboards: true,
            ver_auditoria: true,
        }
    }
}

impl Permisos {
    /// Convierte la mascara de bits almacenada en la base de datos.
    pub fn desde_mascara(mascara: i64) -> Self {
        let b = |bit: u32| mascara & (1i64 << bit) != 0;
        Self {
            consultar: b(0),
            crear_fuentes: b(1),
            modificar_fuentes: b(2),
            eliminar_fuentes: b(3),
            exportar: b(4),
            imprimir: b(5),
            analizar: b(6),
            crear_dashboards: b(7),
            ver_auditoria: b(8),
        }
    }

    pub fn a_mascara(self) -> i64 {
        let mut m = 0i64;
        let mut s = |bit: u32, v: bool| {
            if v {
                m |= 1i64 << bit;
            }
        };
        s(0, self.consultar);
        s(1, self.crear_fuentes);
        s(2, self.modificar_fuentes);
        s(3, self.eliminar_fuentes);
        s(4, self.exportar);
        s(5, self.imprimir);
        s(6, self.analizar);
        s(7, self.crear_dashboards);
        s(8, self.ver_auditoria);
        m
    }
}

/// Datos personales completos exigidos en el registro.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatosPersonales {
    pub nombre: String,
    pub apellidos: String,
    #[serde(default)]
    pub documento: String,
    pub email: String,
    pub telefono: String,
    pub direccion: String,
    #[serde(default)]
    pub numero: String,
    pub codigo_postal: String,
    pub poblacion: String,
    pub provincia: String,
    pub pais: String,
    #[serde(default)]
    pub fecha_nacimiento: String,
    #[serde(default)]
    pub empresa: String,
    #[serde(default)]
    pub cargo: String,
    /// Texto libre de requisitos que el administrador puede bloquear.
    #[serde(default)]
    pub motivo_solicitud: String,
}

impl DatosPersonales {
    /// Valida los campos obligatorios y devuelve la lista de faltantes.
    pub fn validar(&self) -> Vec<String> {
        let mut faltan = Vec::new();
        let pares: [(&str, &str); 10] = [
            ("nombre", &self.nombre),
            ("apellidos", &self.apellidos),
            ("email", &self.email),
            ("telefono", &self.telefono),
            ("direccion", &self.direccion),
            ("codigo_postal", &self.codigo_postal),
            ("poblacion", &self.poblacion),
            ("provincia", &self.provincia),
            ("pais", &self.pais),
            ("motivo_solicitud", &self.motivo_solicitud),
        ];
        for (campo, valor) in pares {
            if valor.trim().is_empty() {
                faltan.push(campo.to_string());
            }
        }
        let email = self.email.trim();
        if !email.is_empty() && !email.contains('@') {
            faltan.push("email (formato incorrecto)".to_string());
        }
        faltan
    }
}

/// Alta de usuario (registro).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SolicitudRegistro {
    pub usuario: String,
    pub contrasena: String,
    #[serde(default)]
    pub repetir_contrasena: String,
    #[serde(flatten)]
    pub datos: DatosPersonales,
}

/// Credenciales de acceso.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Credenciales {
    pub usuario: String,
    pub contrasena: String,
}

/// Usuario almacenado (nunca se expone el hash).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Usuario {
    pub id: i64,
    pub usuario: String,
    pub nombre: String,
    pub apellidos: String,
    pub documento: String,
    pub email: String,
    pub telefono: String,
    pub direccion: String,
    pub numero: String,
    pub codigo_postal: String,
    pub poblacion: String,
    pub provincia: String,
    pub pais: String,
    pub fecha_nacimiento: String,
    pub empresa: String,
    pub cargo: String,
    pub estado: Estado,
    pub rol: Rol,
    pub permisos: Permisos,
    pub creado_en: String,
    pub ultimo_acceso: Option<String>,
    pub notas_admin: String,
}

/// Vista publica de un usuario (lo que ve el propio usuario o el admin).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Sesion {
    pub token: String,
    pub usuario: Usuario,
    pub version_app: String,
    pub es_admin: bool,
}

/// Tipo de fuente de datos soportado.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TipoFuente {
    /// Base de datos local en fichero SQLite.
    Sqlite,
    /// Servidor PostgreSQL remoto o local.
    Postgres,
    /// Servidor MySQL / MariaDB.
    Mysql,
    /// Microsoft SQL Server.
    SqlServer,
    /// MongoDB (NoSQL).
    MongoDb,
    /// Fichero plano CSV.
    Csv,
    /// Fichero JSON.
    Json,
    /// Fichero XML.
    Xml,
    /// Libro Excel (.xlsx) como fuente.
    Excel,
    /// Recurso web (API JSON o CSV) accesible por HTTP/HTTPS.
    Web,
}

impl TipoFuente {
    pub fn desde_txt(txt: &str) -> crate::error::Resultado<Self> {
        Ok(
            match txt.to_ascii_lowercase().replace(['-', ' '], "_").as_str() {
                "sqlite" | "sqlite3" => TipoFuente::Sqlite,
                "postgres" | "postgresql" | "pgsql" => TipoFuente::Postgres,
                "mysql" | "mariadb" => TipoFuente::Mysql,
                "sqlserver" | "mssql" | "sql_server" => TipoFuente::SqlServer,
                "mongodb" | "mongo" => TipoFuente::MongoDb,
                "csv" => TipoFuente::Csv,
                "json" => TipoFuente::Json,
                "xml" => TipoFuente::Xml,
                "excel" | "xlsx" => TipoFuente::Excel,
                "web" | "http" | "https" | "api" | "url" => TipoFuente::Web,
                otro => {
                    return Err(crate::error::Error::TipoNoSoportado(otro.to_string()));
                }
            },
        )
    }

    /// `true` cuando la fuente expone un modelo relacional (tablas y columnas).
    pub fn es_relacional(&self) -> bool {
        matches!(
            self,
            TipoFuente::Sqlite | TipoFuente::Postgres | TipoFuente::Mysql | TipoFuente::SqlServer
        )
    }

    /// `true` cuando la fuente es documental (colecciones sin esquema fijo).
    pub fn es_documental(&self) -> bool {
        matches!(self, TipoFuente::MongoDb | TipoFuente::Web)
    }

    /// `true` cuando la fuente es un fichero o recurso plano (una sola tabla).
    pub fn es_plano(&self) -> bool {
        matches!(
            self,
            TipoFuente::Csv | TipoFuente::Json | TipoFuente::Xml | TipoFuente::Excel
        )
    }

    pub fn etiqueta(&self) -> &'static str {
        match self {
            TipoFuente::Sqlite => "SQLite (fichero local)",
            TipoFuente::Postgres => "PostgreSQL",
            TipoFuente::Mysql => "MySQL / MariaDB",
            TipoFuente::SqlServer => "SQL Server",
            TipoFuente::MongoDb => "MongoDB",
            TipoFuente::Csv => "Fichero CSV",
            TipoFuente::Json => "Fichero JSON",
            TipoFuente::Xml => "Fichero XML",
            TipoFuente::Excel => "Libro Excel",
            TipoFuente::Web => "Recurso web (API)",
        }
    }
}

/// Configuracion de conexion de una fuente. Se serializa como JSON.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigFuente {
    // --- Servidores SQL ---
    #[serde(default)]
    pub host: String,
    #[serde(default)]
    pub puerto: Option<u16>,
    #[serde(default)]
    pub usuario: String,
    /// Contrasena. Nunca se devuelve al frontend.
    #[serde(default, skip_serializing)]
    pub contrasena: String,
    #[serde(default)]
    pub base_datos: String,
    #[serde(default)]
    pub esquema: String,
    #[serde(default)]
    pub ssl: bool,
    #[serde(default)]
    pub tiempo_espera_seg: Option<u64>,

    // --- Ficheros ---
    /// Ruta local absoluta o URL (http/https) segun el tipo.
    #[serde(default)]
    pub ruta: String,

    // --- Web ---
    #[serde(default)]
    pub metodo: String,
    #[serde(default)]
    pub cabeceras: BTreeMap<String, String>,
    #[serde(default)]
    pub cuerpo: String,
    /// Ruta JSON dentro de la respuesta (p. ej. `data.items`).
    #[serde(default)]
    pub ruta_json: String,
    /// `json` o `csv`.
    #[serde(default)]
    pub formato_web: String,
    /// Parametros de consulta estilo `pagina=1&limite=50`.
    #[serde(default)]
    pub parametros: String,

    // --- MongoDB ---
    #[serde(default)]
    pub uri: String,
    #[serde(default)]
    pub coleccion: String,
}

impl ConfigFuente {
    /// Redacta los secretos antes de enviar la configuracion al frontend.
    pub fn redactada(&self) -> Self {
        let mut c = self.clone();
        if !c.contrasena.is_empty() {
            c.contrasena = "********".to_string();
        }
        if !c.uri.is_empty() {
            c.uri = redactar_uri(&c.uri);
        }
        c
    }
}

fn redactar_uri(uri: &str) -> String {
    match (uri.find("://"), uri.rfind('@')) {
        (Some(i), Some(j)) if j > i => {
            let (pre, post) = uri.split_at(j);
            format!("{}:********{}", pre, post)
        }
        _ => uri.to_string(),
    }
}

/// Fuente de datos registrada por un usuario.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Fuente {
    pub id: i64,
    pub id_usuario: i64,
    pub nombre: String,
    pub descripcion: String,
    pub tipo: TipoFuente,
    pub config: ConfigFuente,
    pub activa: bool,
    pub solo_admin: bool,
    pub creado_en: String,
    pub actualizado_en: String,
    /// Comprobacion cacheada del ultimo intento de conexion.
    pub estado_conexion: String,
}

/// Alta / edicion de una fuente.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SolicitudFuente {
    pub nombre: String,
    #[serde(default)]
    pub descripcion: String,
    pub tipo: String,
    #[serde(default)]
    pub config: ConfigFuente,
    #[serde(default = "por_defecto_true")]
    pub activa: bool,
    #[serde(default)]
    pub solo_admin: bool,
    /// Al editar: no tocar la contrasena si llega vacia o enmascarada.
    #[serde(default)]
    pub conservar_secreto: bool,
}

fn por_defecto_true() -> bool {
    true
}

/// Operador de filtrado disponible en la interfaz.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operador {
    Contiene,
    NoContiene,
    EmpiezaCon,
    TerminaCon,
    Igual,
    Distinto,
    Mayor,
    MayorIgual,
    Menor,
    MenorIgual,
    Entre,
    EnLista,
    NoEnLista,
    EsNulo,
    NoEsNulo,
    Regex,
    Vacio,
    NoVacio,
    EsVerdadero,
    EsFalso,
}

impl Operador {
    pub fn desde_txt(txt: &str) -> crate::error::Resultado<Self> {
        use Operador::*;
        Ok(
            match txt.to_ascii_lowercase().replace(['-', ' '], "_").as_str() {
                "contiene" | "like" | "like_%25" => Contiene,
                "no_contiene" | "not_like" => NoContiene,
                "empieza_con" | "starts_with" => EmpiezaCon,
                "termina_con" | "ends_with" => TerminaCon,
                "igual" | "eq" | "equals" => Igual,
                "distinto" | "ne" | "not_equals" => Distinto,
                "mayor" | "gt" | "greater" => Mayor,
                "mayor_igual" | "gte" | "greater_equal" => MayorIgual,
                "menor" | "lt" | "less" => Menor,
                "menor_igual" | "lte" | "less_equal" => MenorIgual,
                "entre" | "between" => Entre,
                "en_lista" | "in" | "in_list" => EnLista,
                "no_en_lista" | "not_in" => NoEnLista,
                "es_nulo" | "is_null" | "null" => EsNulo,
                "no_es_nulo" | "is_not_null" | "not_null" => NoEsNulo,
                "regex" | "expresion_regular" => Regex,
                "vacio" | "is_empty" | "blank" => Vacio,
                "no_vacio" | "is_not_empty" => NoVacio,
                "es_verdadero" | "is_true" => EsVerdadero,
                "es_falso" | "is_falso" | "is_false" => EsFalso,
                otro => {
                    return Err(crate::error::Error::FiltroInvalido(format!(
                        "operador desconocido: {otro}"
                    )));
                }
            },
        )
    }

    /// `true` si el operador necesita un segundo valor (comparacion entre dos).
    pub fn necesita_segundo_valor(&self) -> bool {
        matches!(self, Operador::Entre)
    }

    /// `true` si el operador trabaja sobre listas de valores separados por coma.
    pub fn necesita_lista(&self) -> bool {
        matches!(self, Operador::EnLista | Operador::NoEnLista)
    }

    /// `true` si el operador no usa ningun valor del usuario.
    pub fn es_unario(&self) -> bool {
        matches!(
            self,
            Operador::EsNulo
                | Operador::NoEsNulo
                | Operador::Vacio
                | Operador::NoVacio
                | Operador::EsVerdadero
                | Operador::EsFalso
        )
    }

    pub fn etiqueta(&self) -> &'static str {
        use Operador::*;
        match self {
            Contiene => "contiene",
            NoContiene => "no contiene",
            EmpiezaCon => "empieza por",
            TerminaCon => "termina en",
            Igual => "es igual a",
            Distinto => "es distinto de",
            Mayor => "es mayor que",
            MayorIgual => "es mayor o igual que",
            Menor => "es menor que",
            MenorIgual => "es menor o igual que",
            Entre => "esta entre",
            EnLista => "esta en la lista",
            NoEnLista => "no esta en la lista",
            EsNulo => "es nulo",
            NoEsNulo => "no es nulo",
            Regex => "coincide con expresion regular",
            Vacio => "esta vacio",
            NoVacio => "no esta vacio",
            EsVerdadero => "es verdadero",
            EsFalso => "es falso",
        }
    }
}

/// Union logica entre filtros.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Conector {
    #[default]
    Y,
    O,
}

impl Conector {
    pub fn desde_txt(txt: &str) -> Self {
        match txt.to_ascii_lowercase().as_str() {
            "o" | "or" | "or_1" => Conector::O,
            _ => Conector::Y,
        }
    }
    pub fn sql(&self) -> &'static str {
        match self {
            Conector::Y => "AND",
            Conector::O => "OR",
        }
    }
}

/// Un filtro del usuario.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Filtro {
    pub campo: String,
    pub operador: String,
    #[serde(default)]
    pub valor: String,
    /// Solo para `Entre` y comparaciones de rango.
    #[serde(default)]
    pub valor2: String,
    /// `y` u `o`. El primer filtro puede indicar el modo global.
    #[serde(default)]
    pub conector: String,
    /// `true` para aplicar el filtro al grupo completo.
    #[serde(default)]
    pub activo: bool,
}

impl Filtro {
    pub fn op(&self) -> crate::error::Resultado<Operador> {
        Operador::desde_txt(&self.operador)
    }
    pub fn con(&self) -> Conector {
        Conector::desde_txt(&self.conector)
    }
    /// Convierte `a, b, c` en una lista limpia de valores.
    pub fn lista_valores(&self) -> Vec<String> {
        self.valor
            .split(&['\u{1F}', ',', '\n'][..])
            .map(|s| s.trim().trim_matches(['\'', '"']).to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }
}

/// Peticion de busqueda sobre una o varias fuentes.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeticionBusqueda {
    /// Ids de las fuentes seleccionadas. Vacio = todas las permitidas.
    #[serde(default)]
    pub fuentes: Vec<i64>,
    /// Tablas / colecciones a consultar. Vacio = todas.
    #[serde(default)]
    pub esquemas: Vec<String>,
    /// Texto libre que se busca en todas las columnas de texto.
    #[serde(default)]
    pub texto: String,
    #[serde(default)]
    pub filtros: Vec<Filtro>,
    /// Columnas a devolver. Vacio = todas.
    #[serde(default)]
    pub campos: Vec<String>,
    #[serde(default = "limite_por_defecto")]
    pub limite: u32,
    #[serde(default)]
    pub desplazamiento: u32,
    #[serde(default)]
    pub ordenar_por: String,
    #[serde(default)]
    pub orden_desc: bool,
    /// `y` u `o` a nivel de grupo.
    #[serde(default = "conector_por_defecto")]
    pub modo: String,
    /// Devuelve tambien el total de filas sin paginar.
    #[serde(default = "por_defecto_true")]
    pub contar_total: bool,
}

fn limite_por_defecto() -> u32 {
    200
}
fn conector_por_defecto() -> String {
    "y".to_string()
}
impl Default for PeticionBusqueda {
    fn default() -> Self {
        Self {
            fuentes: Vec::new(),
            esquemas: Vec::new(),
            texto: String::new(),
            filtros: Vec::new(),
            campos: Vec::new(),
            limite: limite_por_defecto(),
            desplazamiento: 0,
            ordenar_por: String::new(),
            orden_desc: false,
            modo: conector_por_defecto(),
            contar_total: por_defecto_true(),
        }
    }
}

/// Un valor de celda. Se serializa como texto legible en la interfaz.
pub type ValorCelda = Option<String>;

/// Fila de resultados.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Fila {
    pub valores: BTreeMap<String, ValorCelda>,
}

/// Bloque de resultados de una consulta.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BloqueResultados {
    pub fuente_id: i64,
    pub fuente_nombre: String,
    pub tipo_fuente: TipoFuente,
    pub esquema: String,
    pub filas: Vec<Fila>,
    pub columnas: Vec<ColumnaInfo>,
    pub total_filas: i64,
    pub filas_omitidas: u64,
    pub duracion_ms: u64,
    pub mensaje: String,
}

/// Metadatos de una columna.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnaInfo {
    pub nombre: String,
    pub tipo: String,
    pub etiqueta: String,
    pub numerica: bool,
    pub es_texto: bool,
    pub es_fecha: bool,
    pub es_booleano: bool,
    pub nulos: i64,
    pub distintos: i64,
    /// Valores mas frecuentes, para poblar el desplegable de filtrado.
    pub valores_frecuentes: Vec<Frecuencia>,
}

/// Valor frecuente de una columna, usado en el desplegable de filtros.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Frecuencia {
    pub valor: String,
    pub cuenta: i64,
    pub porcentaje: f64,
}

/// Respuesta completa de una busqueda sobre multiples fuentes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RespuestaBusqueda {
    pub bloques: Vec<BloqueResultados>,
    pub total_filas: i64,
    pub total_bloques: usize,
    pub duracion_ms: u64,
    pub fuentes_con_error: Vec<String>,
    pub mensaje: String,
    /// Peticiones de refinamiento generadas automaticamente.
    pub sugerencias: Vec<Sugerencia>,
}

/// Sugerencia de filtro detected automaticamente.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sugerencia {
    pub etiqueta: String,
    pub columna: String,
    pub valor: String,
    pub confianza: f64,
}

/// Catalogo de esquemas (tablas) de una fuente.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Esquema {
    pub nombre: String,
    pub tipo: String,
    pub filas_estimadas: i64,
    pub columnas: Vec<ColumnaEsquema>,
}

/// Columna dentro de un esquema.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnaEsquema {
    pub nombre: String,
    pub tipo: String,
    pub obligatorio: bool,
    pub clave_primaria: bool,
}

/// Tipo de grafico soportado en el dashboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TipoGrafico {
    Kpi,
    Barra,
    Linea,
    Area,
    Torta,
    Dispersion,
    Tabla,
    Indicador,
    MapaCalor,
}

impl TipoGrafico {
    pub fn desde_txt(txt: &str) -> Self {
        use TipoGrafico::*;
        match txt.to_ascii_lowercase().replace(['-', ' '], "_").as_str() {
            "kpi" | "cifra" | "numero" => Kpi,
            "barra" | "barras" | "bar" | "columna" | "columnas" => Barra,
            "linea" | "lineas" | "line" => Linea,
            "area" | "areas" => Area,
            "torta" | "circular" | "pie" | "dona" => Torta,
            "dispersion" | "scatter" | "puntos" => Dispersion,
            "tabla" | "table" | "listado" => Tabla,
            "indicador" | "gauge" | "medidor" => Indicador,
            "mapa_calor" | "heatmap" | "calor" => MapaCalor,
            _ => Tabla,
        }
    }
    pub fn etiqueta(&self) -> &'static str {
        use TipoGrafico::*;
        match self {
            Kpi => "Cifra destacada",
            Barra => "Grafico de barras",
            Linea => "Grafico de lineas",
            Area => "Grafico de areas",
            Torta => "Grafico de torta",
            Dispersion => "Grafico de dispersion",
            Tabla => "Tabla",
            Indicador => "Indicador",
            MapaCalor => "Mapa de calor",
        }
    }
}

/// Definicion de un widget del dashboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Widget {
    pub id: String,
    pub titulo: String,
    pub tipo: TipoGrafico,
    pub fuente_id: i64,
    pub esquema: String,
    /// Columna de agrupacion (categorias / eje X).
    pub columna_grupo: String,
    /// Columna de medida (valores / serie Y).
    pub columna_valor: String,
    /// Funcion de agregacion: suma, media, min, max, cuenta, distinta.
    pub agregacion: String,
    pub limite: u32,
    #[serde(default)]
    pub filtros: Vec<Filtro>,
    /// Tamano en la cuadricula: 1, 2 o 3 columnas.
    pub ancho: u32,
    /// Formato de la cifra: numero, moneda, porcentaje, texto.
    pub formato: String,
    /// Texto libre que genera el widget a partir de lenguaje natural.
    #[serde(default)]
    pub peticion_texto: String,
}

/// Peticion de construccion de un dashboard.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeticionDashboard {
    #[serde(default)]
    pub nombre: String,
    /// Ids de fuentes. Vacio = todas.
    #[serde(default)]
    pub fuentes: Vec<i64>,
    /// Esquemas a explorar. Vacio = todos.
    #[serde(default)]
    pub esquemas: Vec<String>,
    /// Peticion en lenguaje natural. Opcional si se eligen widgets a mano.
    #[serde(default)]
    pub instruccion: String,
    /// Graficos comodos que se anaden siempre al panel de control.
    #[serde(default = "graficos_por_defecto")]
    pub widgets_base: Vec<String>,
    /// Maximo de widgets generados a partir de la instruccion.
    #[serde(default = "max_widgets_por_defecto")]
    pub max_widgets: u32,
    /// Filtros aplicados de forma global a todos los widgets.
    #[serde(default)]
    pub filtros: Vec<Filtro>,
}

fn graficos_por_defecto() -> Vec<String> {
    vec![
        "kpi".to_string(),
        "tabla".to_string(),
        "barra".to_string(),
        "linea".to_string(),
    ]
}
fn max_widgets_por_defecto() -> u32 {
    6
}

/// Un punto de datos ya agregado para un grafico.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PuntoGrafico {
    pub etiqueta: String,
    pub valor: f64,
    pub valor_secundario: Option<f64>,
    pub categoria: String,
}

/// Widget listo para pintar, con sus datos ya calculados.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WidgetListo {
    pub id: String,
    pub titulo: String,
    pub tipo: TipoGrafico,
    pub subtitulo: String,
    pub fuente_nombre: String,
    pub esquema: String,
    pub columna_grupo: String,
    pub columna_valor: String,
    pub agregacion: String,
    pub formato: String,
    pub ancho: u32,
    pub puntos: Vec<PuntoGrafico>,
    pub columnas: Vec<ColumnaInfo>,
    pub filas: Vec<Fila>,
    pub total_filas: i64,
    pub valor_destacado: Option<f64>,
    pub texto_destacado: Option<String>,
    pub estado: String,
}

/// Tablero completo.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tablero {
    pub nombre: String,
    pub widgets: Vec<WidgetListo>,
    pub generado_en: String,
    pub duracion_ms: u64,
    pub fuentes: Vec<String>,
    pub esquemas: Vec<String>,
    pub advertencias: Vec<String>,
}

/// Analisis estadistico solicitado.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TipoAnalisis {
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
    Resumen,
}

impl TipoAnalisis {
    pub fn desde_txt(txt: &str) -> Self {
        use TipoAnalisis::*;
        match txt.to_ascii_lowercase().replace(['-', ' '], "_").as_str() {
            "describir" | "describe" | "descripcion" => Describir,
            "nulos" | "nulls" | "valores_nulos" => Nulos,
            "distintos" | "unicos" | "unique" | "distintos_valores" => Distintos,
            "frecuencias" | "frecuencia" | "top" | "valores_mas_frecuentes" => Frecuencias,
            "histograma" | "histogram" => Histograma,
            "correlacion" | "correlacion_pearson" => Correlacion,
            "outliers" | "valores_atipicos" => Outliers,
            "tendencia" | "trend" => Tendencia,
            "calidad" | "calidad_datos" | "quality" => Calidad,
            "duplicados" | "duplicados_valores" => Duplicados,
            _ => Resumen,
        }
    }
    pub fn etiqueta(&self) -> &'static str {
        use TipoAnalisis::*;
        match self {
            Describir => "Descripcion estadistica",
            Nulos => "Analisis de valores nulos",
            Distintos => "Valores distintos",
            Frecuencias => "Valores mas frecuentes",
            Histograma => "Histograma",
            Correlacion => "Matriz de correlacion",
            Outliers => "Valores atipicos (outliers)",
            Tendencia => "Tendencia de la serie",
            Calidad => "Calidad de los datos",
            Duplicados => "Filas duplicadas",
            Resumen => "Resumen completo",
        }
    }
}

/// Peticion de analisis. Requiere confirmacion explicita del usuario.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeticionAnalisis {
    pub fuente_id: i64,
    pub esquema: String,
    #[serde(default)]
    pub columnas: Vec<String>,
    /// Si viene vacio se ejecuta `Resumen`.
    #[serde(default)]
    pub analyses: Vec<String>,
    #[serde(default = "por_defecto_true")]
    pub es_analisis_confirmado: bool,
    #[serde(default)]
    pub filtros: Vec<Filtro>,
    #[serde(default = "limite_por_defecto")]
    pub limite: u32,
    /// Etiqueta para identificar la peticion en la interfaz.
    #[serde(default)]
    pub comentario: String,
}

/// Seccion de resultados de un analisis.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeccionAnalisis {
    pub titulo: String,
    pub tipo: String,
    pub descripcion: String,
    pub filas: Vec<Fila>,
    pub clave: BTreeMap<String, String>,
    pub metricas: Vec<Metrica>,
}

/// Metrica escalar de un analisis.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Metrica {
    pub nombre: String,
    pub valor: String,
    pub unidad: String,
}

/// Respuesta completa de un analisis.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RespuestaAnalisis {
    pub fuente_nombre: String,
    pub esquema: String,
    pub secciones: Vec<SeccionAnalisis>,
    pub filas_analizadas: i64,
    pub columnas_analizadas: Vec<String>,
    pub duracion_ms: u64,
    pub resumen: String,
    pub advertencias: Vec<String>,
}

/// Formatos de exportacion admitidos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FormatoExportacion {
    Excel,
    Pdf,
    Csv,
    Json,
    Html,
    Texto,
}

impl FormatoExportacion {
    pub fn desde_txt(txt: &str) -> crate::error::Resultado<Self> {
        Ok(match txt.to_ascii_lowercase().as_str() {
            "excel" | "xlsx" => FormatoExportacion::Excel,
            "pdf" => FormatoExportacion::Pdf,
            "csv" => FormatoExportacion::Csv,
            "json" => FormatoExportacion::Json,
            "html" => FormatoExportacion::Html,
            "texto" | "txt" | "text" => FormatoExportacion::Texto,
            otro => {
                return Err(crate::error::Error::Exportacion(format!(
                    "formato no soportado: {otro}"
                )));
            }
        })
    }
}

/// Documento a exportar.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Documento {
    pub titulo: String,
    pub subtitulo: String,
    pub pie: String,
    pub bloques: Vec<BloqueResultados>,
    pub secciones: Vec<SeccionAnalisis>,
    pub widgets: Vec<WidgetListo>,
    /// Imagenes PNG (data URL base64) de los graficos a incrustar.
    pub imagenes: Vec<ImagenDocumento>,
    pub generado_por: String,
    pub generado_en: String,
    pub duracion_ms: u64,
}

/// Imagen a incrustar en el PDF, en PNG y en base64.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImagenDocumento {
    pub titulo: String,
    pub base64: String,
    pub ancho_px: u32,
    pub alto_px: u32,
}

/// Resultado de una exportacion: nombre de fichero y contenido en base64.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchivoExportado {
    pub nombre: String,
    pub contenido_base64: String,
    pub tamano_bytes: u64,
    pub formato: String,
    pub mime: String,
}

/// Auditoria de acciones.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistroAuditoria {
    pub id: i64,
    pub id_usuario: i64,
    pub usuario: String,
    pub accion: String,
    pub detalle: String,
    pub exito: bool,
    pub fecha: String,
}

/// Version de la aplicacion publicada en GitHub Releases.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InfoVersion {
    pub version_actual: String,
    pub version_disponible: String,
    pub hay_actualizacion: bool,
    pub url_descarga: String,
    pub notas: String,
    pub publicada_en: String,
    pub nombre_lanzamiento: String,
    pub consultado: bool,
}

/// Informacion del autor, mostrada en el boton de credenciales.
pub const AUTOR_NOMBRE: &str = "Jose Manuel Bernabeu Mejias";
pub const AUTOR_CORREO: &str = "jmbernabu@github.com";
pub const DIRECCION_AUTOR: &str = "Calle Medico Rafael Navarro 2, 2 C";
pub const POBLACION_AUTOR: &str = "Novelda";
pub const CODIGO_POSTAL_AUTOR: &str = "03660";
pub const PROVINCIA_AUTOR: &str = "Alicante";
pub const PAIS_AUTOR: &str = "Espana";
pub const LICENCIA: &str = "MIT";
pub const REPOSITORIO: &str = "https://github.com/JMBermejias/Datasearch";
pub const USUARIO_ADMIN_DEFECTO: &str = "JMBernabeu";

// ======================================================================
// Paleta de la aplicacion (azul claro)
// ======================================================================

/// Azul principal de la interfaz.
pub const COLOR_AZUL: &str = "#0ea5e9";
/// Azul usado en titulos y subrayados.
pub const COLOR_AZUL_OSCURO: &str = "#0284c7";
/// Azul muy claro de fondos y bordes.
pub const COLOR_AZUL_CLARO: &str = "#7dd3fc";
/// Fondo de la zona de trabajo.
pub const COLOR_BLANCO: &str = "#ffffff";
/// Fondo de la aplicacion.
pub const COLOR_FONDO: &str = "#f0f9ff";
/// Color del texto principal.
pub const COLOR_TEXTO: &str = "#0f172a";
/// Color de las lineas de separacion.
pub const COLOR_BORDE: &str = "#bae6fd";

/// Convierte un color hexadecimal `#rrggbb` en componentes 0..=1 para PDF.
pub fn color_rgb(hex: &str) -> (f32, f32, f32) {
    let h = hex.trim_start_matches('#');
    let leer = |i: usize| {
        f32::from(u8::from_str_radix(&h[i..(i + 2).min(h.len())], 16).unwrap_or(0)) / 255.0
    };
    (leer(0), leer(2), leer(4))
}

/// Ficha del autor tal y como se muestra en la ventana de credenciales.
pub fn ficha_autor() -> serde_json::Value {
    serde_json::json!({
        "nombre": AUTOR_NOMBRE,
        "correo": AUTOR_CORREO,
        "direccion": DIRECCION_AUTOR,
        "codigoPostal": CODIGO_POSTAL_AUTOR,
        "poblacion": POBLACION_AUTOR,
        "provincia": PROVINCIA_AUTOR,
        "pais": PAIS_AUTOR,
        "licencia": LICENCIA,
        "repositorio": REPOSITORIO,
        "derechos": format!("Copyright (c) 2026 {AUTOR_NOMBRE}. Todos los derechos reservados."),
    })
}

//! Manejo centralizado de errores de DataSearch.
//!
//! Copyright (c) 2026 Jose Manuel Bernabeu Mejias
//! Licencia MIT

use serde::Serialize;
use std::fmt;

/// ResultadoShortcut usado por todo el nucleo.
pub type Resultado<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Error de base de datos de la aplicacion: {0}")]
    AppDb(String),

    #[error("Error de conexion con la fuente de datos: {0}")]
    Conexion(String),

    #[error("Error al ejecutar la consulta: {0}")]
    Consulta(String),

    #[error("No se ha encontrado ningun resultado")]
    SinResultados,

    #[error("Usuario o contrasena incorrectos")]
    CredencialesInvalidas,

    #[error("El usuario no existe")]
    UsuarioNoExiste,

    #[error("El nombre de usuario ya esta registrado")]
    UsuarioDuplicado,

    #[error("La cuenta esta pendiente de verificacion por el administrador")]
    CuentaPendiente,

    #[error("La cuenta esta suspendida")]
    CuentaSuspendida,

    #[error("Permiso denegado: {0}")]
    PermisoDenegado(String),

    #[error("No ha iniciado sesion")]
    SinSesion,

    #[error("Sesion caducada")]
    SesionCaducada,

    #[error("Faltan datos obligatorios: {0}")]
    DatosIncompletos(String),

    #[error("Formato de filtro incorrecto: {0}")]
    FiltroInvalido(String),

    #[error("Tipo de fuente de datos no soportado: {0}")]
    TipoNoSoportado(String),

    #[error("Error de validacion: {0}")]
    Validacion(String),

    #[error("Error al exportar: {0}")]
    Exportacion(String),

    #[error("Error de red: {0}")]
    Red(String),

    #[error("Error interno: {0}")]
    Interno(String),

    #[error("Operacion cancelada por el usuario")]
    Cancelado,
}

impl Error {
    /// Categoria estable para el frontend (permite traducir o filtrar).
    pub fn codigo(&self) -> &'static str {
        match self {
            Error::AppDb(_) => "APP_DB",
            Error::Conexion(_) => "CONEXION",
            Error::Consulta(_) => "CONSULTA",
            Error::SinResultados => "SIN_RESULTADOS",
            Error::CredencialesInvalidas => "CREDENCIALES_INVALIDAS",
            Error::UsuarioNoExiste => "USUARIO_NO_EXISTE",
            Error::UsuarioDuplicado => "USUARIO_DUPLICADO",
            Error::CuentaPendiente => "CUENTA_PENDIENTE",
            Error::CuentaSuspendida => "CUENTA_SUSPENDIDA",
            Error::PermisoDenegado(_) => "PERMISO_DENEGADO",
            Error::SinSesion => "SIN_SESION",
            Error::SesionCaducada => "SESION_CADUCADA",
            Error::DatosIncompletos(_) => "DATOS_INCOMPLETOS",
            Error::FiltroInvalido(_) => "FILTRO_INVALIDO",
            Error::TipoNoSoportado(_) => "TIPO_NO_SOPORTADO",
            Error::Validacion(_) => "VALIDACION",
            Error::Exportacion(_) => "EXPORTACION",
            Error::Red(_) => "RED",
            Error::Interno(_) => "INTERNO",
            Error::Cancelado => "CANCELADO",
        }
    }

    pub fn es_permiso(&self) -> bool {
        matches!(self, Error::PermisoDenegado(_))
    }
}

impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        Error::AppDb(e.to_string())
    }
}

impl From<sqlx::Error> for Error {
    fn from(e: sqlx::Error) -> Self {
        match e {
            sqlx::Error::RowNotFound => Error::SinResultados,
            sqlx::Error::PoolTimedOut => Error::Conexion("tiempo de espera agotado".into()),
            other => Error::Conexion(other.to_string()),
        }
    }
}

impl From<mongodb::error::Error> for Error {
    fn from(e: mongodb::error::Error) -> Self {
        Error::Conexion(e.to_string())
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Interno(e.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Interno(e.to_string())
    }
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::Red(e.to_string())
    }
}

impl From<csv::Error> for Error {
    fn from(e: csv::Error) -> Self {
        Error::Interno(e.to_string())
    }
}

impl From<quick_xml::Error> for Error {
    fn from(e: quick_xml::Error) -> Self {
        Error::Interno(e.to_string())
    }
}

impl From<quick_xml::events::attributes::AttrError> for Error {
    fn from(e: quick_xml::events::attributes::AttrError) -> Self {
        Error::Interno(e.to_string())
    }
}

impl From<url::ParseError> for Error {
    fn from(e: url::ParseError) -> Self {
        Error::Validacion(format!("URL no valida: {e}"))
    }
}

impl From<argon2::password_hash::Error> for Error {
    fn from(_: argon2::password_hash::Error) -> Self {
        Error::CredencialesInvalidas
    }
}

/// Representacion serializable de un error, consumida por el frontend.
#[derive(Debug, Serialize)]
pub struct ErrorInfo {
    pub codigo: String,
    pub mensaje: String,
}

impl fmt::Display for ErrorInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.mensaje)
    }
}

impl Serialize for Error {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        ErrorInfo {
            codigo: self.codigo().to_string(),
            mensaje: self.to_string(),
        }
        .serialize(s)
    }
}

//! DataSearch - Nucleo de logica.
//!
//! Aplicacion de extraccion, filtrado, analisis y visualizacion de datos
//! procedentes de bases de datos locales, remotas y de la web.
//!
//! Copyright (c) 2026 Jose Manuel Bernabeu Mejias
//! Licencia MIT

pub mod analysis;
pub mod appdb;
pub mod auth;
pub mod drivers;
pub mod error;
pub mod export;
pub mod models;
pub mod query;
pub mod service;
pub mod sources;
pub mod update;
pub mod util;

/// Version de la aplicacion.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Nombre comercial.
pub const NOMBRE_APP: &str = "DataSearch";

/// Titulo con la version, mostrado en la barra lateral.
pub fn titulo_con_version() -> String {
    format!("{NOMBRE_APP} v{VERSION}")
}

pub use error::{Error, Resultado};

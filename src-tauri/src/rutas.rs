//! Rutas del sistema donde DataSearch guarda sus datos.
//!
//! Copyright (c) 2026 Jose Manuel Bernabeu Mejias
//! Licencia MIT

use std::path::PathBuf;

/// Carpeta de datos de la aplicacion.
///
/// - Linux: `~/.local/share/com.jmbernabu.datasearch`
/// - Android: el directorio privado que el sistema concede a la aplicacion
pub fn directorio_datos() -> PathBuf {
    if let Ok(dir) = std::env::var("XDG_DATA_HOME") {
        if !dir.trim().is_empty() {
            return PathBuf::from(dir).join("com.jmbernabu.datasearch");
        }
    }
    #[cfg(target_os = "android")]
    {
        // En Android el contexto de la app se inyecta como variable de entorno.
        if let Ok(dir) = std::env::var("DATASEARCH_DATA_DIR") {
            if !dir.trim().is_empty() {
                return PathBuf::from(dir);
            }
        }
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join("datasearch");
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("com.jmbernabu.datasearch")
}

/// Carpeta donde se guardan las exportaciones y las actualizaciones.
pub fn carpeta_descargas() -> PathBuf {
    if let Ok(dir) = std::env::var("DATASEARCH_DOWNLOADS_DIR") {
        if !dir.trim().is_empty() {
            return PathBuf::from(dir);
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let base = PathBuf::from(home).join("Descargas");
    if base.is_dir() {
        base
    } else {
        PathBuf::from(home).join("Downloads")
    }
}

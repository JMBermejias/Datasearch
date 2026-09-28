//! Busqueda de actualizaciones en las releases de GitHub.
//!
//! Al arrancar, la aplicacion consulta la API publica del repositorio y
//! compara la version publicada con la que esta en ejecucion. Si hay una
//! version nueva, la interfaz muestra la ventana de actualizacion que
//! descarga e instala el fichero.
//!
//! Copyright (c) 2026 Jose Manuel Bernabeu Mejias
//! Licencia MIT

use crate::error::{Error, Resultado};
use crate::models::InfoVersion;
use serde::Deserialize;
use std::time::Duration;

/// Repositorio donde se publican las releases.
pub const REPOSITORIO: &str = crate::models::REPOSITORIO;
/// Propietario y repositorio en formato `duenio/repositorio`.
const DUENIO_REPO: &str = "JMBermejias/Datasearch";

/// Forma de la respuesta de la API de releases de GitHub.
#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    name: Option<String>,
    body: Option<String>,
    draft: bool,
    prerelease: bool,
    published_at: Option<String>,
    html_url: String,
    #[serde(default)]
    assets: Vec<Activo>,
}

#[derive(Debug, Deserialize)]
struct Activo {
    name: String,
    browser_download_url: String,
    /// Tamano del fichero en bytes, para informar al usuario.
    #[allow(dead_code)]
    size: u64,
}

fn cliente() -> Resultado<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent(format!("DataSearch/{}", crate::VERSION))
        .build()
        .map_err(|e| Error::Red(e.to_string()))
}

/// Consulta la ultima release publicada y la compara con la version actual.
///
/// Nunca falla de forma fatal: si no hay red, la aplicacion sigue arrancando
/// y simplemente indica que no pudo comprobar.
pub async fn comprobar_actualizaciones() -> InfoVersion {
    let actual = crate::VERSION.to_string();
    let base = InfoVersion {
        hay_actualizacion: false,
        version_actual: actual.clone(),
        version_disponible: actual.clone(),
        url_descarga: REPOSITORIO.to_string(),
        notas: String::new(),
        publicada_en: String::new(),
        nombre_lanzamiento: String::new(),
        consultado: false,
    };

    match consultar_release().await {
        Ok(release) => {
            let disponible = normalizar_version(&release.tag_name);
            let actual_norm = normalizar_version(&actual);
            let hay = comparar_versiones(&disponible, &actual_norm) == std::cmp::Ordering::Greater;
            let notas = recortar_notas(release.body.as_deref().unwrap_or(""));
            let publicada_en = release.published_at.clone().unwrap_or_default();
            let nombre = release
                .name
                .clone()
                .unwrap_or_else(|| release.tag_name.clone());
            let descarga = mejor_descarga(&release);
            InfoVersion {
                hay_actualizacion: hay,
                notas,
                publicada_en,
                nombre_lanzamiento: nombre,
                url_descarga: descarga,
                version_actual: actual.clone(),
                version_disponible: if disponible.is_empty() {
                    actual
                } else {
                    disponible
                },
                consultado: true,
            }
        }
        Err(e) => InfoVersion {
            notas: format!("No se pudo comprobar si hay actualizaciones: {e}"),
            ..base
        },
    }
}

/// Descarga el JSON de la ultima release estable.
async fn consultar_release() -> Resultado<Release> {
    let url = format!("https://api.github.com/repos/{DUENIO_REPO}/releases/latest");
    let respuesta = cliente()?
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await
        .map_err(|e| Error::Red(format!("No se pudo contactar con GitHub: {e}")))?;

    if !respuesta.status().is_success() {
        return Err(Error::Red(format!(
            "GitHub respondio {}",
            respuesta.status()
        )));
    }
    let release: Release = respuesta
        .json()
        .await
        .map_err(|e| Error::Red(format!("Respuesta de GitHub inesperada: {e}")))?;
    if release.draft || release.prerelease {
        return Err(Error::Red("La ultima release es borrador o previa".into()));
    }
    Ok(release)
}

/// Elige el fichero de instalacion adecuado a la plataforma.
///
/// Linux busca el `.deb`; Android y el resto, el `.apk`.
fn mejor_descarga(release: &Release) -> String {
    for extension in ORDEN_PLATAFORMA {
        if let Some(a) = release
            .assets
            .iter()
            .find(|a| a.name.to_lowercase().ends_with(&format!(".{extension}")))
        {
            return a.browser_download_url.clone();
        }
    }
    release.html_url.clone()
}

/// Extensiones que se prefieren segun el destino de la descarga.
const ORDEN_PLATAFORMA: &[&str] = &["deb", "apk", "appimage", "rpm", "msi", "exe"];

/// Extrae de las notas de la release el enlace de descarga del `.deb`.
pub fn enlace_deb(notas: &str) -> Option<String> {
    notas
        .split_whitespace()
        .map(|t| t.trim_matches(|c| c == '<' || c == '>' || c == '(' || c == ')'))
        .find(|t| t.to_lowercase().ends_with(".deb") && t.starts_with("http"))
        .map(|t| t.to_string())
}

/// Quita la `v` inicial y cualquier sufijo de la etiqueta de la release.
pub fn normalizar_version(etiqueta: &str) -> String {
    etiqueta
        .trim()
        .trim_start_matches(['v', 'V'])
        .split(['-', '+'])
        .next()
        .unwrap_or("")
        .trim()
        .to_string()
}

/// Compara dos versiones punto separadas por puntos.
pub fn comparar_versiones(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let partes = |v: &str| -> Vec<u64> {
        v.split('.')
            .map(|p| {
                p.chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect::<String>()
            })
            .map(|p| p.parse().unwrap_or(0))
            .collect()
    };
    let pa = partes(a);
    let pb = partes(b);
    for i in 0..pa.len().max(pb.len()) {
        let x = pa.get(i).copied().unwrap_or(0);
        let y = pb.get(i).copied().unwrap_or(0);
        match x.cmp(&y) {
            Ordering::Equal => continue,
            otro => return otro,
        }
    }
    Ordering::Equal
}

/// Recorta las notas de la release a un tamano razonable para la ventana.
fn recortar_notas(notas: &str) -> String {
    let limpio = notas.trim();
    if limpio.chars().count() <= 1500 {
        return limpio.to_string();
    }
    let corte: String = limpio.chars().take(1497).collect();
    format!("{corte}...")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normaliza_etiquetas_de_version() {
        assert_eq!(normalizar_version("v1.2.3"), "1.2.3");
        assert_eq!(normalizar_version("V2.0"), "2.0");
        assert_eq!(normalizar_version("1.0.0-beta.1"), "1.0.0");
        assert_eq!(normalizar_version("  3.1  "), "3.1");
    }

    #[test]
    fn compara_versiones() {
        use std::cmp::Ordering;
        assert_eq!(comparar_versiones("1.0.1", "1.0.0"), Ordering::Greater);
        assert_eq!(comparar_versiones("1.0.0", "1.0.0"), Ordering::Equal);
        assert_eq!(comparar_versiones("0.9.9", "1.0.0"), Ordering::Less);
        assert_eq!(comparar_versiones("1.10.0", "1.9.0"), Ordering::Greater);
        assert_eq!(comparar_versiones("2.0", "2.0.0"), Ordering::Equal);
    }

    #[test]
    fn elige_el_deb_para_linux() {
        let release = Release {
            tag_name: "v1.5.0".into(),
            name: None,
            body: None,
            draft: false,
            prerelease: false,
            published_at: None,
            html_url: "https://github.com/JMBermejias/Datasearch/releases/tag/v1.5.0".into(),
            assets: vec![
                Activo {
                    name: "DataSearch-v1.5.0.apk".into(),
                    browser_download_url: "https://ejemplo/DataSearch.apk".into(),
                    size: 1,
                },
                Activo {
                    name: "DataSearch_1.5.0_amd64.deb".into(),
                    browser_download_url: "https://ejemplo/DataSearch.deb".into(),
                    size: 1,
                },
            ],
        };
        assert!(mejor_descarga(&release).ends_with(".deb"));
    }

    #[test]
    fn extrae_el_enlace_del_pdf() {
        let notas =
            "Descarga: https://github.com/x/y/releases/download/v1.0/DataSearch_1.0_amd64.deb";
        assert!(enlace_deb(notas).unwrap().ends_with(".deb"));
        assert!(enlace_deb("sin enlaces").is_none());
    }

    #[test]
    fn recorta_notas_largas() {
        let largo = "a".repeat(3000);
        assert!(recortar_notas(&largo).chars().count() <= 1500);
    }
}

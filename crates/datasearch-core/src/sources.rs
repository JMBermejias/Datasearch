//! Registro y gestion de las fuentes de datos del usuario.
//!
//! Cada usuario puede anadir tantas fuentes como necesite. Las fuentes marcadas
//! como `solo_admin` son invisibles para el resto de cuentas.
//!
//! Copyright (c) 2026 Jose Manuel Bernabeu Mejias
//! Licencia MIT

use crate::appdb::AppDb;
use crate::auth::Contexto;
use crate::drivers;
use crate::error::{Error, Resultado};
use crate::models::{ConfigFuente, Fuente, SolicitudFuente, TipoFuente};
use std::sync::Arc;

/// Servicio de fuentes de datos.
pub struct ServicioFuentes {
    db: Arc<AppDb>,
}

impl ServicioFuentes {
    pub fn nuevo(db: Arc<AppDb>) -> Self {
        Self { db }
    }

    /// Valida que la configuracion tenga lo minimo indispensable segun el tipo.
    pub fn validar_config(tipo: TipoFuente, c: &ConfigFuente) -> Resultado<()> {
        match tipo {
            TipoFuente::Sqlite
            | TipoFuente::Csv
            | TipoFuente::Json
            | TipoFuente::Xml
            | TipoFuente::Excel => {
                if c.ruta.trim().is_empty() {
                    return Err(Error::Validacion(
                        "Indique la ruta del fichero o la URL".into(),
                    ));
                }
                if tipo != TipoFuente::Excel && !c.ruta.trim().to_lowercase().ends_with("http") {
                    let ruta = c.ruta.trim();
                    let parece_web = ruta.starts_with("http://") || ruta.starts_with("https://");
                    if !parece_web && !std::path::Path::new(ruta).exists() {
                        return Err(Error::Validacion(format!(
                            "No se encuentra el fichero: {ruta}"
                        )));
                    }
                }
            }
            TipoFuente::Web => {
                let ruta = c.ruta.trim();
                if !ruta.starts_with("http://") && !ruta.starts_with("https://") {
                    return Err(Error::Validacion(
                        "La URL debe empezar por http:// o https://".into(),
                    ));
                }
                if let Ok(url) = url::Url::parse(ruta) {
                    if url.host_str().is_none() {
                        return Err(Error::Validacion("La URL no tiene servidor".into()));
                    }
                }
            }
            TipoFuente::Postgres
            | TipoFuente::Mysql
            | TipoFuente::SqlServer
            | TipoFuente::MongoDb => {
                if c.host.trim().is_empty() {
                    return Err(Error::Validacion("Indique el servidor".into()));
                }
                if c.base_datos.trim().is_empty() {
                    return Err(Error::Validacion("Indique la base de datos".into()));
                }
                if c.usuario.trim().is_empty() {
                    return Err(Error::Validacion("Indique el usuario".into()));
                }
            }
        }
        Ok(())
    }

    /// Crea una fuente nueva tras comprobar los permisos del usuario.
    pub fn crear(&self, ctx: &Contexto, solicitud: &SolicitudFuente) -> Resultado<Fuente> {
        ctx.exigir(crate::auth::Permiso::CrearFuentes)?;
        let nombre = solicitud.nombre.trim();
        if nombre.len() < 2 {
            return Err(Error::Validacion(
                "El nombre de la fuente debe tener al menos 2 caracteres".into(),
            ));
        }
        let tipo = TipoFuente::desde_txt(&solicitud.tipo)?;
        Self::validar_config(tipo, &solicitud.config)?;

        let config = serde_json::to_string(&solicitud.config)
            .map_err(|e| Error::Interno(format!("No se pudo guardar la configuracion: {e}")))?;
        let solo_admin = solicitud.solo_admin && ctx.es_admin();
        let id = self.db.insertar_fuente(
            ctx.id_usuario,
            nombre,
            solicitud.descripcion.trim(),
            tipo_txt(tipo),
            &config,
            solicitud.activa,
            solo_admin,
        )?;
        self.db.auditar(
            ctx.id_usuario,
            "crea_fuente",
            &format!("Fuente '{nombre}' creada ({})", tipo.etiqueta()),
            true,
        );
        self.db
            .fuente_por_id(id)?
            .ok_or_else(|| Error::Interno("No se pudo leer la fuente creada".into()))
    }

    /// Actualiza una fuente existente.
    pub fn actualizar(
        &self,
        ctx: &Contexto,
        id: i64,
        solicitud: &SolicitudFuente,
    ) -> Resultado<Fuente> {
        let actual = self.obtener(ctx, id)?;
        ctx.exigir(crate::auth::Permiso::ModificarFuentes)?;
        if !ctx.es_admin() && actual.id_usuario != ctx.id_usuario {
            return Err(Error::PermisoDenegado(
                "Solo puede modificar sus propias fuentes".into(),
            ));
        }

        let tipo = TipoFuente::desde_txt(&solicitud.tipo)?;
        // Al editar se conserva el secreto si el usuario no lo reescribe.
        let mut config = solicitud.config.clone();
        if solicitud.conservar_secreto
            && (config.contrasena.is_empty() || config.contrasena == "********")
        {
            config.contrasena = actual.config.contrasena.clone();
        }
        if solicitud.conservar_secreto && config.uri.is_empty() {
            config.uri = actual.config.uri.clone();
        }
        Self::validar_config(tipo, &config)?;

        let json = serde_json::to_string(&config)
            .map_err(|e| Error::Interno(format!("No se pudo guardar la configuracion: {e}")))?;
        self.db.actualizar_fuente(
            id,
            solicitud.nombre.trim(),
            solicitud.descripcion.trim(),
            &json,
            solicitud.activa,
            solicitud.solo_admin && ctx.es_admin(),
        )?;
        self.db.auditar(
            ctx.id_usuario,
            "actualiza_fuente",
            &format!("Fuente '{}' actualizada", solicitud.nombre.trim()),
            true,
        );
        self.db
            .fuente_por_id(id)?
            .ok_or_else(|| Error::Interno("No se pudo leer la fuente".into()))
    }

    /// Elimina una fuente.
    pub fn eliminar(&self, ctx: &Contexto, id: i64) -> Resultado<()> {
        let actual = self.obtener(ctx, id)?;
        ctx.exigir(crate::auth::Permiso::EliminarFuentes)?;
        if !ctx.es_admin() && actual.id_usuario != ctx.id_usuario {
            return Err(Error::PermisoDenegado(
                "Solo puede eliminar sus propias fuentes".into(),
            ));
        }
        self.db.eliminar_fuente(id)?;
        self.db.auditar(
            ctx.id_usuario,
            "elimina_fuente",
            &format!("Fuente '{}' eliminada", actual.nombre),
            true,
        );
        Ok(())
    }

    /// Devuelve una fuente compruebando que el usuario puede usarla.
    pub fn obtener(&self, ctx: &Contexto, id: i64) -> Resultado<Fuente> {
        let fuente = self
            .db
            .fuente_por_id(id)?
            .ok_or_else(|| Error::Validacion("La fuente indicada no existe".into()))?;
        if !self
            .db
            .fuente_permitida(id, ctx.id_usuario, ctx.es_admin())?
        {
            return Err(Error::PermisoDenegado(
                "No tiene acceso a esta fuente de datos".into(),
            ));
        }
        Ok(fuente)
    }

    /// Lista las fuentes visibles para el usuario, con la contrasena oculta.
    pub fn listar(&self, ctx: &Contexto) -> Resultado<Vec<Fuente>> {
        let fuentes = self.db.listar_fuentes(ctx.id_usuario, ctx.es_admin())?;
        Ok(fuentes
            .into_iter()
            .map(|mut f| {
                f.config = f.config.redactada();
                f
            })
            .collect())
    }

    /// Fuentes con las que se puede buscar ahora mismo.
    pub fn activas(&self, ctx: &Contexto) -> Resultado<Vec<Fuente>> {
        Ok(self
            .db
            .listar_fuentes(ctx.id_usuario, ctx.es_admin())?
            .into_iter()
            .filter(|f| f.activa)
            .collect())
    }

    /// Comprueba la conexion con la fuente y recuerda el resultado.
    pub async fn probar(&self, ctx: &Contexto, id: i64) -> Resultado<String> {
        let fuente = self.obtener(ctx, id)?;
        let resultado = drivers::probar(&fuente).await;
        match &resultado {
            Ok(_) => {
                let _ = self.db.marcar_estado_fuente(id, "correcta");
            }
            Err(_) => {
                let _ = self.db.marcar_estado_fuente(id, "error");
            }
        }
        self.db.auditar(
            ctx.id_usuario,
            "prueba_fuente",
            &format!("Prueba de '{}': {}", fuente.nombre, resultado.is_ok()),
            resultado.is_ok(),
        );
        resultado
    }

    /// Lista las tablas o colecciones de una fuente.
    pub async fn esquemas(
        &self,
        ctx: &Contexto,
        id: i64,
    ) -> Resultado<Vec<crate::models::Esquema>> {
        let fuente = self.obtener(ctx, id)?;
        drivers::esquemas(&fuente).await
    }

    /// Devuelve la fuente con la contrasena real (uso interno del nucleo).
    pub fn con_secreto(&self, ctx: &Contexto, id: i64) -> Resultado<Fuente> {
        self.obtener(ctx, id)
    }
}

fn tipo_txt(t: TipoFuente) -> &'static str {
    match t {
        TipoFuente::Sqlite => "sqlite",
        TipoFuente::Postgres => "postgres",
        TipoFuente::Mysql => "mysql",
        TipoFuente::SqlServer => "sqlserver",
        TipoFuente::MongoDb => "mongodb",
        TipoFuente::Csv => "csv",
        TipoFuente::Json => "json",
        TipoFuente::Xml => "xml",
        TipoFuente::Excel => "excel",
        TipoFuente::Web => "web",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::ServicioAuth;
    use crate::models::Credenciales;

    #[test]
    fn valida_ruta_de_fichero_inexistente() {
        let mut c = ConfigFuente::default();
        c.ruta = "/no/existe/este/fichero.csv".into();
        let r = ServicioFuentes::validar_config(TipoFuente::Csv, &c);
        assert!(r.is_err());
    }

    #[test]
    fn valida_url_web() {
        let mut c = ConfigFuente::default();
        c.ruta = "no-es-una-url".into();
        assert!(ServicioFuentes::validar_config(TipoFuente::Web, &c).is_err());
        c.ruta = "https://ejemplo.es/datos.csv".into();
        assert!(ServicioFuentes::validar_config(TipoFuente::Web, &c).is_ok());
    }

    #[test]
    fn exige_campos_de_servidor() {
        let c = ConfigFuente::default();
        assert!(ServicioFuentes::validar_config(TipoFuente::Postgres, &c).is_err());
    }

    #[tokio::test]
    async fn crea_fuente_para_usuario_autorizado() {
        let db = Arc::new(AppDb::en_memoria().unwrap());
        let auth = ServicioAuth::nuevo(db.clone(), "1.0.0");
        let sesion = auth
            .iniciar_sesion(&Credenciales {
                usuario: "JMBernabeu".into(),
                contrasena: crate::auth::CONTRASENA_ADMIN_DEFECTO.into(),
            })
            .unwrap();
        let fuentes = ServicioFuentes::nuevo(db);
        let ctx = auth.contexto(&sesion.token).unwrap();
        let solicitud = SolicitudFuente {
            nombre: "Ventas locales".into(),
            descripcion: String::new(),
            tipo: "web".into(),
            config: ConfigFuente {
                ruta: "https://ejemplo.es/ventas.csv".into(),
                ..drivers::config_ejemplo(TipoFuente::Web)
            },
            activa: true,
            solo_admin: false,
            conservar_secreto: false,
        };
        let creada = fuentes.crear(&ctx, &solicitud).unwrap();
        assert_eq!(creada.nombre, "Ventas locales");
        assert_eq!(creada.tipo, TipoFuente::Web);
        // La clave nunca sale de la base de datos hacia la interfaz.
        assert!(creada.config.contrasena.is_empty());
    }
}

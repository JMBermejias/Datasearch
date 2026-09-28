//! Servicio de autenticacion, usuarios, sesiones y permisos.
//!
//! Copyright (c) 2026 Jose Manuel Bernabeu Mejias
//! Licencia MIT

use crate::appdb::AppDb;
use crate::error::{Error, Resultado};
use crate::models::{
    Credenciales, DatosPersonales, Estado, Permisos, Rol, Sesion, SolicitudRegistro, Usuario,
    USUARIO_ADMIN_DEFECTO,
};
use argon2::password_hash::{
    rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
};
use argon2::Argon2;
use rand::RngCore;
use std::sync::Arc;

/// Horas de validez de una sesion.
pub const HORAS_SESION: i64 = 12;

/// Contrasena del administrador creada en la primera ejecucion.
pub const CONTRASENA_ADMIN_DEFECTO: &str = "@.,JMBT4m4r41971";

/// Estado de autorizacion de una peticion.
#[derive(Debug, Clone)]
pub struct Contexto {
    pub id_usuario: i64,
    pub usuario: String,
    pub rol: Rol,
    pub estado: Estado,
    pub permisos: Permisos,
    pub token: String,
}

impl Contexto {
    pub fn es_admin(&self) -> bool {
        self.rol.es_admin()
    }

    /// Comprueba un permiso concreto. Los administradores tienen todos.
    pub fn exigir(&self, p: Permiso) -> Resultado<()> {
        if self.es_admin() {
            return Ok(());
        }
        let concedido = match p {
            Permiso::Consultar => self.permisos.consultar,
            Permiso::CrearFuentes => self.permisos.crear_fuentes,
            Permiso::ModificarFuentes => self.permisos.modificar_fuentes,
            Permiso::EliminarFuentes => self.permisos.eliminar_fuentes,
            Permiso::Exportar => self.permisos.exportar,
            Permiso::Imprimir => self.permisos.imprimir,
            Permiso::Analizar => self.permisos.analizar,
            Permiso::CrearDashboards => self.permisos.crear_dashboards,
            Permiso::VerAuditoria => self.permisos.ver_auditoria,
        };
        if concedido {
            Ok(())
        } else {
            Err(Error::PermisoDenegado(format!(
                "Su cuenta no tiene permiso para: {}",
                p.etiqueta()
            )))
        }
    }
}

/// Permiso asignable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permiso {
    Consultar,
    CrearFuentes,
    ModificarFuentes,
    EliminarFuentes,
    Exportar,
    Imprimir,
    Analizar,
    CrearDashboards,
    VerAuditoria,
}

impl Permiso {
    pub fn etiqueta(&self) -> &'static str {
        match self {
            Permiso::Consultar => "consultar datos",
            Permiso::CrearFuentes => "crear fuentes de datos",
            Permiso::ModificarFuentes => "modificar fuentes de datos",
            Permiso::EliminarFuentes => "eliminar fuentes de datos",
            Permiso::Exportar => "exportar resultados",
            Permiso::Imprimir => "imprimir resultados",
            Permiso::Analizar => "analizar datos",
            Permiso::CrearDashboards => "crear tableros de control",
            Permiso::VerAuditoria => "ver la auditoria",
        }
    }
}

/// Servicio de autenticacion. Se comparte entre comandos mediante `Arc`.
pub struct ServicioAuth {
    db: Arc<AppDb>,
    version_app: String,
}

impl ServicioAuth {
    pub fn nuevo(db: Arc<AppDb>, version_app: &str) -> Self {
        let s = Self {
            db,
            version_app: version_app.to_string(),
        };
        s.asegurar_admin_por_defecto();
        s
    }

    pub fn db(&self) -> &Arc<AppDb> {
        &self.db
    }

    /// Crea el administrador por defecto si todavia no existe.
    /// Idempotente: solo actua la primera vez.
    pub fn asegurar_admin_por_defecto(&self) {
        if self
            .db
            .usuario_por_nombre(USUARIO_ADMIN_DEFECTO)
            .ok()
            .flatten()
            .is_some()
        {
            return;
        }
        let datos = DatosPersonales {
            nombre: "Jose Manuel".into(),
            apellidos: "Bernabeu Mejias".into(),
            documento: String::new(),
            email: crate::models::AUTOR_CORREO.into(),
            telefono: String::new(),
            direccion: crate::models::DIRECCION_AUTOR.into(),
            numero: "2 C".into(),
            codigo_postal: crate::models::CODIGO_POSTAL_AUTOR.into(),
            poblacion: crate::models::POBLACION_AUTOR.into(),
            provincia: crate::models::PROVINCIA_AUTOR.into(),
            pais: crate::models::PAIS_AUTOR.into(),
            fecha_nacimiento: String::new(),
            empresa: String::new(),
            cargo: "Administrador de DataSearch".into(),
            motivo_solicitud: "Cuenta de propiedad del autor de la aplicacion".into(),
        };
        let hash = Self::hashear(&CONTRASENA_ADMIN_DEFECTO).unwrap_or_default();
        match self.db.insertar_usuario(
            USUARIO_ADMIN_DEFECTO,
            &hash,
            Rol::Administrador,
            Estado::Activo,
            Permisos::default().a_mascara(),
            &datos,
        ) {
            Ok(id) => {
                self.db
                    .auditar(id, "arranque", "Administrador por defecto creado", true);
            }
            Err(e) => eprintln!("[DataSearch] No se pudo crear el administrador por defecto: {e}"),
        }
    }

    // ------------------------------------------------------------------
    // Contrasenas
    // ------------------------------------------------------------------

    /// Genera un hash Argon2id con sal aleatoria.
    pub fn hashear(contrasena: &str) -> Resultado<String> {
        let sal = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(contrasena.as_bytes(), &sal)
            .map_err(|e| Error::Interno(format!("No se pudo cifrar la contrasena: {e}")))?;
        Ok(hash.to_string())
    }

    /// Comprueba una contrasena contra su hash.
    pub fn verificar(contrasena: &str, hash: &str) -> bool {
        match PasswordHash::new(hash) {
            Ok(parseado) => Argon2::default()
                .verify_password(contrasena.as_bytes(), &parseado)
                .is_ok(),
            Err(_) => false,
        }
    }

    /// Valida la robustez minima de una contrasena.
    pub fn validar_contrasena(c: &str) -> Resultado<()> {
        if c.chars().count() < 8 {
            return Err(Error::Validacion(
                "La contrasena debe tener al menos 8 caracteres".into(),
            ));
        }
        if c.chars().count() > 200 {
            return Err(Error::Validacion("La contrasena es demasiado larga".into()));
        }
        let tiene_numero = c.chars().any(|x| x.is_ascii_digit());
        let tiene_letra = c.chars().any(|x| x.is_alphabetic());
        if !tiene_numero || !tiene_letra {
            return Err(Error::Validacion(
                "La contrasena debe combinar al menos letras y numeros".into(),
            ));
        }
        Ok(())
    }

    // ------------------------------------------------------------------
    // Registro y acceso
    // ------------------------------------------------------------------

    /// Registra un usuario nuevo. Nace en estado `pendiente` hasta que el
    /// administrador lo verifique, salvo que se marque como autoaprobado.
    pub fn registrar(
        &self,
        solicitud: &SolicitudRegistro,
        autoaprobar: bool,
    ) -> Resultado<Usuario> {
        let usuario = solicitud.usuario.trim();
        if usuario.len() < 3 {
            return Err(Error::Validacion(
                "El nombre de usuario debe tener al menos 3 caracteres".into(),
            ));
        }
        if !usuario
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-')
        {
            return Err(Error::Validacion(
                "El nombre de usuario solo admite letras, numeros, punto, guion y guion bajo"
                    .into(),
            ));
        }
        if let Some(existente) = self.db.usuario_por_nombre(usuario)? {
            if existente.0 > 0 {
                return Err(Error::UsuarioDuplicado);
            }
        }
        Self::validar_contrasena(&solicitud.contrasena)?;
        if !solicitud.repetir_contrasena.trim().is_empty()
            && solicitud.repetir_contrasena != solicitud.contrasena
        {
            return Err(Error::Validacion("Las contrasenas no coinciden".into()));
        }
        let faltan = solicitud.datos.validar();
        if !faltan.is_empty() {
            return Err(Error::DatosIncompletos(faltan.join(", ")));
        }

        let hash = Self::hashear(&solicitud.contrasena)?;
        let estado = if autoaprobar {
            Estado::Activo
        } else {
            Estado::Pendiente
        };
        let id = self.db.insertar_usuario(
            usuario,
            &hash,
            Rol::Usuario,
            estado,
            Permisos::default().a_mascara(),
            &solicitud.datos,
        )?;
        self.db.auditar(
            id,
            "registro",
            &format!("Usuario {usuario} registrado (estado: {estado:?})"),
            true,
        );
        self.db
            .usuario_por_id(id)?
            .ok_or(Error::Interno("No se pudo leer el usuario creado".into()))
    }

    /// Comprueba si un nombre de usuario esta disponible.
    pub fn usuario_disponible(&self, usuario: &str) -> bool {
        matches!(self.db.usuario_por_nombre(usuario), Ok(None))
    }

    /// Inicia sesion con usuario y contrasena.
    pub fn iniciar_sesion(&self, cred: &Credenciales) -> Resultado<Sesion> {
        let (id, hash, _rol) = self
            .db
            .usuario_por_nombre(cred.usuario.trim())?
            .ok_or(Error::CredencialesInvalidas)?;

        if !Self::verificar(&cred.contrasena, &hash) {
            self.db
                .auditar(id, "acceso", "Contrasena incorrecta", false);
            return Err(Error::CredencialesInvalidas);
        }

        let usuario = self
            .db
            .usuario_por_id(id)?
            .ok_or(Error::CredencialesInvalidas)?;

        match usuario.estado {
            Estado::Pendiente => {
                self.db
                    .auditar(id, "acceso", "Cuenta pendiente de verificacion", false);
                return Err(Error::CuentaPendiente);
            }
            Estado::Suspendido => {
                self.db.auditar(
                    id,
                    "acceso",
                    "Cuenta suspendida por el administrador",
                    false,
                );
                return Err(Error::CuentaSuspendida);
            }
            Estado::Activo => {}
        }

        let token = Self::nuevo_token();
        self.db.crear_sesion(&token, id, HORAS_SESION)?;
        self.db.marcar_acceso(id)?;
        self.db
            .auditar(id, "acceso", "Inicio de sesion correcto", true);

        let usuario = self
            .db
            .usuario_por_id(id)?
            .ok_or(Error::Interno("No se pudo leer el usuario".into()))?;

        Ok(Sesion {
            token,
            es_admin: usuario.rol.es_admin(),
            version_app: self.version_app.clone(),
            usuario,
        })
    }

    /// Cierra la sesion indicada.
    pub fn cerrar_sesion(&self, token: &str) -> Resultado<()> {
        self.db.cerrar_sesion(token)?;
        self.db
            .auditar(0, "cierre_sesion", "Sesion cerrada por el usuario", true);
        Ok(())
    }

    /// Recupera el contexto autorizado a partir de un token.
    pub fn contexto(&self, token: &str) -> Resultado<Contexto> {
        if token.trim().is_empty() {
            return Err(Error::SinSesion);
        }
        let id = self
            .db
            .usuario_de_sesion(token)?
            .ok_or(Error::SesionCaducada)?;
        let u = self.db.usuario_por_id(id)?.ok_or(Error::SesionCaducada)?;
        if u.estado == Estado::Suspendido {
            // Corta las sesiones abiertas de una cuenta suspendida.
            self.db.cerrar_sesiones_usuario(id)?;
            return Err(Error::CuentaSuspendida);
        }
        Ok(Contexto {
            id_usuario: u.id,
            usuario: u.usuario.clone(),
            rol: u.rol,
            estado: u.estado,
            permisos: u.permisos,
            token: token.to_string(),
        })
    }

    /// Usuario asociado a un token valido.
    pub fn usuario_de_token(&self, token: &str) -> Resultado<Usuario> {
        let ctx = self.contexto(token)?;
        self.db
            .usuario_por_id(ctx.id_usuario)?
            .ok_or(Error::SesionCaducada)
    }

    /// Cambia la contrasena del usuario indicado tras comprobar la anterior.
    pub fn cambiar_contrasena(&self, token: &str, actual: &str, nueva: &str) -> Resultado<()> {
        let ctx = self.contexto(token)?;
        let (_, hash, _) = self
            .db
            .usuario_por_nombre(&ctx.usuario)?
            .ok_or(Error::UsuarioNoExiste)?;
        if !Self::verificar(actual, &hash) {
            self.db.auditar(
                ctx.id_usuario,
                "cambio_contrasena",
                "Contrasena actual incorrecta",
                false,
            );
            return Err(Error::CredencialesInvalidas);
        }
        Self::validar_contrasena(nueva)?;
        let nuevo_hash = Self::hashear(nueva)?;
        self.db.actualizar_contrasena(ctx.id_usuario, &nuevo_hash)?;
        self.db.auditar(
            ctx.id_usuario,
            "cambio_contrasena",
            "Contrasena actualizada",
            true,
        );
        Ok(())
    }

    // ------------------------------------------------------------------
    // Administracion
    // ------------------------------------------------------------------

    /// Verifica, suspende o reactiva una cuenta y asigna rol y permisos.
    pub fn admin_actualizar_usuario(
        &self,
        token_admin: &str,
        id_objetivo: i64,
        estado: Estado,
        rol: Rol,
        permisos: Permisos,
        notas: &str,
    ) -> Resultado<Usuario> {
        let ctx = self.contexto(token_admin)?;
        if !ctx.es_admin() {
            return Err(Error::PermisoDenegado(
                "Solo un administrador puede modificar cuentas".into(),
            ));
        }
        let objetivo = self
            .db
            .usuario_por_id(id_objetivo)?
            .ok_or(Error::UsuarioNoExiste)?;
        if objetivo.id == ctx.id_usuario && estado == Estado::Suspendido {
            return Err(Error::PermisoDenegado(
                "El administrador no puede suspender su propia cuenta".into(),
            ));
        }
        // Evita quedarse sin ningun administrador operativo.
        if objetivo.rol.es_admin() && (!rol.es_admin() || estado != Estado::Activo) {
            let otros_activos: Vec<Usuario> =
                self.db
                    .listar_usuarios(Some(Estado::Activo), "", Some(Rol::Administrador))?;
            if otros_activos.iter().all(|u| u.id == objetivo.id) {
                return Err(Error::PermisoDenegado(
                    "Debe existir al menos un administrador activo".into(),
                ));
            }
        }

        self.db
            .actualizar_usuario_admin(id_objetivo, estado, rol, permisos.a_mascara(), notas)?;
        if estado != Estado::Activo {
            self.db.cerrar_sesiones_usuario(id_objetivo)?;
        }
        self.db.auditar(
            ctx.id_usuario,
            "admin_actualiza_usuario",
            &format!(
                "Usuario {} -> estado {:?}, rol {:?}, permisos {}",
                objetivo.usuario,
                estado,
                rol,
                permisos.a_mascara()
            ),
            true,
        );
        self.db
            .usuario_por_id(id_objetivo)?
            .ok_or(Error::UsuarioNoExiste)
    }

    /// Elimina definitivamente una cuenta.
    pub fn admin_eliminar_usuario(&self, token_admin: &str, id_objetivo: i64) -> Resultado<()> {
        let ctx = self.contexto(token_admin)?;
        if !ctx.es_admin() {
            return Err(Error::PermisoDenegado(
                "Solo un administrador puede eliminar cuentas".into(),
            ));
        }
        if id_objetivo == ctx.id_usuario {
            return Err(Error::PermisoDenegado(
                "El administrador no puede eliminarse a si mismo".into(),
            ));
        }
        let objetivo = self
            .db
            .usuario_por_id(id_objetivo)?
            .ok_or(Error::UsuarioNoExiste)?;
        self.db.eliminar_usuario(id_objetivo)?;
        self.db.auditar(
            ctx.id_usuario,
            "admin_elimina_usuario",
            &format!("Usuario {} eliminado", objetivo.usuario),
            true,
        );
        Ok(())
    }

    /// Restablece la contrasena de un usuario a un valor elegido por el admin.
    pub fn admin_restablecer_contrasena(
        &self,
        token_admin: &str,
        id_objetivo: i64,
        nueva: &str,
    ) -> Resultado<()> {
        let ctx = self.contexto(token_admin)?;
        if !ctx.es_admin() {
            return Err(Error::PermisoDenegado(
                "Solo un administrador puede restablecer contrasenas".into(),
            ));
        }
        Self::validar_contrasena(nueva)?;
        let objetivo = self
            .db
            .usuario_por_id(id_objetivo)?
            .ok_or(Error::UsuarioNoExiste)?;
        let hash = Self::hashear(nueva)?;
        self.db.actualizar_contrasena(id_objetivo, &hash)?;
        self.db.cerrar_sesiones_usuario(id_objetivo)?;
        self.db.auditar(
            ctx.id_usuario,
            "admin_restablece_contrasena",
            &format!("Contrasena restablecida para {}", objetivo.usuario),
            true,
        );
        Ok(())
    }

    /// Lista usuarios visibles para el administrador.
    pub fn admin_listar_usuarios(
        &self,
        token_admin: &str,
        estado: Option<Estado>,
        texto: &str,
    ) -> Resultado<Vec<Usuario>> {
        let ctx = self.contexto(token_admin)?;
        if !ctx.es_admin() {
            return Err(Error::PermisoDenegado(
                "Solo un administrador puede ver el listado completo de usuarios".into(),
            ));
        }
        self.db.listar_usuarios(estado, texto, None)
    }

    /// Detalles de un usuario para el administrador (datos y actividad).
    pub fn admin_detalle_usuario(
        &self,
        token_admin: &str,
        id_objetivo: i64,
    ) -> Resultado<serde_json::Value> {
        let ctx = self.contexto(token_admin)?;
        if !ctx.es_admin() {
            return Err(Error::PermisoDenegado(
                "Solo un administrador puede consultar datos de otros usuarios".into(),
            ));
        }
        let u = self
            .db
            .usuario_por_id(id_objetivo)?
            .ok_or(Error::UsuarioNoExiste)?;
        let fuentes = self.db.listar_fuentes(id_objetivo, true)?;
        let auditoria = self.db.auditoria_de_usuario(id_objetivo, 100)?;
        Ok(serde_json::json!({
            "usuario": u,
            "fuentes": fuentes,
            "auditoria": auditoria,
            "conteoFuentes": fuentes.len(),
            "sesionesActivas": self.db.conteo_sesiones(id_objetivo)?,
        }))
    }

    /// Numero total de cuentas y cuentas pendientes (panel del administrador).
    pub fn admin_resumen(&self, token_admin: &str) -> Resultado<serde_json::Value> {
        let ctx = self.contexto(token_admin)?;
        if !ctx.es_admin() {
            return Err(Error::PermisoDenegado(
                "Solo un administrador puede ver el resumen general".into(),
            ));
        }
        let todos = self.db.listar_usuarios(None, "", None)?;
        let por_estado = |e: Estado| todos.iter().filter(|u| u.estado == e).count();
        Ok(serde_json::json!({
            "totalUsuarios": todos.len(),
            "pendientes": por_estado(Estado::Pendiente),
            "activos": por_estado(Estado::Activo),
            "suspendidos": por_estado(Estado::Suspendido),
            "administradores": todos.iter().filter(|u| u.rol.es_admin()).count(),
            "fuentesPorUsuario": self.db.conteo_fuentes_por_usuario()?,
            "ultimaAuditoria": self.db.listar_auditoria(25)?,
        }))
    }

    /// Token aleatorio criptograficamente seguro.
    pub fn nuevo_token() -> String {
        let mut bytes = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut bytes);
        hex::encode(bytes)
    }
}

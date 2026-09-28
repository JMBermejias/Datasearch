//! Base de datos interna de la aplicacion (SQLite).
//!
//! Contiene usuarios, sesiones, fuentes registradas, permisos y auditoria.
//! No guarda jamas contrasenas en claro: se almacenan con Argon2id.
//!
//! Copyright (c) 2026 Jose Manuel Bernabeu Mejias
//! Licencia MIT

use crate::error::{Error, Resultado};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

/// Version del esquema interno. Se incrementa al anadir migraciones.
const VERSION_ESQUEMA: i64 = 1;

/// Conexion a la base de datos de la aplicacion.
///
/// `rusqlite::Connection` no implementa `Sync`, de modo que se protege con un
/// mutex. Gracias a eso `AppDb` es `Send + Sync` y puede compartirse entre los
/// comandos IPC de Tauri sin varios hilos a la vez.
pub struct AppDb {
    conn: Mutex<Connection>,
}

impl AppDb {
    /// Abre (creando si hace falta) la base de datos de la aplicacion.
    pub fn abrir(ruta: &Path) -> Resultado<Self> {
        if let Some(dir) = ruta.parent() {
            if !dir.as_os_str().is_empty() {
                std::fs::create_dir_all(dir)?;
            }
        }
        let conn = Connection::open(ruta)?;
        Self::preparar(conn)
    }

    /// Base de datos en memoria, para pruebas.
    pub fn en_memoria() -> Resultado<Self> {
        let conn = Connection::open_in_memory()?;
        Self::preparar(conn)
    }

    fn preparar(conn: Connection) -> Resultado<Self> {
        conn.pragma_update(None, "journal_mode", "WAL").ok();
        conn.pragma_update(None, "foreign_keys", "ON").ok();
        conn.pragma_update(None, "busy_timeout", 5000).ok();
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.migrar()?;
        Ok(db)
    }

    /// Bloquea la conexion durante una operacion.
    ///
    /// Si un hilo previo dejo el mutex envenenado por un error, se recupera de
    /// todos modos: la base abre en modo WAL y tolera que una sentencia falle.
    pub fn c(&self) -> MutexGuard<'_, Connection> {
        match self.conn.lock() {
            Ok(g) => g,
            Err(envenenado) => envenenado.into_inner(),
        }
    }

    /// Acceso a la conexion, para las consultas prepared.
    pub fn conexion(&self) -> MutexGuard<'_, Connection> {
        self.c()
    }

    /// Aplica el esquema. Es idempotente, asi que puede llamarse al arrancar.
    pub fn migrar(&self) -> Resultado<()> {
        let conn = self.c();
        conn.execute_batch(SCHEMA)?;
        let actual: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap_or(0);
        if actual < VERSION_ESQUEMA {
            conn.pragma_update(None, "user_version", VERSION_ESQUEMA)?;
        }
        Ok(())
    }

    // ------------------------------------------------------------------
    // Usuarios
    // ------------------------------------------------------------------

    /// Inserta un usuario nuevo. Devuelve el id generado.
    #[allow(clippy::too_many_arguments)]
    pub fn insertar_usuario(
        &self,
        usuario: &str,
        hash: &str,
        rol: crate::models::Rol,
        estado: crate::models::Estado,
        mascara_permisos: i64,
        datos: &crate::models::DatosPersonales,
    ) -> Resultado<i64> {
        let ahora = ahora_iso();
        let conn = self.c();
        conn.execute(
            "INSERT INTO usuarios (
                usuario, contrasena_hash, rol, estado, permisos, notas_admin,
                nombre, apellidos, documento, email, telefono, direccion, numero,
                codigo_postal, poblacion, provincia, pais, fecha_nacimiento,
                empresa, cargo, creado_en
             ) VALUES (?1,?2,?3,?4,?5,'',?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20)",
            params![
                usuario,
                hash,
                rol_txt(rol),
                estado_txt(estado),
                mascara_permisos,
                datos.nombre,
                datos.apellidos,
                datos.documento,
                datos.email,
                datos.telefono,
                datos.direccion,
                datos.numero,
                datos.codigo_postal,
                datos.poblacion,
                datos.provincia,
                datos.pais,
                datos.fecha_nacimiento,
                datos.empresa,
                datos.cargo,
                ahora,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Busca un usuario por nombre de usuario, sin distinguir mayusculas.
    /// Devuelve `(id, hash, rol)`.
    pub fn usuario_por_nombre(&self, usuario: &str) -> Resultado<Option<(i64, String, String)>> {
        let conn = self.c();
        let mut stmt = conn.prepare(
            "SELECT id, contrasena_hash, rol FROM usuarios WHERE usuario = ?1 COLLATE NOCASE",
        )?;
        let fila = stmt
            .query_row(params![usuario], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })
            .optional()?;
        Ok(fila)
    }

    /// Carga el usuario completo por id.
    pub fn usuario_por_id(&self, id: i64) -> Resultado<Option<crate::models::Usuario>> {
        let conn = self.c();
        let sql = format!("SELECT {CAMPOS_USUARIO} FROM usuarios WHERE id = ?1");
        let mut stmt = conn.prepare(&sql)?;
        let fila = stmt.query_row(params![id], fila_usuario).optional()?;
        Ok(fila)
    }

    /// Lista usuarios con filtro opcional por estado, texto libre y rol.
    pub fn listar_usuarios(
        &self,
        estado: Option<crate::models::Estado>,
        texto: &str,
        rol: Option<crate::models::Rol>,
    ) -> Resultado<Vec<crate::models::Usuario>> {
        let conn = self.c();
        let sql = format!(
            "SELECT {CAMPOS_USUARIO} FROM usuarios
             WHERE (?1 IS NULL OR estado = ?1)
               AND (?2 = '' OR usuario LIKE ?2 COLLATE NOCASE
                    OR nombre LIKE ?2 COLLATE NOCASE
                    OR apellidos LIKE ?2 COLLATE NOCASE
                    OR email LIKE ?2 COLLATE NOCASE)
               AND (?3 IS NULL OR rol = ?3)
             ORDER BY creado_en DESC"
        );
        let texto = if texto.trim().is_empty() {
            String::new()
        } else {
            format!("%{}%", texto.trim())
        };
        let mut stmt = conn.prepare(&sql)?;
        let filas = stmt.query_map(
            params![estado.map(estado_txt), texto, rol.map(rol_txt)],
            fila_usuario,
        )?;
        let mut salida = Vec::new();
        for f in filas {
            salida.push(f?);
        }
        Ok(salida)
    }

    /// Actualiza estado, rol, permisos y notas del administrador.
    pub fn actualizar_usuario_admin(
        &self,
        id: i64,
        estado: crate::models::Estado,
        rol: crate::models::Rol,
        permisos: i64,
        notas: &str,
    ) -> Resultado<()> {
        let conn = self.c();
        conn.execute(
            "UPDATE usuarios SET estado = ?2, rol = ?3, permisos = ?4, notas_admin = ?5
             WHERE id = ?1",
            params![id, estado_txt(estado), rol_txt(rol), permisos, notas],
        )?;
        Ok(())
    }

    /// Cambia la contrasena de un usuario.
    pub fn actualizar_contrasena(&self, id: i64, hash: &str) -> Resultado<()> {
        let conn = self.c();
        conn.execute(
            "UPDATE usuarios SET contrasena_hash = ?2 WHERE id = ?1",
            params![id, hash],
        )?;
        Ok(())
    }

    /// Actualiza los datos personales de un usuario.
    pub fn actualizar_datos(&self, id: i64, d: &crate::models::DatosPersonales) -> Resultado<()> {
        let conn = self.c();
        conn.execute(
            "UPDATE usuarios SET nombre=?2, apellidos=?3, documento=?4, email=?5, telefono=?6,
                direccion=?7, numero=?8, codigo_postal=?9, poblacion=?10, provincia=?11,
                pais=?12, fecha_nacimiento=?13, empresa=?14, cargo=?15 WHERE id=?1",
            params![
                id,
                d.nombre,
                d.apellidos,
                d.documento,
                d.email,
                d.telefono,
                d.direccion,
                d.numero,
                d.codigo_postal,
                d.poblacion,
                d.provincia,
                d.pais,
                d.fecha_nacimiento,
                d.empresa,
                d.cargo
            ],
        )?;
        Ok(())
    }

    /// Elimina un usuario. Impide borrar al ultimo administrador.
    pub fn eliminar_usuario(&self, id: i64) -> Resultado<()> {
        let conn = self.c();
        let es_admin: i64 = conn.query_row(
            "SELECT COUNT(*) FROM usuarios WHERE id = ?1 AND rol = 'admin'",
            params![id],
            |r| r.get(0),
        )?;
        if es_admin > 0 {
            let otros: i64 = conn.query_row(
                "SELECT COUNT(*) FROM usuarios WHERE rol = 'admin' AND id <> ?1",
                params![id],
                |r| r.get(0),
            )?;
            if otros == 0 {
                return Err(Error::PermisoDenegado(
                    "No se puede eliminar el unico administrador del sistema".into(),
                ));
            }
        }
        conn.execute("DELETE FROM sesiones WHERE id_usuario = ?1", params![id])?;
        conn.execute("DELETE FROM usuarios WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Regista el acceso de un usuario.
    pub fn marcar_acceso(&self, id: i64) -> Resultado<()> {
        let conn = self.c();
        conn.execute(
            "UPDATE usuarios SET ultimo_acceso = ?2 WHERE id = ?1",
            params![id, ahora_iso()],
        )?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // Sesiones
    // ------------------------------------------------------------------

    /// Crea una sesion para un usuario.
    pub fn crear_sesion(&self, token: &str, id_usuario: i64, horas: i64) -> Resultado<()> {
        let caducidad = ahora_iso_mas_horas(horas);
        let conn = self.c();
        conn.execute(
            "INSERT INTO sesiones (token, id_usuario, creada_en, expira_en) VALUES (?1,?2,?3,?4)",
            params![token, id_usuario, ahora_iso(), caducidad],
        )?;
        Ok(())
    }

    /// Valida un token y devuelve el id de usuario asociado.
    pub fn usuario_de_sesion(&self, token: &str) -> Resultado<Option<i64>> {
        let conn = self.c();
        let mut stmt =
            conn.prepare("SELECT id_usuario FROM sesiones WHERE token = ?1 AND expira_en > ?2")?;
        let id = stmt
            .query_row(params![token, ahora_iso()], |r| r.get::<_, i64>(0))
            .optional()?;
        Ok(id)
    }

    /// Elimina una sesion concreta.
    pub fn cerrar_sesion(&self, token: &str) -> Resultado<()> {
        let conn = self.c();
        conn.execute("DELETE FROM sesiones WHERE token = ?1", params![token])?;
        Ok(())
    }

    /// Cierra todas las sesiones de un usuario.
    pub fn cerrar_sesiones_usuario(&self, id_usuario: i64) -> Resultado<()> {
        let conn = self.c();
        conn.execute(
            "DELETE FROM sesiones WHERE id_usuario = ?1",
            params![id_usuario],
        )?;
        Ok(())
    }

    /// Elimina las sesiones caducadas. Devuelve cuantas se han limpiado.
    pub fn purgar_sesiones(&self) -> Resultado<usize> {
        let conn = self.c();
        let n = conn.execute(
            "DELETE FROM sesiones WHERE expira_en <= ?1",
            params![ahora_iso()],
        )?;
        Ok(n)
    }

    /// Numero de sesiones activas de un usuario.
    pub fn conteo_sesiones(&self, id_usuario: i64) -> Resultado<i64> {
        let conn = self.c();
        let n = conn.query_row(
            "SELECT COUNT(*) FROM sesiones WHERE id_usuario = ?1",
            params![id_usuario],
            |r| r.get(0),
        )?;
        Ok(n)
    }

    // ------------------------------------------------------------------
    // Fuentes de datos
    // ------------------------------------------------------------------

    #[allow(clippy::too_many_arguments)]
    pub fn insertar_fuente(
        &self,
        id_usuario: i64,
        nombre: &str,
        descripcion: &str,
        tipo: &str,
        config_json: &str,
        activa: bool,
        solo_admin: bool,
    ) -> Resultado<i64> {
        let ahora = ahora_iso();
        let conn = self.c();
        conn.execute(
            "INSERT INTO fuentes (id_usuario, nombre, descripcion, tipo, config, activa,
                solo_admin, creado_en, actualizado_en) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![
                id_usuario,
                nombre,
                descripcion,
                tipo,
                config_json,
                b_i64(activa),
                b_i64(solo_admin),
                ahora,
                ahora
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn actualizar_fuente(
        &self,
        id: i64,
        nombre: &str,
        descripcion: &str,
        config_json: &str,
        activa: bool,
        solo_admin: bool,
    ) -> Resultado<()> {
        let conn = self.c();
        conn.execute(
            "UPDATE fuentes SET nombre=?2, descripcion=?3, config=?4, activa=?5,
                solo_admin=?6, actualizado_en=?7 WHERE id=?1",
            params![
                id,
                nombre,
                descripcion,
                config_json,
                b_i64(activa),
                b_i64(solo_admin),
                ahora_iso()
            ],
        )?;
        Ok(())
    }

    pub fn eliminar_fuente(&self, id: i64) -> Resultado<()> {
        let conn = self.c();
        conn.execute("DELETE FROM fuentes WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn fuente_por_id(&self, id: i64) -> Resultado<Option<crate::models::Fuente>> {
        let conn = self.c();
        let sql = format!("SELECT {CAMPOS_FUENTE} FROM fuentes WHERE id = ?1");
        let mut stmt = conn.prepare(&sql)?;
        let fila = stmt.query_row(params![id], fila_fuente).optional()?;
        Ok(fila)
    }

    /// Fuentes visibles para un usuario: las suyas y las globales activas.
    pub fn listar_fuentes(
        &self,
        id_usuario: i64,
        es_admin: bool,
    ) -> Resultado<Vec<crate::models::Fuente>> {
        let conn = self.c();
        let sql = if es_admin {
            format!("SELECT {CAMPOS_FUENTE} FROM fuentes ORDER BY nombre COLLATE NOCASE")
        } else {
            format!(
                "SELECT {CAMPOS_FUENTE} FROM fuentes
                 WHERE id_usuario = ?1 OR (solo_admin = 0 AND activa = 1)
                 ORDER BY nombre COLLATE NOCASE"
            )
        };
        let mut stmt = conn.prepare(&sql)?;
        let mapa = |r: &rusqlite::Row<'_>| fila_fuente(r);
        let filas = if es_admin {
            stmt.query_map([], mapa)?
        } else {
            stmt.query_map(params![id_usuario], mapa)?
        };
        let mut salida = Vec::new();
        for f in filas {
            salida.push(f?);
        }
        Ok(salida)
    }

    /// Comprueba si un usuario puede usar una fuente concreta.
    pub fn fuente_permitida(&self, id: i64, id_usuario: i64, es_admin: bool) -> Resultado<bool> {
        if es_admin {
            return Ok(true);
        }
        let conn = self.c();
        let n: i64 = conn.query_row(
            "SELECT COUNT(*) FROM fuentes
             WHERE id = ?1 AND (id_usuario = ?2 OR (solo_admin = 0 AND activa = 1))",
            params![id, id_usuario],
            |r| r.get(0),
        )?;
        Ok(n > 0)
    }

    /// Aplica un cambio de estado de conexion cacheado.
    pub fn marcar_estado_fuente(&self, id: i64, estado: &str) -> Resultado<()> {
        let conn = self.c();
        conn.execute(
            "UPDATE fuentes SET estado_conexion = ?2 WHERE id = ?1",
            params![id, estado],
        )?;
        Ok(())
    }

    /// Numero de fuentes registradas por cada usuario (vista del administrador).
    pub fn conteo_fuentes_por_usuario(&self) -> Resultado<Vec<(String, i64)>> {
        let conn = self.c();
        let mut stmt = conn.prepare(
            "SELECT usuario, COUNT(*) FROM fuentes GROUP BY usuario ORDER BY COUNT(*) DESC",
        )?;
        let filas = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
        let mut salida = Vec::new();
        for f in filas {
            salida.push(f?);
        }
        Ok(salida)
    }

    // ------------------------------------------------------------------
    // Auditoria
    // ------------------------------------------------------------------

    /// Registra una accion. Los fallos de escritura no interrumpen la operacion.
    pub fn auditar(&self, id_usuario: i64, accion: &str, detalle: &str, exito: bool) {
        if let Ok(conn) = self.conn.lock() {
            let _ = conn.execute(
                "INSERT INTO auditoria (id_usuario, accion, detalle, exito, fecha)
                 VALUES (?1,?2,?3,?4,?5)",
                params![id_usuario, accion, detalle, exito, ahora_iso()],
            );
        }
    }

    /// Ultimos registros de auditoria de todo el sistema.
    pub fn listar_auditoria(
        &self,
        limite: u32,
    ) -> Resultado<Vec<crate::models::RegistroAuditoria>> {
        self.auditoria_consulta(
            "SELECT a.id, a.id_usuario, COALESCE(u.usuario,'(eliminado)'), a.accion,
                    a.detalle, a.exito, a.fecha
             FROM auditoria a LEFT JOIN usuarios u ON u.id = a.id_usuario
             ORDER BY a.id DESC LIMIT ?1",
            limite,
            None,
        )
    }

    /// Registros de auditoria de un usuario concreto.
    pub fn auditoria_de_usuario(
        &self,
        id_usuario: i64,
        limite: u32,
    ) -> Resultado<Vec<crate::models::RegistroAuditoria>> {
        self.auditoria_consulta(
            "SELECT a.id, a.id_usuario, COALESCE(u.usuario,'(eliminado)'), a.accion,
                    a.detalle, a.exito, a.fecha
             FROM auditoria a LEFT JOIN usuarios u ON u.id = a.id_usuario
             WHERE a.id_usuario = ?2 ORDER BY a.id DESC LIMIT ?1",
            limite,
            Some(id_usuario),
        )
    }

    fn auditoria_consulta(
        &self,
        sql: &str,
        limite: u32,
        id_usuario: Option<i64>,
    ) -> Resultado<Vec<crate::models::RegistroAuditoria>> {
        let conn = self.c();
        let mut stmt = conn.prepare(sql)?;
        let mapa = |r: &rusqlite::Row<'_>| {
            Ok(crate::models::RegistroAuditoria {
                id: r.get(0)?,
                id_usuario: r.get(1)?,
                usuario: r.get(2)?,
                accion: r.get(3)?,
                detalle: r.get(4)?,
                exito: r.get(5)?,
                fecha: r.get(6)?,
            })
        };
        let filas = match id_usuario {
            Some(id) => stmt.query_map(params![limite, id], mapa)?,
            None => stmt.query_map(params![limite], mapa)?,
        };
        let mut salida = Vec::new();
        for f in filas {
            salida.push(f?);
        }
        Ok(salida)
    }

    // ------------------------------------------------------------------
    // Ajustes
    // ------------------------------------------------------------------

    /// Lee un ajuste por clave.
    pub fn ajuste(&self, clave: &str) -> Resultado<Option<String>> {
        let conn = self.c();
        let v = conn
            .query_row(
                "SELECT valor FROM ajustes WHERE clave = ?1",
                params![clave],
                |r| r.get::<_, String>(0),
            )
            .optional()?;
        Ok(v)
    }

    /// Guarda un ajuste.
    pub fn guardar_ajuste(&self, clave: &str, valor: &str) -> Resultado<()> {
        let conn = self.c();
        conn.execute(
            "INSERT INTO ajustes (clave, valor) VALUES (?1, ?2)
             ON CONFLICT(clave) DO UPDATE SET valor = excluded.valor",
            params![clave, valor],
        )?;
        Ok(())
    }
}

// ----------------------------------------------------------------------
// Utilidades internas
// ----------------------------------------------------------------------

/// SQLite no admite booleanos: se guardan como 0 y 1.
fn b_i64(v: bool) -> i64 {
    i64::from(v)
}

fn rol_txt(r: crate::models::Rol) -> &'static str {
    match r {
        crate::models::Rol::Administrador => "admin",
        crate::models::Rol::Usuario => "usuario",
    }
}

fn estado_txt(e: crate::models::Estado) -> &'static str {
    match e {
        crate::models::Estado::Pendiente => "pendiente",
        crate::models::Estado::Activo => "activo",
        crate::models::Estado::Suspendido => "suspendido",
    }
}

const CAMPOS_USUARIO: &str = "id, usuario, nombre, apellidos, documento, email, telefono,
    direccion, numero, codigo_postal, poblacion, provincia, pais, fecha_nacimiento,
    empresa, cargo, estado, rol, permisos, creado_en, ultimo_acceso, notas_admin";

fn fila_usuario(r: &rusqlite::Row<'_>) -> rusqlite::Result<crate::models::Usuario> {
    use crate::models::{Estado, Permisos, Rol, Usuario};
    let estado: String = r.get(16)?;
    let rol: String = r.get(17)?;
    let permisos: i64 = r.get(18)?;
    Ok(Usuario {
        id: r.get(0)?,
        usuario: r.get(1)?,
        nombre: r.get(2)?,
        apellidos: r.get(3)?,
        documento: r.get(4)?,
        email: r.get(5)?,
        telefono: r.get(6)?,
        direccion: r.get(7)?,
        numero: r.get(8)?,
        codigo_postal: r.get(9)?,
        poblacion: r.get(10)?,
        provincia: r.get(11)?,
        pais: r.get(12)?,
        fecha_nacimiento: r.get(13)?,
        empresa: r.get(14)?,
        cargo: r.get(15)?,
        estado: Estado::desde_txt(&estado),
        rol: Rol::desde_txt(&rol),
        permisos: Permisos::desde_mascara(permisos),
        creado_en: r.get(19)?,
        ultimo_acceso: r.get(20)?,
        notas_admin: r.get(21)?,
    })
}

const CAMPOS_FUENTE: &str = "id, id_usuario, nombre, descripcion, tipo, config, activa,
    solo_admin, creado_en, actualizado_en, estado_conexion";

fn fila_fuente(r: &rusqlite::Row<'_>) -> rusqlite::Result<crate::models::Fuente> {
    use crate::models::{ConfigFuente, Fuente, TipoFuente};
    let tipo: String = r.get(4)?;
    let config_txt: String = r.get(5)?;
    let config: ConfigFuente = serde_json::from_str(&config_txt).unwrap_or_default();
    Ok(Fuente {
        id: r.get(0)?,
        id_usuario: r.get(1)?,
        nombre: r.get(2)?,
        descripcion: r.get(3)?,
        tipo: TipoFuente::desde_txt(&tipo).unwrap_or(TipoFuente::Web),
        config,
        activa: r.get::<_, i64>(6)? != 0,
        solo_admin: r.get::<_, i64>(7)? != 0,
        creado_en: r.get(8)?,
        actualizado_en: r.get(9)?,
        estado_conexion: r.get(10)?,
    })
}

/// Fecha y hora local en ISO-8601 (formato que entiende SQLite y el frontend).
pub fn ahora_iso() -> String {
    time::OffsetDateTime::now_local()
        .unwrap_or_else(|_| time::OffsetDateTime::now_utc())
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| String::from("1970-01-01T00:00:00Z"))
}

/// Fecha y hora desplazada `horas` hacia el futuro, en ISO-8601.
pub fn ahora_iso_mas_horas(horas: i64) -> String {
    let f = time::OffsetDateTime::now_utc() + time::Duration::hours(horas);
    f.format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| String::from("2999-01-01T00:00:00Z"))
}

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS usuarios (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    usuario           TEXT    NOT NULL UNIQUE COLLATE NOCASE,
    contrasena_hash   TEXT    NOT NULL,
    rol               TEXT    NOT NULL DEFAULT 'usuario',
    estado            TEXT    NOT NULL DEFAULT 'pendiente',
    permisos          INTEGER NOT NULL DEFAULT 0,
    notas_admin       TEXT    NOT NULL DEFAULT '',
    nombre            TEXT    NOT NULL DEFAULT '',
    apellidos         TEXT    NOT NULL DEFAULT '',
    documento         TEXT    NOT NULL DEFAULT '',
    email             TEXT    NOT NULL DEFAULT '',
    telefono          TEXT    NOT NULL DEFAULT '',
    direccion         TEXT    NOT NULL DEFAULT '',
    numero            TEXT    NOT NULL DEFAULT '',
    codigo_postal     TEXT    NOT NULL DEFAULT '',
    poblacion         TEXT    NOT NULL DEFAULT '',
    provincia         TEXT    NOT NULL DEFAULT '',
    pais              TEXT    NOT NULL DEFAULT '',
    fecha_nacimiento  TEXT    NOT NULL DEFAULT '',
    empresa           TEXT    NOT NULL DEFAULT '',
    cargo             TEXT    NOT NULL DEFAULT '',
    creado_en         TEXT    NOT NULL,
    ultimo_acceso     TEXT
);

CREATE TABLE IF NOT EXISTS sesiones (
    token       TEXT PRIMARY KEY,
    id_usuario  INTEGER NOT NULL REFERENCES usuarios(id) ON DELETE CASCADE,
    creada_en   TEXT NOT NULL,
    expira_en   TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS fuentes (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    id_usuario        INTEGER NOT NULL REFERENCES usuarios(id) ON DELETE CASCADE,
    nombre            TEXT NOT NULL,
    descripcion       TEXT NOT NULL DEFAULT '',
    tipo              TEXT NOT NULL,
    config            TEXT NOT NULL DEFAULT '{}',
    activa            INTEGER NOT NULL DEFAULT 1,
    solo_admin        INTEGER NOT NULL DEFAULT 0,
    creado_en         TEXT NOT NULL,
    actualizado_en    TEXT NOT NULL,
    estado_conexion   TEXT NOT NULL DEFAULT 'sin_probar'
);

CREATE TABLE IF NOT EXISTS auditoria (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    id_usuario  INTEGER,
    accion      TEXT NOT NULL,
    detalle     TEXT NOT NULL DEFAULT '',
    exito       INTEGER NOT NULL DEFAULT 1,
    fecha       TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS ajustes (
    clave   TEXT PRIMARY KEY,
    valor   TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_sesiones_usuario ON sesiones(id_usuario);
CREATE INDEX IF NOT EXISTS idx_fuentes_usuario  ON fuentes(id_usuario);
CREATE INDEX IF NOT EXISTS idx_auditoria_fecha  ON auditoria(fecha DESC);
"#;

#[cfg(test)]
mod pruebas {
    use super::*;
    use std::sync::Arc;

    /// Los comandos IPC de Tauri exigen que el estado compartido sea
    /// `Send + Sync`. Este test lo fija para que no se rompa en silencio.
    #[test]
    fn appdb_es_send_y_sync() {
        fn exigir<T: Send + Sync>() {}
        exigir::<AppDb>();
        exigir::<Arc<AppDb>>();
    }

    #[test]
    fn el_administrador_por_defecto_se_crea_solo() {
        let db = AppDb::en_memoria().unwrap();
        let auth = crate::auth::ServicioAuth::nuevo(Arc::new(db), "1.0.0");
        let existe = auth.usuario_disponible(crate::models::USUARIO_ADMIN_DEFECTO);
        assert!(!existe, "el administrador por defecto debe existir");
        // La segunda llamada no debe duplicar la cuenta.
        auth.asegurar_admin_por_defecto();
        let db = auth.db();
        let total = db
            .listar_usuarios(None, "", None)
            .unwrap()
            .iter()
            .filter(|u| u.usuario == crate::models::USUARIO_ADMIN_DEFECTO)
            .count();
        assert_eq!(total, 1);
    }

    #[test]
    fn migra_varias_veces_sin_fallo() {
        let db = AppDb::en_memoria().unwrap();
        db.migrar().unwrap();
        db.migrar().unwrap();
    }
}

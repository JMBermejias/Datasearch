//! Conectores hacia los distintos origenes de datos.
//!
//! Cada conector expone la misma interfaz logica:
//!
//! * `esquemas`  -> que tablas o colecciones hay disponibles.
//! * `consultar` -> ejecuta la peticion del usuario y devuelve filas.
//! * `probar`    -> comprueba que la conexion funciona, sin traer datos.
//!
//! Copyright (c) 2026 Jose Manuel Bernabeu Mejias
//! Licencia MIT

use crate::error::{Error, Resultado};
use crate::models::Conector;
use crate::models::{
    BloqueResultados, ColumnaEsquema, ConfigFuente, Esquema, Fila, Fuente, PeticionBusqueda,
    TipoFuente, ValorCelda,
};
use crate::query::{self, Dialecto, Predicado};
use crate::util::{a_f64, aplanar_json, partir_ruta_json, Familia};
use futures_util::TryStreamExt;
use rusqlite::types::ValueRef;
use sqlx::{Column, Connection, Row, TypeInfo};
use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;
use tokio_util::compat::TokioAsyncWriteCompatExt;

/// Tope de seguridad: nunca se traen mas filas de las que el usuario pide.
pub const TOPE_FILAS: usize = 200_000;
/// Tope de tiempo de espera de red, en segundos.
pub const TOPE_TIMEOUT_SEG: u64 = 45;

fn tope_segundos(fuente: &Fuente) -> u64 {
    fuente
        .config
        .tiempo_espera_seg
        .unwrap_or(TOPE_TIMEOUT_SEG)
        .clamp(1, 600)
}

// ======================================================================
// Punto de entrada
// ======================================================================

/// Lista las tablas o colecciones de una fuente.
pub async fn esquemas(fuente: &Fuente) -> Resultado<Vec<Esquema>> {
    match fuente.tipo {
        TipoFuente::Sqlite => esquemas_sqlite(fuente),
        TipoFuente::Postgres => esquemas_postgres(fuente).await,
        TipoFuente::Mysql => esquemas_mysql(fuente).await,
        TipoFuente::SqlServer => esquemas_sqlserver(fuente).await,
        TipoFuente::MongoDb => esquemas_mongo(fuente).await,
        TipoFuente::Csv | TipoFuente::Json | TipoFuente::Xml | TipoFuente::Excel => {
            // Un fichero plano se presenta como un unico esquema.
            Ok(vec![Esquema {
                columnas: columnas_de_esquema_plano(fuente).await?,
                nombre: nombre_pseudo_esquema(fuente),
                tipo: fuente.tipo.etiqueta().to_string(),
                filas_estimadas: -1,
            }])
        }
        TipoFuente::Web => esquemas_web(fuente).await,
    }
}

/// Ejecuta la busqueda del usuario sobre una fuente.
///
/// Nada se consulta hasta que el usuario pulsa el boton de buscar: esta
/// funcion es la unica via por la que se leen datos de un origen.
pub async fn consultar(
    fuente: &Fuente,
    peticion: &PeticionBusqueda,
) -> Resultado<BloqueResultados> {
    let inicio = std::time::Instant::now();
    let mut bloque = match fuente.tipo {
        TipoFuente::Sqlite => consultar_sqlite(fuente, peticion)?,
        TipoFuente::Postgres => consultar_postgres(fuente, peticion).await?,
        TipoFuente::Mysql => consultar_mysql(fuente, peticion).await?,
        TipoFuente::SqlServer => consultar_sqlserver(fuente, peticion).await?,
        TipoFuente::MongoDb => consultar_mongo(fuente, peticion).await?,
        TipoFuente::Csv | TipoFuente::Json | TipoFuente::Xml | TipoFuente::Excel => {
            consultar_plano(fuente, peticion).await?
        }
        TipoFuente::Web => consultar_web(fuente, peticion).await?,
    };
    bloque.duracion_ms = inicio.elapsed().as_millis() as u64;
    Ok(bloque)
}

/// Comprueba que la fuente responde, sin traer datos de negocio.
pub async fn probar(fuente: &Fuente) -> Resultado<String> {
    match fuente.tipo {
        TipoFuente::Sqlite => {
            let conn = abrir_sqlite(fuente)?;
            let tablas: i64 = conn.query_row(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE type IN ('table','view') AND name NOT LIKE 'sqlite_%'",
                [],
                |r| r.get(0),
            )?;
            Ok(format!("Conexion correcta. {tablas} tablas disponibles."))
        }
        TipoFuente::Postgres => {
            let pool = pool_pg(fuente).await?;
            let fila: (String,) = sqlx::query_as("SELECT version()")
                .fetch_one(&pool)
                .await
                .map_err(|e| Error::Conexion(e.to_string()))?;
            pool.close().await;
            Ok(format!(
                "Conexion correcta. PostgreSQL: {}",
                fila.0.chars().take(70).collect::<String>()
            ))
        }
        TipoFuente::Mysql => {
            let mut conn = conectar_mysql(fuente).await?;
            let fila: (String,) = sqlx::query_as("SELECT version()")
                .fetch_one(&mut conn)
                .await
                .map_err(|e| Error::Conexion(e.to_string()))?;
            conn.close().await.ok();
            Ok(format!("Conexion correcta. MySQL/MariaDB: {}", fila.0))
        }
        TipoFuente::SqlServer => {
            ejecutar_tds(fuente, "SELECT 1 AS prueba").await?;
            Ok("Conexion correcta. Microsoft SQL Server.".into())
        }
        TipoFuente::MongoDb => {
            let db = cliente_mongo(fuente).await?;
            let nombres = db
                .list_collection_names()
                .await
                .map_err(|e| Error::Conexion(e.to_string()))?;
            Ok(format!(
                "Conexion correcta. {} colecciones disponibles.",
                nombres.len()
            ))
        }
        TipoFuente::Csv | TipoFuente::Json | TipoFuente::Xml | TipoFuente::Excel => {
            let filas = leer_plano(fuente).await?;
            Ok(format!(
                "Lectura correcta. {} registros y {} columnas.",
                filas.len(),
                query::columnas_de(&filas).len()
            ))
        }
        TipoFuente::Web => {
            let (bytes, ctype) = descargar_web(fuente).await?;
            Ok(format!(
                "Acceso correcto. {} bytes recibidos ({}).",
                bytes.len(),
                if ctype.is_empty() {
                    "tipo desconocido"
                } else {
                    &ctype
                }
            ))
        }
    }
}

/// Nombre logico del unico esquema de un fichero plano.
pub fn nombre_pseudo_esquema(fuente: &Fuente) -> String {
    let ruta = fuente.config.ruta.trim();
    if !ruta.is_empty() {
        if let Some(archivo) = ruta.rsplit('/').next() {
            if !archivo.is_empty() {
                return archivo.to_string();
            }
        }
    }
    fuente.nombre.clone()
}

// ======================================================================
// SQLite local
// ======================================================================

fn abrir_sqlite(fuente: &Fuente) -> Resultado<rusqlite::Connection> {
    let ruta = fuente.config.ruta.trim();
    if ruta.is_empty() {
        return Err(Error::Validacion(
            "Indique la ruta del fichero SQLite".into(),
        ));
    }
    let ruta = ruta.trim_start_matches("file:");
    let conn = rusqlite::Connection::open_with_flags(
        ruta,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
            | rusqlite::OpenFlags::SQLITE_OPEN_URI
            | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| Error::Conexion(format!("No se pudo abrir {ruta}: {e}")))?;
    let _ = conn.busy_timeout(std::time::Duration::from_secs(5));
    Ok(conn)
}

fn valor_sqlite(v: ValueRef<'_>) -> ValorCelda {
    match v {
        ValueRef::Null => None,
        ValueRef::Integer(i) => Some(i.to_string()),
        ValueRef::Real(f) => Some(f.to_string()),
        ValueRef::Text(t) => Some(String::from_utf8_lossy(t).into_owned()),
        ValueRef::Blob(b) => Some(format!("<binario: {} bytes>", b.len())),
    }
}

fn tipos_sqlite(conn: &rusqlite::Connection, tabla: &str) -> Vec<ColumnaEsquema> {
    let sql = format!("PRAGMA table_info({})", query::escapar_identificador(tabla));
    let Ok(mut stmt) = conn.prepare(&sql) else {
        return Vec::new();
    };
    let Ok(mapa) = stmt.query_map([], |r| {
        Ok(ColumnaEsquema {
            nombre: r.get::<_, String>(1)?,
            tipo: r.get::<_, Option<String>>(2)?.unwrap_or_default(),
            obligatorio: r.get::<_, Option<i64>>(3)?.unwrap_or(0) > 0,
            clave_primaria: r.get::<_, Option<i64>>(5)?.unwrap_or(0) > 0,
        })
    }) else {
        return Vec::new();
    };
    mapa.filter_map(|r| r.ok()).collect()
}

fn conteo_seguro(conn: &rusqlite::Connection, tabla: &str) -> i64 {
    let sql = format!(
        "SELECT COUNT(*) FROM {} LIMIT 1",
        query::escapar_identificador(tabla)
    );
    conn.query_row(&sql, [], |r| r.get(0)).unwrap_or(-1)
}

fn esquemas_sqlite(fuente: &Fuente) -> Resultado<Vec<Esquema>> {
    let conn = abrir_sqlite(fuente)?;
    let mut stmt = conn.prepare(
        "SELECT name, type FROM sqlite_master
         WHERE type IN ('table','view') AND name NOT LIKE 'sqlite_%'
         ORDER BY name",
    )?;
    let filas = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    let mut salida = Vec::new();
    for f in filas.flatten() {
        let (nombre, tipo) = f;
        salida.push(Esquema {
            columnas: tipos_sqlite(&conn, &nombre),
            filas_estimadas: conteo_seguro(&conn, &nombre),
            nombre,
            tipo,
        });
    }
    Ok(salida)
}

fn consultar_sqlite(fuente: &Fuente, peticion: &PeticionBusqueda) -> Resultado<BloqueResultados> {
    let conn = abrir_sqlite(fuente)?;
    let inicio = std::time::Instant::now();

    let esquemas_posibles: Vec<String> = if peticion.esquemas.is_empty() {
        esquemas_sqlite(fuente)?
            .into_iter()
            .map(|e| e.nombre)
            .collect()
    } else {
        peticion.esquemas.clone()
    };

    let mut filas: Vec<Fila> = Vec::new();
    let mut columnas: Vec<ColumnaEsquema> = Vec::new();
    let mut total = 0i64;
    let mut omitidas = 0u64;
    let mut esquemas_usados: Vec<String> = Vec::new();

    for esquema in &esquemas_posibles {
        let info_columnas = tipos_sqlite(&conn, esquema);
        let nombres_textuales: Vec<String> = info_columnas
            .iter()
            .filter(|c| {
                let t = c.tipo.to_uppercase();
                t.contains("CHAR") || t.contains("TEXT") || t.contains("CLOB") || t.is_empty()
            })
            .map(|c| c.nombre.clone())
            .collect();

        let (where_, params) = query::construir_where(
            &peticion.filtros,
            &peticion.texto,
            Conector::desde_txt(&peticion.modo),
            Dialecto::Sqlite,
            &nombres_textuales,
        )?;

        let seleccion = if peticion.campos.is_empty() {
            "*".to_string()
        } else {
            peticion
                .campos
                .iter()
                .map(|c| query::escapar_identificador(c))
                .collect::<Vec<_>>()
                .join(", ")
        };

        let mut sql = format!(
            "SELECT {seleccion} FROM {} {where_}",
            query::escapar_identificador(esquema)
        );
        if !peticion.ordenar_por.trim().is_empty() {
            sql.push_str(&format!(
                " ORDER BY {} {}",
                query::escapar_identificador(&peticion.ordenar_por),
                if peticion.orden_desc { "DESC" } else { "ASC" }
            ));
        }
        sql.push_str(&format!(
            " LIMIT {} OFFSET {}",
            peticion.limite.clamp(1, 100_000),
            peticion.desplazamiento
        ));

        let cajas: Vec<Box<dyn rusqlite::ToSql>> = params
            .iter()
            .map(|p| Box::new(p.clone()) as Box<dyn rusqlite::ToSql>)
            .collect();
        let enlazados: Vec<&dyn rusqlite::ToSql> = cajas.iter().map(|b| b.as_ref()).collect();

        let mut stmt = conn.prepare(&sql).map_err(|e| {
            Error::Consulta(format!("SQLite no pudo preparar la consulta: {e}\n{sql}"))
        })?;
        let nombres: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();

        let mapeadas = stmt
            .query_map(enlazados.as_slice(), |r| {
                let mut mapa = BTreeMap::new();
                for (i, nombre) in nombres.iter().enumerate() {
                    mapa.insert(nombre.clone(), valor_sqlite(r.get_ref(i)?));
                }
                Ok(Fila { valores: mapa })
            })
            .map_err(|e| Error::Consulta(e.to_string()))?;

        let mut leidas = 0usize;
        for f in mapeadas.flatten() {
            filas.push(f);
            leidas += 1;
            if leidas >= TOPE_FILAS {
                omitidas += 1;
                break;
            }
        }

        if peticion.contar_total && leidas > 0 {
            let sql_cuenta = format!(
                "SELECT COUNT(*) FROM {} {where_}",
                query::escapar_identificador(esquema)
            );
            if let Ok(n) = conn.query_row(&sql_cuenta, enlazados.as_slice(), |r| r.get::<_, i64>(0))
            {
                total += n;
            }
        } else {
            total += leidas as i64;
        }

        if columnas.is_empty() {
            columnas = info_columnas;
        }
        esquemas_usados.push(esquema.clone());
    }

    let descritas = query::describir_columnas(&filas, 20);
    let columnas = if descritas.is_empty() {
        columnas
            .into_iter()
            .map(|c| {
                let tipo = c.tipo.to_uppercase();
                let (es_fecha, es_booleano) = (tipo.contains("DATE"), tipo.contains("BOOL"));
                crate::models::ColumnaInfo {
                    etiqueta: c.nombre.clone(),
                    tipo: if tipo.is_empty() {
                        "texto".into()
                    } else {
                        tipo
                    },
                    es_texto: true,
                    es_fecha,
                    es_booleano,
                    ..Default::default()
                }
            })
            .collect()
    } else {
        descritas
    };

    Ok(BloqueResultados {
        fuente_id: fuente.id,
        fuente_nombre: fuente.nombre.clone(),
        tipo_fuente: fuente.tipo,
        esquema: esquemas_usados.join(", "),
        filas,
        columnas,
        total_filas: total,
        filas_omitidas: omitidas,
        duracion_ms: inicio.elapsed().as_millis() as u64,
        mensaje: String::new(),
    })
}

// ======================================================================
// PostgreSQL y MySQL mediante sqlx
// ======================================================================

fn puerto_por_defecto(tipo: &TipoFuente) -> u16 {
    match tipo {
        TipoFuente::Postgres => 5432,
        TipoFuente::Mysql => 3306,
        TipoFuente::MongoDb => 27017,
        TipoFuente::SqlServer => 1433,
        _ => 0,
    }
}

/// Construye la URL de conexion a partir de la configuracion del usuario.
fn url_sqlx(fuente: &Fuente) -> Resultado<String> {
    let c = &fuente.config;
    if c.host.trim().is_empty() {
        return Err(Error::Validacion("Indique el servidor".into()));
    }
    if c.base_datos.trim().is_empty() {
        return Err(Error::Validacion("Indique la base de datos".into()));
    }
    if c.usuario.trim().is_empty() {
        return Err(Error::Validacion("Indique el usuario".into()));
    }
    let puerto = c.puerto.unwrap_or_else(|| puerto_por_defecto(&fuente.tipo));
    let usuario = crate::util::escapar_url(&c.usuario);
    let clave = crate::util::escapar_url(&c.contrasena);
    let host = crate::util::escapar_url(&c.host);
    let base = crate::util::escapar_url(&c.base_datos);
    Ok(match fuente.tipo {
        TipoFuente::Postgres => {
            let ssl = if c.ssl { "?sslmode=require" } else { "" };
            format!("postgres://{usuario}:{clave}@{host}:{puerto}/{base}{ssl}")
        }
        _ => format!("mysql://{usuario}:{clave}@{host}:{puerto}/{base}"),
    })
}

async fn pool_pg(fuente: &Fuente) -> Resultado<sqlx::PgPool> {
    let url = url_sqlx(fuente)?;
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(3)
        .acquire_timeout(std::time::Duration::from_secs(tope_segundos(fuente)))
        .connect(&url)
        .await
        .map_err(|e| Error::Conexion(format!("No se pudo conectar a PostgreSQL: {e}")))
}

async fn conectar_mysql(fuente: &Fuente) -> Resultado<sqlx::MySqlConnection> {
    let url = url_sqlx(fuente)?;
    let opciones = sqlx::mysql::MySqlConnectOptions::from_str(&url)
        .map_err(|e| Error::Conexion(format!("Direccion no valida: {e}")))?;
    sqlx::Connection::connect_with(&opciones)
        .await
        .map_err(|e| Error::Conexion(format!("No se pudo conectar a MySQL/MariaDB: {e}")))
}

/// Nombre de tabla cualificado, respetando el esquema indicado.
fn nombre_tabla_calificada(fuente: &Fuente, tabla: &str) -> String {
    let base = match fuente.tipo {
        TipoFuente::Mysql => Dialecto::MySql.identificar(tabla),
        _ => Dialecto::Postgres.identificar(tabla),
    };
    let esquema = fuente.config.esquema.trim();
    if esquema.is_empty() {
        base
    } else {
        match fuente.tipo {
            TipoFuente::Mysql => {
                format!("{}.{}", Dialecto::MySql.identificar(esquema), base)
            }
            _ => format!("{}.{}", Dialecto::Postgres.identificar(esquema), base),
        }
    }
}

fn dialecto_de(tipo: TipoFuente) -> Dialecto {
    match tipo {
        TipoFuente::Postgres => Dialecto::Postgres,
        TipoFuente::Mysql => Dialecto::MySql,
        TipoFuente::SqlServer => Dialecto::SqlServer,
        _ => Dialecto::Sqlite,
    }
}

/// Convierte una fila de PostgreSQL en una `Fila` de DataSearch.
///
/// Se consulta el tipo declarado de cada columna para no perder precision ni
/// truncar cadenas Unicode o campos JSON.
fn fila_desde_pg(fila: &sqlx::postgres::PgRow) -> Fila {
    let mut mapa: BTreeMap<String, Option<String>> = BTreeMap::new();
    for (i, col) in fila.columns().iter().enumerate() {
        let tipo = col.type_info().name().to_uppercase();
        let texto: Option<Option<String>> = match tipo.as_str() {
            "BOOL" => fila
                .try_get::<Option<bool>, _>(i)
                .ok()
                .map(|v| v.map(|x| x.to_string())),
            "INT2" | "INT4" | "INT8" | "OID" => fila
                .try_get::<Option<i64>, _>(i)
                .ok()
                .map(|v| v.map(|x| x.to_string())),
            "FLOAT4" | "FLOAT8" => fila
                .try_get::<Option<f64>, _>(i)
                .ok()
                .map(|v| v.map(|x| x.to_string())),
            // JSON, UUID y DECIMAL se leen como texto para no perder fidelity.
            "BYTEA" => fila
                .try_get::<Option<Vec<u8>>, _>(i)
                .ok()
                .map(|v| v.map(|b| format!("<binario: {} bytes>", b.len()))),
            // NUMERIC y DECIMAL llegan como texto para no perder decimales.
            _ => fila.try_get::<Option<String>, _>(i).ok(),
        };
        mapa.insert(
            col.name().to_string(),
            texto.flatten().filter(|t| !t.is_empty()),
        );
    }
    Fila { valores: mapa }
}

fn fila_desde_mysql(fila: &sqlx::mysql::MySqlRow) -> Fila {
    let mut mapa: BTreeMap<String, Option<String>> = BTreeMap::new();
    for (i, col) in fila.columns().iter().enumerate() {
        let tipo = col.type_info().name().to_uppercase();
        let texto: Option<Option<String>> = match tipo.as_str() {
            "TINYINT" | "SMALLINT" | "INT" | "BIGINT" | "MEDIUMINT" | "YEAR" => fila
                .try_get::<Option<i64>, _>(i)
                .ok()
                .map(|v| v.map(|x| x.to_string())),
            "FLOAT" | "DOUBLE" => fila
                .try_get::<Option<f64>, _>(i)
                .ok()
                .map(|v| v.map(|x| x.to_string())),
            "BOOL" | "BOOLEAN" => fila
                .try_get::<Option<bool>, _>(i)
                .ok()
                .map(|v| v.map(|x| x.to_string())),
            "BLOB" | "BINARY" | "VARBINARY" | "TINYBLOB" | "MEDIUMBLOB" | "LONGBLOB" => fila
                .try_get::<Option<Vec<u8>>, _>(i)
                .ok()
                .map(|v| v.map(|b| format!("<binario: {} bytes>", b.len()))),
            _ => fila.try_get::<Option<String>, _>(i).ok(),
        };
        mapa.insert(
            col.name().to_string(),
            texto.flatten().filter(|t| !t.is_empty()),
        );
    }
    Fila { valores: mapa }
}

async fn columnas_pg(pool: &sqlx::PgPool, tabla: &str) -> Resultado<Vec<ColumnaEsquema>> {
    let sql = "SELECT column_name, data_type, is_nullable
               FROM information_schema.columns
               WHERE table_name = $1 AND table_schema = current_schema()
               ORDER BY ordinal_position";
    let filas = sqlx::query(sql)
        .bind(tabla)
        .fetch_all(pool)
        .await
        .map_err(|e| Error::Consulta(e.to_string()))?;
    Ok(filas
        .iter()
        .filter_map(|r| {
            Some(ColumnaEsquema {
                nombre: r.try_get::<String, _>(0).ok()?,
                tipo: r.try_get::<String, _>(1).unwrap_or_default(),
                obligatorio: r.try_get::<String, _>(2).ok()? == "NO",
                clave_primaria: false,
            })
        })
        .collect())
}

async fn esquemas_postgres(fuente: &Fuente) -> Resultado<Vec<Esquema>> {
    let pool = pool_pg(fuente).await?;
    let esquema_filtro = if fuente.config.esquema.trim().is_empty() {
        "current_schema()".to_string()
    } else {
        format!("'{}'", fuente.config.esquema.replace('\'', "''"))
    };
    let sql = format!(
        "SELECT table_name FROM information_schema.tables
         WHERE table_schema = {esquema_filtro} ORDER BY table_name"
    );
    let raws = sqlx::query(&sql)
        .fetch_all(&pool)
        .await
        .map_err(|e| Error::Consulta(e.to_string()))?;
    let nombres: Vec<String> = raws
        .iter()
        .filter_map(|r| r.try_get::<String, _>(0).ok())
        .collect();
    let mut salida = Vec::new();
    for n in nombres {
        let columnas = columnas_pg(&pool, &n).await?;
        salida.push(Esquema {
            nombre: n,
            tipo: "tabla".into(),
            filas_estimadas: -1,
            columnas,
        });
    }
    pool.close().await;
    Ok(salida)
}

async fn consultar_postgres(
    fuente: &Fuente,
    peticion: &PeticionBusqueda,
) -> Resultado<BloqueResultados> {
    let inicio = std::time::Instant::now();
    let pool = pool_pg(fuente).await?;
    let esquemas_posibles: Vec<String> = if peticion.esquemas.is_empty() {
        esquemas_postgres(fuente)
            .await?
            .into_iter()
            .map(|e| e.nombre)
            .collect()
    } else {
        peticion.esquemas.clone()
    };

    let mut filas: Vec<Fila> = Vec::new();
    let mut total = 0i64;

    for esquema in &esquemas_posibles {
        let columnas = columnas_pg(&pool, esquema).await?;
        let textuales: Vec<String> = columnas
            .iter()
            .filter(|c| {
                let t = c.tipo.to_uppercase();
                t.contains("CHAR") || t.contains("TEXT")
            })
            .map(|c| c.nombre.clone())
            .collect();

        let (where_, params) = query::construir_where(
            &peticion.filtros,
            &peticion.texto,
            Conector::desde_txt(&peticion.modo),
            Dialecto::Postgres,
            &textuales,
        )?;

        let tabla = nombre_tabla_calificada(fuente, esquema);
        let seleccion = if peticion.campos.is_empty() {
            "*".to_string()
        } else {
            peticion
                .campos
                .iter()
                .map(|c| Dialecto::Postgres.identificar(c))
                .collect::<Vec<_>>()
                .join(", ")
        };

        let mut sql = format!("SELECT {seleccion} FROM {tabla} {where_}");
        if !peticion.ordenar_por.trim().is_empty() {
            sql.push_str(&format!(
                " ORDER BY {} {}",
                Dialecto::Postgres.identificar(&peticion.ordenar_por),
                if peticion.orden_desc { "DESC" } else { "ASC" }
            ));
        }
        sql.push_str(&format!(
            " LIMIT {} OFFSET {}",
            peticion.limite.clamp(1, 100_000),
            peticion.desplazamiento
        ));

        let mut q = sqlx::query(&sql);
        for p in &params {
            q = q.bind(p);
        }
        let obtenidas = q
            .fetch_all(&pool)
            .await
            .map_err(|e| Error::Consulta(format!("{e}\nSQL: {sql}")))?;

        for r in obtenidas.iter().take(TOPE_FILAS) {
            filas.push(fila_desde_pg(r));
        }

        if peticion.contar_total {
            let sql_cuenta = format!("SELECT COUNT(*) FROM {tabla} {where_}");
            let mut qc = sqlx::query(&sql_cuenta);
            for p in &params {
                qc = qc.bind(p);
            }
            if let Ok(r) = qc.fetch_one(&pool).await {
                if let Ok(n) = r.try_get::<i64, _>(0) {
                    total += n;
                }
            }
        } else {
            total += obtenidas.len() as i64;
        }
    }
    pool.close().await;

    let omitidas = if filas.len() >= TOPE_FILAS { 1 } else { 0 };
    Ok(BloqueResultados {
        fuente_id: fuente.id,
        fuente_nombre: fuente.nombre.clone(),
        tipo_fuente: fuente.tipo,
        esquema: esquemas_posibles.join(", "),
        columnas: query::describir_columnas(&filas, 20),
        total_filas: total,
        filas_omitidas: omitidas,
        filas,
        duracion_ms: inicio.elapsed().as_millis() as u64,
        mensaje: String::new(),
    })
}

async fn columnas_mysql(fuente: &Fuente, tabla: &str) -> Resultado<Vec<ColumnaEsquema>> {
    let mut conn = conectar_mysql(fuente).await?;
    let sql = "SELECT column_name, data_type, is_nullable
               FROM information_schema.columns
               WHERE table_schema = DATABASE() AND table_name = ?
               ORDER BY ordinal_position";
    let filas = sqlx::query(sql)
        .bind(tabla)
        .fetch_all(&mut conn)
        .await
        .map_err(|e| Error::Consulta(e.to_string()))?;
    conn.close().await.ok();
    Ok(filas
        .iter()
        .filter_map(|r| {
            Some(ColumnaEsquema {
                nombre: r.try_get::<String, _>(0).ok()?,
                tipo: r.try_get::<String, _>(1).unwrap_or_default(),
                obligatorio: r.try_get::<String, _>(2).ok()? == "NO",
                clave_primaria: false,
            })
        })
        .collect())
}

async fn esquemas_mysql(fuente: &Fuente) -> Resultado<Vec<Esquema>> {
    let mut conn = conectar_mysql(fuente).await?;
    let raws = sqlx::query(
        "SELECT table_name FROM information_schema.tables
         WHERE table_schema = DATABASE() ORDER BY table_name",
    )
    .fetch_all(&mut conn)
    .await
    .map_err(|e| Error::Consulta(e.to_string()))?;
    conn.close().await.ok();
    let nombres: Vec<String> = raws
        .iter()
        .filter_map(|r| r.try_get::<String, _>(0).ok())
        .collect();

    let mut salida = Vec::new();
    for n in nombres {
        let columnas = columnas_mysql(fuente, &n).await?;
        salida.push(Esquema {
            nombre: n,
            tipo: "tabla".into(),
            filas_estimadas: -1,
            columnas,
        });
    }
    Ok(salida)
}

async fn consultar_mysql(
    fuente: &Fuente,
    peticion: &PeticionBusqueda,
) -> Resultado<BloqueResultados> {
    let inicio = std::time::Instant::now();
    let mut conn = conectar_mysql(fuente).await?;
    let esquemas_posibles: Vec<String> = if peticion.esquemas.is_empty() {
        esquemas_mysql(fuente)
            .await?
            .into_iter()
            .map(|e| e.nombre)
            .collect()
    } else {
        peticion.esquemas.clone()
    };

    let mut filas: Vec<Fila> = Vec::new();
    let mut total = 0i64;

    for esquema in &esquemas_posibles {
        let columnas = columnas_mysql(fuente, esquema).await?;
        let textuales: Vec<String> = columnas
            .iter()
            .filter(|c| {
                let t = c.tipo.to_uppercase();
                t.contains("CHAR") || t.contains("TEXT") || t.contains("JSON")
            })
            .map(|c| c.nombre.clone())
            .collect();

        let (where_, params) = query::construir_where(
            &peticion.filtros,
            &peticion.texto,
            Conector::desde_txt(&peticion.modo),
            Dialecto::MySql,
            &textuales,
        )?;

        let tabla = nombre_tabla_calificada(fuente, esquema);
        let seleccion = if peticion.campos.is_empty() {
            "*".to_string()
        } else {
            peticion
                .campos
                .iter()
                .map(|c| Dialecto::MySql.identificar(c))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let mut sql = format!("SELECT {seleccion} FROM {tabla} {where_}");
        if !peticion.ordenar_por.trim().is_empty() {
            sql.push_str(&format!(
                " ORDER BY {} {}",
                Dialecto::MySql.identificar(&peticion.ordenar_por),
                if peticion.orden_desc { "DESC" } else { "ASC" }
            ));
        }
        sql.push_str(&format!(
            " LIMIT {} OFFSET {}",
            peticion.limite.clamp(1, 100_000),
            peticion.desplazamiento
        ));

        let mut q = sqlx::query(&sql);
        for p in &params {
            q = q.bind(p);
        }
        let obtenidas = q
            .fetch_all(&mut conn)
            .await
            .map_err(|e| Error::Consulta(format!("{e}\nSQL: {sql}")))?;

        for r in obtenidas.iter().take(TOPE_FILAS) {
            filas.push(fila_desde_mysql(r));
        }

        if peticion.contar_total {
            let sql_cuenta = format!("SELECT COUNT(*) FROM {tabla} {where_}");
            let mut qc = sqlx::query(&sql_cuenta);
            for p in &params {
                qc = qc.bind(p);
            }
            if let Ok(r) = qc.fetch_one(&mut conn).await {
                if let Ok(n) = r.try_get::<i64, _>(0) {
                    total += n;
                }
            }
        } else {
            total += obtenidas.len() as i64;
        }
    }
    conn.close().await.ok();

    let omitidas = if filas.len() >= TOPE_FILAS { 1 } else { 0 };
    Ok(BloqueResultados {
        fuente_id: fuente.id,
        fuente_nombre: fuente.nombre.clone(),
        tipo_fuente: fuente.tipo,
        esquema: esquemas_posibles.join(", "),
        columnas: query::describir_columnas(&filas, 20),
        total_filas: total,
        filas_omitidas: omitidas,
        filas,
        duracion_ms: inicio.elapsed().as_millis() as u64,
        mensaje: String::new(),
    })
}

// ======================================================================
// SQL Server (TDS) mediante tiberius
// ======================================================================

/// Cliente TDS. Tokio usa sus propios rasgos de entrada/salida, por lo que el
/// socket se adapta con `tokio_util::compat`.
type ClienteTds = tiberius::Client<tokio_util::compat::Compat<tokio::net::TcpStream>>;

fn config_tds(fuente: &Fuente) -> Resultado<tiberius::Config> {
    let c = &fuente.config;
    if c.host.trim().is_empty() {
        return Err(Error::Validacion("Indique el servidor".into()));
    }
    if c.base_datos.trim().is_empty() {
        return Err(Error::Validacion("Indique la base de datos".into()));
    }
    let url = format!(
        "sqlserver://{};user={};password={};TrustServerCertificate=true",
        c.host.trim(),
        c.usuario.trim(),
        c.contrasena.trim()
    );
    tiberius::Config::from_jdbc_string(&url)
        .map_err(|e| Error::Conexion(format!("Configuracion de SQL Server no valida: {e}")))
}

async fn cliente_tds(fuente: &Fuente) -> Resultado<ClienteTds> {
    let cfg = config_tds(fuente)?;
    let direccion = cfg.get_addr();
    let socket = tokio::net::TcpStream::connect(&direccion)
        .await
        .map_err(|e| Error::Conexion(format!("No se pudo conectar a {direccion}: {e}")))?;
    let _ = socket.set_nodelay(true);
    tiberius::Client::connect(cfg, socket.compat_write())
        .await
        .map_err(|e| Error::Conexion(format!("Fallo al autenticar en SQL Server: {e}")))
}

/// Ejecuta una consulta y devuelve las filas como mapas `columna -> valor`.
///
/// T-SQL no admite marcadores de posicion, de modo que los valores que inyecta
/// el usuario se interpolan siempre despues de `escapar_sql`.
async fn ejecutar_tds(
    fuente: &Fuente,
    consulta: &str,
) -> Resultado<Vec<BTreeMap<String, Option<String>>>> {
    let mut cliente = cliente_tds(fuente).await?;
    let mut flujo = cliente
        .query(consulta, &[])
        .await
        .map_err(|e| Error::Consulta(format!("Error en SQL Server: {e}\nSQL: {consulta}")))?;

    let columnas: Vec<(String, tiberius::ColumnType)> = match flujo.columns().await {
        Ok(algunas) => algunas
            .map(|cs| {
                cs.iter()
                    .map(|c| (c.name().to_string(), c.column_type()))
                    .collect()
            })
            .unwrap_or_default(),
        Err(_) => Vec::new(),
    };

    let resultados = flujo
        .into_results()
        .await
        .map_err(|e| Error::Consulta(e.to_string()))?;
    cliente.close().await.ok();

    let mut salida: Vec<BTreeMap<String, Option<String>>> = Vec::new();
    for fila in resultados.into_iter().flatten() {
        let mut mapa = BTreeMap::new();
        for (i, (nombre, tipo)) in columnas.iter().enumerate() {
            mapa.insert(nombre.clone(), celda_tds(&fila, i, *tipo));
        }
        salida.push(mapa);
        if salida.len() >= TOPE_FILAS {
            break;
        }
    }
    Ok(salida)
}

/// Convierte una celda de SQL Server a texto segun su tipo declarado.
fn celda_tds(fila: &tiberius::Row, indice: usize, tipo: tiberius::ColumnType) -> Option<String> {
    use tiberius::ColumnType as Ct;
    let texto = match tipo {
        Ct::Bit => fila
            .try_get::<bool, _>(indice)
            .ok()
            .flatten()
            .map(|v| v.to_string()),
        Ct::Int1 | Ct::Int2 | Ct::Int4 | Ct::Int8 => fila
            .try_get::<i64, _>(indice)
            .ok()
            .flatten()
            .map(|v| v.to_string()),
        Ct::Float4 | Ct::Float8 | Ct::Floatn => fila
            .try_get::<f64, _>(indice)
            .ok()
            .flatten()
            .map(|v| v.to_string()),
        Ct::Guid => fila
            .try_get::<uuid::Uuid, _>(indice)
            .ok()
            .flatten()
            .map(|v| v.to_string()),
        Ct::Datetime | Ct::Datetime2 | Ct::Datetime4 | Ct::Datetimen | Ct::DatetimeOffsetn => fila
            .try_get::<chrono::NaiveDateTime, _>(indice)
            .ok()
            .flatten()
            .map(|v| v.format("%Y-%m-%d %H:%M:%S").to_string())
            .or_else(|| {
                fila.try_get::<chrono::NaiveDate, _>(indice)
                    .ok()
                    .flatten()
                    .map(|v| v.format("%Y-%m-%d").to_string())
            }),
        Ct::Daten => fila
            .try_get::<chrono::NaiveDate, _>(indice)
            .ok()
            .flatten()
            .map(|v| v.format("%Y-%m-%d").to_string()),
        Ct::Timen => fila
            .try_get::<chrono::NaiveTime, _>(indice)
            .ok()
            .flatten()
            .map(|v| v.format("%H:%M:%S").to_string()),
        Ct::BigBinary | Ct::Image | Ct::BigVarBin => fila
            .try_get::<&[u8], _>(indice)
            .ok()
            .flatten()
            .map(|b| format!("<binario: {} bytes>", b.len())),
        // Numeric, Money, Xml y el resto de textos se leen como cadena.
        _ => fila
            .try_get::<&str, _>(indice)
            .ok()
            .flatten()
            .map(|v| v.to_string()),
    };
    texto.filter(|t| !t.is_empty())
}

/// Sustituye los marcadores `?` por literales escapados.
fn sustituir_marcadores(sql: &str, params: &[String]) -> String {
    if params.is_empty() {
        return sql.to_string();
    }
    let mut salida = String::with_capacity(sql.len() + params.len() * 8);
    for (i, trozo) in sql.split('?').enumerate() {
        salida.push_str(trozo);
        if let Some(p) = params.get(i) {
            salida.push_str(&format!("'{}'", crate::util::escapar_sql(p)));
        }
    }
    salida
}

/// Columnas de una tabla de SQL Server, leidas del catalogo.
async fn columnas_tds(
    fuente: &Fuente,
    esquema: &str,
    tabla: &str,
) -> Resultado<Vec<ColumnaEsquema>> {
    let sql = format!(
        "SELECT COLUMN_NAME, DATA_TYPE, IS_NULLABLE FROM INFORMATION_SCHEMA.COLUMNS \
         WHERE TABLE_SCHEMA = '{}' AND TABLE_NAME = '{}' ORDER BY ORDINAL_POSITION",
        crate::util::escapar_sql(esquema),
        crate::util::escapar_sql(tabla)
    );
    let filas = ejecutar_tds(fuente, &sql).await?;
    Ok(filas
        .into_iter()
        .filter_map(|m| {
            let nombre = m.get("COLUMN_NAME")?.clone()?;
            if nombre.is_empty() {
                return None;
            }
            Some(ColumnaEsquema {
                tipo: m.get("DATA_TYPE").cloned().flatten().unwrap_or_default(),
                obligatorio: m
                    .get("IS_NULLABLE")
                    .cloned()
                    .flatten()
                    .map(|v| v == "NO")
                    .unwrap_or(false),
                clave_primaria: false,
                nombre,
            })
        })
        .collect())
}

/// Construye el SQL de la consulta del usuario para SQL Server.
async fn construir_sql_tds(fuente: &Fuente, peticion: &PeticionBusqueda) -> Resultado<String> {
    let esquema = if fuente.config.esquema.trim().is_empty() {
        "dbo".to_string()
    } else {
        fuente.config.esquema.trim().to_string()
    };
    let tabla = peticion
        .esquemas
        .first()
        .cloned()
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| esquema.clone());

    let columnas = columnas_tds(fuente, &esquema, &tabla)
        .await
        .unwrap_or_default();
    let textuales: Vec<String> = columnas
        .iter()
        .filter(|c| {
            let t = c.tipo.to_uppercase();
            t.contains("CHAR") || t.contains("TEXT")
        })
        .map(|c| c.nombre.clone())
        .collect();

    let (where_, params) = query::construir_where(
        &peticion.filtros,
        &peticion.texto,
        Conector::desde_txt(&peticion.modo),
        Dialecto::SqlServer,
        &textuales,
    )?;
    let where_ = sustituir_marcadores(&where_, &params);

    let seleccion = if peticion.campos.is_empty() {
        "*".to_string()
    } else {
        peticion
            .campos
            .iter()
            .map(|c| Dialecto::SqlServer.identificar(c))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut sql = format!(
        "SELECT {seleccion} FROM {}.{} {where_}",
        Dialecto::SqlServer.identificar(&esquema),
        Dialecto::SqlServer.identificar(&tabla)
    );
    if !peticion.ordenar_por.trim().is_empty() {
        sql.push_str(&format!(
            " ORDER BY {} {}",
            Dialecto::SqlServer.identificar(&peticion.ordenar_por),
            if peticion.orden_desc { "DESC" } else { "ASC" }
        ));
    }
    sql.push_str(&format!(
        " OFFSET {} ROWS FETCH NEXT {} ROWS ONLY",
        peticion.desplazamiento,
        peticion.limite.clamp(1, 100_000)
    ));
    Ok(sql)
}

async fn esquemas_sqlserver(fuente: &Fuente) -> Resultado<Vec<Esquema>> {
    let filas = ejecutar_tds(
        fuente,
        "SELECT TABLE_NAME FROM INFORMATION_SCHEMA.TABLES ORDER BY TABLE_NAME",
    )
    .await?;
    Ok(filas
        .into_iter()
        .filter_map(|m| {
            Some(Esquema {
                nombre: m.get("TABLE_NAME")?.clone()?,
                tipo: "tabla".into(),
                filas_estimadas: -1,
                columnas: Vec::new(),
            })
        })
        .collect())
}

async fn consultar_sqlserver(
    fuente: &Fuente,
    peticion: &PeticionBusqueda,
) -> Resultado<BloqueResultados> {
    let inicio = std::time::Instant::now();
    let sql = construir_sql_tds(fuente, peticion).await?;
    let celdas = ejecutar_tds(fuente, &sql).await?;
    let filas: Vec<Fila> = celdas.into_iter().map(|valores| Fila { valores }).collect();
    Ok(BloqueResultados {
        fuente_id: fuente.id,
        fuente_nombre: fuente.nombre.clone(),
        tipo_fuente: fuente.tipo,
        esquema: peticion.esquemas.join(", "),
        columnas: query::describir_columnas(&filas, 20),
        total_filas: filas.len() as i64,
        filas_omitidas: 0,
        filas,
        duracion_ms: inicio.elapsed().as_millis() as u64,
        mensaje: "En SQL Server el total se calcula sobre las filas devueltas.".into(),
    })
}

// ======================================================================
// MongoDB
// ======================================================================

async fn cliente_mongo(fuente: &Fuente) -> Resultado<mongodb::Database> {
    let uri = if fuente.config.uri.trim().is_empty() {
        let c = &fuente.config;
        if c.host.trim().is_empty() {
            return Err(Error::Validacion("Indique la URI o el servidor".into()));
        }
        format!(
            "mongodb://{}:{}@{}:{}/",
            c.usuario.trim(),
            c.contrasena.trim(),
            c.host.trim(),
            c.puerto.unwrap_or(27017)
        )
    } else {
        fuente.config.uri.trim().to_string()
    };
    let cliente = mongodb::Client::with_uri_str(&uri)
        .await
        .map_err(|e| Error::Conexion(format!("No se pudo conectar a MongoDB: {e}")))?;
    let base = if fuente.config.base_datos.trim().is_empty() {
        "admin".to_string()
    } else {
        fuente.config.base_datos.trim().to_string()
    };
    Ok(cliente.database(&base))
}

async fn esquemas_mongo(fuente: &Fuente) -> Resultado<Vec<Esquema>> {
    let db = cliente_mongo(fuente).await?;
    let mut nombres = db
        .list_collection_names()
        .await
        .map_err(|e| Error::Conexion(e.to_string()))?;
    nombres.sort();
    Ok(nombres
        .into_iter()
        .map(|n| Esquema {
            nombre: n,
            tipo: "coleccion".into(),
            filas_estimadas: -1,
            columnas: Vec::new(),
        })
        .collect())
}

fn documentos_a_filas(valores: &[bson::Document]) -> Vec<Fila> {
    valores
        .iter()
        .map(|d| match serde_json::to_value(d) {
            Ok(json) => Fila {
                valores: aplanar_json(&json, "")
                    .into_iter()
                    .map(|(k, v)| (k, Some(v)))
                    .collect(),
            },
            Err(_) => Fila::default(),
        })
        .collect()
}

async fn consultar_mongo(
    fuente: &Fuente,
    peticion: &PeticionBusqueda,
) -> Resultado<BloqueResultados> {
    let db = cliente_mongo(fuente).await?;
    let colecciones: Vec<String> = if peticion.esquemas.is_empty() {
        esquemas_mongo(fuente)
            .await?
            .into_iter()
            .map(|e| e.nombre)
            .collect()
    } else {
        peticion.esquemas.clone()
    };

    let limite = peticion.limite.clamp(1, TOPE_FILAS as u32);
    let mut todas: Vec<Fila> = Vec::new();
    for nombre in &colecciones {
        let cursor = db
            .collection::<bson::Document>(nombre)
            .find(bson::doc! {})
            .limit(limite as i64)
            .await
            .map_err(|e| Error::Consulta(e.to_string()))?;
        let docs: Vec<bson::Document> = cursor
            .try_collect()
            .await
            .map_err(|e| Error::Consulta(e.to_string()))?;
        todas.extend(documentos_a_filas(&docs));
    }

    // El filtrado de MongoDB se evalua en memoria para que se comporte igual
    // que en el resto de conectores.
    let mut peticion = peticion.clone();
    if peticion.esquemas.is_empty() {
        peticion.esquemas = colecciones.clone();
    }
    let finales = query::filtrar_filas(todas, &peticion, "")?;
    Ok(BloqueResultados {
        fuente_id: fuente.id,
        fuente_nombre: fuente.nombre.clone(),
        tipo_fuente: fuente.tipo,
        esquema: colecciones.join(", "),
        columnas: query::describir_columnas(&finales, 20),
        total_filas: finales.len() as i64,
        filas_omitidas: 0,
        filas: finales,
        duracion_ms: 0,
        mensaje: String::new(),
    })
}

// ======================================================================
// Ficheros planos: CSV, JSON, XML y Excel
// ======================================================================

async fn columnas_de_esquema_plano(fuente: &Fuente) -> Resultado<Vec<ColumnaEsquema>> {
    let filas = leer_plano(fuente).await?;
    let info = query::describir_columnas(&filas, 5);
    Ok(query::columnas_de(&filas)
        .into_iter()
        .map(|n| {
            let d = info.iter().find(|c| c.nombre == n);
            ColumnaEsquema {
                nombre: n,
                tipo: d.map(|c| c.tipo.clone()).unwrap_or_else(|| "texto".into()),
                obligatorio: d.map(|c| c.nulos == 0).unwrap_or(false),
                clave_primaria: false,
            }
        })
        .collect())
}

fn construir_cliente(fuente: &Fuente) -> Resultado<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(tope_segundos(fuente)))
        .user_agent(format!("DataSearch/{}", crate::VERSION))
        .build()
        .map_err(|e| Error::Red(e.to_string()))
}

/// Lee el contenido de una fuente plana, tanto local como por URL.
async fn leer_plano(fuente: &Fuente) -> Resultado<Vec<Fila>> {
    let ruta = fuente.config.ruta.trim();
    if ruta.is_empty() {
        return Err(Error::Validacion(
            "Indique la ruta del fichero o la URL".into(),
        ));
    }
    let bytes = if ruta.starts_with("http://") || ruta.starts_with("https://") {
        descargar_web(fuente).await?.0
    } else {
        std::fs::read(ruta).map_err(|e| Error::Interno(format!("No se pudo leer {ruta}: {e}")))?
    };
    parsear_plano(fuente, &bytes)
}

/// Convierte los bytes de un fichero plano en filas.
fn parsear_plano(fuente: &Fuente, bytes: &[u8]) -> Resultado<Vec<Fila>> {
    match fuente.tipo {
        TipoFuente::Csv => parsear_csv(&String::from_utf8_lossy(bytes)),
        TipoFuente::Xml => parsear_xml(&String::from_utf8_lossy(bytes)),
        TipoFuente::Excel => parsear_excel(bytes),
        // JSON y Web: si el contenido no es JSON valido se trata como CSV.
        _ => match serde_json::from_str::<serde_json::Value>(&String::from_utf8_lossy(bytes)) {
            Ok(v) => Ok(json_a_filas(&v, 0)),
            Err(_) => parsear_csv(&String::from_utf8_lossy(bytes)),
        },
    }
}

fn parsear_csv(texto: &str) -> Resultado<Vec<Fila>> {
    let mut lector = csv::ReaderBuilder::new()
        .flexible(true)
        .delimiter(detectar_delimitador(texto))
        .trim(csv::Trim::All)
        .from_reader(texto.as_bytes());
    let cabeceras = normalizar_cabeceras(
        lector
            .headers()
            .map_err(|e| Error::Interno(format!("CSV invalido: {e}")))?
            .iter()
            .map(crate::util::limpiar_nombre_columna)
            .collect(),
    );

    let mut filas = Vec::new();
    for registro in lector.records().take(TOPE_FILAS) {
        let Ok(registro) = registro else { continue };
        filas.push(Fila {
            valores: cabeceras
                .iter()
                .enumerate()
                .map(|(i, h)| {
                    let v = registro.get(i).unwrap_or("").trim().to_string();
                    (h.clone(), if v.is_empty() { None } else { Some(v) })
                })
                .collect(),
        });
    }
    Ok(filas)
}

fn detectar_delimitador(texto: &str) -> u8 {
    let primera_linea = texto.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
    let candidatos = *b",;\t|";
    let mut mejor = b',';
    let mut mejor_cuenta = 0usize;
    for c in candidatos {
        let cuenta = primera_linea.as_bytes().iter().filter(|b| **b == c).count();
        if cuenta > mejor_cuenta {
            mejor_cuenta = cuenta;
            mejor = c;
        }
    }
    mejor
}

fn normalizar_cabeceras(mut cabeceras: Vec<String>) -> Vec<String> {
    let mut vistas = BTreeSet::new();
    for c in cabeceras.iter_mut() {
        let mut base = crate::util::limpiar_nombre_columna(c);
        if base.is_empty() {
            base = "columna".to_string();
        }
        let mut n = 1;
        while !vistas.insert(base.clone()) {
            n += 1;
            base = format!("{base}_{n}");
        }
        *c = base;
    }
    cabeceras
}

/// Aplana un JSON arbitrario a filas. Un array de objetos genera una fila por
/// elemento; un objeto suelto genera una sola fila.
fn json_a_filas(valor: &serde_json::Value, profundidad: usize) -> Vec<Fila> {
    if profundidad > 6 {
        return Vec::new();
    }
    match valor {
        serde_json::Value::Array(lista) => lista
            .iter()
            .take(TOPE_FILAS)
            .flat_map(|v| json_a_filas(v, profundidad + 1))
            .collect(),
        serde_json::Value::Object(_) => {
            if let Some(mapa) = valor.as_object() {
                // Un envoltorio tipico de API es `{ "data": [ {...} ] }`.
                let candidatos: Vec<(&String, &Vec<serde_json::Value>)> = mapa
                    .iter()
                    .filter_map(|(k, v)| v.as_array().map(|a| (k, a)))
                    .filter(|(_, a)| a.iter().any(|e| e.is_object()))
                    .collect();
                if candidatos.len() == 1 && mapa.len() <= 3 {
                    return candidatos[0]
                        .1
                        .iter()
                        .take(TOPE_FILAS)
                        .flat_map(|e| {
                            let plano: BTreeMap<String, Option<String>> = aplanar_json(e, "")
                                .into_iter()
                                .map(|(k, v)| (k, Some(v)))
                                .collect();
                            if plano.is_empty() {
                                vec![Fila {
                                    valores: BTreeMap::from([(
                                        "valor".to_string(),
                                        Some(crate::util::json_a_texto(e)),
                                    )]),
                                }]
                            } else {
                                vec![Fila { valores: plano }]
                            }
                        })
                        .collect();
                }
            }
            vec![Fila {
                valores: aplanar_json(valor, "")
                    .into_iter()
                    .map(|(k, v)| (k, Some(v)))
                    .collect(),
            }]
        }
        serde_json::Value::Null => Vec::new(),
        otro => vec![Fila {
            valores: BTreeMap::from([("valor".to_string(), Some(crate::util::json_a_texto(otro)))]),
        }],
    }
}

/// Lee un XML y toma cada elemento del segundo nivel como una fila.
///
/// La ruta del elemento dentro de su padre se usa como nombre de columna, de
/// forma que `catalogo/producto/nombre` queda disponible como columna
/// `producto.nombre`. Los atributos se anaden con el sufijo `@atributo`.
fn parsear_xml(texto: &str) -> Resultado<Vec<Fila>> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut lector = Reader::from_str(texto);
    lector.config_mut().trim_text(true);
    lector.config_mut().check_end_names = false;

    let mut filas: Vec<Fila> = Vec::new();
    // Pila de rutas de los elementos abiertos.
    let mut pila: Vec<String> = Vec::new();
    // `true` mientras el elemento actual sea una hoja (sin hijos).
    let mut es_hoja = true;
    let mut texto = String::new();
    let mut en_fila = false;

    // Vuelca el texto acumulado del elemento que acaba de terminar.
    let cerrar_hoja = |filas: &mut Vec<Fila>, ruta: &str, texto: &mut String, en_fila: bool| {
        let limpio = texto.trim().to_string();
        texto.clear();
        if !limpio.is_empty() && en_fila {
            if let Some(fila) = filas.last_mut() {
                fila.valores.insert(ruta.to_string(), Some(limpio));
            }
        }
    };

    loop {
        match lector.read_event() {
            Ok(Event::Start(e)) => {
                // El elemento que se abria puede haber aportado texto.
                if let Some(ruta) = pila.last().cloned() {
                    cerrar_hoja(&mut filas, &ruta, &mut texto, en_fila);
                }
                let nombre = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                let limpio = crate::util::limpiar_nombre_columna(&nombre);
                // El primer elemento es la raiz y no forma parte de la ruta.
                let ruta = if pila.len() <= 1 {
                    limpio
                } else {
                    format!("{}.{}", pila[1..].join("."), limpio)
                };

                let mut atributos: Vec<(String, Option<String>)> = Vec::new();
                for a in e.attributes().flatten() {
                    atributos.push((
                        format!("{ruta}@{}", String::from_utf8_lossy(a.key.as_ref())),
                        Some(String::from_utf8_lossy(&a.value).into_owned()),
                    ));
                }

                if pila.len() == 1 {
                    // Cada elemento de este nivel abre una fila nueva.
                    filas.push(Fila {
                        valores: atributos.into_iter().collect(),
                    });
                    en_fila = true;
                } else if let Some(fila) = filas.last_mut() {
                    for (k, v) in atributos {
                        fila.valores.insert(k, v);
                    }
                }

                pila.push(ruta);
                es_hoja = true;
            }
            Ok(Event::Text(t)) => {
                texto.push_str(&t.unescape().unwrap_or_default());
            }
            Ok(Event::CData(d)) => {
                texto.push_str(&String::from_utf8_lossy(d.as_ref()));
            }
            Ok(Event::End(_)) => {
                let ruta = pila.pop().unwrap_or_default();
                if es_hoja {
                    cerrar_hoja(&mut filas, &ruta, &mut texto, en_fila);
                } else {
                    texto.clear();
                }
                if pila.len() <= 1 {
                    en_fila = false;
                }
                es_hoja = false;
            }
            Ok(Event::Empty(e)) => {
                // Elemento vacio: solo aportan sus atributos.
                let nombre = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                let limpio = crate::util::limpiar_nombre_columna(&nombre);
                // El primer elemento es la raiz y no forma parte de la ruta.
                let ruta = if pila.len() <= 1 {
                    limpio
                } else {
                    format!("{}.{}", pila[1..].join("."), limpio)
                };
                if let Some(fila) = filas.last_mut() {
                    for a in e.attributes().flatten() {
                        fila.valores.insert(
                            format!("{ruta}@{}", String::from_utf8_lossy(a.key.as_ref())),
                            Some(String::from_utf8_lossy(&a.value).into_owned()),
                        );
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(Error::Interno(format!("XML invalido: {e}"))),
            _ => {}
        }
        if filas.len() >= TOPE_FILAS {
            break;
        }
    }

    filas.retain(|f| {
        f.valores
            .values()
            .any(|v| v.as_deref().map(|s| !s.is_empty()).unwrap_or(false))
    });
    Ok(filas)
}

fn parsear_excel(bytes: &[u8]) -> Resultado<Vec<Fila>> {
    use calamine::{Reader, Xlsx};
    let mut libro: Xlsx<_> = calamine::Xlsx::new(std::io::Cursor::new(bytes))
        .map_err(|e| Error::Interno(format!("No se pudo leer el Excel: {e}")))?;
    let hojas: Vec<String> = libro.sheet_names().to_vec();
    let hoja = hojas
        .first()
        .cloned()
        .ok_or_else(|| Error::Interno("El libro Excel no tiene hojas".into()))?;
    let celdas = libro
        .worksheet_range(&hoja)
        .map_err(|e| Error::Interno(format!("La hoja '{hoja}' no se pudo leer: {e}")))?;

    let mut iter = celdas.rows();
    let cabeceras = iter
        .next()
        .map(|fila| {
            normalizar_cabeceras(
                fila.iter()
                    .map(|c| crate::util::limpiar_nombre_columna(&c.to_string()))
                    .collect(),
            )
        })
        .unwrap_or_default();
    if cabeceras.is_empty() {
        return Ok(Vec::new());
    }

    let mut filas = Vec::new();
    for fila in iter.take(TOPE_FILAS) {
        filas.push(Fila {
            valores: cabeceras
                .iter()
                .enumerate()
                .map(|(i, h)| {
                    let v = fila
                        .get(i)
                        .map(|c| c.to_string().trim().to_string())
                        .unwrap_or_default();
                    (h.clone(), if v.is_empty() { None } else { Some(v) })
                })
                .collect(),
        });
    }
    Ok(filas)
}

async fn consultar_plano(
    fuente: &Fuente,
    peticion: &PeticionBusqueda,
) -> Resultado<BloqueResultados> {
    let todas = leer_plano(fuente).await?;
    let total = todas.len() as i64;
    let filtradas = query::filtrar_filas(todas, peticion, "")?;
    Ok(BloqueResultados {
        fuente_id: fuente.id,
        fuente_nombre: fuente.nombre.clone(),
        tipo_fuente: fuente.tipo,
        esquema: nombre_pseudo_esquema(fuente),
        columnas: query::describir_columnas(&filtradas, 20),
        total_filas: if peticion.contar_total {
            total
        } else {
            filtradas.len() as i64
        },
        filas_omitidas: 0,
        filas: filtradas,
        duracion_ms: 0,
        mensaje: String::new(),
    })
}

// ======================================================================
// Fuentes web
// ======================================================================

async fn descargar_web(fuente: &Fuente) -> Resultado<(Vec<u8>, String)> {
    let ruta = fuente.config.ruta.trim();
    if ruta.is_empty() {
        return Err(Error::Validacion("Indique la URL del recurso".into()));
    }
    let cliente = construir_cliente(fuente)?;
    let mut url =
        url::Url::parse(ruta).map_err(|e| Error::Validacion(format!("URL no valida: {e}")))?;
    if !fuente.config.parametros.trim().is_empty() {
        let extra: Vec<(String, String)> = fuente
            .config
            .parametros
            .split('&')
            .filter_map(|kv| {
                kv.split_once('=')
                    .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
            })
            .collect();
        url.query_pairs_mut().extend_pairs(extra);
    }

    let metodo = if fuente.config.metodo.trim().is_empty() {
        "GET"
    } else {
        fuente.config.metodo.trim()
    };
    let mut peticion = match metodo.to_uppercase().as_str() {
        "POST" => cliente.post(url.clone()),
        "PUT" => cliente.put(url.clone()),
        "DELETE" => cliente.delete(url.clone()),
        _ => cliente.get(url.clone()),
    };
    for (clave, valor) in &fuente.config.cabeceras {
        peticion = peticion.header(clave, valor);
    }
    if !fuente.config.cuerpo.trim().is_empty() {
        peticion = match serde_json::from_str::<serde_json::Value>(&fuente.config.cuerpo) {
            Ok(cuerpo) => peticion.json(&cuerpo),
            Err(_) => peticion.body(fuente.config.cuerpo.clone()),
        };
    }

    let respuesta = peticion
        .send()
        .await
        .map_err(|e| Error::Red(format!("No se pudo acceder a {ruta}: {e}")))?;
    let estado = respuesta.status();
    let ctype = respuesta
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    if !estado.is_success() {
        return Err(Error::Red(format!(
            "El servidor respondio {estado} a {ruta}"
        )));
    }
    let bytes = respuesta
        .bytes()
        .await
        .map_err(|e| Error::Red(e.to_string()))?;
    if bytes.len() > 64 * 1024 * 1024 {
        return Err(Error::Red("El recurso supera los 64 MB permitidos".into()));
    }
    Ok((bytes.to_vec(), ctype))
}

/// Navega por una ruta con puntos (`data.items`) dentro de un JSON.
fn extraer_de_ruta(valor: &serde_json::Value, ruta: &str) -> serde_json::Value {
    if ruta.trim().is_empty() {
        return valor.clone();
    }
    let mut actual = valor.clone();
    for parte in partir_ruta_json(ruta) {
        actual = match actual {
            serde_json::Value::Object(m) => m.get(&parte).cloned().unwrap_or_default(),
            serde_json::Value::Array(a) => a
                .get(parte.parse::<usize>().unwrap_or(usize::MAX))
                .cloned()
                .unwrap_or_default(),
            _ => return serde_json::Value::Null,
        };
    }
    actual
}

fn web_es_csv(fuente: &Fuente, ctype: &str, texto: &str) -> bool {
    fuente.tipo == TipoFuente::Csv
        || fuente.config.formato_web.eq_ignore_ascii_case("csv")
        || (!ctype.contains("json") && texto.trim_start().starts_with(','))
}

fn filas_web(fuente: &Fuente, texto: &str, ctype: &str) -> Resultado<Vec<Fila>> {
    if web_es_csv(fuente, ctype, texto) {
        return parsear_csv(texto);
    }
    let valor: serde_json::Value = serde_json::from_str(texto)
        .map_err(|e| Error::Red(format!("La respuesta no es JSON valido: {e}")))?;
    Ok(json_a_filas(
        &extraer_de_ruta(&valor, &fuente.config.ruta_json),
        0,
    ))
}

fn esquema_desde_filas(nombre: &str, filas: &[Fila]) -> Esquema {
    let info = query::describir_columnas(filas, 5);
    Esquema {
        columnas: query::columnas_de(filas)
            .into_iter()
            .map(|n| ColumnaEsquema {
                tipo: info
                    .iter()
                    .find(|c| c.nombre == n)
                    .map(|c| c.tipo.clone())
                    .unwrap_or_else(|| "texto".into()),
                obligatorio: false,
                clave_primaria: false,
                nombre: n,
            })
            .collect(),
        nombre: nombre.to_string(),
        tipo: "tabla".into(),
        filas_estimadas: filas.len() as i64,
    }
}

async fn esquemas_web(fuente: &Fuente) -> Resultado<Vec<Esquema>> {
    let (bytes, ctype) = descargar_web(fuente).await?;
    let filas = filas_web(fuente, &String::from_utf8_lossy(&bytes), &ctype)?;
    Ok(vec![esquema_desde_filas("recurso_web", &filas)])
}

async fn consultar_web(
    fuente: &Fuente,
    peticion: &PeticionBusqueda,
) -> Resultado<BloqueResultados> {
    let (bytes, ctype) = descargar_web(fuente).await?;
    let todas = filas_web(fuente, &String::from_utf8_lossy(&bytes), &ctype)?;
    let total = todas.len() as i64;

    let mut peticion = peticion.clone();
    if peticion.esquemas.is_empty() {
        peticion.esquemas = vec!["recurso_web".to_string()];
    }
    let filtradas = query::filtrar_filas(todas, &peticion, "recurso_web")?;
    Ok(BloqueResultados {
        fuente_id: fuente.id,
        fuente_nombre: fuente.nombre.clone(),
        tipo_fuente: fuente.tipo,
        esquema: "recurso_web".into(),
        columnas: query::describir_columnas(&filtradas, 20),
        total_filas: if peticion.contar_total {
            total
        } else {
            filtradas.len() as i64
        },
        filas_omitidas: 0,
        filas: filtradas,
        duracion_ms: 0,
        mensaje: String::new(),
    })
}

// ======================================================================
// Utilidades compartidas por los conectores
// ======================================================================

/// Columnas que parecen texto, para la busqueda en texto libre.
pub fn columnas_textuales(bloque: &BloqueResultados) -> Vec<String> {
    bloque
        .columnas
        .iter()
        .filter(|c| c.es_texto || c.tipo == Familia::Json.etiqueta())
        .map(|c| c.nombre.clone())
        .collect()
}

/// Convierte un valor de columna a numero, si se puede.
pub fn valor_numerico(bloque: &BloqueResultados, columna: &str) -> Option<f64> {
    bloque
        .columnas
        .iter()
        .find(|c| c.nombre == columna)
        .and(None::<f64>)
        .or(None)
}

/// Traduce un `Predicado` a su descripcion legible, para las advertencias.
pub fn describir_predicado(p: &Predicado) -> String {
    match p {
        Predicado::Columna {
            columna, op, valor, ..
        } => format!("{columna} {} {valor}", op.etiqueta()),
        Predicado::TextoLibre(t) => format!("texto '{t}'"),
    }
}

/// Dialecto que corresponde a un tipo de fuente.
pub fn dialecto(tipo: TipoFuente) -> Dialecto {
    dialecto_de(tipo)
}

/// Configuracion de ejemplo para el formulario de alta de fuente.
pub fn config_ejemplo(tipo: TipoFuente) -> ConfigFuente {
    ConfigFuente {
        puerto: match tipo {
            TipoFuente::Postgres => Some(5432),
            TipoFuente::Mysql => Some(3306),
            TipoFuente::MongoDb => Some(27017),
            TipoFuente::SqlServer => Some(1433),
            _ => None,
        },
        metodo: "GET".into(),
        formato_web: "json".into(),
        ..Default::default()
    }
}

/// Un valor de ejemplo para probar los filtros desde la interfaz.
pub fn valor_de_ejemplo(fila: &Fila, columna: &str) -> Option<String> {
    fila.valores.get(columna).cloned().flatten()
}

/// Convierte un texto a numero, reexportado para los comandos.
pub fn a_numero(texto: &str) -> Option<f64> {
    a_f64(texto)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fuente_de_prueba(tipo: TipoFuente) -> Fuente {
        Fuente {
            id: 1,
            id_usuario: 1,
            nombre: "prueba".into(),
            descripcion: String::new(),
            tipo,
            config: ConfigFuente::default(),
            activa: true,
            solo_admin: false,
            creado_en: String::new(),
            actualizado_en: String::new(),
            estado_conexion: String::new(),
        }
    }

    #[test]
    fn detecta_delimitador_csv() {
        assert_eq!(detectar_delimitador("a;b;c\n1;2;3"), b';');
        assert_eq!(detectar_delimitador("a,b,c\n1,2,3"), b',');
        assert_eq!(detectar_delimitador("a\tb\n1\t2"), b'\t');
    }

    #[test]
    fn normaliza_cabeceras_repetidas() {
        let salida = normalizar_cabeceras(vec![
            "nombre".into(),
            "nombre".into(),
            "".into(),
            "apellido".into(),
        ]);
        assert_eq!(salida, vec!["nombre", "nombre_2", "columna", "apellido"]);
    }

    #[test]
    fn parsea_csv_con_valor_vacio() {
        let filas = parsear_csv("nombre;importe\nAna;100\n;200\n").unwrap();
        assert_eq!(filas.len(), 2);
        assert_eq!(
            filas[0].valores.get("nombre").unwrap().as_deref(),
            Some("Ana")
        );
        assert_eq!(filas[1].valores.get("nombre"), Some(&None));
    }

    #[test]
    fn parsea_json_anidado() {
        let json = r#"[{"id":1,"cliente":{"nombre":"Ana"}},{"id":2,"cliente":{"nombre":"Luis"}}]"#;
        let filas = parsear_plano(&fuente_de_prueba(TipoFuente::Json), json.as_bytes()).unwrap();
        assert_eq!(filas.len(), 2);
        assert_eq!(
            filas[0].valores.get("cliente.nombre").unwrap().as_deref(),
            Some("Ana")
        );
    }

    #[test]
    fn parsea_json_con_envoltorio_de_api() {
        let json = r#"{"data":[{"sku":"A1","q":2},{"sku":"B2","q":5}]}"#;
        let filas = parsear_plano(&fuente_de_prueba(TipoFuente::Web), json.as_bytes()).unwrap();
        assert_eq!(filas.len(), 2);
        assert_eq!(filas[1].valores.get("sku").unwrap().as_deref(), Some("B2"));
    }

    #[test]
    fn parsea_xml() {
        let xml = r#"<catalogo><producto id="1"><nombre>Teclado</nombre><precio>25</precio></producto><producto id="2"><nombre>Raton</nombre><precio>15</precio></producto></catalogo>"#;
        let filas = parsear_xml(xml).unwrap();
        assert_eq!(filas.len(), 2);
        assert_eq!(
            filas[0].valores.get("producto.nombre").unwrap().as_deref(),
            Some("Teclado")
        );
    }

    #[test]
    fn extrae_ruta_json() {
        let v: serde_json::Value = serde_json::json!({"data":{"items":[{"a":1}]}});
        let extraido = extraer_de_ruta(&v, "data.items");
        assert_eq!(extraido.as_array().unwrap().len(), 1);
    }

    #[test]
    fn url_de_conexion_escapa_credenciales() {
        let mut fuente = fuente_de_prueba(TipoFuente::Postgres);
        fuente.config.host = "servidor local".into();
        fuente.config.usuario = "usuario".into();
        fuente.config.contrasena = "clave@con:caracteres".into();
        fuente.config.base_datos = "mi base".into();
        let url = url_sqlx(&fuente).unwrap();
        assert!(url.starts_with("postgres://usuario:"));
        assert!(url.contains("servidor%20local"));
        assert!(!url.contains(' '));
    }

    #[test]
    fn marcadores_escapados_en_tsql() {
        let sql = sustituir_marcadores(
            "WHERE a = ? AND b LIKE ?",
            &["O'Brien".into(), "%x%".into()],
        );
        assert_eq!(sql, "WHERE a = 'O''Brien' AND b LIKE '%x%'");
    }
}

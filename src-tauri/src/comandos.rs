//! Capa de comandos IPC de DataSearch.
//!
//! Cada funcion `#[tauri::command]` es la unica puerta de entrada del
//! frontend al nucleo. Todas exigen un token de sesion valido y comprueban los
//! permisos del usuario antes de tocar datos.
//!
//! Copyright (c) 2026 Jose Manual Bernabeu Mejias
//! Licencia MIT

use datasearch_core::appdb::AppDb;
use datasearch_core::auth::{Contexto, ServicioAuth};
use datasearch_core::error::{Error, Resultado};
use datasearch_core::export;
use datasearch_core::models::{
    self, Credenciales, Documento, Estado, FormatoExportacion, ImagenDocumento, Permisos,
    PeticionAnalisis, PeticionBusqueda, PeticionDashboard, Rol, SolicitudFuente, SolicitudRegistro,
    Usuario,
};
use datasearch_core::service::ServicioBusqueda;
use datasearch_core::sources::ServicioFuentes;
use datasearch_core::update;
use datasearch_core::{NOMBRE_APP, VERSION};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};

/// Estado compartido por todos los comandos.
pub struct AppState {
    pub db: Arc<AppDb>,
    pub auth: Arc<ServicioAuth>,
    pub fuentes: Arc<ServicioFuentes>,
    pub busqueda: Arc<ServicioBusqueda>,
    /// Ruta de la base de datos interna, para el guardado de exportaciones.
    pub ruta_db: PathBuf,
    pub carpeta_descargas: PathBuf,
}

impl AppState {
    /// Crea el estado y asegura que el administrador por defecto existe.
    pub fn nuevo() -> Resultado<Self> {
        let base = crate::rutas::directorio_datos();
        std::fs::create_dir_all(&base)?;
        let ruta_db = base.join("datasearch.db");
        let db = Arc::new(AppDb::abrir(&ruta_db)?);
        // Las sesiones caducadas se limpian en cada arranque.
        let _ = db.purgar_sesiones();
        let auth = Arc::new(ServicioAuth::nuevo(db.clone(), VERSION));
        Ok(Self {
            fuentes: Arc::new(ServicioFuentes::nuevo(db.clone())),
            busqueda: Arc::new(ServicioBusqueda::nuevo(db.clone())),
            auth,
            db,
            ruta_db,
            carpeta_descargas: crate::rutas::carpeta_descargas(),
        })
    }
}

/// Macro para obtener el contexto autorizado a partir del token.
macro_rules! contexto {
    ($estado:expr, $token:expr) => {
        $estado.auth.contexto($token.as_str())?
    };
}

// ======================================================================
// Informacion general
// ======================================================================

/// Datos que la interfaz necesita antes de iniciar sesion.
#[tauri::command]
pub fn info_aplicacion() -> serde_json::Value {
    serde_json::json!({
        "nombre": NOMBRE_APP,
        "version": VERSION,
        "titulo": datasearch_core::titulo_con_version(),
        "repositorio": models::REPOSITORIO,
        "licencia": models::LICENCIA,
        "autor": models::AUTOR_NOMBRE,
        "copyright": format!("Copyright (c) 2026 {}", models::AUTOR_NOMBRE),
        "autorFicha": models::ficha_autor(),
        "pie": format!(
            "Copyright (c) 2026 {} - Licencia {} - Repositorio: {}",
            models::AUTOR_NOMBRE,
            models::LICENCIA,
            models::REPOSITORIO
        ),
        "tiposFuente": tipos_fuente(),
        "operadores": operadores(),
        "analisis": datasearch_core::analysis::catalogo_analisis(),
        "formatos": export::formatos_disponibles(),
        "graficos": graficos_disponibles(),
        "usuarioAdminDefecto": models::USUARIO_ADMIN_DEFECTO,
    })
}

fn tipos_fuente() -> Vec<serde_json::Value> {
    use models::TipoFuente::*;
    [
        Sqlite, Postgres, Mysql, SqlServer, MongoDb, Csv, Json, Xml, Excel, Web,
    ]
    .iter()
    .map(|t| {
        serde_json::json!({
            "valor": format!("{t:?}").to_lowercase(),
            "etiqueta": t.etiqueta(),
            "relacional": t.es_relacional(),
            "documental": t.es_documental(),
            "plano": t.es_plano(),
            "puertoPorDefecto": datasearch_core::drivers::config_ejemplo(*t).puerto,
        })
    })
    .collect()
}

fn operadores() -> Vec<serde_json::Value> {
    use models::Operador::*;
    [
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
    ]
    .iter()
    .map(|o| {
        serde_json::json!({
            "valor": format!("{o:?}").to_lowercase(),
            "etiqueta": o.etiqueta(),
            "necesitaSegundo": o.necesita_segundo_valor(),
            "necesitaLista": o.necesita_lista(),
            "unario": o.es_unario(),
        })
    })
    .collect()
}

fn graficos_disponibles() -> Vec<serde_json::Value> {
    use models::TipoGrafico::*;
    [
        Kpi, Barra, Linea, Area, Torta, Dispersion, Tabla, Indicador, MapaCalor,
    ]
    .iter()
    .map(|g| serde_json::json!({ "valor": g.etiqueta(), "nombre": format!("{g:?}") }))
    .collect()
}

// ======================================================================
// Acceso y usuarios
// ======================================================================

/// Registra una cuenta nueva. Nace pendiente de verificacion.
#[tauri::command]
pub fn registrar(
    estado: State<'_, AppState>,
    peticion: SolicitudRegistro,
    autoaprobar: Option<bool>,
) -> Resultado<Usuario> {
    estado
        .auth
        .registrar(&peticion, autoaprobar.unwrap_or(false))
}

/// Indica si un nombre de usuario esta libre.
#[tauri::command]
pub fn usuario_disponible(estado: State<'_, AppState>, usuario: String) -> bool {
    estado.auth.usuario_disponible(usuario.trim())
}

/// Inicia sesion y devuelve el token junto con los datos del usuario.
#[tauri::command]
pub fn iniciar_sesion(
    estado: State<'_, AppState>,
    credenciales: Credenciales,
) -> Resultado<models::Sesion> {
    estado.auth.iniciar_sesion(&credenciales)
}

/// Cierra la sesion actual.
#[tauri::command]
pub fn cerrar_sesion(estado: State<'_, AppState>, token: String) -> Resultado<()> {
    estado.auth.cerrar_sesion(&token)
}

/// Devuelve el usuario asociado a un token, para restaurar la sesion.
#[tauri::command]
pub fn sesion_actual(estado: State<'_, AppState>, token: String) -> Resultado<Usuario> {
    estado.auth.usuario_de_token(&token)
}

/// Lista de administradores predeterminada, mostrada en la pantalla de acceso
/// como recordatorio de las credenciales iniciales.
#[tauri::command]
pub fn credenciales_iniciales() -> serde_json::Value {
    serde_json::json!({
        "usuario": models::USUARIO_ADMIN_DEFECTO,
        "aviso": "Usuario administrador creado en la primera ejecucion. Cambie la contrasena tras el primer acceso.",
    })
}

// ======================================================================
// Panel de administracion
// ======================================================================

#[tauri::command]
pub fn admin_listar_usuarios(
    estado: State<'_, AppState>,
    token: String,
    filtro_estado: Option<Estado>,
    texto: Option<String>,
) -> Resultado<Vec<Usuario>> {
    estado
        .auth
        .admin_listar_usuarios(&token, filtro_estado, texto.as_deref().unwrap_or(""))
}

#[tauri::command]
pub fn admin_actualizar_usuario(
    estado: State<'_, AppState>,
    token: String,
    id_usuario: i64,
    nuevo_estado: Estado,
    nuevo_rol: Rol,
    permisos: Permisos,
    notas: String,
) -> Resultado<Usuario> {
    estado.auth.admin_actualizar_usuario(
        &token,
        id_usuario,
        nuevo_estado,
        nuevo_rol,
        permisos,
        &notas,
    )
}

#[tauri::command]
pub fn admin_eliminar_usuario(
    estado: State<'_, AppState>,
    token: String,
    id_usuario: i64,
) -> Resultado<()> {
    estado.auth.admin_eliminar_usuario(&token, id_usuario)
}

#[tauri::command]
pub fn admin_restablecer_contrasena(
    estado: State<'_, AppState>,
    token: String,
    id_usuario: i64,
    nueva_contrasena: String,
) -> Resultado<()> {
    estado
        .auth
        .admin_restablecer_contrasena(&token, id_usuario, &nueva_contrasena)
}

#[tauri::command]
pub fn admin_detalle_usuario(
    estado: State<'_, AppState>,
    token: String,
    id_usuario: i64,
) -> Resultado<serde_json::Value> {
    estado.auth.admin_detalle_usuario(&token, id_usuario)
}

#[tauri::command]
pub fn admin_resumen(estado: State<'_, AppState>, token: String) -> Resultado<serde_json::Value> {
    estado.auth.admin_resumen(&token)
}

// ======================================================================
// Cuenta propia
// ======================================================================

/// Datos completos del usuario conectado, para la ventana de credenciales.
#[tauri::command]
pub fn mi_perfil(estado: State<'_, AppState>, token: String) -> Resultado<serde_json::Value> {
    let ctx = contexto!(estado, token);
    let usuario = estado.auth.usuario_de_token(&token)?;
    let fuentes = estado.fuentes.listar(&ctx)?;
    let auditoria = if ctx.es_admin() {
        estado
            .db
            .auditoria_de_usuario(ctx.id_usuario, 40)
            .unwrap_or_default()
    } else if usuario.permisos.ver_auditoria {
        estado
            .db
            .auditoria_de_usuario(ctx.id_usuario, 40)
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    Ok(serde_json::json!({
        "usuario": usuario,
        "fuentes": fuentes,
        "auditoria": auditoria,
        "sesionesActivas": estado.db.conteo_sesiones(ctx.id_usuario)?,
        "versionApp": VERSION,
        "fichaAutor": models::ficha_autor(),
    }))
}

/// Actualiza los datos personales de la cuenta conectada.
#[tauri::command]
pub fn actualizar_mi_perfil(
    estado: State<'_, AppState>,
    token: String,
    datos: models::DatosPersonales,
) -> Resultado<Usuario> {
    let ctx = contexto!(estado, token);
    let faltan = datos.validar();
    if !faltan.is_empty() {
        return Err(Error::DatosIncompletos(faltan.join(", ")));
    }
    estado.db.actualizar_datos(ctx.id_usuario, &datos)?;
    estado.db.auditar(
        ctx.id_usuario,
        "perfil",
        "Datos personales actualizados",
        true,
    );
    estado.auth.usuario_de_token(&token)
}

#[tauri::command]
pub fn cambiar_contrasena(
    estado: State<'_, AppState>,
    token: String,
    actual: String,
    nueva: String,
) -> Resultado<()> {
    estado.auth.cambiar_contrasena(&token, &actual, &nueva)
}

// ======================================================================
// Fuentes de datos
// ======================================================================

#[tauri::command]
pub fn listar_fuentes(
    estado: State<'_, AppState>,
    token: String,
) -> Resultado<Vec<models::Fuente>> {
    let ctx = contexto!(estado, token);
    estado.fuentes.listar(&ctx)
}

#[tauri::command]
pub fn crear_fuente(
    estado: State<'_, AppState>,
    token: String,
    peticion: SolicitudFuente,
) -> Resultado<models::Fuente> {
    let ctx = contexto!(estado, token);
    estado.fuentes.crear(&ctx, &peticion)
}

#[tauri::command]
pub fn actualizar_fuente(
    estado: State<'_, AppState>,
    token: String,
    id_fuente: i64,
    peticion: SolicitudFuente,
) -> Resultado<models::Fuente> {
    let ctx = contexto!(estado, token);
    estado.fuentes.actualizar(&ctx, id_fuente, &peticion)
}

#[tauri::command]
pub fn eliminar_fuente(
    estado: State<'_, AppState>,
    token: String,
    id_fuente: i64,
) -> Resultado<()> {
    let ctx = contexto!(estado, token);
    estado.fuentes.eliminar(&ctx, id_fuente)
}

#[tauri::command]
pub async fn probar_fuente(
    estado: State<'_, AppState>,
    token: String,
    id_fuente: i64,
) -> Resultado<String> {
    let ctx = contexto!(estado, token);
    estado.fuentes.probar(&ctx, id_fuente).await
}

#[tauri::command]
pub async fn esquemas_de_fuente(
    estado: State<'_, AppState>,
    token: String,
    id_fuente: i64,
) -> Resultado<Vec<models::Esquema>> {
    let ctx = contexto!(estado, token);
    estado.fuentes.esquemas(&ctx, id_fuente).await
}

/// Filtros sugeridos a partir de una muestra de los datos.
#[tauri::command]
pub async fn sugerir_filtros(
    estado: State<'_, AppState>,
    token: String,
    id_fuente: i64,
    esquema: String,
) -> Resultado<serde_json::Value> {
    let ctx = contexto!(estado, token);
    let peticion = PeticionBusqueda {
        fuentes: vec![id_fuente],
        esquemas: if esquema.trim().is_empty() {
            Vec::new()
        } else {
            vec![esquema]
        },
        limite: 200,
        contar_total: false,
        ..Default::default()
    };
    let respuesta = estado.busqueda.buscar(&ctx, &peticion).await?;
    Ok(serde_json::json!({
        "columnas": respuesta
            .bloques
            .iter()
            .flat_map(|b| b.columnas.clone())
            .collect::<Vec<_>>(),
        "sugerencias": respuesta.sugerencias,
        "valoresMuestra": respuesta
            .bloques
            .iter()
            .flat_map(|b| b.filas.clone())
            .take(20)
            .collect::<Vec<_>>(),
    }))
}

// ======================================================================
// Busqueda
// ======================================================================

/// Unico punto por el que se leen datos: solo se ejecuta al pulsar Buscar.
#[tauri::command]
pub async fn buscar(
    app: AppHandle,
    estado: State<'_, AppState>,
    token: String,
    peticion: PeticionBusqueda,
) -> Resultado<models::RespuestaBusqueda> {
    let ctx = contexto!(estado, token);
    let total_fuentes = peticion.fuentes.len();
    let _ = app.emit("busqueda:iniciada", total_fuentes);
    let respuesta = estado.busqueda.buscar(&ctx, &peticion).await?;
    let _ = app.emit("busqueda:terminada", respuesta.total_filas);
    Ok(respuesta)
}

/// Muestra una tabla concreta, para la vista de detalle de un bloque.
#[tauri::command]
pub async fn mostrar_datos(
    estado: State<'_, AppState>,
    token: String,
    id_fuente: i64,
    esquema: String,
    limite: Option<u32>,
) -> Resultado<Documento> {
    let ctx = contexto!(estado, token);
    estado
        .busqueda
        .mostrar_datos(&ctx, id_fuente, &esquema, limite.unwrap_or(200))
        .await
}

// ======================================================================
// Tableros de control
// ======================================================================

/// Construye el tablero solicitado. Acepta la caja de texto y el desplegable.
#[tauri::command]
pub async fn crear_dashboard(
    app: AppHandle,
    estado: State<'_, AppState>,
    token: String,
    peticion: PeticionDashboard,
) -> Resultado<models::Tablero> {
    let ctx = contexto!(estado, token);
    let _ = app.emit("dashboard:iniciado", peticion.instruccion.clone());
    let tablero = estado.busqueda.crear_dashboard(&ctx, &peticion).await?;
    let _ = app.emit("dashboard:terminado", tablero.widgets.len());
    Ok(tablero)
}

/// Refresca un widget concreto del tablero.
#[tauri::command]
pub async fn recalcular_widget(
    estado: State<'_, AppState>,
    token: String,
    widget: models::Widget,
) -> Resultado<models::WidgetListo> {
    let ctx = contexto!(estado, token);
    estado.busqueda.calcular_widget(&ctx, &widget, &[]).await
}

// ======================================================================
// Analisis
// ======================================================================

/// Ejecuta el analisis. Exige que el usuario haya pulsado el boton de
/// confirmacion: sin `esAnalisisConfirmado` no se calcula nada.
#[tauri::command]
pub async fn analizar(
    app: AppHandle,
    estado: State<'_, AppState>,
    token: String,
    peticion: PeticionAnalisis,
) -> Resultado<models::RespuestaAnalisis> {
    let ctx = contexto!(estado, token);
    if !peticion.es_analisis_confirmado {
        let _ = app.emit(
            "analisis:requiere_confirmacion",
            peticion.comentario.clone(),
        );
        return Err(Error::Validacion(
            "Pulse 'Confirmar y analizar' para ejecutar el analisis".into(),
        ));
    }
    let _ = app.emit("analisis:iniciado", peticion.analyses.clone());
    let respuesta = estado.busqueda.analizar(&ctx, &peticion).await?;
    let _ = app.emit("analisis:terminado", respuesta.secciones.len());
    Ok(respuesta)
}

// ======================================================================
// Exportacion
// ======================================================================

/// Prepara un documento a partir de los resultados o del tablero en pantalla.
#[tauri::command]
pub fn preparar_documento(
    token: String,
    titulo: String,
    subtitulo: String,
    origen: Option<Documento>,
) -> Resultado<Documento> {
    let _ = &token;
    let base = origen.unwrap_or(Documento {
        titulo: String::new(),
        subtitulo: String::new(),
        pie: String::new(),
        bloques: Vec::new(),
        secciones: Vec::new(),
        widgets: Vec::new(),
        imagenes: Vec::new(),
        generado_por: String::new(),
        generado_en: String::new(),
        duracion_ms: 0,
    });
    Ok(Documento {
        titulo: if titulo.trim().is_empty() {
            format!("Informe de {NOMBRE_APP}")
        } else {
            titulo
        },
        subtitulo,
        generado_en: datasearch_core::appdb::ahora_iso(),
        ..base
    })
}

/// Exporta el documento en el formato pedido y devuelve el contenido en base64
/// junto con el nombre de fichero, para que el frontend lo descargue.
#[tauri::command]
pub fn exportar(
    estado: State<'_, AppState>,
    token: String,
    documento: Documento,
    formato: String,
) -> Resultado<models::ArchivoExportado> {
    let ctx = contexto!(estado, token);
    ctx.exigir(datasearch_core::auth::Permiso::Exportar)?;
    if export::documento_vacio(&documento) {
        return Err(Error::Exportacion(
            "No hay datos que exportar. Pulse primero el boton Buscar.".into(),
        ));
    }
    let documento = Documento {
        generado_por: if documento.generado_por.is_empty() {
            ctx.usuario.clone()
        } else {
            documento.generado_por
        },
        generado_en: if documento.generado_en.is_empty() {
            datasearch_core::appdb::ahora_iso()
        } else {
            documento.generado_en
        },
        ..documento
    };
    let archivo = export::exportar(&documento, &formato)?;
    estado.db.auditar(
        ctx.id_usuario,
        "exporta",
        &format!("{} ({} bytes)", archivo.nombre, archivo.tamano_bytes),
        true,
    );
    Ok(archivo)
}

/// Prepara un HTML listo para imprimir desde el navegador integrado.
#[tauri::command]
pub fn preparar_impresion(
    estado: State<'_, AppState>,
    token: String,
    documento: Documento,
) -> Resultado<String> {
    let ctx = contexto!(estado, token);
    ctx.exigir(datasearch_core::auth::Permiso::Imprimir)?;
    let documento = Documento {
        generado_por: ctx.usuario.clone(),
        generado_en: datasearch_core::appdb::ahora_iso(),
        ..documento
    };
    let bytes = export::a_html(&documento)?;
    String::from_utf8(bytes).map_err(|e| Error::Exportacion(e.to_string()))
}

/// Ruta sugerida para guardar una exportacion.
#[tauri::command]
pub fn carpeta_destino(estado: State<'_, AppState>) -> String {
    estado.carpeta_descargas.to_string_lossy().to_string()
}

// ======================================================================
// Actualizaciones
// ======================================================================

/// Comprueba si hay una version nueva publicada en GitHub.
#[tauri::command]
pub async fn comprobar_actualizaciones() -> models::InfoVersion {
    update::comprobar_actualizaciones().await
}

/// Descarga el fichero de la version nueva y devuelve su ruta local.
///
/// En Linux se deja el `.deb` en la carpeta de descargas del usuario y se
/// abre, para que el instalador del sistema la aplique.
#[tauri::command]
pub async fn descargar_actualizacion(
    app: AppHandle,
    url: String,
    nombre_fichero: Option<String>,
) -> Resultado<serde_json::Value> {
    let carpeta = crate::rutas::carpeta_descargas();
    std::fs::create_dir_all(&carpeta)?;
    let nombre = nombre_fichero.unwrap_or_else(|| {
        url.rsplit('/')
            .next()
            .filter(|s| !s.is_empty())
            .unwrap_or("DataSearch-actualizacion.deb")
            .to_string()
    });
    let destino = carpeta.join(&nombre);
    let ruta_destino = destino.to_string_lossy().to_string();

    let _ = app.emit("actualizacion:inicio", ruta_destino.clone());
    let cliente = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(900))
        .user_agent(format!("{NOMBRE_APP}/{VERSION}"))
        .build()
        .map_err(|e| Error::Red(e.to_string()))?;
    let respuesta = cliente
        .get(&url)
        .send()
        .await
        .map_err(|e| Error::Red(format!("No se pudo iniciar la descarga: {e}")))?;
    if !respuesta.status().is_success() {
        return Err(Error::Red(format!(
            "La descarga fallo con el estado {}",
            respuesta.status()
        )));
    }

    let destino = std::fs::File::create(&destino)
        .map_err(|e| Error::Red(format!("No se pudo crear el fichero: {e}")))?;
    let mut total = 0u64;
    let mut flujo = respuesta.bytes_stream();
    use futures_util::StreamExt;
    let mut escritor = destino;
    while let Some(trozo) = flujo.next().await {
        let trozo = trozo.map_err(|e| Error::Red(e.to_string()))?;
        use std::io::Write;
        escritor
            .write_all(&trozo)
            .map_err(|e| Error::Red(format!("No se pudo escribir el fichero: {e}")))?;
        total += trozo.len() as u64;
        let _ = app.emit("actualizacion:progreso", total);
    }
    escritor.flush().map_err(|e| Error::Red(e.to_string()))?;
    let _ = app.emit("actualizacion:fin", total);

    Ok(serde_json::json!({
        "ruta": ruta_destino,
        "nombre": nombre,
        "bytes": total,
        "esDeb": nombre.to_lowercase().ends_with(".deb"),
    }))
}

/// Abre el fichero descargado con la aplicacion por defecto del sistema.
#[tauri::command]
pub fn abrir_actualizacion(app: AppHandle, ruta: String) -> Resultado<()> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_path(ruta, None::<&str>)
        .map_err(|e| Error::Interno(format!("No se pudo abrir el instalador: {e}")))
}

/// Cierra la aplicacion para que el usuario pueda reiniciarla.
#[tauri::command]
pub fn cerrar_para_actualizar(app: AppHandle) -> Resultado<()> {
    app.exit(0);
    Ok(())
}

// ======================================================================
// Registro en el arranque
// ======================================================================

/// Muestra la ventana principal cuando el frontend ya esta listo.
pub fn mostrar_principal(app: &AppHandle) {
    if let Some(ventana) = app.get_webview_window("principal") {
        let _ = ventana.show();
        let _ = ventana.set_focus();
    }
}

#[tauri::command]
pub fn mostrar_ventana_principal(app: AppHandle) {
    mostrar_principal(&app);
}

/// Abre la ventana de actualizacion con los datos de la nueva version.
pub fn abrir_ventana_actualizacion(app: &AppHandle, info: &models::InfoVersion) {
    use tauri::WebviewUrl;
    if let Some(ventana) = app.get_webview_window("actualizacion") {
        let _ = ventana.show();
        let _ = ventana.set_focus();
        let _ = ventana.emit("actualizacion:datos", info);
        return;
    }
    let url = WebviewUrl::App("actualizacion.html".into());
    match tauri::WebviewWindowBuilder::new(app, "actualizacion", url)
        .title("Actualizacion de DataSearch")
        .inner_size(640.0, 560.0)
        .resizable(true)
        .center()
        .build()
    {
        Ok(ventana) => {
            let _ = ventana.emit("actualizacion:datos", info);
        }
        Err(e) => eprintln!("[DataSearch] No se pudo abrir la ventana de actualizacion: {e}"),
    }
}

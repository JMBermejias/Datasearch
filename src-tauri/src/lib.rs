//! DataSearch - aplicacion de extraccion, filtrado y analisis de datos.
//!
//! Copyright (c) 2026 Jose Manuel Bernabeu Mejias
//! Licencia MIT

pub mod comandos;
pub mod rutas;

use tauri::{Emitter, Manager};

/// Ejecuta la aplicacion.
///
/// La ventana principal se declara oculta en `tauri.conf.json`; el frontend la
/// muestra cuando ha terminado de pintar, de modo que nunca se ve una pantalla
/// en blanco ni un destello.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            // Se arranca la comprobacion de actualizaciones en segundo plano.
            // Si no hay conexion, la aplicacion sigue funcionando igual.
            if let Some(ventana) = app.get_webview_window("principal") {
                let _ = ventana.hide();
                // En escritorio se abre maximizada; en movil la ventana ya
                // ocupa toda la pantalla.
                #[cfg(desktop)]
                let _ = ventana.maximize();
            }
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                comprobar_actualizaciones(handle).await;
            });
            Ok(())
        })
        .on_window_event(|ventana, evento| {
            // Al cerrarse la ventana principal se cierra toda la aplicacion.
            if matches!(evento, tauri::WindowEvent::Destroyed) && ventana.label() == "principal" {
                std::process::exit(0);
            }
        })
        .invoke_handler(tauri::generate_handler![
            comandos::info_aplicacion,
            comandos::credenciales_iniciales,
            comandos::registrar,
            comandos::usuario_disponible,
            comandos::iniciar_sesion,
            comandos::cerrar_sesion,
            comandos::sesion_actual,
            comandos::mostrar_ventana_principal,
            comandos::mi_perfil,
            comandos::actualizar_mi_perfil,
            comandos::cambiar_contrasena,
            comandos::admin_listar_usuarios,
            comandos::admin_actualizar_usuario,
            comandos::admin_eliminar_usuario,
            comandos::admin_restablecer_contrasena,
            comandos::admin_detalle_usuario,
            comandos::admin_resumen,
            comandos::listar_fuentes,
            comandos::crear_fuente,
            comandos::actualizar_fuente,
            comandos::eliminar_fuente,
            comandos::probar_fuente,
            comandos::esquemas_de_fuente,
            comandos::sugerir_filtros,
            comandos::buscar,
            comandos::mostrar_datos,
            comandos::crear_dashboard,
            comandos::recalcular_widget,
            comandos::analizar,
            comandos::preparar_documento,
            comandos::exportar,
            comandos::preparar_impresion,
            comandos::carpeta_destino,
            comandos::comprobar_actualizaciones,
            comandos::descargar_actualizacion,
            comandos::abrir_actualizacion,
            comandos::cerrar_para_actualizar,
        ])
        .manage(
            comandos::AppState::nuevo()
                .unwrap_or_else(|e| panic!("no se pudo inicializar DataSearch: {e}")),
        )
        .build(tauri::generate_context!())
        .expect("no se pudo iniciar la aplicacion")
        .run(|_app, _evento| {
            // La aplicacion se cierra al cerrar la ventana principal.
        });
}

/// Comprueba si hay una version nueva publicada en GitHub.
///
/// Si la hay, abre la ventana de actualizacion con los datos de la release.
pub async fn comprobar_actualizaciones(app: tauri::AppHandle) {
    let info = datasearch_core::update::comprobar_actualizaciones().await;
    if info.hay_actualizacion {
        let _ = app.emit("actualizacion:disponible", &info);
        comandos::abrir_ventana_actualizacion(&app, &info);
    } else {
        let _ = app.emit("actualizacion:sin_novedades", &info);
    }
}

/// Ficha del autor, usada por la ventana de credenciales.
pub fn ficha_autor() -> serde_json::Value {
    datasearch_core::models::ficha_autor()
}

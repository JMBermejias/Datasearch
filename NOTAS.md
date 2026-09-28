# DataSearch 1.0.1

**Copyright (c) 2026 Jose Manuel Bernabeu Mejias · Licencia MIT**

Primera version publica de DataSearch, aplicacion para extraer, filtrar,
analizar y visualizar datos de cualquier origen.

## Instalacion

### Linux (Zorin OS)

```bash
sudo apt install ./DataSearch_1.0.0_amd64.deb
```

La aplicacion queda en el menu de aplicaciones y se puede lanzar con
`datasearch-tauri`.

### Android

Copie el fichero `.apk` en el telefono y abralo.

El APK **no esta firmado con una clave de Google Play**, por lo que Android
lo marcara como de origen desconocido. Para instalarlo:

1. Abra **Ajustes → Aplicaciones → Acceso especial → Instalar aplicaciones
   desconocidas** (en Android 8 o posterior puede estar en **Ajustes →
   Seguridad**).
2. Elija la aplicacion desde la que abre el fichero (archivos o navegador).
3. Active **Permitir de esta fuente**.
4. Vuelva a abrir el `.apk`.

Requiere Android 7.0 (API 24) o superior y unos 60 MB de espacio.

Consulte [`docs/INSTALACION.md`](docs/INSTALACION.md) para el detalle de la
instalacion en Linux y en Android.

## Primer acceso

| Dato | Valor |
|------|-------|
| Usuario | `JMBernabeu` |
| Contrasena | la definida por el autor |

Cambie la contrasena en cuanto entre: **Credenciales → Cambiar contrasena**.

El resto de usuarios se registran indicando todos sus datos personales; la
cuenta queda pendiente hasta que el administrador la verifica.

## Correcciones de esta version

- **Se corrige el icono del escritorio.** El fichero `.desktop` declaraba
  `Icon=datasearch` pero los iconos se instalaban como `datasearch-tauri.png`,
  de modo que el nombre no coincidia y Zorin OS mostraba el icono generico.
  Ahora el ejecutable se llama `datasearch` y tanto el icono como la linea
  `Exec` usan ese mismo nombre.
- La plantilla `.desktop` usa ya las variables de Tauri (`{{exec}}` e
  `{{icon}}`) en lugar de valores fijos, de modo que la coincidencia se
  mantiene aunque cambie el nombre del binario.

## Novedades de esta version

- Panel lateral azul claro con navegacion y zona de trabajo blanca.
- Buscador con fuentes y esquemas multiples, texto libre y hasta 21
  operadores de filtrado combinables con Y / O.
- Desplegable de filtrado con buscador y sugerencias automaticas.
- Generador de tableros a partir de una peticion en lenguaje natural o del
  desplegable de esquemas.
- Analisis estadistico con boton de confirmacion obligatorio.
- Exportacion a Excel, PDF, CSV, JSON, HTML y texto plano, mas impresion.
- Registro de usuarios con verificacion y permisos por parte del
  administrador.
- Comprobacion de actualizaciones al arrancar, con descarga e instalacion
  automatica.

## Novedades

El changelog completo de cada version se genera automaticamente.

---

Copyright (c) 2026 Jose Manuel Bernabeu Mejias
Calle Medico Rafael Navarro 2, 2 C - 03660 Novelda, Alicante, Espana

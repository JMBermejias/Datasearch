# Instalación de DataSearch

**Copyright (c) 2026 Jose Manuel Bernabeu Mejias · Licencia MIT**

Los paquetes se descargan de la página de
[*Releases*](https://github.com/JMBermejias/Datasearch/releases).

---

## Linux (Zorin OS)

### Opción 1 · Gráfica (recomendada)

1. Abra la [*release*](https://github.com/JMBermejias/Datasearch/releases/latest).
2. Descargue `DataSearch_<version>_amd64.deb`.
3. En Zorin OS puede abrir el fichero con **Gestor de software** y pulsar
   **Instalar**, o con **Gestor de archivos** y seleccionarlo.
4. Si el sistema pide una contraseña, introduzca la de su usuario: se necesita
   para instalar programas.

### Opción 2 · Terminal

```bash
sudo apt install ./DataSearch_1.0.0_amd64.deb
```

La aplicación aparece en el menú de aplicaciones bajo el nombre **DataSearch**
y también se puede lanzar desde la terminal:

```bash
datasearch
```

### Desinstalación

```bash
sudo apt remove datasearch
```

Los datos de la aplicación (cuentas, fuentes, auditoría) quedan en:

```
~/.local/share/com.jmbernabu.datasearch/datasearch.db
```

Para borrarlos también, elimine ese directorio.

### Problemas frecuentes

**«No se puede abrir el archivo .deb»**
El fichero se ha descargado parcialmente. Vuelva a descargarlo.

**Faltan librerías del sistema**

```bash
sudo apt install libwebkit2gtk-4.1-0 libgtk-3-0 libayatana-appindicator3-1
```

**El sistema avisa de que el paquete no es de una fuente fiable**
Es el aviso normal de APT al instalar un programa que no viene de los
repositorios oficiales. Puede continuar si conoce la procedencia del fichero.

---

## Android

1. Descargue el fichero `.apk` de la [*release*](https://github.com/JMBermejias/Datasearch/releases/latest).
2. Páselo al teléfono (por cable, tarjeta SD, Bluetooth o descarga directa).
3. Ábralo desde el explorador de archivos o desde el navegador.

### Si Android bloquea la instalación

El APK no está firmado con una clave de Google Play, de modo que el sistema lo
marca como de origen desconocido. Para permitirlo:

1. Abra **Ajustes → Aplicaciones → Acceso especial → Instalar aplicaciones
   desconocidas**.
2. Elija la aplicación desde la que abre el fichero (archivos o navegador).
3. Active el interruptor **Permitir de esta fuente**.
4. Vuelva a abrir el `.apk`.

> En Android 8 o superior el menú puede llamarse **«Instalar apps
> desconocidas»** y estar dentro de **Ajustes → Seguridad**.

### Requisitos

- Android 7.0 (API 24) o superior.
- Arquitectura `arm64-v8a` (casi todos los teléfonos actuales), `armeabi-v7a`
  (teléfonos antiguos) o `x86_64` (emuladores).
- Unos 60 MB de espacio.

---

## Verificar la integridad del APK

Si dispone de la clave pública del autor, puede comprobar la firma antes de
instalar. Con `apksigner`:

```bash
apksigner verify --verbose DataSearch_1.0.0.apk
```

---

## Problemas de conexion con bases de datos

| Síntoma | Causa habitual |
|---------|----------------|
| «Se agotó el tiempo de espera» | La direccion o el puerto no son correctos, o un cortafuegos bloquea la conexion. |
| «El servidor ha rechazado la conexion» | La base de datos exige TLS y no se ha activado la casilla **SSL**. |
| «Acceso denegado para el usuario» | Usuario o contrasena incorrectos. |
| La consulta no devuelve filas | Los filtros son demasiado restrictivos, o la tabla esta vacia. Pruebe a quitar los filtros. |
| Fichero SQLite: «no se encuentra el fichero» | La ruta debe ser absoluta, por ejemplo `/home/usuario/datos.db`. |

---

*Copyright (c) 2026 Jose Manuel Bernabeu Mejias. Todos los derechos
reservados. Licencia MIT.*

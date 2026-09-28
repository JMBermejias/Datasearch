# DataSearch

**Copyright (c) 2026 Jose Manuel Bernabeu Mejias · Licencia MIT**

Aplicación de escritorio y móvil para **extraer, filtrar, analizar y
visualizar datos** procedentes de cualquier base de datos local, remota o
publicada en la web.

Calle Medico Rafael Navarro 2, 2 C — 03660 Novelda, Alicante, España
Repositorio: <https://github.com/JMBermejias/Datasearch>

---

## Índice

- [Qué hace](#qué-hace)
- [Instalación](#instalación)
- [Primer acceso](#primer-acceso)
- [Guía de uso](#guía-de-uso)
- [Tipos de fuente de datos](#tipos-de-fuente-de-datos)
- [Sistema de filtrado](#sistema-de-filtrado)
- [Panel de control](#panel-de-control)
- [Análisis de datos](#análisis-de-datos)
- [Exportación e impresión](#exportación-e-impresión)
- [Usuarios, roles y permisos](#usuarios-roles-y-permisos)
- [Actualizaciones](#actualizaciones)
- [Compilación desde el código](#compilación-desde-el-código)
- [Arquitectura del proyecto](#arquitectura-del-proyecto)
- [Licencia](#licencia)

---

## Qué hace

DataSearch se abre en una ventana con un **panel lateral azul claro** de
navegación y una **zona de trabajo blanca**. En esa zona se encuentra el
**panel de control** con todo lo necesario para trabajar con los datos:

- **Panel de control** con el generador de tableros y los accesos a todas las
  secciones.
- **Buscador** con selección múltiple de fuentes y esquemas, búsqueda en texto
  libre y un **sistema de filtrado desplegable** combinable con Y / O.
- **Generador de tableros** que crea el panel de control que usted pida, con
  una caja de texto en lenguaje natural o mediante el desplegable de esquemas.
- **Análisis** estadístico que solo se ejecuta tras pulsar el botón de
  confirmación.
- **Fuentes de datos**: añada tantas como necesite.
- **Credenciales**: sus datos y los del autor, con opción de editarlos.
- **Usuarios y permisos**: solo para el administrador del sistema.

**Ningún dato se lee de una fuente hasta que usted pulsa «Buscar».**

---

## Instalación

Descargue el instalador desde la sección
[*Releases*](https://github.com/JMBermejias/Datasearch/releases).

### Linux (Zorin OS y otras distribuciones)

```bash
# 1. Descargue el fichero .deb desde la release
wget https://github.com/JMBermejias/Datasearch/releases/latest/download/DataSearch_amd64.deb

# 2. Instálelo con su gestor de paquetes
sudo apt install ./DataSearch_amd64.deb
```

La aplicación queda disponible en el menú de aplicaciones y también en la
terminal:

```bash
datasearch-tauri
```

Dependencias del sistema (ya incluidas normalmente en Zorin OS):

```bash
sudo apt install libwebkit2gtk-4.1-0 libgtk-3-0 libayatana-appindicator3-1
```

### Android

Descargue el fichero `.apk` de la release, cópielo en el teléfono y ábralo.
Como el APK no está firmado con una clave de Google Play, Android lo marcará
como de origen desconocido: active **«Instalar aplicaciones desconocidas»**
para el explorador que lo abra (Ajustes → Aplicaciones → Acceso especial, o
Ajustes → Seguridad según la versión).

Las instrucciones completas están en [docs/INSTALACION.md](docs/INSTALACION.md).

Requisitos: Android 7.0 (API 24) o superior, arquitectura `arm64-v8a` o
`aarch64`.

---

## Primer acceso

La aplicación crea automáticamente la cuenta de administrador en su primera
ejecución:

| Dato        | Valor                          |
|-------------|--------------------------------|
| Usuario     | `JMBernabeu`                   |
| Contraseña  | la definida por el autor       |

> **Importante:** en cuanto entre con esta cuenta, abra **Credenciales →
> Cambiar contrasena** y sustituya la contraseña inicial.

Para registrarse con su propia cuenta:

1. Pulse **Registrarse** en la pantalla de acceso.
2. Introduzca nombre de usuario y contraseña.
3. Complete **todos** sus datos personales: son obligatorios.
4. Envíe la solicitud.

La cuenta queda **pendiente** hasta que el administrador la verifique. Una vez
verificada, accede con su nombre de usuario y su contraseña.

---

## Guía de uso

### 1. Añada sus fuentes de datos

Vaya a **Fuentes de datos → Anadir fuente**. Elija el tipo, indique el
servidor o la ruta y pulse **Anadir y probar**: la aplicación comprueba la
conexión en el momento y avisa del resultado.

### 2. Busque

En **Buscar datos**:

1. Seleccione una o varias fuentes en el desplegable.
2. Seleccione los esquemas (tablas o colecciones) que necesite, o déjelo en
   blanco para consultar todos.
3. Escriba el texto que debe contener. Admite formatos avanzados:
   - `ventas` — busca en todas las columnas de texto.
   - `pais:ES` — solo en la columna `pais`.
   - `"San Javier"` — frase exacta.
   - `pais:ES "San Javier"` — combinación de ambos.
4. Añada los filtros que necesite.
5. Pulse **Buscar**.

Los resultados se muestran en una tabla ordenable por cualquier columna. El
nivel de detalle de cada columna —nulos, distintos y valores más frecuentes—
le ayuda a construir los filtros.

### 3. Exporte o imprima

En la barra de resultados o del tablero hay botones para **Exportar** (Excel,
PDF, CSV, JSON, HTML, texto) e **Imprimir**. Todos los documentos llevan el
pie con el copyright y la licencia.

---

## Tipos de fuente de datos

| Tipo | Descripción | Campos principales |
|------|-------------|---------------------|
| **SQLite** | Fichero de base de datos en el equipo | Ruta del fichero |
| **PostgreSQL** | Servidor local o remoto | Servidor, puerto, usuario, contraseña, base de datos, esquema |
| **MySQL / MariaDB** | Servidor local o remoto | Ídem |
| **SQL Server** | Microsoft SQL Server | Ídem, puerto 1433 |
| **MongoDB** | Base de datos documental | URI, base de datos, colección |
| **CSV** | Fichero plano separado por comas, punto y coma, tabulador o barra | Ruta o URL |
| **JSON** | Fichero JSON (anidado o en array) | Ruta o URL |
| **XML** | Fichero XML | Ruta o URL |
| **Excel** | Libro `.xlsx` | Ruta o URL |
| **Recurso web** | Cualquier API por HTTP o HTTPS | URL, método, cabeceras, cuerpo, ruta JSON, parámetros |

Las **cabeceras HTTP** de las fuentes web se editan como pares
`Nombre: valor` separados por comas y salto de línea, por ejemplo:

```
Authorization: Bearer mi-token
Accept: application/json
```

---

## Sistema de filtrado

El filtrado se combine libremente con **Y** (todas las condiciones) u **O**
(alguna condición). Operadores disponibles:

| Operador | Para qué sirve |
|----------|----------------|
| contiene / no contiene | subcadena, sin distinguir mayúsculas ni acentos |
| empieza por / termina en | prefijos y sufijos |
| es igual a / es distinto de | igualdad exacta |
| es mayor que, mayor o igual, menor, menor o igual | comparaciones numéricas y de fechas |
| está entre | rango con dos límites |
| está / no está en la lista | varios valores separados por comas |
| es nulo / no es nulo | valores ausentes |
| coincide con expresión regular | patrones complejos |
| está vacío / no está vacío | cadenas vacías |
| es verdadero / es falso |booleanos |

**Desplegable de valores.** Cuando una columna tiene pocos valores distintos,
la aplicación ofrece sus valores más frecuentes en un desplegable con
buscador, para que elija sin escribir.

**Sugerencias automáticas.** Tras una búsqueda se proponen filtros relevantes
basados en la frecuencia con la que aparecen los valores. Pulse la
sugerencia que le interese y se aplicará al instante.

---

## Panel de control

En **Panel de control** tiene dos formas de pedir el tablero, y puede
combinarlas:

1. **Caja de texto.** Escriba lo que necesita en lenguaje natural:
   - `ventas por zona en torta`
   - `evolucion mensual del importe`
   - `comparativa por region`
   - `total de importes`

   La aplicación interpreta la petición, elige las columnas adecuadas y
   genera los gráficos.

2. **Desplegable de esquemas.** Seleccione las fuentes y las tablas concretas
   y pulse **Generar panel de control**.

Además puede añadir los gráficos base del panel principal (cifra destacada,
barras, líneas, tabla) y fijar el número máximo de gráficos.

Cada widget se puede **refrescar** y **exportar como imagen PNG** por separado.

---

## Análisis de datos

En **Análisis**:

1. Seleccione la fuente y el esquema.
2. Marque los análisis que necesite.
3. Pulse **Confirmar y analizar** y confirme en el diálogo.

El botón de confirmación es obligatorio: sin él, el núcleo no ejecuta nada.

Análisis disponibles: descripción estadística, valores nulos, valores
distintos, frecuencias, histograma, correlación, valores atípicos, tendencia,
calidad de los datos, filas duplicadas y resumen completo.

---

## Exportación e impresión

| Formato | Contenido |
|---------|-----------|
| **Excel (.xlsx)** | Portada, una hoja por fuente, una por análisis, una por gráfico, con formato y autofiltro |
| **PDF** | Informe paginado con portada, tablas, gráficos incrustados y pie con el copyright |
| **CSV** | Separado por punto y coma, listo para Excel |
| **JSON** | Estructura completa, útil para integrar con otras herramientas |
| **HTML** | Informe listo para el navegador |
| **Texto** | Tablas de ancho fijo |

La **impresión** abre el informe en el navegador del sistema con el diseño
preparado para papel.

---

## Usuarios, roles y permisos

### Registro

Todo usuario nuevo debe registrarse indicando **todos sus datos
personales**. La cuenta queda *pendiente* hasta que el administrador la
verifique.

### Administrador

El administrador puede:

- Ver **todos** los usuarios registrados y sus datos completos.
- **Verificar** cuentas pendientes (activarlas), reactivarlas o **suspenderlas**.
- Asignar el **rol** (usuario o administrador).
- Asignar **permisos** de forma individual:
  consultar, crear, modificar y eliminar fuentes, exportar, imprimir, analizar,
  crear tableros y ver la auditoría.
- **Restablecer** la contraseña de cualquier cuenta, lo que cierra sus sesiones
  abiertas.
- **Eliminar** cuentas.
- Revisar la **actividad** de cada usuario.

El sistema impide eliminar al único administrador activo y protege al
administrador de la sesión frente a suspending o eliminarse a sí mismo.

### Permisos por usuario

| Permiso | Permite |
|---------|---------|
| Consultar datos | Usar el buscador y el panel de control |
| Crear fuentes | Anadir fuentes de datos |
| Modificar fuentes | Editar fuentes existentes |
| Eliminar fuentes | Borrar fuentes |
| Exportar | Descargar resultados en cualquier formato |
| Imprimir | Imprimir informes |
| Analizar | Ejecutar análisis estadísticos |
| Crear tableros | Generar paneles de control |
| Ver auditoría | Consultar el registro de actividad |

Los administradores tienen todos los permisos por defecto.

---

## Actualizaciones

Al arrancar, la aplicación consulta la API de GitHub y compara la versión
publicada con la instalada:

- Si **hay versión nueva**, se abre la ventana de actualización con las
  novedades y un botón para **descargarla e instalarla automáticamente**.
  En Linux se abre el instalador del sistema; en Android se deja el `.apk`
  descargado para instalarlo.
- Si **está actualizada**, la versión se muestra en la parte superior del panel
  lateral.
- Puede comprobarlas en cualquier momento desde el indicador
  **«Actualizar»** del panel lateral o desde **Ajustes → Actualizaciones**.

Si la comprobación falla por falta de conexión, la aplicación sigue
funcionando con normalidad.

---

## Compilación desde el código

### Requisitos

- Rust 1.77 o superior
- Node.js 20 o superior
- Linux: `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`,
  `libayatana-appindicator3-dev`, `librsvg2-dev`, `patchelf`
- Android: Android SDK, NDK y `ANDROID_HOME` configurado

```bash
# Dependencias de Linux
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev

# Dependencias de proyecto
npm install

# Compilar en modo desarrollo
npm run tauri dev

# Generar el paquete de Linux
npm run tauri build

# Generar el APK de Android
npm run tauri android build --apk

# Ejecutar las pruebas del núcleo
cargo test -p datasearch-core
```

---

## Arquitectura del proyecto

```
DataSearch/
├── Cargo.toml                  Espacio de trabajo de Rust
├── package.json                Dependencias del frontend
├── vite.config.ts              Configuración de compilación del frontend
├── index.html                  Ventana principal
├── actualizacion.html          Ventana de actualización
├── src/                        Interfaz de usuario (TypeScript)
│   ├── main.ts                 Punto de entrada, panel lateral y navegación
│   ├── main-actualizacion.ts    Lógica de la ventana de actualización
│   ├── api.ts                  Tipos y comandos IPC
│   ├── estado.ts               Estado global de la aplicación
│   ├── styles.css              Hoja de estilos azul claro
│   ├── graficos.ts             Motor de gráficos en canvas
│   ├── iconos.ts               Iconos SVG en línea
│   ├── ui.ts                   Componentes reutilizables
│   └── paginas/                Acceso, panel, buscar, análisis, fuentes,
│                               credenciales, usuarios y ajustes
├── crates/datasearch-core/     Núcleo de lógica en Rust
│   ├── models.rs               Modelos de dominio
│   ├── appdb.rs                Base de datos interna de la aplicación
│   ├── auth.rs                 Usuarios, sesiones, roles y permisos
│   ├── sources.rs              Registro de fuentes de datos
│   ├── drivers.rs              Conectores (SQLite, PostgreSQL, MySQL,
│   │                           SQL Server, MongoDB, CSV, JSON, XML, Excel, web)
│   ├── query.rs                Construcción de SQL y filtrado en memoria
│   ├── analysis.rs             Análisis estadístico
│   ├── service.rs              Búsqueda, tableros y confirmación
│   ├── export.rs               Exportación a Excel, PDF, CSV, JSON, HTML
│   └── update.rs               Comprobación de actualizaciones
├── src-tauri/                  Aplicación Tauri v2
│   ├── src/comandos.rs         Comandos IPC
│   ├── src/rutas.rs            Rutas del sistema
│   ├── tauri.conf.json         Configuración y empaquetado
│   └── icons/                  Iconos de la aplicación
├── .github/workflows/          Compilación automática y releases
└── docs/                       Documentación adicional
    ├── INSTALACION.md            Instalación en Linux y en Android
    └── SEGURIDAD.md              Cómo se protegen los datos
```

La lógica de negocio no depende de la interfaz: el núcleo Rust se puede
utilizar y probar de forma independiente, tal y como se comprueba con sus
**62 pruebas automáticas**.

---

## Licencia

Distribuida bajo la **licencia MIT**, redactada íntegramente en español.
Consulte el fichero [LICENSE](LICENSE) para el texto completo.

**Copyright (c) 2026 Jose Manuel Bernabeu Mejias.**
Todos los derechos reservados.

Calle Medico Rafael Navarro 2, 2 C · 03660 Novelda · Alicante · España

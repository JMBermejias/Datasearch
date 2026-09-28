# Seguridad de DataSearch

**Copyright (c) 2026 Jose Manuel Bernabeu Mejias · Licencia MIT**

---

## Cómo se guardan las contraseñas

- Las contraseñas **nunca** se almacenan en texto legible.
- Se cifran con **Argon2id**, un algoritmo de derivación-resistant a ataques
  por fuerza bruta, con sal aleatoria por usuario.
- La verificación se hace comparando hashes, nunca descifrando.
- Si un usuario olvida su contraseña, el administrador debe **restablecerla**;
  no existe forma de recuperarla.

## Sesiones

- Al iniciar sesión se genera un token aleatorio de 32 bytes (256 bits) con un
  generador criptográficamente seguro.
- Las sesiones caducan a las **12 horas**.
- Al cambiar o restablecer una contraseña, al suspender una cuenta o al
  eliminarla, **se cierran todas sus sesiones abiertas** de inmediato.
- En cada arranque de la aplicación se purgan las sesiones caducadas.

## Control de acceso

- Toda lectura o escritura pasa por un comando IPC que **exige un token
  válido** y comprueba el permiso concreto de la acción.
- Los administradores tienen todos los permisos; el resto solo accede a lo
  que su rol les concede.
- Las fuentes marcadas como **solo administrador** son invisibles para el
  resto de cuentas.
- Cada usuario solo puede modificar o eliminar **sus propias** fuentes.

## Inyección de SQL

- Los valores que introduce el usuario **nunca** se concatenan en la consulta.
  Se envían siempre como **parámetros enlazados** (`?`), tanto en SQLite como
  en PostgreSQL, MySQL y MongoDB.
- La única excepción inevitable es **SQL Server**, cuyo protocolo TDS no admite
  marcadores de posición con este controlador. Ahí los valores se interpolan
  **después de escaparlos** con la función `escapar_sql`, que duplica las
  comillas simples.
- Los nombres de tabla y columna se comparan contra el catálogo real de la
  base de datos antes de usarse.

## Datos de conexión

- Las contraseñas de las bases de datos se guardan cifradas junto al resto de
  datos de la aplicación, en la base interna.
- **Nunca se devuelven a la interfaz**: la respuesta de «listar fuentes»
  siempre las sustituye por `********`.
- Al editar una fuente sin escribir contraseña, se conserva la anterior.

## Comprobación de actualizaciones

- La consulta a GitHub usa **solo** la API pública de releases, sin token y
  sin datos personales.
- Si no hay conexión, la aplicación **arranca igualmente** y lo indica.
- La descarga se muestra antes de empezar: el usuario decide si instalarla.

## Datos personales

- Al registrarse se solicitan **todos** los datos personales, y la cuenta queda
  **pendiente** hasta que el administrador la verifica.
- Los datos se guardan en la base local de la aplicación
  (`~/.local/share/com.jmbernabu/datasearch.datasearch.db`).
- **No se envía ninguna información a servidores externos.** La única petición
  de red aparte de las bases de datos del usuario es la comprobación de
  actualizaciones en GitHub, que no transmite datos personales.
- La aplicación **no incluye telemetría** de ningún tipo.

## Auditoría

Toda acción relevante queda registrada: altas, accesos fallidos, altas y
bajas de fuentes, búsquedas, exportaciones, cambios de permisos y cambios de
contraseñas. El administrador puede revisar la actividad de cada usuario.

## Denuncia de una vulnerabilidad

Si detecta un problema de seguridad, comuníquelo al autor a través de
<https://github.com/JMBermejias/Datasearch/issues>.

---

*Copyright (c) 2026 Jose Manuel Bernabeu Mejias. Todos los derechos
reservados. Licencia MIT.*

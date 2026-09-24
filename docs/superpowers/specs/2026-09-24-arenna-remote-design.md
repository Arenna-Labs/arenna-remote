# Arenna Remote — diseño v1

Fecha: 2026-09-24 · Estado: aprobado por delegación (el usuario delegó autonomía total)

## 1. Objetivo

Sustituir el script PowerShell que "maquillaba" una instalación oficial de
RustDesk por un **producto propio**: un fork de RustDesk 1.4.9 rebrandeado
como **Arenna Remote** (Arenna Labs S.L.), que se conecta por defecto al
servidor propio de la empresa y se actualiza desde las GitHub Releases de
`Arenna-Labs/arenna-remote`.

### Lo que dijo el usuario
- Rebranding completo, "incluso a nivel de código".
- Servidor propio en la VPS de `rustdesk.arenna38.com`; **no romper el
  servidor actual hasta que lo nuevo funcione perfectamente**.
- Control de actualizaciones basado en las releases de GitHub de este repo.
- Nombre: *Arenna Remote*. Plataformas v1: **Windows x64 y Android**.
- Repo público (AGPL-3.0 + releases descargables sin token).

### Criterios de éxito
1. Un tag `vX.Y.Z` produce automáticamente una GitHub Release con el
   instalador Windows y los APK Android firmados.
2. El cliente Windows y el Android arrancan con marca Arenna (nombre, iconos,
   colores, textos, enlaces, metadatos del ejecutable) y se registran en el
   servidor propio sin configuración manual.
3. Una conexión real entre dos clientes Arenna Remote funciona a través del
   servidor propio.
4. Un cliente Windows instalado con la versión N se actualiza solo a la N+1
   publicada en GitHub, verificando la firma del binario.
5. El servidor queda en rustdesk-server 1.1.16 (Docker, reproducible desde
   `server/`), con la misma key, sin cortar a los clientes existentes.

### Fuera de alcance (v1)
macOS, iOS, Linux, MSI, firma Authenticode (no hay certificado), servidor de
API/libreta de direcciones (21114), publicación en Play Store.

## 2. Estrategia de repositorio

- Rama **`upstream`**: solo snapshots prístinos de RustDesk (con
  `libs/hbb_common` vendorizado y sin `.github/`). Actualizar a una versión
  nueva = nuevo commit en `upstream` + `git merge upstream` en `main`.
- Rama **`main`**: upstream + nuestros cambios, en commits temáticos.
- `branding/`: fuente de marca (logo SVG, tipografía Inter OFL) y
  `generate_icons.py`, que regenera todos los iconos del árbol.
- `server/`: despliegue Docker del servidor y scripts de operación.
- `.github/workflows/release.yml`: único workflow (los de upstream no se
  importan).

## 3. Identidad

| Concepto | Valor |
|---|---|
| Nombre visible | `Arenna Remote` |
| `APP_NAME` interno (rutas, servicio, esquema URI, IPC) | `ArennaRemote` (sin espacios: se usa como esquema URI y en `lang.rs` se asume alfanumérico) |
| Organización | Arenna Labs S.L. — https://arennalabs.com |
| Android `applicationId` y paquete Kotlin | `com.arennalabs.remote` |
| Esquema de enlaces profundos | `arennaremote://` |
| Colores | `#00A850` (acento), `#008257` (botones, contraste AA con blanco), `#21CC5A` |

Decisiones:
- `is_custom_client()` pasa a devolver siempre `false`: en upstream significa
  "cliente Pro configurado con un `custom.txt` firmado por RustDesk", y cuando
  el nombre no es "RustDesk" desactiva actualizaciones, cambia defaults y
  muestra "Powered by RustDesk". Arenna Remote es un producto de primera parte.
- El reemplazo automático "RustDesk" → nombre de app en las traducciones
  (`src/lang.rs`) usará el **nombre visible**.
- Nombres internos invisibles se conservan para no romper el puente FFI ni
  complicar los merges: crate `rustdesk`, librería `librustdesk`, paquete Dart
  `flutter_hbb`, canales `org.rustdesk.*`, JNI `Java_ffi_FFI_*`.
- Avisos legales AGPL: el "Acerca de" muestra © Arenna Labs S.L., "basado en
  RustDesk (© Purslane Tech Pte. Ltd.)", licencia AGPL-3.0 y enlace al código.

### Nombre visible vs. nombre interno
`APP_NAME` (`ArennaRemote`) sigue gobernando todo lo que el sistema usa como
identificador: carpeta de configuración, tubería IPC, nombre y ejecutable del
servicio, carpeta de instalación, esquema URI, clave de desinstalación,
nombre de impresora. Una función nueva `get_app_display_name()` ("Arenna
Remote") se usa solo donde lo ve una persona:
- reemplazo de "RustDesk" en traducciones (`src/lang.rs`);
- títulos de ventana (export `get_rustdesk_app_name` + búsqueda de la ventana
  por título en `core_main.rs`, que deben coincidir);
- accesos directos (`Arenna Remote.lnk`, carpeta del menú Inicio,
  `Desinstalar`/`Uninstall` y el de la bandeja) — creación y borrado usan el
  mismo helper;
- `DisplayName` y `Publisher` ("Arenna Labs S.L.") en Programas y
  características; `DisplayName` del servicio; tooltip de la bandeja;
- Flutter: nueva FFI `main_get_app_display_name_sync` para títulos y textos.

### Coexistencia con RustDesk oficial
Los clientes actuales tienen RustDesk oficial instalado; Arenna Remote debe
poder convivir y nunca tocarlo:
- se elimina la búsqueda de la clave Inno `IS1` de RustDesk (si existiera, el
  instalador desinstalaría RustDesk y borraría su carpeta);
- el directorio de staging deja de ser `C:\ProgramData\RustDesk\...`;
- el broker de privacidad pasa a `RuntimeBroker_arennaremote.exe` (upstream
  hace `taskkill` por nombre);
- el autoextraíble descomprime en `%LOCALAPPDATA%\ArennaRemote`, no en
  `%LOCALAPPDATA%\rustdesk` (lo usa el updater de RustDesk);
- ejecutable `ArennaRemote.exe` (`BINARY_NAME`), librería `librustdesk.dll`
  (interna, sin colisión: vive en su propia carpeta).

### Sin llamadas a infraestructura de RustDesk
- La API cae por defecto a `https://admin.rustdesk.com` cuando el servidor es
  el *default* compilado → se devuelve cadena vacía y se fija el builtin
  `register-device=N` (sin heartbeat/sysinfo; el hbbs OSS no tiene API).
- La comprobación de versión deja de llamar a `api.rustdesk.com`.
- Enlaces de la UI → arennalabs.com / repo de GitHub; se eliminan los de
  precios, documentación y "setup server".
- No se empaqueta el driver de impresora remota: su adaptador solo funciona
  con binarios firmados por RustDesk.

## 4. Servidor

- Por defecto: `RENDEZVOUS_SERVERS = ["rustdesk.arenna38.com"]` y
  `RS_PUB_KEY` = clave pública del servidor actual (se lee de la VPS).
- Se ocultan los ajustes de servidor en el cliente (builtin
  `hide-server-settings`) para que los clientes no los cambien por error.
- Despliegue: `rustdesk/rustdesk-server:1.1.16` con `network_mode: host`,
  `hbbr -k _` (el relay también exige la key), puertos 21115/tcp,
  21116/tcp+udp, 21117/tcp. 21114/21118/21119 cerrados.
- Migración sin corte:
  1. Inventario solo-lectura del despliegue actual y copia de seguridad de
     `id_ed25519*` y la base de datos.
  2. Instancia de prueba en paralelo en 21125–21129 con **copia** de la key.
  3. Validar clientes nuevos contra la instancia de prueba.
  4. Corte: parar el antiguo, arrancar el nuevo en los puertos estándar con
     la misma key y la BD migrada; comprobar que el `Key:` es idéntico.
  5. Rollback documentado (volver a arrancar el antiguo).

## 5. Actualizaciones

- **Versión de producto** separada de la de protocolo: `crate::VERSION`
  sigue siendo `1.4.9` (los peers la usan para activar funciones). La nueva
  `ARENNA_VERSION` se inyecta en compilación desde el tag (`vX.Y.Z` →
  `X.Y.Z`); en builds locales vale `0.0.0`.
- **Comprobación**: petición a
  `https://github.com/Arenna-Labs/arenna-remote/releases/latest`
  siguiendo la redirección hasta `/releases/tag/vX.Y.Z` (sin API → sin límite
  de 60 req/h, sin JSON). Se compara con `ARENNA_VERSION`.
- **Asset Windows**: `arenna-remote-X.Y.Z-x86_64.exe` (instalador
  autoextraíble) + `arenna-remote-X.Y.Z-x86_64.exe.sig`.
- **Firma del update**: CI firma el `.exe` con una clave ed25519 (secreto de
  Actions); el cliente embebe la clave pública y verifica la firma
  (`sodiumoxide`, ya presente) antes de ejecutar nada, tanto en el update
  automático como en el manual. Motivo: el updater ejecuta el binario como
  SYSTEM y el cliente HTTP de upstream acepta certificados inválidos como
  fallback.
- **Windows**: auto-update silencioso activado por defecto en instalaciones
  (el usuario puede desactivarlo); banner "nueva versión" en la home.
- **Android**: aviso "nueva versión" que abre la página de la release (APK
  firmado siempre con la misma keystore → se instala encima).

## 6. Build y releases (GitHub Actions)

Workflow `release.yml` en tags `v*` (y `workflow_dispatch` para builds de
prueba sin publicar):
1. `bridge` (ubuntu-22.04): flutter_rust_bridge 1.80.1.
2. `windows` (windows-2022): Flutter 3.24.5 + motor parcheado de RustDesk,
   Rust 1.75, vcpkg `120deac…` estático, `build.py --flutter --hwcodec
   --vram`, drivers usbmmidd/impresora, empaquetado portable → `.exe`,
   firma ed25519.
3. `android` (ubuntu-24.04, matriz arm64/armv7/x86_64): NDK r28c,
   cargo-ndk 3.1.2, APK por ABI.
4. `android-universal`: APK universal.
5. `release`: firma APKs con la keystore propia y publica todo en una única
   release.

Secretos: `ANDROID_SIGNING_KEY`, `ANDROID_ALIAS`,
`ANDROID_KEY_STORE_PASSWORD`, `ANDROID_KEY_PASSWORD`,
`UPDATE_SIGNING_KEY` (ed25519).

## 7. Pruebas

- Unitarias Rust (TDD) para el código nuevo: parseo de tag/versión, nombre
  de asset, verificación de firma, defaults de identidad.
- Build CI completo Windows + Android.
- E2E Android: emulador (KVM en Docker), instalar APK, comprobar arranque,
  marca y registro en el servidor (log de hbbs).
- E2E Windows: ejecutar el `.exe` en el host Windows, comprobar marca y
  registro; conexión Windows ↔ Android a través del servidor de prueba.
- Update: publicar dos versiones (prerelease de prueba → release) y
  comprobar el salto de versión en un Windows instalado.

# Arenna Remote v1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Convertir el árbol RustDesk 1.4.9 fusionado en `main` en *Arenna Remote* (Windows x64 + Android), con servidor propio, releases automáticas en GitHub y auto-update firmado.

**Architecture:** Todo el código propio de Arenna vive en dos módulos nuevos (`libs/hbb_common/src/arenna.rs` para lógica pura y testeable, `src/arenna.rs` para la integración con red/plataforma); el resto son ediciones mínimas y localizadas sobre upstream para que `git merge upstream` siga siendo viable. La CI (`.github/workflows/release.yml`) es la única que compila Windows/Android y publica releases.

**Tech Stack:** Rust 1.75 (CI) / 1.94 (local), Flutter 3.24.5, flutter_rust_bridge 1.80.1, vcpkg `120deac3062162151622ca4860575a33844ba10b`, NDK r28c, GitHub Actions, Docker (`rustdesk/rustdesk-server:1.1.16`), Python 3 (`cryptography`) para firmar.

**Spec:** `docs/superpowers/specs/2026-09-24-arenna-remote-design.md`

## Global Constraints

- `APP_NAME` = `ArennaRemote` (sin espacios). Nombre visible = `Arenna Remote`.
- Organización visible: `Arenna Labs S.L.`, web `https://arennalabs.com`.
- Repo de releases: `Arenna-Labs/arenna_support_app`; tags `vX.Y.Z`.
- `crate::VERSION` permanece `1.4.9` (versión de protocolo). Versión de producto = `ARENNA_VERSION` (env de compilación, por defecto `0.0.0`).
- Servidor por defecto `rustdesk.arenna38.com`, clave pública leída del `id_ed25519.pub` de la VPS.
- Android `applicationId` y paquete Kotlin: `com.arennalabs.remote`; esquema URI `arennaremote`.
- Asset Windows: `arenna-remote-X.Y.Z-x86_64.exe` + `.sig` (ed25519, base64). APKs: `arenna-remote-X.Y.Z-{universal,aarch64,armv7,x86_64}.apk`.
- Colores: acento `#00A850`, botones `#008257`, claro `#21CC5A`.
- Nunca tocar ficheros/servicios/procesos de RustDesk oficial instalado en la misma máquina.
- No romper el servidor actual de la VPS hasta validar el nuevo.
- Mantener nombres internos: crate `rustdesk`, lib `librustdesk`, paquete Dart `flutter_hbb`, JNI `Java_ffi_FFI_*`, canales `org.rustdesk.*`, driver/IDs firmados.

## Review Focus

1. **Tag con prefijo `v` o sufijos raros** (`v1.2.3`, `V1.2.3`, `1.2.3-1`, sin releases → redirige a `/releases`): la comprobación no debe ofrecer update falso ni crashear. → tests en Task 2.
2. **Firma ausente, corrupta o de otra clave**: el update NO debe ejecutarse y el fichero descargado se borra. → tests en Task 2 + integración en Task 5.
3. **RustDesk oficial instalado en el mismo equipo**: instalar/desinstalar/actualizar Arenna Remote no borra ni mata nada de RustDesk. → revisión de rutas en Task 4 (grep de `RustDesk`/`rustdesk` en comandos) y prueba E2E en Task 10.
4. **Equipo sin conexión o GitHub caído**: la comprobación falla en silencio (log) sin bloquear la UI ni el servicio. → manejo de errores en Task 5.
5. **Primera ejecución con config vacía**: el cliente se registra en `rustdesk.arenna38.com` sin que el usuario toque nada y los ajustes de servidor están ocultos. → tests de defaults en Task 1 y E2E en Task 10.

---

## File Structure

| Fichero | Responsabilidad |
|---|---|
| `libs/hbb_common/src/arenna.rs` (nuevo) | Constantes de marca, versión de producto, parseo de tag, nombre de asset, verificación de firma. Sin red. Tests unitarios. |
| `libs/hbb_common/src/lib.rs` | `pub mod arenna;` |
| `libs/hbb_common/src/config.rs` | `APP_NAME`, servidor, clave, defaults builtin. |
| `src/arenna.rs` (nuevo) | Comprobación de versión contra GitHub, descarga/verificación de `.sig`. |
| `src/common.rs`, `src/lang.rs`, `src/flutter.rs`, `src/core_main.rs`, `src/tray.rs`, `src/flutter_ffi.rs`, `src/updater.rs`, `src/ui_interface.rs` | Enganches mínimos a `arenna`. |
| `src/platform/windows.rs`, `src/privacy_mode/win_topmost_window.rs`, `libs/portable/*` | Instalación, coexistencia, metadatos. |
| `flutter/lib/**`, `flutter/windows/**`, `flutter/android/**` | Marca en UI y plataformas. |
| `.github/workflows/release.yml`, `.github/patches/*`, `.github/scripts/sign_update.py` | CI y firma. |
| `server/*` | Despliegue del servidor. |
| `README.md`, `docs/arenna/*.md` | Documentación de producto/operación. |

---

### Task 0: CI de release (primero, para calentar cachés)

**Files:**
- Create: `.github/workflows/release.yml`, `.github/patches/flutter_3.24.4_dropdown_menu_enableFilter.diff` (copia de upstream), `.github/scripts/sign_update.py`

**Interfaces:**
- Produces: artefactos `arenna-remote-$VERSION-x86_64.exe(.sig)`, `arenna-remote-$VERSION-*.apk`; env `ARENNA_VERSION` disponible en cargo.

- [ ] Step 1: Copiar el diff de Flutter desde upstream.
- [ ] Step 2: Escribir `release.yml` con jobs `version` → `bridge` → (`windows`, `android` matriz 3 ABIs) → `android-universal` → `release`. `workflow_dispatch` con input `version` (sin publicar) y `push: tags: ['v*']` (publica). Basado en `flutter-build.yml` upstream: engine rustdesk para Windows, parche Flutter, vcpkg x-gha, `build.py --portable --flutter --skip-portable-pack --hwcodec --vram`, usbmmidd, packer, sin MSI, sin driver de impresora, sin firma Authenticode. `ARENNA_VERSION` exportada a `$GITHUB_ENV`. Nombre del exe dentro del bundle: el de `BINARY_NAME` (`ArennaRemote.exe` tras Task 4; el workflow usa una variable `APP_EXE`).
- [ ] Step 3: `sign_update.py`: lee clave privada ed25519 (base64 de 32 bytes seed) de `UPDATE_SIGNING_KEY`, escribe `<file>.sig` = base64(firma 64 bytes).
- [ ] Step 4: Hacer público el repo, crear secretos Android + `UPDATE_SIGNING_KEY`, push de `main` y `upstream`, lanzar `workflow_dispatch` y vigilar.
- [ ] Step 5: Commit `ci: release workflow for Windows x64 and Android`.

### Task 1: Identidad y defaults en hbb_common

**Files:**
- Create: `libs/hbb_common/src/arenna.rs`
- Modify: `libs/hbb_common/src/lib.rs`, `libs/hbb_common/src/config.rs:72,120-121`

**Interfaces:**
- Produces: `hbb_common::arenna::{DISPLAY_NAME, COMPANY, WEBSITE, RELEASES_REPO, PRODUCT_VERSION, UPDATE_PUBLIC_KEY}`, `fn apply_builtin_defaults()`.

- [ ] Step 1: Test que falla — `config::APP_NAME` lee `ArennaRemote`, `RENDEZVOUS_SERVERS == ["rustdesk.arenna38.com"]`, `apply_builtin_defaults()` deja `hide-server-settings=Y`, `register-device=N` en `BUILTIN_SETTINGS`.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identity_defaults() {
        assert_eq!(*crate::config::APP_NAME.read().unwrap(), "ArennaRemote");
        assert_eq!(crate::config::RENDEZVOUS_SERVERS, &["rustdesk.arenna38.com"]);
        assert_eq!(DISPLAY_NAME, "Arenna Remote");
    }
    #[test]
    fn builtin_defaults_hide_server_and_disable_api() {
        apply_builtin_defaults();
        let b = crate::config::BUILTIN_SETTINGS.read().unwrap();
        assert_eq!(b.get("hide-server-settings").map(|s| s.as_str()), Some("Y"));
        assert_eq!(b.get("register-device").map(|s| s.as_str()), Some("N"));
    }
}
```
- [ ] Step 2: `cd libs/hbb_common && cargo test arenna` → FAIL (módulo inexistente).
- [ ] Step 3: Implementar constantes + `apply_builtin_defaults()` (inserta solo si la clave no existe) y editar `config.rs`.
- [ ] Step 4: `cargo test arenna` → PASS.
- [ ] Step 5: Commit.

### Task 2: Lógica pura de actualizaciones (TDD)

**Files:** Modify `libs/hbb_common/src/arenna.rs`

**Interfaces:**
- Produces:
  - `fn version_from_release_url(url: &str) -> Option<String>` (`.../releases/tag/v1.2.3` → `Some("1.2.3")`; cualquier otra URL → `None`).
  - `fn is_newer(candidate: &str, current: &str) -> bool` (usa `get_version_number`).
  - `fn windows_asset_name(version: &str, arch: &str) -> String` → `arenna-remote-{version}-{arch}.exe`.
  - `fn release_download_url(version: &str, asset: &str) -> String` → `https://github.com/Arenna-Labs/arenna_support_app/releases/download/v{version}/{asset}`.
  - `fn verify_update_signature(data: &[u8], sig_b64: &str, pk_b64: &str) -> ResultType<()>`.

- [ ] Step 1: Tests que fallan:

```rust
#[test]
fn parses_release_tag_urls() {
    let base = "https://github.com/Arenna-Labs/arenna_support_app/releases";
    assert_eq!(version_from_release_url(&format!("{base}/tag/v1.2.3")), Some("1.2.3".into()));
    assert_eq!(version_from_release_url(&format!("{base}/tag/V2.0.0")), Some("2.0.0".into()));
    assert_eq!(version_from_release_url(&format!("{base}/tag/1.0.1")), Some("1.0.1".into()));
    assert_eq!(version_from_release_url(base), None);
    assert_eq!(version_from_release_url(&format!("{base}/tag/vnext")), None);
    assert_eq!(version_from_release_url(""), None);
}
#[test]
fn compares_versions() {
    assert!(is_newer("1.0.1", "1.0.0"));
    assert!(is_newer("1.1.0", "1.0.9"));
    assert!(!is_newer("1.0.0", "1.0.0"));
    assert!(!is_newer("0.9.0", "1.0.0"));
    assert!(is_newer("1.0.0", "0.0.0"));
}
#[test]
fn builds_asset_names_and_urls() {
    assert_eq!(windows_asset_name("1.2.3", "x86_64"), "arenna-remote-1.2.3-x86_64.exe");
    assert_eq!(
        release_download_url("1.2.3", "arenna-remote-1.2.3-x86_64.exe"),
        "https://github.com/Arenna-Labs/arenna_support_app/releases/download/v1.2.3/arenna-remote-1.2.3-x86_64.exe"
    );
}
#[test]
fn verifies_signatures() {
    use sodiumoxide::crypto::sign;
    let (pk, sk) = sign::gen_keypair();
    let data = b"installer bytes";
    let sig = sign::sign_detached(data, &sk);
    let pk_b64 = crate::base64::encode(pk.0, crate::base64::Variant::Original);
    let sig_b64 = crate::base64::encode(sig.to_bytes(), crate::base64::Variant::Original);
    assert!(verify_update_signature(data, &sig_b64, &pk_b64).is_ok());
    assert!(verify_update_signature(b"tampered", &sig_b64, &pk_b64).is_err());
    assert!(verify_update_signature(data, "", &pk_b64).is_err());
    assert!(verify_update_signature(data, "not base64!", &pk_b64).is_err());
    let (other_pk, _) = sign::gen_keypair();
    let other_b64 = crate::base64::encode(other_pk.0, crate::base64::Variant::Original);
    assert!(verify_update_signature(data, &sig_b64, &other_b64).is_err());
}
```
(Ajustar a la API base64 que exponga hbb_common; si no re-exporta `base64`, usar el crate `base64` 0.22 directamente con `general_purpose::STANDARD`.)
- [ ] Step 2: `cargo test arenna` → FAIL.
- [ ] Step 3: Implementar (tag: `rsplit('/')`, quitar `v`/`V`, validar que todos los componentes separados por `.`/`-` son numéricos; firma: decode base64 → `sign::Signature::from_bytes` + `sign::PublicKey::from_slice` + `verify_detached`).
- [ ] Step 4: `cargo test arenna` → PASS. Commit.

### Task 3: Nombre visible y enganches de identidad en el crate principal

**Files:** `src/common.rs` (`is_custom_client`, `get_api_server_`, nueva `get_app_display_name`), `src/lang.rs:225-248`, `src/flutter.rs:191`, `src/core_main.rs:867`, `src/tray.rs:77,83`, `src/flutter_ffi.rs` (nueva `main_get_app_display_name_sync`), `flutter/lib/web/bridge.dart` (stub), `src/auth_2fa.rs:17`, `src/platform/windows.rs:3816`.

- [ ] Step 1: `is_custom_client()` → `false` con comentario; `get_app_display_name()` → `hbb_common::arenna::DISPLAY_NAME`.
- [ ] Step 2: `lang.rs`: reemplazar "RustDesk" por `get_app_display_name()`; `powered_by_me` y `upgrade_rustdesk_server_pro*` se dejan igual.
- [ ] Step 3: export `get_rustdesk_app_name` y búsqueda de ventana en `core_main.rs` usan el nombre visible (mismo valor en ambos).
- [ ] Step 4: tooltip bandeja, ISSUER 2FA y caption de message box → nombre visible.
- [ ] Step 5: `get_api_server_` → `""` en vez de `admin.rustdesk.com`.
- [ ] Step 6: llamar a `hbb_common::arenna::apply_builtin_defaults()` al inicio (`core_main::core_main`, `flutter_ffi::initialize`, `service.rs`, y en `startServer` de Android — donde upstream llama a `load_custom_client`).
- [ ] Step 7: nueva FFI `pub fn main_get_app_display_name_sync() -> SyncReturn<String>` (Dart: `bind.mainGetAppDisplayNameSync()`) + stub en `web/bridge.dart`.
- [ ] Step 8: Commit.

### Task 4: Windows — instalación, coexistencia y metadatos

**Files:** `src/platform/windows.rs`, `src/privacy_mode/win_topmost_window.rs:33`, `libs/portable/src/main.rs:20,219,235`, `libs/portable/Cargo.toml`, `libs/portable/src/res/label.png`, `libs/portable/generate.py:100`, `flutter/windows/CMakeLists.txt:3,7`, `flutter/windows/runner/Runner.rc`, `build.py:456`, `Cargo.toml` (authors/description/winres).

- [ ] Step 1: eliminar la consulta `IS1` en `get_valid_subkey()`.
- [ ] Step 2: staging dir en `%ProgramData%\<APP_NAME>\...`.
- [ ] Step 3: helper `fn shortcut_name() -> String` (= nombre visible) usado en **todas** las creaciones/borrados de `.lnk` (`install_me`, `get_tray_shortcut`, `get_uninstall`, `update_me`) y carpeta del menú Inicio en `get_install_info_with_subkey`.
- [ ] Step 4: `DisplayName` = nombre visible, `Publisher` = `Arenna Labs S.L.`, `DisplayVersion` = `PRODUCT_VERSION` (`Version` interno sigue `crate::VERSION`); DisplayName del servicio = `"<visible> Service"`.
- [ ] Step 5: broker `RuntimeBroker_arennaremote.exe` en los dos sitios; `APP_PREFIX = "ArennaRemote"` en el packer; winres/label.png del packer; `BINARY_NAME ArennaRemote`; `Runner.rc`; `build.py`/`generate.py` con el nuevo exe.
- [ ] Step 6: `grep -n 'RustDesk\|rustdesk' src/platform/windows.rs` y revisar que ningún comando de borrado/kill apunta a rutas de RustDesk.
- [ ] Step 7: Commit.

### Task 5: Canal de actualizaciones (integración)

**Files:** Create `src/arenna.rs`; modify `src/lib.rs`, `src/common.rs` (`check_software_update`, `do_check_software_update`), `src/updater.rs:122-160,207+`, `src/flutter_ffi.rs:2856-2883,2961-2980`, `src/platform/windows.rs` (`try_remove_temp_update_files`), `src/ui_interface.rs:get_version`, `libs/hbb_common/src/config.rs` (default `allow-auto-update`).

**Interfaces:**
- Consumes: funciones de Task 2.
- Produces: `crate::arenna::fetch_latest_release_url() -> ResultType<String>` (async), `crate::arenna::verify_downloaded_update(path: &Path, download_url: &str) -> ResultType<()>`.

- [ ] Step 1: `do_check_software_update` → GET a `https://github.com/Arenna-Labs/arenna_support_app/releases/latest` (sigue redirecciones, `User-Agent: ArennaRemote/<ver>`), `version_from_release_url(resp.url())`, `is_newer(v, PRODUCT_VERSION)`; guarda en `SOFTWARE_UPDATE_URL` la URL `.../releases/tag/vX.Y.Z` (forma upstream). Errores → `Err` (se registran) y nunca panic.
- [ ] Step 2: quitar el early-return de custom client en `check_software_update`.
- [ ] Step 3: `updater.rs`: URL de descarga = `release_download_url(v, windows_asset_name(v, arch))`; nunca MSI; tras descargar, `verify_downloaded_update` (descarga `<url>.sig`, verifica; si falla borra y `bail!`).
- [ ] Step 4: FFI `download-file-<ver>` → `windows_asset_name`; `update-me` verifica firma antes de `update_to`.
- [ ] Step 5: limpieza de temporales con prefijo `arenna-remote-`.
- [ ] Step 6: `allow-auto-update` activo por defecto en Windows (`!= "N"`).
- [ ] Step 7: `get_version()` de UI → `PRODUCT_VERSION`.
- [ ] Step 8: Commit.

### Task 6: Flutter — marca en la UI

**Files:** `flutter/lib/common.dart` (MyTheme colores, `loadPowered`, `appDisplayName`, `getWindowName`, `checkUpdate`), `flutter/lib/main.dart:504`, `flutter/lib/desktop/pages/desktop_setting_page.dart` (About, toggles update), `flutter/lib/desktop/pages/desktop_home_page.dart:84,433-457`, `flutter/lib/desktop/pages/install_page.dart:168,190,192`, `flutter/lib/desktop/pages/connection_page.dart:44,83-92`, `flutter/lib/desktop/widgets/update_progress.dart`, `flutter/lib/desktop/widgets/tabbar_widget.dart:644`, `flutter/lib/mobile/pages/{settings_page,connection_page,home_page,server_page}.dart`, `flutter/lib/common/widgets/toolbar.dart:20`, `flutter/assets/*` (ya generados).

- [ ] Step 1: colores de marca en `MyTheme`.
- [ ] Step 2: enlaces rustdesk.com → arennalabs.com / releases; quitar setup-server tip y pricing.
- [ ] Step 3: About: © Arenna Labs S.L., "Basado en RustDesk © Purslane Tech Pte. Ltd. — AGPL-3.0", enlace al código fuente.
- [ ] Step 4: banner de update desktop sin gates de custom client / URI prefix; changelog → release tag de Arenna; mobile: banner en connection_page con URL de la release.
- [ ] Step 5: títulos visibles vía `appDisplayName`.
- [ ] Step 6: ocultar opción de impresora en install_page si no existe `drivers/RustDeskPrinterDriver` (no la empaquetamos).
- [ ] Step 7: Commit.

### Task 7: Android — marca y paquete

**Files:** `flutter/android/app/build.gradle` (applicationId, namespace, signing), `flutter/android/app/src/{main,debug,profile}/AndroidManifest.xml`, `flutter/android/app/src/main/kotlin/com/carriez/flutter_hbb/*` → `.../com/arennalabs/remote/*`, `flutter/android/app/src/main/kotlin/ffi.kt:8`, `res/values/strings.xml`, `res/values/colors.xml`, `res/drawable/floating_window.xml`, `MainService.kt` (notificaciones), `BootReceiver.kt`.

- [ ] Step 1: `git mv` del paquete Kotlin + `package` en los 12 ficheros + import en `ffi.kt`.
- [ ] Step 2: manifest: `package`, label `Arenna Remote`, `Arenna Remote Input`, `android:scheme="arennaremote"`, acción DEBUG_BOOT.
- [ ] Step 3: strings/notificaciones/colores/floating window con la A de Arenna.
- [ ] Step 4: `applicationId "com.arennalabs.remote"`.
- [ ] Step 5: `grep -rn "carriez\|RustDesk" flutter/android` → solo quedan referencias internas justificadas.
- [ ] Step 6: Commit.

### Task 8: Build CI completo con marca + verificación de artefactos

- [ ] Step 1: push + `workflow_dispatch version=0.9.0`.
- [ ] Step 2: arreglar fallos de compilación hasta verde (cada fallo → systematic-debugging).
- [ ] Step 3: descargar artefactos; verificar firma `.sig` localmente con `sign_update.py --verify`; `aapt dump badging` del APK (package, label, versionName).

### Task 9: Servidor

**Files:** Create `server/docker-compose.yml`, `server/docker-compose.test.yml`, `server/README.md`, `server/backup-keys.sh`.

- [ ] Step 1: inventario solo-lectura de la VPS (versión, forma de despliegue, ruta de claves, firewall) y backup de `id_ed25519*` + BD a `/root/arenna-backup-<fecha>/` y a local (fuera del repo).
- [ ] Step 2: instancia de prueba (21125–21129) con copia de la clave; abrir puertos de prueba en el firewall.
- [ ] Step 3: validar un cliente contra `rustdesk.arenna38.com:21126`.
- [ ] Step 4: corte a 1.1.16 en puertos estándar con la misma clave; verificar `Key:` idéntico y clientes existentes conectando; rollback documentado.
- [ ] Step 5: cerrar puertos de prueba; commit de `server/`.

### Task 10: E2E, release v1.0.0 y documentación

- [ ] Step 1: Android en emulador (Docker+KVM): instalar APK, abrir, capturas, comprobar ID registrado (log hbbs).
- [ ] Step 2: Windows en el host: ejecutar `.exe` en modo portable, comprobar marca/registro; conexión Windows ↔ Android.
- [ ] Step 3: update: publicar `v0.9.0`, instalar en Windows, publicar `v1.0.0`, comprobar auto-update (log) y versión nueva.
- [ ] Step 4: README (producto, instalación, operación, cómo publicar una versión, cómo actualizar upstream), `docs/arenna/`.
- [ ] Step 5: revisión final de rama (agente revisor), commit, release `v1.0.0`.

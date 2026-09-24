<p align="center">
  <img src="flutter/assets/logo_light.png" alt="Arenna Remote" width="300"><br>
  Asistencia remota de <a href="https://arennalabs.com">Arenna Labs S.L.</a>
</p>

**Arenna Remote** es la aplicación de soporte remoto de Arenna Labs: una
versión propia y rebrandeada de [RustDesk](https://github.com/rustdesk/rustdesk)
que se conecta a nuestro servidor (`rustdesk.arenna38.com`) sin ninguna
configuración y se actualiza desde las
[releases de este repositorio](https://github.com/Arenna-Labs/arenna-remote/releases).

| Plataforma | Descarga |
|---|---|
| Windows 10/11 x64 | `arenna-remote-X.Y.Z-x86_64.exe` |
| Android 5.1+ | `arenna-remote-X.Y.Z-universal.apk` (o el APK de tu arquitectura) |

➡️ **[Última versión](https://github.com/Arenna-Labs/arenna-remote/releases/latest)**

---

## Uso

### Windows

1. Descarga y ejecuta `arenna-remote-X.Y.Z-x86_64.exe`. Arranca directamente
   (modo portable): el cliente ve su **ID** y **contraseña** para dárselas al
   técnico.
2. Para dejarlo instalado (servicio en segundo plano, acceso sin que el
   cliente esté delante, actualizaciones automáticas), pulsa **Instalar** en
   la propia app. Se instala en `C:\Program Files\ArennaRemote` con accesos
   directos "Arenna Remote" en el escritorio y el menú Inicio.
3. Convive sin problemas con un RustDesk oficial instalado en el mismo equipo
   (carpetas, servicio y procesos distintos), aunque lo recomendable es
   desinstalar RustDesk.

> Windows SmartScreen puede avisar de "editor desconocido" porque el
> ejecutable no lleva firma Authenticode (requiere un certificado de firma de
> código de pago). Pulsar *Más información → Ejecutar de todas formas*.

### Android

Instala el APK (hay que permitir "instalar apps desconocidas"). Para que el
técnico pueda ver y controlar el móvil: *Compartir pantalla* y, para el
control táctil, activar el servicio de accesibilidad **Arenna Remote Input**.

## Actualizaciones

- Cada versión es una GitHub Release con tag `vX.Y.Z`.
- Las instalaciones de Windows comprueban al arrancar y una vez al día
  `…/releases/latest`. Si hay una versión nueva la descargan y se actualizan
  solas **cuando no hay ninguna sesión activa** (opción *Actualización
  automática* en Ajustes → General, activada por defecto). También aparece un
  aviso en la pantalla principal para actualizar a mano.
- Antes de ejecutar un instalador descargado, el cliente verifica su firma
  Ed25519 (`.sig` de la release, que cubre el nombre del fichero y su
  contenido) con las claves públicas compiladas en la app. El auto-update
  descarga y verifica en memoria y guarda el instalador en
  `C:\Program Files\ArennaRemote\update`, bloqueado hasta ejecutarlo. Un
  fichero sin firma válida se descarta.
- Android muestra un aviso con enlace a la release (el APK está firmado
  siempre con la misma clave, así que se instala encima).

## Servidor

Ver [`server/README.md`](server/README.md): despliegue Docker, puertos,
clave del servidor, copias de seguridad y actualización.

---

## Desarrollo

### Estructura del repositorio

| Ruta / rama | Contenido |
|---|---|
| rama `upstream` | Snapshots prístinos de RustDesk (hoy 1.4.9) con `libs/hbb_common` vendorizado. Solo recibe versiones nuevas de upstream. |
| rama `main` | `upstream` + los cambios de Arenna. |
| `libs/hbb_common/src/arenna.rs` | Identidad del producto (nombres, servidor y clave, clave de firma de updates, versión) y lógica pura del canal de actualizaciones, con tests. |
| `src/arenna.rs` | Comprobación de versión contra GitHub y verificación de la firma de las descargas. |
| `branding/` | Logo fuente (`logo.svg`), tipografía Inter (OFL) y `generate_icons.py`, que regenera todos los iconos del árbol. |
| `server/` | Despliegue y operación del servidor. |
| `.github/workflows/release.yml` | Build de Windows x64 y Android, firma y publicación. |
| `docs/superpowers/` | Diseño y plan de implementación. |

Casi todo el resto es código de RustDesk con cambios mínimos y localizados
(buscar `Arenna` en los comentarios) para que los merges de upstream sigan
siendo sencillos.

### Publicar una versión

```bash
git tag v1.2.3
git push origin v1.2.3
```

La CI compila, firma y publica la release (como borrador hasta que están
todos los ficheros; después pasa a *latest* y los clientes la ven). La
versión sale del tag: no hay que tocar ningún fichero. Un push a una rama
`feat/**` o un lanzamiento manual del workflow hace el mismo build sin
publicar ni firmar (artefactos en la ejecución de Actions).

Secretos del *environment* `release` de GitHub Actions (solo accesible desde
tags `v*`; únicamente el job final `release` los ve):

| Secreto | Qué es |
|---|---|
| `ANDROID_SIGNING_KEY` | Keystore Android en base64 |
| `ANDROID_ALIAS`, `ANDROID_KEY_STORE_PASSWORD`, `ANDROID_KEY_PASSWORD` | Alias y contraseñas de la keystore |
| `UPDATE_SIGNING_KEY` | Semilla Ed25519 (base64) que firma los instaladores de Windows |

⚠️ La keystore Android y la clave `UPDATE_SIGNING_KEY` **no se pueden
perder**: sin la keystore los APK nuevos no se instalan encima de los
antiguos, y sin la clave de firma los Windows instalados rechazan las
actualizaciones. Deben estar guardadas en un gestor de contraseñas de la
empresa.

Rotar la clave de updates: añadir la nueva clave pública a
`UPDATE_PUBLIC_KEYS` (`libs/hbb_common/src/arenna.rs`) y poner en
`UPDATE_SIGNING_KEY` las dos semillas separadas por coma; cada `.sig`
llevará una firma por clave. Cuando todos los clientes tengan una versión
que confíe en la nueva, se retira la antigua. La CI comprueba antes de
publicar que la firma verifica con las claves compiladas en la app.

Opcional y recomendable: exigir aprobación manual en el environment
`release` (*Settings → Environments → release → Required reviewers*) para
que ninguna release se publique sin que alguien la apruebe.

### Actualizar a una versión nueva de RustDesk

```bash
git checkout upstream
# sustituir el árbol por el nuevo tag de rustdesk/rustdesk, con
# libs/hbb_common copiado como ficheros normales y sin .github/
git commit -am "Upstream RustDesk X.Y.Z snapshot"
git checkout main
git merge upstream      # resolver conflictos en los puntos marcados "Arenna"
```

Revisar después las versiones de toolchain de `release.yml` contra el
`flutter-build.yml` del upstream nuevo.

### Tests

```bash
cd libs/hbb_common && cargo test --lib -- --test-threads=1
```

El crate principal solo compila con las dependencias de vcpkg/sistema; la
CI lo compila para Windows y Android en cada push a `feat/**`.

### Regenerar iconos

```bash
pip install cairosvg pillow
python3 branding/generate_icons.py
```

---

## Licencia

Arenna Remote se basa en [RustDesk](https://github.com/rustdesk/rustdesk)
(© Purslane Tech Pte. Ltd. y colaboradores) y, como RustDesk, se distribuye
bajo la licencia **AGPL-3.0** ([LICENCE](LICENCE)). El código fuente
completo de cada versión está en este repositorio.

# Servidor de Arenna Remote

Arenna Remote usa el servidor OSS de RustDesk (`rustdesk/rustdesk-server`),
desplegado con Docker en la VPS de `rustdesk.arenna38.com`
(217.154.183.110), carpeta `/opt/rustdesk-server`.

| Proceso | Función | Puertos |
|---|---|---|
| `hbbs` | Registro de IDs, rendezvous, NAT traversal | 21115/tcp, 21116/tcp, 21116/udp |
| `hbbr` | Relay cuando no hay conexión directa | 21117/tcp |

21118/21119 (websocket, solo para el cliente web) están abiertos en el
firewall pero Arenna Remote no los usa. 21114 no existe en la versión OSS.

Los clientes llevan **compilados** el host y la clave pública del servidor
(`libs/hbb_common/src/arenna.rs`: `SERVER_HOST`, `SERVER_PUBLIC_KEY`); no hay
que configurar nada en cada equipo.

## La clave del servidor

`/opt/rustdesk-server/data/id_ed25519` (privada) y `id_ed25519.pub` (pública,
`7c6bMZlmhqyWFvA3sFQDtQKQr29M+Z2CsjbYMLJdQ8g=`).

- Si se pierde o cambia, **todos los clientes instalados dejan de conectar**
  hasta que se publique una versión nueva con la clave nueva.
- `-k _` en hbbs y hbbr hace que el servidor rechace a quien no tenga la
  clave (los intentos fallidos aparecen como `invalid key` en el log).
- No arrancar nunca hbbs con `-k <clave pública>`: se queda sin clave privada
  y los clientes 1.4.x avisan de que no pueden verificar el cifrado.

Copia de seguridad (claves + base de datos + compose) a un fichero local:

```bash
server/backup.sh root@rustdesk.arenna38.com ~/backups
```

Guárdala fuera del repositorio (contiene la clave privada).

## Comprobaciones rápidas

```bash
ssh root@rustdesk.arenna38.com
cd /opt/rustdesk-server
docker compose ps
docker logs hbbs 2>&1 | grep 'Key:'      # debe mostrar la clave pública de arriba
docker logs --since 1h hbbs              # registros y conexiones recientes
ufw status | grep 2111                   # 21115-21119 permitidos
```

## Actualizar el servidor

1. Hacer backup (`server/backup.sh`).
2. Cambiar el tag de imagen en `docker-compose.yml` (este repo y la VPS).
3. En la VPS: `docker compose pull && docker compose up -d` (corte de ~2 s;
   los clientes reconectan solos).
4. Verificar que `docker logs hbbs | grep Key:` muestra la misma clave.

La versión en producción es la 1.1.16 (digest `sha256:8ecdab65…`). El
compose de la VPS aún usa el tag `latest` (misma imagen hoy); el de este repo
fija `1.1.16` para que una reinstalación sea reproducible. Aplicarlo en la
VPS provoca un reinicio de ~2 s de los contenedores.

## Reinstalar en una VPS nueva

1. Instalar Docker y abrir en el firewall 21115/tcp, 21116/tcp, 21116/udp y
   21117/tcp (y 22 para SSH).
2. Copiar este `docker-compose.yml` a `/opt/rustdesk-server/` y restaurar
   `data/` desde el backup (`tar xzf … -C /opt/rustdesk-server`), con
   `chmod 600 data/id_ed25519`.
3. `docker compose up -d` y comprobar el `Key:`.
4. Apuntar el DNS de `rustdesk.arenna38.com` a la IP nueva.

Si hubiera que cambiar de dominio o de clave, se cambia `SERVER_HOST` /
`SERVER_PUBLIC_KEY` en `libs/hbb_common/src/arenna.rs` y se publica una
versión: los Windows instalados se actualizan solos **antes** de retirar el
servidor antiguo (mantener ambos en paralelo hasta entonces).

## Otros servicios de la VPS

La VPS también aloja nginx (80/443), uptime-kuma y aplicaciones Node.js.
fail2ban protege SSH (3 intentos → baneo de 1 h de **todos** los puertos vía
ufw): al conectar por SSH, usar siempre el usuario correcto.

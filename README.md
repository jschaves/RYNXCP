# RYNXCP

Panel de control de hosting **original, escrito desde cero en Rust** para Debian/Ubuntu.
Gestiona VPS y servidores dedicados con un modelo declarativo: describes el estado
deseado (cuentas, sitios, runtimes, DNS, bases de datos, FTP, backups, cortafuegos) y
el daemon lo reconcilia contra el sistema operativo.

No es un fork de cPanel, Plesk, HestiaCP ni Vesta. Arquitectura, protocolo, motor de
reconciliaciÃƒÂ³n y UI son propios.

## Funciones

| ÃƒÂrea | Incluye |
| --- | --- |
| Cuentas | Usuarios Linux aislados, home, cuota declarada, suspensiÃƒÂ³n y borrado |
| Web | Vhosts Nginx, alias, TLS, PHP-FPM, reverse proxy, cabeceras de seguridad |
| Runtimes | PHP, Node.js, Python, Ruby, Java, .NET, Go y sitios estÃƒÂ¡ticos |
| DNS | Zonas BIND autoritativas, registros A/AAAA/CNAME/MX/TXT/NS/SRV/CAA |
| Correo | SPF, DKIM (clave RSA + tablas OpenDKIM), DMARC, MX y guÃƒÂ­a de PTR |
| Bases de datos | MariaDB por dominio, usuario y contraseÃƒÂ±a propios, acceso local |
| FTP | Usuarios virtuales Pure-FTPd con chroot al docroot, lectura o escritura |
| Backups | Home + configuraciÃƒÂ³n gestionada, retenciÃƒÂ³n, restauraciÃƒÂ³n y timer diario |
| Certificados | Let's Encrypt con SANs, renovaciÃƒÂ³n masiva y timer diario |
| Monitor | Estado/CPU/RAM/uptime por servicio y series histÃƒÂ³ricas de 7 dÃƒÂ­as |
| Cortafuegos | Reglas nftables, IPs baneadas, listas blancas y baneo automÃƒÂ¡tico |
| Seguridad | Argon2id, lockout por cuenta+IP, auth log para fail2ban, RBAC |
| Consola | Acciones privilegiadas con lista cerrada, salida y auditorÃƒÂ­a |
| Programas | Inventario instalado, actualizaciones pendientes y botÃƒÂ³n de upgrade |
| Usuarios | Roles owner/admin/operator/viewer, suplantaciÃƒÂ³n auditada y revocaciÃƒÂ³n |

## Arquitectura

```
 navegador
    Ã¢â€â€š  HTTPS + cookie HttpOnly + CSRF
    Ã¢â€“Â¼
Ã¢â€Å’Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€Â        Unix socket 0660 (grupo rynx)
Ã¢â€â€š rynx-api  (usuario rynx)     Ã¢â€â€š  CBOR  + token + SO_PEERCRED
Ã¢â€â€š Ã‚Â· Axum, sesiones, RBAC       Ã¢â€â€š Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€Â
Ã¢â€â€š Ã‚Â· Argon2id, lockout, logs    Ã¢â€â€š                                Ã¢â€â€š
Ã¢â€â€š Ã‚Â· panel.db (SQLite separada) Ã¢â€â€š                                Ã¢â€“Â¼
Ã¢â€â€Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€Ëœ                 Ã¢â€Å’Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€Â
                                                 Ã¢â€â€š rynxd  (root, sin red en prod)Ã¢â€â€š
                                                 Ã¢â€â€š Ã‚Â· valida todo otra vez        Ã¢â€â€š
                                                 Ã¢â€â€š Ã‚Â· reconciliaciÃƒÂ³n declarativa  Ã¢â€â€š
                                                 Ã¢â€â€š Ã‚Â· useradd/chown/systemctl     Ã¢â€â€š
                                                 Ã¢â€â€š Ã‚Â· nginx, php-fpm, bind,       Ã¢â€â€š
                                                 Ã¢â€â€š   mariadb, pure-ftpd, nft     Ã¢â€â€š
                                                 Ã¢â€â€š Ã‚Â· state.db + audit.log        Ã¢â€â€š
                                                 Ã¢â€â€Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€Ëœ
```

Dos bases de datos separadas a propÃƒÂ³sito:

- `panel.db`: usuarios del panel, hashes Argon2id, sesiones y eventos de autenticaciÃƒÂ³n.
- `state.db`: estado de hosting deseado. Solo lo escribe el daemon.

Comprometer la API web no concede escritura directa sobre el estado del hosting.

## Seguridad

- **Rust**: sin GC, sin `null`, concurrencia sin carreras de datos.
- **Daemon root mÃƒÂ­nimo**: no escucha red en producciÃƒÂ³n; solo un socket Unix `0660`
  en el grupo `rynx` y verifica credenciales del par (`SO_PEERCRED`).
- **Doble autenticaciÃƒÂ³n de transporte**: token de 256 bits comparado en tiempo
  constante + UID/GID del proceso llamante.
- **Cero shells**: todo comando usa `argv` explÃƒÂ­cito y entorno limpio. Nunca
  `sh -c` ni interpolaciÃƒÂ³n.
- **ValidaciÃƒÂ³n en dos capas**: API y daemon. Dominios, usuarios, rutas, puertos,
  versiones, IPs, nombres DNS, variables y comandos tienen listas blancas.
- **Escritura atÃƒÂ³mica** con `O_NOFOLLOW`, permisos restrictivos y verificaciÃƒÂ³n de
  que el destino no escapa de su raÃƒÂ­z. Nginx se prueba con `nginx -t` y se revierte
  si falla; las zonas con `named-checkzone`.
- **Sesiones**: token opaco; solo se guarda su SHA-256. Cookie `HttpOnly`,
  `SameSite=Strict`, `Secure` en producciÃƒÂ³n, expiraciÃƒÂ³n absoluta e inactividad.
- **Anti fuerza bruta**: ventana deslizante + lockout por cuenta+IP y por IP,
  retardo en fallo, eventos persistentes y log estructurado para fail2ban.
- **Listas blancas de IP**: tienen prioridad sobre los baneos del cortafuegos.
- **SSH siempre permitido** en nftables para evitar bloqueos accidentales.
- **RBAC** con protecciÃƒÂ³n del ÃƒÂºltimo owner, autoprotecciÃƒÂ³n y suplantaciÃƒÂ³n auditada.
- **AuditorÃƒÂ­a encadenada**: cada evento incluye `prev_hash`; editar o borrar una
  lÃƒÂ­nea rompe la cadena y la verificaciÃƒÂ³n lo detecta.
- **Cabeceras**: CSP estricta, `nosniff`, `DENY`, HSTS, `no-store` en `/api`.

## Runtimes

| Runtime | Mecanismo | Versiones sugeridas |
| --- | --- | --- |
| PHP | FPM pool por cuenta, `open_basedir`, funciones peligrosas deshabilitadas | 8.1 Ã¢â‚¬â€œ 8.5 |
| Node.js | systemd + reverse proxy | 18, 20, 22, 24 |
| Python | gunicorn / uvicorn bajo systemd | 3.10 Ã¢â‚¬â€œ 3.13 |
| Ruby | puma / rackup bajo systemd | 3.2 Ã¢â‚¬â€œ 3.4 |
| Java | `java -jar` bajo systemd | 11, 17, 21 |
| .NET | `dotnet App.dll` bajo systemd | 8, 9 |
| Go | binario compilado bajo systemd | n/a |
| EstÃƒÂ¡tico | Nginx directo | n/a |

Cada aplicaciÃƒÂ³n recibe usuario sin shell, puerto loopback validado, unidad systemd
endurecida, archivo de entorno `0640` y vhost con cabeceras.

## DNS y correo (SPF Ã‚Â· DKIM Ã‚Â· DMARC)

Con `dns_enabled = true` cada sitio genera una zona BIND con SOA, NS, A/AAAA, CNAME,
CAA y los registros de correo. Con `mail_enabled = true` y `mail_hostname` configurado
se aÃƒÂ±aden:

- `MX` al host de correo.
- `TXT @` con `v=spf1 mx a ~all`.
- `TXT _dmarc` con `p=quarantine`, `adkim=s`, `aspf=s`.
- `TXT <selector>._domainkey` con la clave pÃƒÂºblica DKIM (RSA 2048).
- Entradas en `KeyTable` y `SigningTable` de OpenDKIM.

El **PTR inverso** lo controla el proveedor de la IP, no el panel: debe resolver al
`mail_hostname`. VerifÃƒÂ­calo con `dig -x TU_IP`. Esto es imprescindible para no caer en
spam junto con SPF/DKIM/DMARC.

El panel provisiona DNS y firma DKIM; el servidor de correo (Postfix/Dovecot) y los
buzones no se instalan automÃƒÂ¡ticamente. La UI muestra esta limitaciÃƒÂ³n.

## Bases de datos

Una base MariaDB por dominio (`rynx_<dominio>`), `utf8mb4`, con usuario exclusivo y
contraseÃƒÂ±a aleatoria de 32 caracteres. El usuario solo accede por `localhost`. La
contraseÃƒÂ±a se guarda en `state.db` (permisos `0600`) y se muestra al administrador.

## FTP

Usuarios virtuales Pure-FTPd mapeados al UID de la cuenta (lectura/escritura) o al
usuario `rynxftp` (solo lectura), con chroot al document root del sitio. Se puede usar
FTPS explÃƒÂ­cito. La contraseÃƒÂ±a se genera si no se indica.

## Backups

Cada copia incluye home, vhost, zona DNS, pool PHP, unidades systemd y ficheros de
entorno. Se guardan en `/var/backups/rynxcp/<cuenta>/`, se conservan las ÃƒÂºltimas 7 por
cuenta y pueden restaurarse desde la UI. Se ejecutan a diario con un timer systemd.

Restaurar extrae solo rutas gestionadas (`home/<cuenta>`, nginx, php, systemd, apps),
rechaza `..` y rutas absolutas, y re-aplica el estado deseado despuÃƒÂ©s.

## Monitor y consola

El daemon muestrea cada 60 s: load, memoria, disco, servicios activos y estado/CPU/RAM
de nginx, PHP-FPM, aplicaciones y el propio daemon. Se conservan 7 dÃƒÂ­as.

La consola admin **no es un shell**. Solo ejecuta acciones cerradas (test/reload de
Nginx, reinicio/logs de una app, recarga de PHP-FPM, reconciliaciÃƒÂ³n, renovaciÃƒÂ³n TLS,
backups, disco) y cada ejecuciÃƒÂ³n queda auditada.

## InstalaciÃƒÂ³n en Debian/Ubuntu

Dependencias del sistema (instala lo que vayas a usar):

```bash
sudo apt update
sudo apt install -y build-essential pkg-config nginx certbot \
  php8.3-fpm php8.3-cli php8.3-mysql \
  mariadb-server bind9 bind9-utils \
  pure-ftpd opendkim opendkim-tools fail2ban nftables \
  nodejs python3-venv ruby-full default-jre dotnet-sdk-8.0 golang
```

Compila e instala:

```bash
cargo build --release
sudo bash deploy/install.sh
```

El instalador crea grupo/usuario `rynx`, `rynxftp`, directorios, token, log de
autenticaciÃƒÂ³n, fail2ban, logrotate, unidades y timers; arranca `rynxd` y `rynx-api`.

```bash
sudo rynxctl admin create admin --email tu@example.com
sudo cp deploy/nginx/panel.conf.example /etc/nginx/sites-available/rynxcp-panel
sudo ln -s /etc/nginx/sites-available/rynxcp-panel /etc/nginx/sites-enabled/
sudo certbot --nginx -d panel.example.com
```

Si Nginx termina TLS delante de la API, activa `trust_proxy_headers = true` en
`/etc/rynxcp/api.toml`.

Timers instalados:

- `rynxcp-cert-renew.timer`: renueva certificados cada dÃƒÂ­a.
- `rynxcp-backup.timer`: backups nocturnos.

## Desarrollo local (Windows o Linux)

Modo simulaciÃƒÂ³n: renderiza todos los artefactos en `./data/daemon/sandbox` y no toca
el sistema operativo.

```bash
cargo build
# terminal 1
target/debug/rynxd.exe --config rynxd.toml
# terminal 2
target/debug/rynxctl.exe admin create admin --email tu@example.com
# terminal 3
target/debug/rynx-api.exe --config rynx-api.toml
```

Abre <http://127.0.0.1:8080>. En Linux usa los binarios sin `.exe`.

## CLI

```bash
rynxctl admin create <usuario> --email <email> [--role owner|admin|operator|viewer]
rynxctl admin list
rynxctl admin passwd <usuario>
rynxctl admin disable|enable <usuario>
rynxctl daemon ping|info
rynxctl cert renew [--staging]
rynxctl backup all
rynxctl backup list
rynxctl monitor
rynxctl audit --limit 20
rynxctl reconcile
```

## API

Salvo login, health y meta, todo exige sesiÃƒÂ³n; los mÃƒÂ©todos mutantes exigen
`x-csrf-token`.

```
POST   /api/v1/auth/login
POST   /api/v1/auth/logout
GET    /api/v1/auth/me
POST   /api/v1/auth/stop-impersonation
POST   /api/v1/users/{username}/impersonate

GET|POST   /api/v1/accounts          DELETE /api/v1/accounts/{username}
GET|POST   /api/v1/websites          DELETE /api/v1/websites/{domain}
GET|POST   /api/v1/php-pools         DELETE /api/v1/php-pools/{username}
GET|POST   /api/v1/apps              DELETE /api/v1/apps/{username}/{name}
GET|POST   /api/v1/certs             POST   /api/v1/certs/renew
GET|POST   /api/v1/backups           DELETE /api/v1/backups/{id}
POST       /api/v1/backups/{id}/restore
GET        /api/v1/dns               GET    /api/v1/dns/{domain}
POST       /api/v1/dns/{domain}/records
DELETE     /api/v1/dns/{domain}/records/{id}
POST       /api/v1/dns/{domain}/mail
GET|POST   /api/v1/databases         DELETE /api/v1/databases/{id}
GET|POST   /api/v1/ftp               DELETE /api/v1/ftp/{id}
GET        /api/v1/monitor/services
GET        /api/v1/monitor/history?minutes=1440
GET        /api/v1/security/auth-events
GET        /api/v1/security/stats
GET        /api/v1/firewall
POST       /api/v1/firewall/rules    DELETE /api/v1/firewall/rules/{id}
POST       /api/v1/firewall/block    DELETE /api/v1/firewall/block/{ip}
POST       /api/v1/firewall/whitelist DELETE /api/v1/firewall/whitelist/{ip}
GET        /api/v1/system/programs
GET        /api/v1/system/updates    POST   /api/v1/system/updates/apply
POST       /api/v1/console
GET|POST   /api/v1/users             PATCH|DELETE /api/v1/users/{username}
POST       /api/v1/reconcile
```

## Estructura

```
crates/
  rynx-proto/   protocolo CBOR versionado y modelos
  rynx-core/    config, validaciÃƒÂ³n, polÃƒÂ­tica, crypto, auditorÃƒÂ­a, plantillas
  rynx-panel/   identidad del panel, Argon2id, sesiones, RBAC, auth events
  rynxd/        daemon root, executor Linux/simulaciÃƒÂ³n, reconciliaciÃƒÂ³n
  rynx-api/     Axum, seguridad, CSRF, lÃƒÂ­mites, SPA
  rynx-cli/     rynxctl
web/            SPA sin build step
deploy/         systemd, nginx, fail2ban, logrotate, configs, instalador
scripts/        smoke test end-to-end
```

## Tests

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
powershell -ExecutionPolicy Bypass -File scripts/smoke.ps1   # Windows
```

## LÃƒÂ­mites conocidos

- El correo (Postfix/Dovecot/buzones) no se instala automÃƒÂ¡ticamente; el panel prepara
  DNS, DKIM y las tablas de OpenDKIM.
- El PTR inverso se configura en el proveedor de la IP.
- FTP usa un certificado TLS global de Pure-FTPd (FTPS explÃƒÂ­cito).
- Multi-nodo, DNS secundario, mÃƒÂ©tricas remotas y 2FA quedan para siguientes versiones.



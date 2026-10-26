# RYNXCP

<p align="center">
  <img src="logo-readme.jpg" alt="RYNXCP" width="360" />
</p>

A **from-scratch hosting control panel written in Rust** for Debian/Ubuntu VPS and
dedicated servers. It follows a declarative model: you describe the desired state
(accounts, sites, runtimes, DNS, databases, FTP, backups, firewall) and the daemon
reconciles the operating system towards it.

It is not a fork of cPanel, Plesk, HestiaCP or Vesta. The architecture, protocol,
reconciliation engine and UI are original.

## Features

| Area | What is included |
| --- | --- |
| Accounts | Isolated Linux users, package-assigned quota/backend, real suspension (Nginx/apps/FTP) and deletion |
| Web | Nginx virtual hosts, aliases, TLS, PHP-FPM, reverse proxy, security headers |
| Runtimes | PHP, Node.js, Python, Ruby, Java, .NET, Go and static sites |
| DNS | Authoritative BIND zones, A/AAAA/CNAME/MX/TXT/NS/SRV/CAA records |
| Email | Postfix + Dovecot mailboxes, aliases/forwarding, quotas, DKIM/OpenDKIM, DMARC, SRV, TLS-RPT and Roundcube webmail |
| Panel email | Local Postfix notifications for credentials, password resets, certificate alerts and delivery tests |
| Databases | MariaDB per domain, dedicated user/password, local-only access |
| FTP | Pure-FTPd virtual users, chrooted to the document root, read-only or read-write |
| Backups | Home + managed configuration, retention, restore and daily timer |
| Certificates | Let's Encrypt with SANs, bulk renewal and daily timer |
| Monitoring | Per-service state/CPU/RAM/uptime and 7-day historical series |
| Firewall | nftables rules, banned IPs, allowlists and automatic brute-force bans |
| Security | Argon2id, account+IP lockout, fail2ban auth log, RBAC |
| Console | Closed-list privileged actions with output and audit trail |
| Programs | Installed software inventory, pending updates and upgrade button |
| Users | owner/admin/operator/viewer roles, audited impersonation and revocation |
| Packages | Reusable hosting plans with resource limits, templates and a seeded editable default |

Creating a hosting account can also create a linked panel user (viewer by
default, operator scoped to that account) with a generated password. The Accounts view exposes
an "Entrar" action that impersonates that user. Emails are unique across panel
users and hosting accounts, so an administrator email cannot be reused for an
account. Linked users follow the account when it is suspended or deleted.
Linked users are scoped to their own account: they only see and manage that
account's sites, apps, PHP pools, databases, FTP, DNS, certificates and backups,
and global views (users, firewall, console, system, monitor, audit) are denied.

The web UI is available in Spanish, English (US), Portuguese, French, Arabic
(with RTL), Russian, Chinese, Japanese, Italian and German. It defaults to the
browser language, falls back to English (US), remembers the choice in
localStorage, and exposes a language selector on the login screen and in the
top bar.

Domains are stored as Punycode (IDN A-labels). The web UI converts Unicode
domains automatically (for example `laniñadelasmargaritas.es` becomes
`xn--laniadelasmargaritas-66b.es`); direct API/CLI calls must use the Punycode
form.

When a Static or PHP site is first applied, the daemon creates its document root
and, if there is no `index.html` or `index.php`, a welcome page in the UI
language of the panel user who created the site. Existing content is never
overwritten.

Mail is enabled per website from the **Correo** view. RYNXCP provisions
`mail.<domain>` for SMTP/IMAP/POP3, `webmail.<domain>` for Roundcube and the
complete DNS security set: MX, SPF, DKIM (`mail._domainkey`), `_domainkey`,
DMARC, TLS-RPT, CAA, A/AAAA, `webmail`/`imap`/`smtp`/`pop`/`autodiscover` and
SRV records for IMAPS, IMAP, POP3S, POP3, submission, submissions, Sieve and
autodiscover. Certificates cover `webmail`, `mail`, `imap`, `smtp`, `pop`,
`autodiscover` and `autoconfig`.

Mailboxes support per-account quotas, local aliases, forwarding and a domain
catch-all. Roundcube is installed with the Managesieve plugin, so each user can
configure spam filters, rules and autoresponders from the webmail UI. Mail
delivery requires outbound port 25 and a matching PTR; if the provider blocks
port 25, use an SMTP relay instead.

### Panel notifications

The panel sends its own transactional mail through Postfix on loopback
(`127.0.0.1:25`) using `noreply@<panel-domain>`. The pre-installer enables this
for FQDN hosts and publishes only the outbound records: `mail.<panel-domain>` A,
SPF, DKIM (`mail._domainkey`), `_domainkey` and DMARC. No MX is added because
the panel only sends admin mail.

Notifications cover generated panel credentials, password-reset links and
certificate-job failures. Administrators can send a test from **Sistema >
Correo del panel** (`GET/POST /api/v1/system/email*`). Password resets use
single-use SHA-256-hashed tokens that expire after 60 minutes; requesting a new
link invalidates the previous one. Because the panel domain intentionally has no
MX, `smtp_alert_to` should be an external mailbox (the installer warns when the
admin email is on the panel domain). Delivery is best-effort: failures are
logged and never block the operation that triggered the message. Serve the panel
over HTTPS so credentials and reset links are not sent over plaintext HTTP.

Manual production configuration in `/etc/rynxcp/api.toml`:

```toml
smtp_enabled = true
smtp_host = "127.0.0.1"
smtp_port = 25
smtp_from = "noreply@panel.example.com"
smtp_alert_to = "admin@example.com"
smtp_tls = "none"
public_url = "https://panel.example.com"
```

For an existing installation, generate the panel DKIM key and records with:

```bash
sudo bash deploy/panel-dns-zone.sh panel.example.com 203.0.113.10 mail
```

The daemon preserves panel DKIM entries by appending
`/etc/opendkim/KeyTable.panel` and `SigningTable.panel` whenever it rewrites the
main OpenDKIM tables.

## Packages

Administrators manage hosting plans from **Administración > Paquetes**. The view
lists every package with its main limits and exposes **Nuevo paquete**; each row
can be edited and non-default packages can be deleted. The seeded **Por defecto**
package is editable but cannot be deleted.

Each package stores:

| Section | Fields |
| --- | --- |
| General | name, account MB, bandwidth, backups (1-3) and incremental backup |
| Web | web domain/alias limits, backend (static, PHP-FPM, Node.js, Python, Ruby, Java, .NET, Go) and Nginx proxy template (Caching, Hosting, Suspended) |
| DNS | template (child-ns, no-mail, Gmail, Office 365, Zoho), calculated zone/record counters and nameserver list |
| Email | email domain/account limits and rate limit |
| Databases | database limit |
| System | system database limit |

Every numeric limit accepts 0-1000. **0 means unlimited**, and the form exposes a
**Sin límite** button on every field where that applies. Nameservers start with
`ns1.example.tld` and `ns2.example.tld`; the form can add more. DNS zone and
record counters are calculated and shown read-only.

Packages live in `panel.db` (`/var/lib/rynxcp/panel/panel.db`) and are seeded on
first API start, so a clean installation already contains the editable default
package. The auto-installers need no extra step: they start the API, which
creates the table and the default plan.

When a hosting account is created, the form asks for a package. Its limits are
inherited and shown read-only; the only package field the account can override
is **Backend plantilla**, preselected from the package and editable at creation
or later from the account's **Backend** action. The account list shows the
assigned package and backend. Package CRUD is admin-only; operators can list
packages to assign them, and scoped operators can change only their own backend.
A package cannot be deleted while it is assigned to an account.

```
GET|POST   /api/v1/packages          PATCH|DELETE /api/v1/packages/{id}
GET        /api/v1/packages/assignments
PATCH      /api/v1/packages/assignments/{account}
```

Creating an enabled site with `tls = "lets_encrypt"` also starts a certificate
job immediately. The API returns the job in `cert_job`; the panel polls
`GET /api/v1/certs/jobs/{id}` and shows the request, activation and final result
(issuer and expiry on success, certbot/daemon output on failure). A failed
issuance does not remove the site: it remains available over HTTP and the
certificate page and warning banner show the problem.

## Architecture

```
 browser
    |  HTTPS + HttpOnly cookie + CSRF
    v
+------------------------------+        Unix socket 0660 (group rynx)
| rynx-api  (user rynx)        |  CBOR  + token + SO_PEERCRED
| - Axum, sessions, RBAC       | -------------------------------+
| - Argon2id, lockout, logs    |                                |
| - panel.db (separate SQLite) |                                v
+------------------------------+                 +-------------------------------+
                                                 | rynxd  (root, no network)     |
                                                 | - re-validates every request  |
                                                 | - declarative reconciliation  |
                                                 | - useradd/chown/systemctl     |
                                                 | - nginx, php-fpm, bind,       |
                                                 |   mariadb, pure-ftpd, nft     |
                                                 | - state.db + audit.log        |
                                                 +-------------------------------+
```

Two databases are separated on purpose:

- `panel.db`: panel users, Argon2id hashes, sessions, authentication events and hosting packages.
- `state.db`: desired hosting state. Only the daemon writes to it.

Compromising the web API does not grant direct write access to hosting state.

## Security

- **Rust**: no garbage collector, no `null`, data-race-free concurrency.
- **Minimal root daemon**: no network listener in production; only a `0660` Unix
  socket in group `rynx`, with peer credential verification (`SO_PEERCRED`).
- **Two-factor transport authentication**: 256-bit token compared in constant time
  plus caller UID/GID validation.
- **No shells**: every command uses an explicit `argv` and a scrubbed environment.
  No `sh -c`, no string interpolation.
- **Two validation layers**: API and daemon. Domains, users, paths, ports, versions,
  IPs, DNS names, environment variables and commands are strictly allowlisted.
- **Atomic writes** with `O_NOFOLLOW`, restrictive permissions and confinement to
  the managed root. Nginx configs are tested with `nginx -t` and rolled back on
  failure; zones are checked with `named-checkzone`.
- **Sessions**: opaque tokens; only their SHA-256 is stored. Cookie is `HttpOnly`,
  `SameSite=Strict`, `Secure` in production, with absolute and idle expiry.
- **Brute-force protection**: sliding window + lockout per account+IP and per IP,
  failure delay, persistent events and a structured fail2ban log.
- **IP allowlists** take precedence over firewall bans.
- **Firewall defaults**: nftables uses a default-drop input policy. SSH, HTTP,
  HTTPS, DNS (53/tcp+udp) and FTPS (21 plus passive 30000-50000) are always
  allowed; SSH cannot be denied from the panel to prevent lockouts.
- **RBAC** with last-owner protection, self-protection and audited impersonation.
- **Tamper-evident audit log**: every event commits to the previous hash; editing
  or deleting a line breaks the chain and verification detects it.
- **Headers**: strict CSP, `nosniff`, `DENY`, HSTS, `no-store` on `/api`.

## Runtimes

| Runtime | Mechanism | Suggested versions |
| --- | --- | --- |
| PHP | Per-account FPM pool, `open_basedir`, dangerous functions disabled | 8.1 - 8.5 |
| Node.js | systemd + reverse proxy | 18, 20, 22, 24 |
| Python | gunicorn / uvicorn under systemd | 3.10 - 3.13 |
| Ruby | puma / rackup under systemd | 3.2 - 3.4 |
| Java | `java -jar` under systemd | 11, 17, 21 |
| .NET | `dotnet App.dll` under systemd | 8, 9 |
| Go | compiled binary under systemd | n/a |
| Static | Nginx directly | n/a |

Each application gets a shell-less user, a validated loopback port, a hardened
systemd unit, a `0640` environment file and a vhost with security headers.

## DNS and email (SPF / DKIM / DMARC)

With `dns_enabled = true`, every site gets a BIND zone with SOA, NS, A/AAAA, CNAME,
CAA and email records. With `mail_enabled = true` and `mail_hostname` configured,
the panel also adds:

- `MX` pointing to the mail host.
- `TXT @` with `v=spf1 mx a ~all`.
- `TXT _dmarc` with `p=quarantine`, `adkim=s`, `aspf=s`.
- `TXT <selector>._domainkey` with the DKIM public key (RSA 2048).
- `KeyTable` and `SigningTable` entries for OpenDKIM.

The **reverse PTR** is controlled by the IP provider, not the panel: it must resolve
to `mail_hostname`. Verify with `dig -x YOUR_IP`. Together with SPF/DKIM/DMARC this
is essential to keep email out of spam.

The panel provisions DNS and DKIM signing material. The one-shot pre-installer
also installs Postfix, Dovecot, OpenDKIM and Roundcube; for a manual install run
`deploy/mail-setup.sh` before creating mail domains.

## Databases

One MariaDB database per domain (`rynx_<domain>`), `utf8mb4`, with a dedicated user
and a random 32-character password. The user can only connect over `localhost`.
The password is stored in `state.db` (`0600`) and shown to administrators.

## FTP

Pure-FTPd virtual users mapped to the account UID (read-write) or to the `rynxftp`
user (read-only), chrooted to the site document root. Explicit FTPS is supported.
A password is generated when not provided.

## Backups

Each backup includes the home directory, vhost, DNS zone, PHP pool, systemd units and
environment files. Backups are stored in `/var/backups/rynxcp/<account>/`, the last 7
per account are retained, and they can be restored from the UI. A systemd timer runs
them daily.

Restore only extracts managed paths (`home/<account>`, nginx, php, systemd, apps),
rejects `..` and absolute paths, and re-applies desired state afterwards.

## Monitoring and console

The daemon samples every 60 seconds: load, memory, disk, active services and
state/CPU/RAM for nginx, PHP-FPM, applications and the daemon itself. Seven days of
history are retained.

The admin console **is not a shell**. It only runs closed actions (Nginx test/reload,
app restart/logs, PHP-FPM reload, reconciliation, TLS renewal, backups, disk usage)
and every execution is audited.

## One-shot pre-installer (recommended)

`deploy/preinstall.sh` prepares a fresh Debian/Ubuntu server end to end:

```bash
sudo bash deploy/preinstall.sh
```

It is interactive and asks for:

1. **Panel host** - detected automatically (`hostname -f`, otherwise the public IP)
   and offered as the default.
2. **Admin email** - defaults to `admin@<host>` when the host is a domain.
3. **Admin password** - hidden input, confirmed, minimum 12 characters and at least
   three character classes; it is never written to the log.

Non-interactive usage:

```bash
sudo RYNX_ADMIN_PASSWORD='Your-Strong-Pass1' \
  bash deploy/preinstall.sh --host panel.example.com --email admin@example.com --yes
```

Add `--panel-port 2083` (or `RYNX_PANEL_PORT=2083`) to publish the panel on a
custom port. Port 80 is still used for the ACME HTTP-01 challenge and redirects
to the panel port.

What it installs and configures:

- Security updates plus `unattended-upgrades`.
- **PHP 8.5** from the security-maintained Ondrej (Ubuntu) or Sury (Debian)
  repository, with hardened `php.ini` defaults and OPcache.
- Nginx, Certbot, MariaDB, BIND9, Pure-FTPd, OpenDKIM tooling, fail2ban, nftables,
  logrotate.
- Node.js, Python 3, Ruby, Java, Go and .NET when the distribution provides it.
- The Rust toolchain when missing, then `cargo build --release` and `deploy/install.sh`.
- Postfix, Dovecot (IMAP/POP3/LMTP/Sieve/Managesieve), OpenDKIM and Roundcube
  via `deploy/mail-setup.sh` (also invoked by the pre-installer; set
  `RYNX_SKIP_MAIL=1` to skip it).
- DNS, database, FTP and firewall enabled; the SSH port is detected from
  `sshd_config` and always kept open. Mail ports 25/110/143/465/587/993/995 and
  4190 are allowed by the nftables rules.
- Public authoritative BIND via `deploy/bind-public.sh`: external queries are
  allowed (`allow-query any`) while recursion stays limited to localhost. The
  nameserver A records/glue still have to be created at the registrar.
- A separate BIND zone for the panel hostname, generated by
  `deploy/panel-dns-zone.sh` from the hostname/IP entered by the installer, with
  `ns1.<panel-host>` and `ns2.<panel-host>` A records. This keeps the advertised
  nameservers valid without creating a website for the panel host. When the mail
  stack is installed, it also creates `mail.<panel-host>` plus outbound SPF,
  DKIM and DMARC, generates the panel DKIM key and configures OpenDKIM.
- Panel SMTP on `127.0.0.1:25` with `noreply@<panel-host>`, used for credential
  emails, password resets and certificate alerts. The **Sistema** view exposes a
  delivery test.
- The panel database is initialized on first start, including the editable
  **Por defecto** package available under **Administración > Paquetes**.
- A panel virtual host and, for a domain with working DNS, a Let's Encrypt
  certificate with HTTP-to-HTTPS redirect. If issuance fails, the panel stays on
  HTTP and the script explains how to retry.

The panel is published at `http(s)://<host>[:port]` and the administrator is created with
the email/password you entered. Email is opt-in per website from the **Correo**
view; enabling it starts the webmail certificate job and provisions the mail DNS
records.

## Installing on Debian/Ubuntu (manual)

Prefer the pre-installer above unless you need full control.

System dependencies. Add the PHP 8.5 repository first (Ondrej PPA on Ubuntu, Sury on
Debian); the pre-installer does this automatically:

```bash
sudo apt update
sudo apt install -y build-essential pkg-config nginx certbot \
  php8.5-fpm php8.5-cli php8.5-mysql \
  mariadb-server bind9 bind9-utils \
  pure-ftpd opendkim opendkim-tools fail2ban nftables \
  nodejs python3-venv ruby-full default-jre dotnet-sdk-8.0 golang
```

Build and install:

```bash
cargo build --release
sudo bash deploy/install.sh
sudo bash deploy/mail-setup.sh
sudo bash deploy/panel-dns-zone.sh panel.example.com 203.0.113.10 mail
```

The installer creates the `rynx`/`rynxftp` users and groups, directories, token,
authentication log, fail2ban integration, logrotate rules, units and timers, then
starts `rynxd` and `rynx-api`.

```bash
sudo rynxctl admin create admin --email you@example.com
sudo cp deploy/nginx/panel.conf.example /etc/nginx/sites-available/rynxcp-panel
sudo ln -s /etc/nginx/sites-available/rynxcp-panel /etc/nginx/sites-enabled/
sudo certbot --nginx -d panel.example.com
```

If Nginx terminates TLS in front of the API, set `trust_proxy_headers = true` in
`/etc/rynxcp/api.toml`.

Installed timers:

- `rynxcp-cert-renew.timer`: renews certificates daily.
- `rynxcp-backup.timer`: nightly backups.

The panel shows a warning banner when a certificate is expiring, expired,
missing or in error, with a shortcut to the certificate page. Automatic renewal
failures are recorded in the `rynxcp-cert-renew` journal and reflected in that
banner on the next data refresh.

## Local development (Windows or Linux)

Simulation mode renders every artifact under `./data/daemon/sandbox` and never
touches the operating system.

```bash
cargo build
# terminal 1
target/debug/rynxd.exe --config rynxd.toml
# terminal 2
target/debug/rynxctl.exe admin create admin --email you@example.com
# terminal 3
target/debug/rynx-api.exe --config rynx-api.toml
```

Open <http://127.0.0.1:8080>. On Linux, use the binaries without `.exe`.

## CLI

```bash
rynxctl admin create <user> --email <email> [--role owner|admin|operator|viewer]
rynxctl admin list
rynxctl admin passwd <user>
rynxctl admin disable|enable <user>
rynxctl daemon ping|info
rynxctl cert renew [--staging]
rynxctl backup all
rynxctl backup list
rynxctl monitor
rynxctl audit --limit 20
rynxctl reconcile
```

## API

Except for login, password reset, health and meta, every route requires a session;
mutating methods require `x-csrf-token`.

```
POST   /api/v1/auth/login
POST   /api/v1/auth/forgot-password
POST   /api/v1/auth/reset-password
POST   /api/v1/auth/logout
GET    /api/v1/auth/me
POST   /api/v1/auth/stop-impersonation
POST   /api/v1/users/{username}/impersonate

GET|POST   /api/v1/accounts          DELETE /api/v1/accounts/{username}
GET|POST   /api/v1/websites          DELETE /api/v1/websites/{domain}
GET|POST   /api/v1/php-pools         DELETE /api/v1/php-pools/{username}
GET|POST   /api/v1/apps              DELETE /api/v1/apps/{username}/{name}
GET|POST   /api/v1/certs             POST   /api/v1/certs/renew
POST       /api/v1/certs/jobs        GET    /api/v1/certs/jobs/{id}
GET|POST   /api/v1/mail/domains       DELETE /api/v1/mail/domains/{domain}
POST       /api/v1/mail/domains/{domain}/tls
GET|POST   /api/v1/mail/accounts      DELETE /api/v1/mail/accounts/{id}
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
GET        /api/v1/system/email      POST   /api/v1/system/email/test
POST       /api/v1/console
GET|POST   /api/v1/users             PATCH|DELETE /api/v1/users/{username}
GET|POST   /api/v1/packages          PATCH|DELETE /api/v1/packages/{id}
GET        /api/v1/packages/assignments
PATCH      /api/v1/packages/assignments/{account}
POST       /api/v1/reconcile
```

## Layout

```
crates/
  rynx-proto/   versioned CBOR protocol and shared models
  rynx-core/    config, validation, policy, crypto, audit, templates
  rynx-panel/   panel identity, Argon2id, sessions, RBAC, auth events, packages
  rynxd/        root daemon, Linux/simulation executor, reconciliation
  rynx-api/     Axum, security, CSRF, rate limits, SPA
  rynx-cli/     rynxctl
web/            SPA with no build step
deploy/         systemd, nginx, fail2ban, logrotate, configs, installer, preinstaller
scripts/        end-to-end smoke test
```

## Tests

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
powershell -ExecutionPolicy Bypass -File scripts/smoke.ps1   # Windows
```

## License

RYNXCP is free software: you can redistribute it and/or modify it under the terms
of the **GNU Affero General Public License**, version 3 or (at your option) any later
version. See [LICENSE](LICENSE) for the full text.

The software is provided **"AS IS", WITHOUT WARRANTY OF ANY KIND**, and the authors
accept **no liability** for any damage, data loss, service interruption or security
incident arising from its use. Sections 15 and 16 of the license contain the full
warranty disclaimer and limitation of liability.

Because the AGPL applies to network use, if you run a modified version of RYNXCP as
a service you must offer its complete corresponding source code to remote users.

Copyright (C) 2026 RYNXCP contributors.

## Known limitations

- Reverse PTR is configured at the IP provider.
- FTP uses Pure-FTPd's global TLS certificate (explicit FTPS).
- Package limits are stored and shown, and the account disk quota is derived
  from the package; web/mail/database/backup counts are not enforced as hard
  limits yet.
- Multi-node, secondary DNS, remote metrics and 2FA are planned for later versions.

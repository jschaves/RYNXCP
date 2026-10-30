# Automated installers

RYNXCP ships two one-shot installers under `deploy/`. Both clone the current
`main` branch and run `deploy/preinstall.sh`, so they install the same hardened
panel stack:

- RYNXCP daemon, API, CLI and web panel.
- Nginx, PHP 8.5, MariaDB, BIND9, Pure-FTPd, OpenDKIM, fail2ban and nftables.
- Postfix + Dovecot (2.3/2.4 compatible), quotas, Sieve and Roundcube.
- ClamAV for SMTP and daily filesystem scans, plus SpamAssassin SMTP scoring.
- Optional AI agent with local Ollama and/or an explicitly enabled external provider.
- Hosting packages with the seeded **Por defecto** plan.
- The per-site file manager and phpMyAdmin behind the panel login.
- Let's Encrypt, backup and antivirus timers.

Run them on a clean, 64-bit Debian 13 (Trixie) or Ubuntu 26.04 LTS server as
root. Debian 12 (Bookworm) and Ubuntu 24.04 LTS are supported through the
Sury/Ondrej PHP 8.5 repositories. Debian 10/11, Ubuntu 20.04/22.04 and Linux
derivatives are not recommended or tested. They are intended for fresh VPS
installations.

## Panel only: `autoinstall.sh`

Direct download:

```bash
curl -fsSL https://raw.githubusercontent.com/jschaves/RYNXCP/main/deploy/autoinstall.sh \
  -o /root/autoinstall.sh
sudo bash /root/autoinstall.sh
```

Fully non-interactive. With no arguments it auto-detects the panel host and
admin email, generates a strong admin password, and installs everything. The
generated password is printed once and stored in
`/root/rynxcp-credentials.txt` (mode `0600`).

The panel is published on **port 2083** by default
(`https://<host>:2083`, or `http://<ip>:2083` without TLS). Port 80 remains
dedicated to the ACME challenge and redirects to 2083.

Useful options:

```bash
sudo bash /root/autoinstall.sh \
  --host panel.example.com \
  --email admin@example.com

sudo RYNX_ADMIN_PASSWORD='Strong-Pass1' \
  sudo bash /root/autoinstall.sh --host panel.example.com --email admin@example.com
```

| Option | Description |
| --- | --- |
| `--host <fqdn\|ip>` | Panel hostname or public IPv4. Auto-detected when omitted. |
| `--email <address>` | Admin email. Defaults to `admin@<host>` for domains. |
| `--password <value>` | Admin password. Prefer `RYNX_ADMIN_PASSWORD`. |
| `--panel-port <port>` | Public panel port (default `2083`; `80` is kept for ACME). |
| `--agent-llm <mode>` | `rules` (default), `local`, `api` or `hybrid`. `local`/`hybrid` provision Ollama; `api` uses the external provider. |
| `--agent-model <name>` | Ollama model for local/hybrid; defaults to a model sized for host RAM. |
| `--agent-api-url <url>` | External HTTPS OpenAI-compatible endpoint for `api`/`hybrid`. |
| `--agent-api-model <name>` | External model name for `api`/`hybrid`. |
| `--agent-api-key-file <path>` | File containing the external API key; copied to `/etc/rynxcp/agent.env` (`0600`, group `rynx`). |
| `--no-tls` | Do not request a Let's Encrypt certificate. |
| `--repo <url>` | RYNXCP Git repository. |
| `--ref <ref>` | Branch/tag to install (default `main`). |
| `--dir <path>` | Checkout directory (default `/opt/rynxcp-src`). |
| `--source-dir <path>` | Reuse an existing checkout instead of cloning. |
| `--yes` | Accepted for compatibility; the script is non-interactive. |

Set `RYNX_SKIP_PHPMYADMIN=1` to skip phpMyAdmin and `RYNX_SKIP_MAIL=1` to skip
the mail stack and its SMTP antivirus/antispam milters (both are installed by
default). The system ClamAV scan timer is still installed.

### Local AI agent

The agent runs deterministic rules by default and can enrich them with a local
LLM. Enable it from the installer, not with manual commands:

```bash
sudo RYNX_AGENT_LLM=local bash /root/autoinstall.sh \
  --host panel.example.com --email admin@example.com

sudo bash /root/autoinstall.sh --agent-llm hybrid --agent-model qwen2.5:3b
```

The installer downloads Ollama when missing, pins it to `127.0.0.1:11434`,
limits it to one loaded model/worker, pulls the model and writes the `[agent]`
section of `/etc/rynxcp/api.toml`. Re-running the installer is safe and
self-healing. If provisioning fails, it keeps `mode = "rules"` so the panel
still starts and the agent continues with deterministic findings.

External providers are opt-in through the same installer:

```bash
sudo bash /root/autoinstall.sh --agent-llm api \
  --agent-api-url https://provider.example/v1 \
  --agent-api-model model-name \
  --agent-api-key-file /root/provider.key
```

The key file is copied to `/etc/rynxcp/agent.env` with mode `0600` and group
`rynx`; `rynx-api.service` loads it through `EnvironmentFile`. The installer
requires HTTPS, and the daemon runtime rejects private, loopback, link-local and
metadata addresses while pinning the resolved public IP. `--agent-llm hybrid`
uses the local model first and the external provider as fallback.

## Panel + MagicRita: `autoinstall-panel-magicrita.sh`

Direct download:

```bash
curl -fsSL https://raw.githubusercontent.com/jschaves/RYNXCP/main/deploy/autoinstall-panel-magicrita.sh \
  -o /root/autoinstall-panel-magicrita.sh
sudo RYNX_ADMIN_PASSWORD='Strong-Pass1' \
  bash /root/autoinstall-panel-magicrita.sh \
  --panel-host panel.example.com \
  --panel-email admin@example.com \
  --magicrita-domain chat.example.com \
  --yes
```

This installer additionally clones and builds
[MagicRita](https://github.com/jschaves/magicrita), creates the
`magicrita-signal` service on `127.0.0.1:8787`, publishes it with Nginx and
issues its Let's Encrypt certificate.

`--magicrita-domain` is required. The panel part uses the same
`deploy/preinstall.sh`, so the panel, file manager, phpMyAdmin, packages and
security fixes are identical to the panel-only installer.

Common options:

| Option | Description |
| --- | --- |
| `--magicrita-domain <fqdn>` | Public MagicRita domain (required). |
| `--magicrita-email <address>` | Let's Encrypt email (default `admin@<domain>`). |
| `--magicrita-admin-user <user>` | MagicRita admin user (default `admin`). |
| `--magicrita-admin-password <pass>` | MagicRita admin password; generated when omitted. |
| `--magicrita-admin-path <path>` | MagicRita admin path (default `topogue`). |
| `--magicrita-port <port>` | Loopback relay port (default `8787`). |
| `--panel-host`, `--panel-email`, `--panel-password`, `--panel-port` | Forwarded to the panel installer. |
| `--panel-agent-llm`, `--panel-agent-model` | Forwarded to the panel installer for local LLM provisioning. |
| `--panel-agent-api-url`, `--panel-agent-api-model`, `--panel-agent-api-key-file` | Forwarded for external provider provisioning. |
| `--rynxcp-ref <ref>`, `--rynxcp-dir <dir>` | RYNXCP source selection. |
| `--repo-token <token>` | GitHub token for private repositories. |
| `--no-tls` | Do not request Let's Encrypt certificates. |
| `--skip-rynxcp` | Update only MagicRita. |
| `--yes` | Non-interactive mode; requires a panel password in `RYNX_ADMIN_PASSWORD` or `--panel-password`. |

The combined installer is interactive unless `--yes` is passed. It does not
generate the panel password itself; provide `RYNX_ADMIN_PASSWORD` or
`--panel-password` when using `--yes`.

## Post-install verification

```bash
systemctl status rynxd rynx-api postfix dovecot nginx php8.5-fpm
systemctl status clamav-daemon clamav-milter spamd spamass-milter
systemctl status ollama   # only when --agent-llm local/hybrid was used
systemctl list-timers 'rynxcp-*'
sudo doveconf -n
sudo nginx -t
sudo rynxctl daemon ping
```

Open the panel, go to **Administración > Paquetes**, **Archivos** and
**Bases de datos > phpMyAdmin**. phpMyAdmin requires the active panel session
and then a valid MariaDB login.

For TLS and mail, the domain DNS must already point at the server; that is the
only step outside the script.

## Security notes

- Both installers must run as root on a fresh server.
- The panel API binds to `127.0.0.1:8080`; only Nginx exposes it.
- phpMyAdmin is gated by an Nginx `auth_request` against the panel session and
  is not reachable without logging in.
- The file manager is scoped to each site document root and runs with the
  account's filesystem UID/GID; symlink edits and traversal are rejected, and
  symlinks are listed but cannot be opened for editing.
- Admin credentials are never written to the installer logs. Remove
  `/root/rynxcp-credentials.txt` after storing the generated password.
- Review `/var/log/rynxcp/autoinstall.log` and
  `/var/log/rynxcp/preinstall.log` if an installation fails.

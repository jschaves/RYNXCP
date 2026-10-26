#!/usr/bin/env bash
# Instalador de RYNXCP + MagicRita desde Git.
#
# Descarga directa desde GitHub:
#   curl -fsSL https://raw.githubusercontent.com/jschaves/RYNXCP/main/deploy/autoinstall-panel-magicrita.sh -o /root/autoinstall-panel-magicrita.sh
#   sudo bash /root/autoinstall-panel-magicrita.sh --magicrita-domain chat.example.com --yes
#
# Pensado para un Ubuntu/Debian limpio (objetivo: Ubuntu 26.04 LTS).
# Ejecutar como root:
#
#   sudo bash autoinstall-panel-magicrita.sh \
#     --panel-host panel.example.com \
#     --panel-email admin@example.com \
#     --magicrita-domain chat.example.com \
#     --magicrita-email admin@example.com \
#     --yes
#
# El password del panel se recomienda por entorno, no por argumento:
#
#   sudo RYNX_ADMIN_PASSWORD='...' bash autoinstall-panel-magicrita.sh ... --yes
#
# Qué hace:
#   1. Clona/actualiza RYNXCP y ejecuta su deploy/preinstall.sh. La API crea el
#      paquete editable "Por defecto" al arrancar, disponible en Administración
#      > Paquetes. También instala el gestor de archivos por sitio y phpMyAdmin
#      tras el login del panel.
#   2. Clona/actualiza MagicRita, la compila y la publica con nginx.
#   3. Crea magicrita-signal en 127.0.0.1:8787 (nunca expuesto).
#   4. Emite el certificado Let's Encrypt del dominio de MagicRita.
#
# No usa UFW: el firewall lo gestiona RYNXCP con nftables.
#
# Antes de ejecutar: haz un snapshot/backup del VPS. El preinstall de RYNXCP
# hace "apt upgrade" e instala nginx, PHP 8.5, MariaDB, BIND9, Pure-FTPd,
# OpenDKIM, fail2ban, nftables y varios runtimes.
#
# Para actualizar MagicRita sin reinstalar RYNXCP:
#   sudo bash autoinstall-panel-magicrita.sh --skip-rynxcp --magicrita-domain chat.example.com --yes

set -Eeuo pipefail
export DEBIAN_FRONTEND=noninteractive

# ---------------------------------------------------------------------------
# Configuración (todo se puede sobreescribir por entorno o por argumento)
# ---------------------------------------------------------------------------
RYNXCP_REPO="${RYNXCP_REPO:-https://github.com/jschaves/RYNXCP.git}"
RYNXCP_REF="${RYNXCP_REF:-main}"
RYNXCP_DIR="${RYNXCP_DIR:-/opt/rynxcp-src}"

MAGICRITA_REPO="${MAGICRITA_REPO:-https://github.com/jschaves/magicrita.git}"
MAGICRITA_REF="${MAGICRITA_REF:-main}"
MAGICRITA_DIR="${MAGICRITA_DIR:-/opt/magicrita}"

MAGICRITA_DOMAIN="${MAGICRITA_DOMAIN:-}"
MAGICRITA_EMAIL="${MAGICRITA_EMAIL:-}"
MAGICRITA_ADMIN_USER="${MAGICRITA_ADMIN_USER:-admin}"
MAGICRITA_ADMIN_PASSWORD="${MAGICRITA_ADMIN_PASSWORD:-}"
MAGICRITA_ADMIN_PATH="${MAGICRITA_ADMIN_PATH:-topogue}"
MAGICRITA_PORT="${MAGICRITA_PORT:-8787}"

PANEL_HOST="${RYNX_HOST:-${PANEL_HOST:-}}"
PANEL_EMAIL="${RYNX_ADMIN_EMAIL:-${PANEL_EMAIL:-}}"
PANEL_PASSWORD="${RYNX_ADMIN_PASSWORD:-${PANEL_PASSWORD:-}}"
PANEL_PORT="${RYNX_PANEL_PORT:-${PANEL_PORT:-80}}"
GITHUB_TOKEN="${GITHUB_TOKEN:-}"

ASSUME_YES=0
NO_TLS=0
SKIP_RYNXCP=0

VHOST_FILE="/etc/nginx/sites-available/magicrita.conf"
VHOST_LINK="/etc/nginx/sites-enabled/magicrita.conf"
SERVICE="magicrita-signal"
MAGICRITA_USER="magicrita"

NODE_BIN=""
NPM_BIN=""
TLS_OK=0
GENERATED_PASSWORD=""
ASKPASS_FILE=""

# ---------------------------------------------------------------------------
# Utilidades
# ---------------------------------------------------------------------------
log()  { printf '\n\033[1;32m==>\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33mwarning:\033[0m %s\n' "$*" >&2; }
die()  { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

cleanup() {
  local file
  for file in "${ASKPASS_FILE:-}"; do
    if [[ -n "$file" && -f "$file" ]]; then
      rm -f "$file"
    fi
  done
}
trap cleanup EXIT

usage() {
  cat <<'EOF'
Uso: sudo bash autoinstall-panel-magicrita.sh --magicrita-domain DOMINIO [opciones]

Obligatorio:
  --magicrita-domain DOMINIO      Dominio público de MagicRita (FQDN).

Panel RYNXCP (se reenvían a deploy/preinstall.sh):
  --panel-host HOST               Host/IP del panel. Si se omite, preinstall lo detecta.
  --panel-email EMAIL             Email del administrador del panel.
  --panel-password PASS           Password del panel (mejor RYNX_ADMIN_PASSWORD).
  --panel-port PORT               Puerto público del panel (por defecto 80).
                                  El 80 se mantiene para el reto ACME.

MagicRita:
  --magicrita-email EMAIL         Email para Let's Encrypt (por defecto admin@DOMINIO).
  --magicrita-admin-user USER     Usuario del panel de MagicRita (por defecto admin).
  --magicrita-admin-password PASS Password del panel de MagicRita (si no, se genera).
  --magicrita-admin-path PATH     Ruta del panel (por defecto topogue).
  --magicrita-port PORT           Puerto loopback del relay (por defecto 8787).
  --magicrita-ref REF             Rama/tag del repo (por defecto main).
  --magicrita-dir DIR             Directorio (por defecto /opt/magicrita).

Repositorios:
  --rynxcp-ref REF                Rama/tag de RYNXCP (por defecto main).
  --rynxcp-dir DIR                Directorio fuente (por defecto /opt/rynxcp-src).
  --repo-token TOKEN              Token para repos privados (o GITHUB_TOKEN).

Modos:
  --no-tls                        No pedir certificados Let's Encrypt.
  --skip-rynxcp                   No instalar/actualizar RYNXCP.
  --yes                           Sin preguntas interactivas.
  -h, --help                      Esta ayuda.

Variables de entorno equivalentes: RYNX_HOST, RYNX_ADMIN_EMAIL,
RYNX_ADMIN_PASSWORD, RYNX_PANEL_PORT, MAGICRITA_*, GITHUB_TOKEN, RYNXCP_REF, RYNXCP_DIR.
EOF
}

require_root() {
  [[ "${EUID}" -eq 0 ]] || die "ejecuta este script como root (sudo bash autoinstall-panel-magicrita.sh ...)"
}

detect_os() {
  [[ -r /etc/os-release ]] || die "/etc/os-release no existe; se requiere Ubuntu o Debian"
  # shellcheck disable=SC1091
  . /etc/os-release
  case "${ID:-}" in
    ubuntu|debian) ;;
    *) die "distribución no soportada: ${ID:-desconocida} (se requiere Ubuntu o Debian)" ;;
  esac
}

preflight_services() {
  local svc
  for svc in apache2 caddy httpd; do
    if systemctl is-active --quiet "$svc" 2>/dev/null; then
      die "$svc está activo y ocupará 80/443; deténlo antes de instalar"
    fi
  done
}

preflight_dns() {
  (( NO_TLS )) && return 0
  is_domain "$MAGICRITA_DOMAIN" || return 0
  command -v getent >/dev/null 2>&1 || return 0
  local resolved public
  resolved="$(getent ahostsv4 "$MAGICRITA_DOMAIN" 2>/dev/null | awk 'NR==1 {print $1}' || true)"
  public="$(curl -4 -fsS --max-time 6 https://api.ipify.org 2>/dev/null || true)"
  if [[ -z "$resolved" ]]; then
    warn "DNS: $MAGICRITA_DOMAIN no resuelve todavía; Let's Encrypt fallará"
  elif [[ -n "$public" && "$resolved" != "$public" ]]; then
    warn "DNS: $MAGICRITA_DOMAIN apunta a $resolved, pero la IP pública es $public"
  fi
}

is_domain() {
  [[ "$1" =~ ^([a-z0-9]([a-z0-9-]{0,61}[a-z0-9])?\.)+[a-z]{2,63}$ ]]
}

to_ascii_domain() {
  local value="$1"
  [[ -n "$value" ]] || { printf '%s' "$value"; return; }
  value="${value%.}"
  if command -v idn2 >/dev/null 2>&1; then
    printf '%s' "$value" | idn2 2>/dev/null || printf '%s' "$value"
  elif command -v python3 >/dev/null 2>&1; then
    python3 -c 'import sys
try:
    print(sys.argv[1].encode("idna").decode("ascii"))
except Exception:
    print(sys.argv[1])' "$value"
  else
    printf '%s' "$value"
  fi
}

to_ascii_email() {
  local email="$1" local_part domain
  if [[ "$email" != *@* ]]; then printf '%s' "$email"; return; fi
  local_part="${email%@*}"
  domain="${email#*@}"
  printf '%s@%s' "$local_part" "$(to_ascii_domain "$domain")"
}

is_ipv4() {
  local ip="$1" octet
  [[ "$ip" =~ ^([0-9]{1,3}\.){3}[0-9]{1,3}$ ]] || return 1
  IFS=. read -r -a parts <<<"$ip"
  for octet in "${parts[@]}"; do
    (( octet >= 0 && octet <= 255 )) || return 1
  done
}

validate_admin_path() {
  local path="$1"
  [[ "$path" =~ ^[A-Za-z0-9][A-Za-z0-9_-]{0,63}$ ]] || \
    die "ruta de administración inválida: '$path' (letras, números, - o _)"
  case "${path,,}" in
    welcome|unlock|legal|people|saved|protocolo|discover|compose|messages|settings|n|p|brand|beta|signal|admin-api|moderation|assets)
      die "ruta de administración reservada: '$path'" ;;
  esac
}

# ---------------------------------------------------------------------------
# Argumentos
# ---------------------------------------------------------------------------
while [[ $# -gt 0 ]]; do
  case "$1" in
    --magicrita-domain)
      [[ -n "${2:-}" ]] || die "falta valor para --magicrita-domain"
      MAGICRITA_DOMAIN="$2"; shift 2 ;;
    --magicrita-email)
      [[ -n "${2:-}" ]] || die "falta valor para --magicrita-email"
      MAGICRITA_EMAIL="$2"; shift 2 ;;
    --magicrita-admin-user)
      [[ -n "${2:-}" ]] || die "falta valor para --magicrita-admin-user"
      MAGICRITA_ADMIN_USER="$2"; shift 2 ;;
    --magicrita-admin-password)
      [[ -n "${2:-}" ]] || die "falta valor para --magicrita-admin-password"
      MAGICRITA_ADMIN_PASSWORD="$2"; shift 2 ;;
    --magicrita-admin-path)
      [[ -n "${2:-}" ]] || die "falta valor para --magicrita-admin-path"
      MAGICRITA_ADMIN_PATH="$2"; shift 2 ;;
    --magicrita-port)
      [[ -n "${2:-}" ]] || die "falta valor para --magicrita-port"
      MAGICRITA_PORT="$2"; shift 2 ;;
    --magicrita-ref)
      [[ -n "${2:-}" ]] || die "falta valor para --magicrita-ref"
      MAGICRITA_REF="$2"; shift 2 ;;
    --magicrita-dir)
      [[ -n "${2:-}" ]] || die "falta valor para --magicrita-dir"
      MAGICRITA_DIR="$2"; shift 2 ;;
    --panel-host)
      [[ -n "${2:-}" ]] || die "falta valor para --panel-host"
      PANEL_HOST="$2"; shift 2 ;;
    --panel-email)
      [[ -n "${2:-}" ]] || die "falta valor para --panel-email"
      PANEL_EMAIL="$2"; shift 2 ;;
    --panel-password)
      [[ -n "${2:-}" ]] || die "falta valor para --panel-password"
      PANEL_PASSWORD="$2"; shift 2 ;;
    --panel-port)
      [[ -n "${2:-}" ]] || die "falta valor para --panel-port"
      PANEL_PORT="$2"; shift 2 ;;
    --rynxcp-ref)
      [[ -n "${2:-}" ]] || die "falta valor para --rynxcp-ref"
      RYNXCP_REF="$2"; shift 2 ;;
    --rynxcp-dir)
      [[ -n "${2:-}" ]] || die "falta valor para --rynxcp-dir"
      RYNXCP_DIR="$2"; shift 2 ;;
    --repo-token)
      [[ -n "${2:-}" ]] || die "falta valor para --repo-token"
      GITHUB_TOKEN="$2"; shift 2 ;;
    --no-tls)     NO_TLS=1; shift ;;
    --skip-rynxcp) SKIP_RYNXCP=1; shift ;;
    --yes)        ASSUME_YES=1; shift ;;
    -h|--help)    usage; exit 0 ;;
    *) die "opción desconocida: '$1' (usa --help)" ;;
  esac
done

require_root
detect_os

if [[ -n "$PANEL_HOST" ]]; then
  PANEL_HOST="${PANEL_HOST#http://}"
  PANEL_HOST="${PANEL_HOST#https://}"
  PANEL_HOST="${PANEL_HOST%%/*}"
  PANEL_HOST="${PANEL_HOST,,}"
  if ! is_ipv4 "$PANEL_HOST"; then
    PANEL_HOST="$(to_ascii_domain "$PANEL_HOST")"
  fi
fi
if [[ -n "$PANEL_EMAIL" ]]; then
  PANEL_EMAIL="$(to_ascii_email "$PANEL_EMAIL")"
fi

# ---------------------------------------------------------------------------
# Validación
# ---------------------------------------------------------------------------
if [[ -z "$MAGICRITA_DOMAIN" ]]; then
  if [[ -t 0 ]]; then
    read -r -p "Dominio de MagicRita (ej. chat.example.com): " MAGICRITA_DOMAIN || true
  fi
  [[ -n "$MAGICRITA_DOMAIN" ]] || die "define --magicrita-domain"
fi
MAGICRITA_DOMAIN="${MAGICRITA_DOMAIN,,}"
MAGICRITA_DOMAIN="${MAGICRITA_DOMAIN#http://}"
MAGICRITA_DOMAIN="${MAGICRITA_DOMAIN#https://}"
MAGICRITA_DOMAIN="${MAGICRITA_DOMAIN%%/*}"
if ! is_ipv4 "$MAGICRITA_DOMAIN"; then
  MAGICRITA_DOMAIN="$(to_ascii_domain "$MAGICRITA_DOMAIN")"
fi

if ! is_domain "$MAGICRITA_DOMAIN"; then
  if (( NO_TLS )) && is_ipv4 "$MAGICRITA_DOMAIN"; then
    warn "usas una IP sin TLS: el navegador no tratará el origen como seguro y WebCrypto puede fallar"
  else
    die "dominio inválido: '$MAGICRITA_DOMAIN'"
  fi
fi

if [[ -z "$MAGICRITA_EMAIL" ]]; then
  if is_domain "$MAGICRITA_DOMAIN"; then
    MAGICRITA_EMAIL="admin@$MAGICRITA_DOMAIN"
  else
    MAGICRITA_EMAIL="admin@example.com"
  fi
fi
MAGICRITA_EMAIL="$(to_ascii_email "$MAGICRITA_EMAIL")"
[[ "$MAGICRITA_EMAIL" =~ ^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,24}$ ]] || \
  die "email inválido: '$MAGICRITA_EMAIL'"

[[ "$MAGICRITA_ADMIN_USER" =~ ^[A-Za-z0-9._-]{1,64}$ ]] || die "usuario de MagicRita inválido"
validate_admin_path "$MAGICRITA_ADMIN_PATH"

[[ "$MAGICRITA_PORT" =~ ^[0-9]+$ ]] || die "puerto inválido: '$MAGICRITA_PORT'"
MAGICRITA_PORT=$((10#$MAGICRITA_PORT))
(( MAGICRITA_PORT >= 1024 && MAGICRITA_PORT <= 65535 )) || die "el puerto debe estar entre 1024 y 65535"

[[ "$PANEL_PORT" =~ ^[0-9]+$ ]] || die "puerto de panel inválido: '$PANEL_PORT'"
PANEL_PORT=$((10#$PANEL_PORT))
(( PANEL_PORT >= 1 && PANEL_PORT <= 65535 )) || die "el puerto del panel debe estar entre 1 y 65535"
case "$PANEL_PORT" in
  22|7801|8080|"$MAGICRITA_PORT") die "el puerto del panel $PANEL_PORT está reservado o en uso" ;;
esac
case "$MAGICRITA_PORT" in
  80|443|8080|7801) die "el puerto $MAGICRITA_PORT está reservado por RYNXCP" ;;
esac

if [[ -n "$MAGICRITA_ADMIN_PASSWORD" ]]; then
  (( ${#MAGICRITA_ADMIN_PASSWORD} >= 8 && ${#MAGICRITA_ADMIN_PASSWORD} <= 128 )) || \
    die "el password de MagicRita debe tener entre 8 y 128 caracteres"
  if [[ "$MAGICRITA_ADMIN_PASSWORD" == *[[:space:]]* || \
        "$MAGICRITA_ADMIN_PASSWORD" == *"'"* || \
        "$MAGICRITA_ADMIN_PASSWORD" == *'"'* || \
        "$MAGICRITA_ADMIN_PASSWORD" == *\\* ]]; then
    die "el password de MagicRita no puede contener espacios, comillas ni barra invertida"
  fi
fi

[[ "$MAGICRITA_DIR" =~ ^/[A-Za-z0-9._/-]+$ ]] || die "ruta inválida para MagicRita: '$MAGICRITA_DIR'"
[[ "$RYNXCP_DIR" =~ ^/[A-Za-z0-9._/-]+$ ]] || die "ruta inválida para RYNXCP: '$RYNXCP_DIR'"
for ref in "$MAGICRITA_REF" "$RYNXCP_REF"; do
  [[ "$ref" =~ ^[A-Za-z0-9._/-]+$ ]] || die "ref inválida: '$ref'"
  [[ "$ref" == -* ]] && die "ref inválida: '$ref'"
done

if (( ASSUME_YES )) && [[ -z "$PANEL_PASSWORD" && -z "${RYNX_ADMIN_PASSWORD:-}" && $SKIP_RYNXCP -eq 0 ]]; then
  die "con --yes define --panel-password o RYNX_ADMIN_PASSWORD para el panel"
fi

for secret_name in PANEL_PASSWORD GITHUB_TOKEN; do
  secret="${!secret_name:-}"
  [[ "$secret" != *$'\n'* && "$secret" != *$'\r'* ]] || \
    die "$secret_name no puede contener saltos de línea"
done

if [[ -n "$PANEL_HOST" && "${PANEL_HOST,,}" == "$MAGICRITA_DOMAIN" ]]; then
  die "el panel RYNXCP y MagicRita no pueden usar el mismo dominio ('$MAGICRITA_DOMAIN')"
fi

# ---------------------------------------------------------------------------
# Git (token opcional para repos privados)
# ---------------------------------------------------------------------------
setup_git_auth() {
  if [[ -n "$GITHUB_TOKEN" ]]; then
    ASKPASS_FILE="$(mktemp)"
    cat >"$ASKPASS_FILE" <<'ASKPASS'
#!/bin/sh
case "$1" in
  *Username*|*username*) printf '%s\n' "x-access-token" ;;
  *) printf '%s\n' "${GITHUB_TOKEN}" ;;
esac
ASKPASS
    chmod 700 "$ASKPASS_FILE"
    export GIT_ASKPASS="$ASKPASS_FILE" GIT_TERMINAL_PROMPT=0 \
      GIT_ASKPASS_REQUIRE=force GITHUB_TOKEN LC_ALL=C LANG=C
  fi
}

sync_repo() {
  local url="$1" dir="$2" ref="$3"
  if [[ -d "$dir/.git" ]]; then
    log "Actualizando $(basename "$dir") a '$ref'"
    git -C "$dir" -c safe.directory="$dir" fetch --depth 1 origin "$ref"
    if git -C "$dir" -c safe.directory="$dir" rev-parse --verify --quiet "origin/$ref" >/dev/null; then
      git -C "$dir" -c safe.directory="$dir" checkout -f -B "$ref" "origin/$ref"
    else
      git -C "$dir" -c safe.directory="$dir" checkout -f "$ref"
    fi
  elif [[ -e "$dir" && -n "$(ls -A "$dir" 2>/dev/null || true)" ]]; then
    die "$dir existe y no es un repositorio git; muévelo o bórralo antes de continuar"
  else
    log "Clonando $url ('$ref')"
    git clone --depth 1 --branch "$ref" "$url" "$dir"
  fi
}

# ---------------------------------------------------------------------------
# RYNXCP
# ---------------------------------------------------------------------------
install_rynxcp() {
  sync_repo "$RYNXCP_REPO" "$RYNXCP_DIR" "$RYNXCP_REF"

  local -a args=()
  if [[ -n "$PANEL_HOST" ]]; then args+=(--host "$PANEL_HOST"); fi
  if [[ -n "$PANEL_EMAIL" ]]; then args+=(--email "$PANEL_EMAIL"); fi
  args+=(--panel-port "$PANEL_PORT")
  if (( NO_TLS )); then args+=(--no-tls); fi
  if (( ASSUME_YES )); then args+=(--yes); fi
  if [[ -n "$PANEL_PASSWORD" ]]; then
    export RYNX_ADMIN_PASSWORD="$PANEL_PASSWORD"
  fi

  log "Instalando RYNXCP (deploy/preinstall.sh)"
  ( cd "$RYNXCP_DIR" && RYNX_SKIP_MAIL=0 bash deploy/preinstall.sh "${args[@]}" )
  unset RYNX_ADMIN_PASSWORD
}

check_panel_domain_conflict() {
  local panel_site="/etc/nginx/sites-available/rynxcp-panel"
  [[ -f "$panel_site" ]] || return 0
  local name
  while IFS= read -r name; do
    if [[ "${name,,}" == "$MAGICRITA_DOMAIN" ]]; then
      die "RYNXCP ya usa '$MAGICRITA_DOMAIN' para el panel; elige otro dominio para MagicRita o --panel-host distinto"
    fi
  done < <(awk '/^[[:space:]]*server_name[[:space:]]/ {gsub(/;/, " "); for (i = 2; i <= NF; i++) print $i}' "$panel_site")
}

# ---------------------------------------------------------------------------
# Node.js
# ---------------------------------------------------------------------------
node_version_ok() {
  "$1" -e \
    'const [a,b]=process.versions.node.split(".").map(Number); process.exit((a===20&&b>=19)||(a===22&&b>=12)||a>22?0:1)' \
    >/dev/null 2>&1
}

ensure_node() {
  local node_bin
  node_bin="$(command -v node || true)"

  if [[ -n "$node_bin" ]] && node_version_ok "$node_bin"; then
    NODE_BIN="$node_bin"
  else
    log "Instalando Node.js 22 (NodeSource)"
    curl -fsSL https://deb.nodesource.com/setup_22.x | bash -
    apt-get install -y nodejs
    NODE_BIN="$(command -v node || true)"
    [[ -n "$NODE_BIN" ]] || die "Node.js no quedó instalado"
    node_version_ok "$NODE_BIN" || \
      die "se necesita Node.js 20.19+ o 22.12+ para compilar MagicRita"
  fi

  NPM_BIN="$(command -v npm || true)"
  if [[ -z "$NPM_BIN" ]]; then
    apt-get install -y npm
    NPM_BIN="$(command -v npm || true)"
    [[ -n "$NPM_BIN" ]] || die "npm no quedó instalado"
  fi

  log "Node $("$NODE_BIN" -v) / npm $("$NPM_BIN" -v)"
}

check_port_free() {
  command -v ss >/dev/null 2>&1 || return 0
  if ss -lntH "sport = :$MAGICRITA_PORT" 2>/dev/null | grep -q .; then
    if systemctl is-active --quiet "$SERVICE" 2>/dev/null; then
      log "El puerto $MAGICRITA_PORT lo usa $SERVICE; se reiniciará"
    else
      die "el puerto $MAGICRITA_PORT ya está ocupado por otro proceso (ss -lntp)"
    fi
  fi
}

# ---------------------------------------------------------------------------
# nginx
# ---------------------------------------------------------------------------
print_proxy_locations() {
  local port="$1"
  cat <<EOF
    location /signal/ {
        proxy_pass http://127.0.0.1:${port}/;
        proxy_http_version 1.1;
        proxy_set_header Upgrade \$http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host \$host;
        # Se sobrescribe: con \$proxy_add_x_forwarded_for el cliente podría
        # inyectar una IP falsa y esquivar los límites por IP del relay.
        proxy_set_header X-Forwarded-For \$remote_addr;
        proxy_read_timeout 86400;
        proxy_send_timeout 86400;
    }

    location /moderation/ {
        proxy_pass http://127.0.0.1:${port}/moderation/;
        proxy_set_header Host \$host;
        proxy_set_header X-Forwarded-For \$remote_addr;
    }

    location /admin-api/ {
        proxy_pass http://127.0.0.1:${port}/admin-api/;
        proxy_set_header Host \$host;
        proxy_set_header X-Forwarded-For \$remote_addr;
        proxy_set_header Authorization \$http_authorization;
    }

    location /admin-path {
        proxy_pass http://127.0.0.1:${port}/admin-path;
        proxy_set_header Host \$host;
        proxy_set_header X-Forwarded-For \$remote_addr;
    }

    location /beta {
        proxy_pass http://127.0.0.1:${port};
        proxy_set_header Host \$host;
        proxy_set_header X-Forwarded-For \$remote_addr;
    }

    location /brand {
        proxy_pass http://127.0.0.1:${port};
        proxy_set_header Host \$host;
        proxy_set_header X-Forwarded-For \$remote_addr;
    }
EOF
}

print_app_location() {
  local connect="$1"
  local hsts="$2"
  cat <<EOF
    location / {
        try_files \$uri \$uri/ /index.html =404;
        add_header Content-Security-Policy "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; media-src 'self' data: blob:; connect-src ${connect}; font-src 'self' data:; object-src 'none'; base-uri 'self'; form-action 'self'; frame-ancestors 'none'" always;
        add_header X-Content-Type-Options nosniff always;
        add_header X-Frame-Options DENY always;
        add_header Referrer-Policy no-referrer always;
        add_header Permissions-Policy "camera=(self), microphone=(self), geolocation=()" always;
EOF
  if (( hsts )); then
    cat <<EOF
        add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;
EOF
  fi
  cat <<EOF
    }
EOF
}

write_http_vhost() {
  {
    cat <<EOF
# managed-by: magicrita-installer
server {
    listen 80;
    listen [::]:80;
    server_name $MAGICRITA_DOMAIN;

    root $MAGICRITA_DIR/dist;
    index index.html;
    server_tokens off;
    client_max_body_size 32m;

    location ^~ /.well-known/acme-challenge/ {
        root /var/www/_acme;
        default_type "text/plain";
    }

EOF
    print_proxy_locations "$MAGICRITA_PORT"
    print_app_location "'self'" 0
    cat <<EOF
}
EOF
  } >"$VHOST_FILE"
}

write_https_vhost() {
  {
    cat <<EOF
# managed-by: magicrita-installer
server {
    listen 80;
    listen [::]:80;
    server_name $MAGICRITA_DOMAIN;

    location ^~ /.well-known/acme-challenge/ {
        root /var/www/_acme;
        default_type "text/plain";
    }

    location / {
        return 301 https://\$host\$request_uri;
    }
}

server {
    listen 443 ssl;
    listen [::]:443 ssl;
    http2 on;
    server_name $MAGICRITA_DOMAIN;

    root $MAGICRITA_DIR/dist;
    index index.html;
    server_tokens off;
    client_max_body_size 32m;

    ssl_certificate     /etc/letsencrypt/live/$MAGICRITA_DOMAIN/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/$MAGICRITA_DOMAIN/privkey.pem;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_prefer_server_ciphers off;
    ssl_session_cache shared:MAGICRITA:10m;
    ssl_session_timeout 1d;
    ssl_session_tickets off;

    location ^~ /.well-known/acme-challenge/ {
        root /var/www/_acme;
        default_type "text/plain";
    }

EOF
    print_proxy_locations "$MAGICRITA_PORT"
    print_app_location "'self' wss://$MAGICRITA_DOMAIN" 1
    cat <<EOF
}
EOF
  } >"$VHOST_FILE"
}

enable_vhost() {
  ln -sfn "$VHOST_FILE" "$VHOST_LINK"
  if ! nginx -t; then
    warn "nginx rechazó la configuración; se deshabilita MagicRita"
    rm -f "$VHOST_LINK"
    nginx -t || true
    die "revisa $VHOST_FILE"
  fi
  systemctl reload nginx
}

cert_exists() {
  [[ -s "/etc/letsencrypt/live/$MAGICRITA_DOMAIN/fullchain.pem" && \
     -s "/etc/letsencrypt/live/$MAGICRITA_DOMAIN/privkey.pem" ]]
}

obtain_cert() {
  install -d -m 0755 /var/www/_acme
  certbot certonly --webroot -w /var/www/_acme -d "$MAGICRITA_DOMAIN" \
    --non-interactive --agree-tos -m "$MAGICRITA_EMAIL" \
    --keep-until-expiring --deploy-hook "systemctl reload nginx"
}

# ---------------------------------------------------------------------------
# MagicRita
# ---------------------------------------------------------------------------
install_magicrita() {
  if ! id -u "$MAGICRITA_USER" >/dev/null 2>&1; then
    log "Creando usuario de sistema '$MAGICRITA_USER'"
    useradd --system --home-dir "$MAGICRITA_DIR" --shell /usr/sbin/nologin \
      --user-group "$MAGICRITA_USER"
  fi

  install -d -m 0755 "$MAGICRITA_DIR"
  sync_repo "$MAGICRITA_REPO" "$MAGICRITA_DIR" "$MAGICRITA_REF"
  chown -R "$MAGICRITA_USER:$MAGICRITA_USER" "$MAGICRITA_DIR"
  chmod 0755 "$MAGICRITA_DIR"
  install -d -m 0750 -o "$MAGICRITA_USER" -g "$MAGICRITA_USER" "$MAGICRITA_DIR/data"
  install -d -m 0750 -o "$MAGICRITA_USER" -g "$MAGICRITA_USER" /var/cache/magicrita-npm

  local env_file="$MAGICRITA_DIR/.env"
  if [[ ! -f "$env_file" ]]; then
    local admin_password="$MAGICRITA_ADMIN_PASSWORD"
    if [[ -z "$admin_password" ]]; then
      admin_password="$(openssl rand -hex 24)"
      GENERATED_PASSWORD="$admin_password"
    fi
    cat >"$env_file" <<EOF
ADMIN_USER=$MAGICRITA_ADMIN_USER
ADMIN_PASSWORD=$admin_password
RUTA_ADMINISTRACION=$MAGICRITA_ADMIN_PATH
PORT=$MAGICRITA_PORT
HOST=127.0.0.1
TRUST_PROXY=1
EOF
    log "Creado $env_file"
  else
    log "Conservando $env_file existente"
    if [[ -n "$MAGICRITA_ADMIN_PASSWORD" ]]; then
      warn "se ignora --magicrita-admin-password porque $env_file ya existe"
    fi
    if ! grep -q '^HOST=127\.0\.0\.1$' "$env_file"; then
      die "$env_file debe fijar HOST=127.0.0.1 (el relay no debe escuchar en público)"
    fi
    grep -Eq '^ADMIN_USER=.+' "$env_file" || \
      warn "$env_file no define ADMIN_USER; el panel de administración quedará deshabilitado"
    grep -Eq '^ADMIN_PASSWORD=.+' "$env_file" || \
      warn "$env_file no define ADMIN_PASSWORD; el panel de administración quedará deshabilitado"
    grep -q '^TRUST_PROXY=1$' "$env_file" || \
      warn "$env_file no fija TRUST_PROXY=1; detrás de nginx se perdería la IP real"
    local env_path
    env_path="$(sed -n 's/^RUTA_ADMINISTRACION=//p' "$env_file" | tail -n1)"
    if [[ -n "$env_path" ]]; then MAGICRITA_ADMIN_PATH="$env_path"; fi
  fi
  chown "$MAGICRITA_USER:$MAGICRITA_USER" "$env_file"
  chmod 0600 "$env_file"

  # El puerto real lo manda .env; nginx debe apuntar al mismo.
  if ! grep -Eq '^PORT=[0-9]+$' "$env_file"; then
    die "$env_file debe definir PORT con un número (por ejemplo PORT=8787)"
  fi
  local env_port
  env_port="$(sed -n 's/^PORT=//p' "$env_file" | tail -n1)"
  env_port=$((10#$env_port))
  if (( env_port < 1024 || env_port > 65535 )); then
    die "PORT inválido en $env_file: $env_port"
  fi
  if (( env_port != MAGICRITA_PORT )); then
    warn "$env_file usa PORT=$env_port; se usará ese puerto para nginx"
    MAGICRITA_PORT="$env_port"
  fi

  check_port_free

  log "Instalando dependencias (npm ci)"
  runuser -u "$MAGICRITA_USER" -- /usr/bin/env -C "$MAGICRITA_DIR" \
    HOME="$MAGICRITA_DIR" PATH="/usr/local/bin:/usr/bin:/bin" \
    npm_config_cache=/var/cache/magicrita-npm \
    "$NPM_BIN" ci

  local build_signal=""
  if (( NO_TLS == 0 )); then
    build_signal="wss://$MAGICRITA_DOMAIN/signal/"
  fi

  log "Compilando MagicRita"
  runuser -u "$MAGICRITA_USER" -- /usr/bin/env -C "$MAGICRITA_DIR" \
    HOME="$MAGICRITA_DIR" PATH="/usr/local/bin:/usr/bin:/bin" \
    npm_config_cache=/var/cache/magicrita-npm \
    NODE_OPTIONS=--max-old-space-size=2048 \
    VITE_SIGNAL_URL="$build_signal" \
    "$NPM_BIN" run build

  [[ -f "$MAGICRITA_DIR/dist/index.html" ]] || \
    die "el build no generó $MAGICRITA_DIR/dist/index.html"
  chmod -R a+rX "$MAGICRITA_DIR/dist"

  log "Instalando servicio $SERVICE"
  cat >"/etc/systemd/system/$SERVICE.service" <<EOF
[Unit]
Description=MagicRita signal
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=$MAGICRITA_USER
Group=$MAGICRITA_USER
WorkingDirectory=$MAGICRITA_DIR
ExecStart=$NODE_BIN server/signal.mjs
Restart=always
RestartSec=3
EnvironmentFile=$MAGICRITA_DIR/.env
Environment=NODE_OPTIONS=--max-old-space-size=2048
LimitNOFILE=65535
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=$MAGICRITA_DIR/data
PrivateTmp=true

[Install]
WantedBy=multi-user.target
EOF

  systemctl daemon-reload
  systemctl enable "$SERVICE" >/dev/null
  systemctl restart "$SERVICE"

  sleep 1
  systemctl is-active --quiet "$SERVICE" || \
    die "$SERVICE no arrancó (journalctl -u $SERVICE -n 80 --no-pager)"
  if ! ss -lntH "sport = :$MAGICRITA_PORT" 2>/dev/null | grep -q "127.0.0.1:$MAGICRITA_PORT"; then
    warn "no se ve el relay en 127.0.0.1:$MAGICRITA_PORT (revisa ss -lntp)"
  fi

  # nginx: HTTP primero para el reto ACME, luego HTTPS si hay certificado.
  command -v nginx >/dev/null 2>&1 || die "nginx no está instalado"
  systemctl enable nginx >/dev/null 2>&1 || true
  systemctl is-active --quiet nginx || systemctl start nginx
  install -d -m 0755 /var/www/_acme

  if (( NO_TLS == 0 )); then
    command -v certbot >/dev/null 2>&1 || { apt-get update; apt-get install -y certbot; }
    systemctl enable --now certbot.timer >/dev/null 2>&1 || true

    install -d -m 0755 /etc/letsencrypt/renewal-hooks/deploy
    cat >/etc/letsencrypt/renewal-hooks/deploy/reload-nginx.sh <<'EOF'
#!/bin/sh
systemctl reload nginx
EOF
    chmod 0755 /etc/letsencrypt/renewal-hooks/deploy/reload-nginx.sh

    if cert_exists; then
      TLS_OK=1
    else
      log "Publicando HTTP para el reto ACME de $MAGICRITA_DOMAIN"
      write_http_vhost
      enable_vhost
      log "Solicitando certificado Let's Encrypt"
      if obtain_cert; then
        TLS_OK=1
      else
        warn "no se pudo emitir el certificado; MagicRita queda en HTTP"
      fi
    fi
  fi

  if (( TLS_OK == 1 )); then
    write_https_vhost
  else
    write_http_vhost
  fi
  enable_vhost

  log "Verificando el sitio"
  if (( TLS_OK == 1 )); then
    curl -fsS --max-time 10 --resolve "$MAGICRITA_DOMAIN:443:127.0.0.1" \
      "https://$MAGICRITA_DOMAIN/" >/dev/null || \
      warn "no se pudo verificar https://$MAGICRITA_DOMAIN/ desde el propio servidor"
  else
    curl -fsS --max-time 10 --resolve "$MAGICRITA_DOMAIN:80:127.0.0.1" \
      "http://$MAGICRITA_DOMAIN/" >/dev/null || \
      warn "no se pudo verificar http://$MAGICRITA_DOMAIN/ desde el propio servidor"
  fi
}

print_summary() {
  local scheme="http"
  if (( TLS_OK == 1 )); then scheme="https"; fi

  printf '\n\033[1;32mRYNXCP + MagicRita instalados\033[0m\n\n'
  cat <<EOF
  MagicRita:   ${scheme}://${MAGICRITA_DOMAIN}
  Admin MR:    ${scheme}://${MAGICRITA_DOMAIN}/${MAGICRITA_ADMIN_PATH}
  Relay:       127.0.0.1:${MAGICRITA_PORT} (loopback, nunca público)
  Servicio:    systemctl status ${SERVICE}
  Logs:        journalctl -u ${SERVICE} -f
  Config:      ${MAGICRITA_DIR}/.env

EOF

  if [[ -n "$GENERATED_PASSWORD" ]]; then
    cat <<EOF
  Credenciales del panel de MagicRita (guárdalas ahora):
    usuario:    ${MAGICRITA_ADMIN_USER}
    contraseña: ${GENERATED_PASSWORD}

  Están en ${MAGICRITA_DIR}/.env (0600). Para rotarlas, edita el archivo
  y ejecuta: systemctl restart ${SERVICE}

EOF
  fi

  if [[ -n "$PANEL_HOST" ]]; then
    echo "  Panel RYNXCP: revisa la salida de preinstall (host ${PANEL_HOST})"
  else
    echo "  Panel RYNXCP: revisa la salida de preinstall (host autodetectado)"
  fi
  echo
  echo "  Actualizar MagicRita sin tocar RYNXCP: vuelve a ejecutar este script"
  echo "  con --skip-rynxcp y el mismo --magicrita-domain."
  echo "  No crees en RYNXCP un sitio con el dominio de MagicRita: ya lo gestiona"
  echo "  este script (nginx lo sirve desde /etc/nginx/sites-available/magicrita.conf)."
  echo
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
setup_git_auth

log "Instalando dependencias base (git, curl, ca-certificates, openssl)"
apt-get update
apt-get install -y --no-install-recommends git curl ca-certificates openssl patch

preflight_services
preflight_dns

if (( SKIP_RYNXCP )); then
  warn "se omite RYNXCP (--skip-rynxcp)"
  command -v nginx >/dev/null 2>&1 || \
    die "nginx no está instalado; no uses --skip-rynxcp en un servidor sin RYNXCP"
else
  install_rynxcp
fi

check_panel_domain_conflict
ensure_node
install_magicrita
print_summary

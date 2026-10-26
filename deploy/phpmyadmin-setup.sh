#!/usr/bin/env bash
# Install phpMyAdmin behind the panel login.
#
# The web root is only reachable through Nginx auth_request against the panel
# API, so an unauthenticated visitor is redirected to the panel login page.
set -Eeuo pipefail

PHPMYADMIN_VERSION="${PHPMYADMIN_VERSION:-5.2.3}"
PHPMYADMIN_ROOT="/usr/share/phpmyadmin"
PHPMYADMIN_DATA="/var/lib/phpmyadmin"
SNIPPET="/etc/nginx/snippets/rynxcp-phpmyadmin.conf"
PHP_SOCKET="/run/php/php8.5-fpm.sock"

log() { printf '\n\033[1;32m==>\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33mwarning:\033[0m %s\n' "$*" >&2; }
die() { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

[[ "${EUID}" -eq 0 ]] || die "run this script as root"

log "Installing phpMyAdmin ${PHPMYADMIN_VERSION} dependencies"
export DEBIAN_FRONTEND=noninteractive
apt-get update
apt-get install -y --no-install-recommends \
  ca-certificates curl openssl tar \
  php8.5-mysql php8.5-mbstring php8.5-zip php8.5-xml php8.5-gd

if [[ ! -f "$PHPMYADMIN_ROOT/index.php" ]]; then
  log "Downloading phpMyAdmin ${PHPMYADMIN_VERSION}"
  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' EXIT
  url="https://files.phpmyadmin.net/phpMyAdmin/${PHPMYADMIN_VERSION}/phpMyAdmin-${PHPMYADMIN_VERSION}-all-languages.tar.gz"
  curl -fsSL --retry 3 --max-time 300 "$url" -o "$tmp/phpmyadmin.tar.gz"
  tar -xzf "$tmp/phpmyadmin.tar.gz" -C "$tmp"
  rm -rf "$PHPMYADMIN_ROOT"
  install -d -m 0750 "$PHPMYADMIN_ROOT"
  cp -a "$tmp/phpMyAdmin-${PHPMYADMIN_VERSION}-all-languages/." "$PHPMYADMIN_ROOT/"
  rm -rf "$tmp"
  rm -rf "$PHPMYADMIN_ROOT/install"
  trap - EXIT
fi

install -d -o www-data -g www-data -m 0700 "$PHPMYADMIN_DATA" "$PHPMYADMIN_DATA/tmp"
chown -R www-data:www-data "$PHPMYADMIN_DATA"

if [[ ! -s "$PHPMYADMIN_ROOT/config.inc.php" ]]; then
  log "Creating the phpMyAdmin configuration"
  secret="$(openssl rand -hex 32)"
  cat >"$PHPMYADMIN_ROOT/config.inc.php" <<EOF
<?php
declare(strict_types=1);
\$cfg['blowfish_secret'] = '${secret}';
\$cfg['Servers'][1]['auth_type'] = 'cookie';
\$cfg['Servers'][1]['host'] = '127.0.0.1';
\$cfg['Servers'][1]['compress'] = false;
\$cfg['Servers'][1]['AllowNoPassword'] = false;
\$cfg['TempDir'] = '${PHPMYADMIN_DATA}/tmp';
\$cfg['UploadDir'] = '';
\$cfg['SaveDir'] = '';
EOF
fi

chown -R root:www-data "$PHPMYADMIN_ROOT"
find "$PHPMYADMIN_ROOT" -type d -exec chmod 0750 {} +
find "$PHPMYADMIN_ROOT" -type f -exec chmod 0640 {} +
chown root:www-data "$PHPMYADMIN_ROOT/config.inc.php"
chmod 0640 "$PHPMYADMIN_ROOT/config.inc.php"

[[ -S "$PHP_SOCKET" ]] || warn "$PHP_SOCKET is missing; php8.5-fpm must be running"

log "Writing the Nginx auth_request snippet"
install -d -m 0755 /etc/nginx/snippets
cat >"$SNIPPET" <<'EOF'
# managed-by: RYNXCP
# phpMyAdmin is only reachable by a logged-in panel user.
location = /_rynx_auth {
    internal;
    proxy_pass http://127.0.0.1:8080/api/v1/auth/me;
    proxy_pass_request_body off;
    proxy_set_header Content-Length "";
    proxy_set_header Cookie $http_cookie;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;
}

location = /phpmyadmin {
    return 301 /phpmyadmin/;
}

location = /phpmyadmin/config.inc.php {
    auth_request /_rynx_auth;
    deny all;
}

location ~* /config\.inc\.php$ {
    auth_request /_rynx_auth;
    deny all;
}

location ^~ /phpmyadmin/libraries/ {
    auth_request /_rynx_auth;
    deny all;
}

location ^~ /phpmyadmin/setup/ {
    auth_request /_rynx_auth;
    deny all;
}

location /phpmyadmin/ {
    auth_request /_rynx_auth;
    error_page 401 = @rynx_login;
    alias /usr/share/phpmyadmin/;
    index index.php;
}

location ~ ^/phpmyadmin/(.+\.php)$ {
    auth_request /_rynx_auth;
    error_page 401 = @rynx_login;
    alias /usr/share/phpmyadmin/$1;
    include fastcgi_params;
    fastcgi_param SCRIPT_FILENAME /usr/share/phpmyadmin/$1;
    fastcgi_param SCRIPT_NAME /phpmyadmin/$1;
    fastcgi_param DOCUMENT_ROOT /usr/share/phpmyadmin;
    fastcgi_pass unix:/run/php/php8.5-fpm.sock;
}

location @rynx_login {
    return 302 /;
}
EOF
chmod 0644 "$SNIPPET"

log "phpMyAdmin installed; it will be protected by the panel vhost include"

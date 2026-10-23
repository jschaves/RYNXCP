#!/usr/bin/env bash
# RYNXCP installer for Debian/Ubuntu.
# Build first:  cargo build --release
set -euo pipefail

if [[ "${EUID}" -ne 0 ]]; then
  echo "error: run as root" >&2
  exit 1
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_DIR="${ROOT_DIR}/target/release"

for bin in rynxd rynx-api rynxctl; do
  if [[ ! -x "${BIN_DIR}/${bin}" ]]; then
    echo "error: ${BIN_DIR}/${bin} not found. Run: cargo build --release" >&2
    exit 1
  fi
done

echo "==> checking runtime dependencies"
for cmd in nginx systemctl openssl; do
  command -v "${cmd}" >/dev/null 2>&1 || echo "warning: '${cmd}' is not installed"
done
command -v certbot >/dev/null 2>&1 || echo "warning: certbot is not installed (Let's Encrypt will be unavailable)"
command -v php-fpm8.3 >/dev/null 2>&1 || echo "warning: php-fpm is not installed (PHP sites will be unavailable)"

echo "==> creating users and directories"
getent group rynx >/dev/null || groupadd --system rynx
getent passwd rynx >/dev/null || useradd --system --gid rynx --home-dir /var/lib/rynxcp --shell /usr/sbin/nologin rynx

install -d -m 0755 /usr/local/bin
install -d -m 0755 /etc/rynxcp /etc/rynxcp/apps
install -d -m 0755 /usr/share/rynxcp /usr/share/rynxcp/web
install -d -m 0750 /var/lib/rynxcp /var/lib/rynxcp/rynxd /var/lib/rynxcp/panel /var/lib/rynxcp/apps
install -d -m 0750 /var/log/rynxcp
chown root:rynx /var/log/rynxcp
touch /var/log/rynxcp/auth.log
chown rynx:rynx /var/log/rynxcp/auth.log
chmod 0640 /var/log/rynxcp/auth.log
install -d -m 0755 /var/www/_acme
install -d -m 0755 /etc/nginx/sites-available/rynxcp /etc/nginx/sites-enabled
install -d -m 0755 /run/rynxcp

echo "==> installing binaries"
install -m 0755 "${BIN_DIR}/rynxd" /usr/local/bin/rynxd
install -m 0755 "${BIN_DIR}/rynx-api" /usr/local/bin/rynx-api
install -m 0755 "${BIN_DIR}/rynxctl" /usr/local/bin/rynxctl

echo "==> installing configuration"
if [[ ! -f /etc/rynxcp/rynxd.toml ]]; then
  install -m 0640 "${ROOT_DIR}/deploy/config/rynxd.toml.example" /etc/rynxcp/rynxd.toml
fi
if [[ ! -f /etc/rynxcp/api.toml ]]; then
  install -m 0640 "${ROOT_DIR}/deploy/config/api.toml.example" /etc/rynxcp/api.toml
fi

if [[ ! -f /etc/rynxcp/daemon.token ]]; then
  openssl rand -hex 32 > /etc/rynxcp/daemon.token
fi
chown root:rynx /etc/rynxcp/daemon.token
chmod 0640 /etc/rynxcp/daemon.token
chown root:rynx /etc/rynxcp/rynxd.toml /etc/rynxcp/api.toml
chown -R rynx:rynx /var/lib/rynxcp/panel
chown -R rynx:rynx /var/lib/rynxcp/apps

echo "==> optional services layout"
install -d -m 0755 /etc/bind/zones /etc/opendkim/keys /etc/pure-ftpd /var/backups/rynxcp
chown -R root:root /var/backups/rynxcp
getent passwd rynxftp >/dev/null || useradd --system --no-create-home --shell /usr/sbin/nologin rynxftp || true
if [[ -d /etc/pure-ftpd/conf ]]; then
  echo yes > /etc/pure-ftpd/conf/ChrootEveryone
  echo yes > /etc/pure-ftpd/conf/NoAnonymous
  echo /etc/pure-ftpd/pureftpd.pdb > /etc/pure-ftpd/conf/PureDB
fi
if [[ -f /etc/bind/named.conf.local ]] && ! grep -q 'named.conf.rynxcp' /etc/bind/named.conf.local; then
  echo 'include "/etc/bind/named.conf.rynxcp";' >> /etc/bind/named.conf.local
fi

echo "==> installing fail2ban integration"
if command -v fail2ban-client >/dev/null 2>&1; then
  install -d -m 0755 /etc/fail2ban/filter.d /etc/fail2ban/jail.d
  install -m 0644 "${ROOT_DIR}/deploy/fail2ban/filter.d/rynxcp.conf" /etc/fail2ban/filter.d/rynxcp.conf
  install -m 0644 "${ROOT_DIR}/deploy/fail2ban/jail.d/rynxcp.local" /etc/fail2ban/jail.d/rynxcp.local
  systemctl enable --now fail2ban >/dev/null 2>&1 || true
  systemctl restart fail2ban >/dev/null 2>&1 || true
else
  echo "warning: fail2ban not installed; install it to ban brute-force sources automatically"
fi

echo "==> installing web panel"
rm -rf /usr/share/rynxcp/web/*
cp -r "${ROOT_DIR}/web/." /usr/share/rynxcp/web/
chown -R root:root /usr/share/rynxcp/web
chmod -R 0755 /usr/share/rynxcp/web

echo "==> installing log rotation"
if [[ -d /etc/logrotate.d ]]; then
  install -m 0644 "${ROOT_DIR}/deploy/logrotate/rynxcp" /etc/logrotate.d/rynxcp
fi

echo "==> installing systemd units"
install -m 0644 "${ROOT_DIR}/deploy/systemd/rynxd.service" /etc/systemd/system/rynxd.service
install -m 0644 "${ROOT_DIR}/deploy/systemd/rynx-api.service" /etc/systemd/system/rynx-api.service
install -m 0644 "${ROOT_DIR}/deploy/systemd/rynxcp-cert-renew.service" /etc/systemd/system/rynxcp-cert-renew.service
install -m 0644 "${ROOT_DIR}/deploy/systemd/rynxcp-cert-renew.timer" /etc/systemd/system/rynxcp-cert-renew.timer
install -m 0644 "${ROOT_DIR}/deploy/systemd/rynxcp-backup.service" /etc/systemd/system/rynxcp-backup.service
install -m 0644 "${ROOT_DIR}/deploy/systemd/rynxcp-backup.timer" /etc/systemd/system/rynxcp-backup.timer
systemctl daemon-reload

echo "==> starting services"
systemctl enable --now rynxd.service
systemctl enable --now rynx-api.service
systemctl enable --now rynxcp-cert-renew.timer
systemctl enable --now rynxcp-backup.timer

echo
echo "RYNXCP installed."
echo "1. Create the first administrator:"
echo "     rynxctl admin create admin --email you@example.com"
echo "2. Publish the panel:"
echo "     cp ${ROOT_DIR}/deploy/nginx/panel.conf.example /etc/nginx/sites-available/rynxcp-panel"
echo "     edit server_name and certificate paths, then enable it"
echo "     certbot --nginx -d panel.example.com"
echo "3. Browse to https://panel.example.com"
echo
echo "Daemon:  systemctl status rynxd"
echo "API:     systemctl status rynx-api"
echo "Audit:   /var/log/rynxcp/audit.log"







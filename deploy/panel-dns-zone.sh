#!/usr/bin/env bash
# Create a small authoritative zone for the panel hostname.
#
# The nameservers advertised by RYNXCP are ns1.<panel-host> and
# ns2.<panel-host>. If the panel hostname is not itself a website managed by
# the panel, BIND has no zone for it and answers REFUSED, which also makes the
# nameservers invalid at domain registrars.
#
# This script creates a dedicated zone outside named.conf.rynxcp (the file the
# panel rewrites), so reconciles do not remove it.
#
# Usage:
#   sudo bash deploy/panel-dns-zone.sh <panel-domain> [public-ipv4]
#
# The panel domain must be a FQDN (for example rynxcp.com). If the IPv4 is
# omitted, it is read from /etc/rynxcp/rynxd.toml (default_ipv4).

set -Eeuo pipefail

[[ "${EUID}" -eq 0 ]] || { echo "run as root" >&2; exit 1; }
[[ $# -ge 1 ]] || { echo "usage: $0 <panel-domain> [public-ipv4]" >&2; exit 2; }

DOMAIN="${1,,}"
DOMAIN="${DOMAIN%.}"
IP="${2:-}"

[[ "$DOMAIN" =~ ^([a-z0-9]([a-z0-9-]{0,61}[a-z0-9])?\.)+[a-z]{2,63}$ ]] || {
  echo "error: invalid panel domain '$DOMAIN'" >&2
  exit 2
}

if [[ -z "$IP" ]]; then
  IP="$(sed -nE 's/^[[:space:]]*default_ipv4[[:space:]]*=[[:space:]]*"([^"]+)".*/\1/p' /etc/rynxcp/rynxd.toml 2>/dev/null | head -n1)"
fi
[[ "$IP" =~ ^([0-9]{1,3}\.){3}[0-9]{1,3}$ ]] || {
  echo "error: no valid public IPv4 (pass it as the second argument)" >&2
  exit 2
}

ZONE_DIR="/etc/bind/zones"
ZONE_FILE="${ZONE_DIR}/${DOMAIN}.db"
PANEL_CONF="/etc/bind/named.conf.rynxcp-panel"
LOCAL_CONF="/etc/bind/named.conf.local"
MANAGED_CONF="/etc/bind/named.conf.rynxcp"
SERIAL="$(date +%Y%m%d%H)"

command -v named-checkconf >/dev/null 2>&1 || {
  echo "error: BIND is not installed" >&2
  exit 1
}
[[ -f "$LOCAL_CONF" ]] || {
  echo "error: $LOCAL_CONF not found; BIND is not configured for RYNXCP" >&2
  exit 1
}

# Do not fight a zone that the panel manages from a website.
if [[ -f "$MANAGED_CONF" ]] && grep -q "zone \"${DOMAIN}\"" "$MANAGED_CONF"; then
  echo "zone ${DOMAIN} is managed by RYNXCP; leaving it unchanged"
  exit 0
fi
if [[ -f "$ZONE_FILE" ]] && grep -q "managed-by: RYNXCP panel" "$ZONE_FILE"; then
  :
elif [[ -f "$ZONE_FILE" ]]; then
  backup="${ZONE_FILE}.rynxcp-panel.bak"
  cp -a "$ZONE_FILE" "$backup"
  echo "backup: $backup"
fi

install -d -m 0755 "$ZONE_DIR"

cat >"$ZONE_FILE" <<EOF
; managed-by: RYNXCP panel
\$TTL 300
@   IN SOA ns1.${DOMAIN}. hostmaster.${DOMAIN}. (
        ${SERIAL} ; serial
        3600      ; refresh
        900       ; retry
        1209600   ; expire
        300 )     ; minimum
    IN NS ns1.${DOMAIN}.
    IN NS ns2.${DOMAIN}.
    IN A ${IP}
ns1 IN A ${IP}
ns2 IN A ${IP}
EOF
chmod 0644 "$ZONE_FILE"
chown root:bind "$ZONE_FILE" 2>/dev/null || true

cat >"$PANEL_CONF" <<EOF
// managed-by: RYNXCP panel
zone "${DOMAIN}" {
    type master;
    file "${ZONE_FILE}";
};
EOF
chmod 0644 "$PANEL_CONF"

if ! grep -q 'named.conf.rynxcp-panel' "$LOCAL_CONF"; then
  printf '\ninclude "/etc/bind/named.conf.rynxcp-panel";\n' >>"$LOCAL_CONF"
fi

if ! named-checkzone "$DOMAIN" "$ZONE_FILE" >/dev/null; then
  echo "error: named-checkzone rejected ${ZONE_FILE}" >&2
  exit 1
fi

if ! named-checkconf >/dev/null 2>&1; then
  echo "error: named-checkconf rejected the panel zone; fix ${PANEL_CONF}" >&2
  exit 1
fi

systemctl reload named 2>/dev/null || \
  systemctl reload bind9 2>/dev/null || \
  systemctl restart named 2>/dev/null || \
  systemctl restart bind9 2>/dev/null || true

echo "panel DNS zone ${DOMAIN} created:"
echo "  ns1.${DOMAIN} -> ${IP}"
echo "  ns2.${DOMAIN} -> ${IP}"
echo "Create glue records for those names at the ${DOMAIN} registrar."

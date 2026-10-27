#!/usr/bin/env bash
# Create a small authoritative zone for the panel hostname and optional
# outbound mail DNS (SPF, DKIM, DMARC) for admin notifications.
#
# The nameservers advertised by RYNXCP are ns1.<panel-host> and
# ns2.<panel-host>. If the panel hostname is not itself a website managed by
# the panel, BIND has no zone for it and answers REFUSED, which also makes the
# nameservers invalid at domain registrars.
#
# This script creates a dedicated zone outside named.conf.rynxcp (the file the
# panel rewrites), so reconciles do not remove it. When a DKIM selector is
# passed, it also generates the key, updates OpenDKIM tables and publishes the
# outbound-only records used by noreply@<panel-domain>. No MX is added: the
# panel uses Postfix on loopback for sending, not for receiving admin mail.
#
# Usage:
#   sudo bash deploy/panel-dns-zone.sh <panel-domain> [public-ipv4] [dkim-selector]
#
# Examples:
#   sudo bash deploy/panel-dns-zone.sh rynxcp.com 217.216.77.229
#   sudo bash deploy/panel-dns-zone.sh rynxcp.com 217.216.77.229 mail
#
# If the IPv4 is omitted, it is read from /etc/rynxcp/rynxd.toml (default_ipv4).

set -Eeuo pipefail

[[ "${EUID}" -eq 0 ]] || { echo "run as root" >&2; exit 1; }
[[ $# -ge 1 ]] || { echo "usage: $0 <panel-domain> [public-ipv4] [dkim-selector]" >&2; exit 2; }

DOMAIN="${1,,}"
DOMAIN="${DOMAIN%.}"
IP="${2:-}"
DKIM_SELECTOR="${3:-}"
DKIM_SELECTOR="${DKIM_SELECTOR,,}"

[[ "$DOMAIN" =~ ^([a-z0-9]([a-z0-9-]{0,61}[a-z0-9])?\.)+[a-z]{2,63}$ ]] || {
  echo "error: invalid panel domain '$DOMAIN'" >&2
  exit 2
}
if [[ -n "$DKIM_SELECTOR" ]]; then
  [[ "$DKIM_SELECTOR" =~ ^[a-z0-9]([a-z0-9-]{0,30}[a-z0-9])?$ ]] || {
    echo "error: invalid DKIM selector '$DKIM_SELECTOR'" >&2
    exit 2
  }
fi

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
if [[ -f "$ZONE_FILE" ]]; then
  previous_serial="$(sed -nE 's/^[[:space:]]*([0-9]{1,10})[[:space:]]*;[[:space:]]*serial.*/\1/p' "$ZONE_FILE" | head -n1)"
  if [[ -n "$previous_serial" ]] && (( 10#$SERIAL <= 10#$previous_serial )); then
    SERIAL=$(( 10#$previous_serial + 1 ))
  fi
fi

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

MAIL_RECORDS=""
DKIM_TXT=""
if [[ -n "$DKIM_SELECTOR" ]]; then
  command -v opendkim-genkey >/dev/null 2>&1 || {
    echo "error: opendkim-genkey is required for DKIM setup" >&2
    exit 1
  }
  command -v openssl >/dev/null 2>&1 || {
    echo "error: openssl is required for DKIM setup" >&2
    exit 1
  }

  KEY_DIR="/etc/opendkim/keys/${DOMAIN}"
  PRIVATE_KEY="${KEY_DIR}/${DKIM_SELECTOR}.private"
  if id opendkim >/dev/null 2>&1; then
    install -d -o opendkim -g opendkim -m 0750 "$KEY_DIR"
  else
    install -d -m 0750 "$KEY_DIR"
  fi
  if [[ ! -s "$PRIVATE_KEY" ]]; then
    tmp="$(mktemp -d)"
    trap 'rm -rf "$tmp"' EXIT
    opendkim-genkey -b 2048 -d "$DOMAIN" -s "$DKIM_SELECTOR" -D "$tmp" >/dev/null
    if id opendkim >/dev/null 2>&1; then
      install -m 0600 -o opendkim -g opendkim "$tmp/${DKIM_SELECTOR}.private" "$PRIVATE_KEY"
    else
      install -m 0600 "$tmp/${DKIM_SELECTOR}.private" "$PRIVATE_KEY"
    fi
    rm -rf "$tmp"
    trap - EXIT
  fi
  chown opendkim:opendkim "$PRIVATE_KEY" 2>/dev/null || true

  public_key="$(openssl rsa -in "$PRIVATE_KEY" -pubout 2>/dev/null | sed '/^-----/d' | tr -d '\r\n')"
  [[ -n "$public_key" ]] || {
    echo "error: cannot derive the DKIM public key from $PRIVATE_KEY" >&2
    exit 1
  }
  DKIM_TXT='"v=DKIM1; h=sha256; k=rsa; p=" '
  while IFS= read -r chunk; do
    DKIM_TXT+="\"${chunk}\" "
  done < <(printf '%s' "$public_key" | fold -w 255)
  DKIM_TXT="${DKIM_TXT% }"

  MAIL_RECORDS=$(cat <<EOF
mail IN A ${IP}
@    IN TXT "v=spf1 a:mail.${DOMAIN} -all"
_dmarc IN TXT "v=DMARC1; p=quarantine; adkim=s; aspf=s; pct=100"
_domainkey IN TXT "t=y; o=~"
${DKIM_SELECTOR}._domainkey IN TXT ${DKIM_TXT}
EOF
)

  KEYTABLE="/etc/opendkim/KeyTable"
  SIGNINGTABLE="/etc/opendkim/SigningTable"
  PANEL_KEYTABLE="/etc/opendkim/KeyTable.panel"
  PANEL_SIGNINGTABLE="/etc/opendkim/SigningTable.panel"
  install -d -m 0750 /etc/opendkim
  [[ -f "$KEYTABLE" ]] || : >"$KEYTABLE"
  [[ -f "$SIGNINGTABLE" ]] || : >"$SIGNINGTABLE"
  [[ -f "$PANEL_KEYTABLE" ]] || printf '# managed-by: RYNXCP panel\n' >"$PANEL_KEYTABLE"
  [[ -f "$PANEL_SIGNINGTABLE" ]] || printf '# managed-by: RYNXCP panel\n' >"$PANEL_SIGNINGTABLE"

  key_line="${DKIM_SELECTOR}._domainkey.${DOMAIN} ${DOMAIN}:${DKIM_SELECTOR}:${PRIVATE_KEY}"
  signing_line="*@${DOMAIN} ${DKIM_SELECTOR}._domainkey.${DOMAIN}"
  if ! grep -Fqx "$key_line" "$PANEL_KEYTABLE"; then
    printf '%s\n' "$key_line" >>"$PANEL_KEYTABLE"
  fi
  if ! grep -Fqx "$signing_line" "$PANEL_SIGNINGTABLE"; then
    printf '%s\n' "$signing_line" >>"$PANEL_SIGNINGTABLE"
  fi
  # The daemon appends these panel files whenever it rewrites the main tables.
  # Populate the main tables now so DKIM works before the next reconcile.
  if ! grep -Fqx "$key_line" "$KEYTABLE"; then
    printf '%s\n' "$key_line" >>"$KEYTABLE"
  fi
  if ! grep -Fqx "$signing_line" "$SIGNINGTABLE"; then
    printf '%s\n' "$signing_line" >>"$SIGNINGTABLE"
  fi
  chown -R opendkim:opendkim /etc/opendkim 2>/dev/null || true
  chmod 0640 "$KEYTABLE" "$SIGNINGTABLE" "$PANEL_KEYTABLE" "$PANEL_SIGNINGTABLE" 2>/dev/null || true
fi

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
${MAIL_RECORDS}
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

if [[ -n "$DKIM_SELECTOR" ]]; then
  systemctl reload opendkim 2>/dev/null || systemctl restart opendkim 2>/dev/null || true
fi

echo "panel DNS zone ${DOMAIN} created:"
echo "  ns1.${DOMAIN} -> ${IP}"
echo "  ns2.${DOMAIN} -> ${IP}"
if [[ -n "$DKIM_SELECTOR" ]]; then
  echo "  mail.${DOMAIN} -> ${IP}"
  echo "  outbound SPF/DKIM/DMARC for noreply@${DOMAIN} (selector ${DKIM_SELECTOR})"
  echo "  no MX record added: the panel only sends mail"
fi
echo "Create glue records for ns1 and ns2 at the ${DOMAIN} registrar."

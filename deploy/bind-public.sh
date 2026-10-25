#!/usr/bin/env bash
# Make BIND answer public authoritative queries.
#
# RYNXCP serves authoritative zones for hosted domains. Debian/Ubuntu ship
# named.conf.options with a restrictive allow-query in some releases, which
# makes external resolvers receive REFUSED. This script keeps recursion local
# but allows anyone to query the authoritative zones.
#
# It is idempotent. If named-checkconf rejects the result, the previous file is
# restored.

set -Eeuo pipefail

[[ "${EUID}" -eq 0 ]] || { echo "run as root" >&2; exit 1; }

FILE="/etc/bind/named.conf.options"
[[ -f "$FILE" ]] || exit 0
command -v named-checkconf >/dev/null 2>&1 || exit 0

backup="$(mktemp)"
tmp="$(mktemp)"
cp -a "$FILE" "$backup"

# Drop old public/recursion/listener directives (they are re-added below).
grep -vE '^[[:space:]]*(listen-on|listen-on-v6|allow-query|allow-recursion|allow-transfer|recursion)[[:space:]]' "$FILE" >"$tmp" || true

if ! grep -qE '^[[:space:]]*options[[:space:]]*\{' "$tmp"; then
  rm -f "$backup" "$tmp"
  echo "warning: /etc/bind/named.conf.options has no options block; BIND left unchanged" >&2
  exit 0
fi

awk '
  /^[[:space:]]*options[[:space:]]*\{/ && !inserted {
    print
    print "    // managed-by: RYNXCP (public authoritative DNS, local-only recursion)"
    print "    listen-on { any; };"
    print "    listen-on-v6 { any; };"
    print "    allow-query { any; };"
    print "    recursion yes;"
    print "    allow-recursion { localhost; 127.0.0.0/8; ::1; };"
    print "    allow-transfer { none; };"
    inserted = 1
    next
  }
  { print }
' "$tmp" >"${tmp}.new"

install -m 0644 -o root -g root "${tmp}.new" "$FILE"

if ! named-checkconf >/dev/null 2>&1; then
  install -m 0644 -o root -g root "$backup" "$FILE"
  rm -f "$backup" "$tmp" "${tmp}.new"
  echo "error: named-checkconf rejected the public DNS settings; restored the previous file" >&2
  exit 1
fi

systemctl reload named 2>/dev/null || \
  systemctl reload bind9 2>/dev/null || \
  systemctl restart named 2>/dev/null || \
  systemctl restart bind9 2>/dev/null || true

rm -f "$backup" "$tmp" "${tmp}.new"
echo "BIND public DNS enabled: allow-query any, recursion limited to localhost"

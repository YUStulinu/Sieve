#!/usr/bin/env bash
# A local IMAP and SMTP server for development and tests (GreenMail, in memory): IMAP on 3143, SMTP on
# 3025, any user name logs in with any password, mailboxes are created on first use. Ctrl+C stops it.
#   scripts/test-server.sh
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
jar="$root/third_party/downloads/greenmail-standalone-2.1.15.jar"
[ -f "$jar" ] || { echo "run scripts/fetch-deps.sh first" >&2; exit 1; }
java="${JAVA:-java}"
if [ -n "${JAVA_HOME:-}" ]; then java="$JAVA_HOME/bin/java"; fi
exec "$java" -Dgreenmail.setup.test.imap -Dgreenmail.setup.test.smtp -Dgreenmail.hostname=127.0.0.1 \
  -Dgreenmail.auth.disabled -Dgreenmail.verbose=false -jar "$jar"

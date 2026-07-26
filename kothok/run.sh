#!/bin/sh
set -u
HERE="$(dirname "$0")"
BIN="${HERE}/kothok"
NICKEL_STACK="nickel nickel.orig hindenburg sickel fickel strickel fontickel foxitpdf iink fmon"
KEEP_ALIVE="adobehost btservice mtkbtd"
echo "run.sh: killing render stack (keeping $KEEP_ALIVE)..."
killall -q -TERM $NICKEL_STACK 2>/dev/null
sleep 0.3
killall -q -KILL $NICKEL_STACK 2>/dev/null
echo "run.sh: launching reader..."
"$BIN" "$@" 2>/mnt/onboard/.adds/kothok.err
RC=$?
echo "run.sh: reader exited rc=$RC"
sync
reboot
sleep 60

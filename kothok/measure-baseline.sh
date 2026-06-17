#!/bin/sh
# Phase 0.1 / Step 1 — battery baseline measurement (Kobo Libra Colour, MT8110, FW 4.45).
# Device-side sampler. Caller sets up the device STATE first, then samples it:
#   sh measure-baseline.sh power <label> [duration_sec]   # current/voltage/capacity over a window
#   sh measure-baseline.sh top   [snapshots]               # audio-worker CPU% audit (reader awake)
# CSV -> /mnt/onboard/.adds/battery-baseline.csv ; summary also printed to stdout.
# Units ASSUMED microamps/microvolts (typical fuel gauge); raw values are logged so the
# convention can be confirmed from the first-sample readback. current_now sign varies by driver.

set -u

ADDSDIR=/mnt/onboard/.adds
LOG=$ADDSDIR/battery-baseline.csv
INTERVAL=5
DEFAULT_DUR=600

find_psu() {
    for d in /sys/class/power_supply/*; do
        [ -r "$d/current_now" ] || continue
        printf '%s' "$d"; return 0
    done
    return 1
}

rawread() { v=$(cat "$1" 2>/dev/null | tr -d '[:space:]'); printf '%s' "$v"; }

cmd_power() {
    label=${1:-}
    dur=${2:-$DEFAULT_DUR}
    [ -n "$label" ] || { echo "usage: $0 power <label> [duration_sec]"; exit 2; }
    node=$(find_psu) || { echo "ERROR: no /sys/class/power_supply/*/current_now found"; exit 1; }
    mkdir -p "$ADDSDIR"
    [ -s "$LOG" ] || printf 'label,epoch,capacity_pct,voltage_uv,current_ua,current_abs_ua\n' > "$LOG"

    status=$(rawread "$node/status")
    echo "# node=$node status=$status label=$label duration=${dur}s interval=${INTERVAL}s"
    case "$status" in
        Charging) echo "# WARNING: CHARGING — idle baseline will be skewed; unplug and retry." ;;
    esac

    n=0; si=0; sv=0; first=""
    end=$(( $(date +%s) + dur ))
    while [ "$(date +%s)" -lt "$end" ]; do
        ts=$(date +%s)
        ci=$(rawread "$node/current_now"); cv=$(rawread "$node/voltage_now"); cap=$(rawread "$node/capacity")
        ci=${ci:-0}; cv=${cv:-0}
        ca=${ci#-}
        [ "$n" -eq 0 ] && first=$ci
        printf '%s,%s,%s,%s,%s,%s\n' "$label" "$ts" "$cap" "$cv" "$ci" "$ca" >> "$LOG"
        si=$(( si + ca )); sv=$(( sv + cv )); n=$(( n + 1 ))
        sleep "$INTERVAL"
    done

    [ "$n" -gt 0 ] || { echo "ERROR: zero samples"; exit 1; }
    ai=$(( si / n )); av=$(( sv / n ))
    echo "# ---- SUMMARY $label samples=$n ----"
    echo "first_raw_current_now=$first   (sanity: ~XX000 => uA; ~XX => mA)"
    echo "avg_current_abs_uA=$ai   avg_current_mA=$(( ai / 1000 ))"
    echo "avg_voltage_uV=$av   avg_voltage_mV=$(( av / 1000 ))"
}

cmd_top() {
    snaps=${1:-6}
    echo "# top audit (reader must be running, awake, NOT playing); Edge-TTS worker should read ~0%"
    echo "# pidof kothok = $(pidof kothok 2>/dev/null || echo none)"
    i=0
    while [ "$i" -lt "$snaps" ]; do
        echo "---- top $((i+1))/$snaps ----"
        top -b -n 1 2>/dev/null | head -n 18
        sleep 2
        i=$(( i + 1 ))
    done
}

case ${1:-} in
    power) shift; cmd_power "$@" ;;
    top)   shift; cmd_top "$@" ;;
    *) echo "usage: $0 {power <label> [sec] | top [snaps]}"; exit 2 ;;
esac

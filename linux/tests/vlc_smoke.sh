#!/usr/bin/env bash
# Real tuner test, run as the desktop user with other receiver applications closed.
set -euo pipefail
FREQUENCY="${1:?Frequency in Hz, e.g. 641143000}"
PROGRAM="${2:?Broadcast program ID, e.g. 17056}"
OUTPUT="${3:-/tmp/open-volar-s-vlc-test}"
SYSTEM="${4:-isdb-t}"
case "$SYSTEM" in isdb-t|dvb-t) ;; *) echo "Expected isdb-t or dvb-t" >&2; exit 2 ;; esac
[[ "$FREQUENCY" =~ ^[0-9]+$ && "$PROGRAM" =~ ^[0-9]+$ ]] || exit 2
mkdir -p "$OUTPUT"
timeout 35s cvlc -I dummy -vv "$SYSTEM://frequency=$FREQUENCY:bandwidth=6" \
  --dvb-adapter=0 --program="$PROGRAM" --run-time=12 --play-and-exit \
  --sout "#standard{access=file,mux=ts,dst=$OUTPUT/reception.ts}" > "$OUTPUT/vlc.log" 2>&1
[[ -s "$OUTPUT/reception.ts" ]]
ffprobe -v error -show_entries stream=codec_name,width,height,sample_rate,channels \
  -of json "$OUTPUT/reception.ts" > "$OUTPUT/streams.json"
printf 'VLC reception saved to %s\n' "$OUTPUT"

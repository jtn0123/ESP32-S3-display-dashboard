#!/usr/bin/env bash
# Comprehensive 10-min diagnostic harness: serial (if available), ping, /ping, /health, SSE
# Usage: ./tests/scripts/diagnose.sh <device-ip> [duration_secs]

set -euo pipefail

DEVICE_IP=${1:-}
DURATION=${2:-600}
INTERVAL=${INTERVAL:-5}
CURL="curl -sS"

if [ -z "$DEVICE_IP" ]; then
  echo "Usage: $0 <device-ip> [duration_secs]"; exit 1
fi

STAMP=$(date +%Y%m%d-%H%M%S)
OUT_DIR="diag-logs/${STAMP}-${DEVICE_IP}"
mkdir -p "$OUT_DIR"

PING_LOG="$OUT_DIR/ping.log"
P_HTTP_LOG="$OUT_DIR/http_ping.tsv"   # ts\thttp_code\tconnect_ms\ttotal_ms (deprecated; use network.tsv)
H_HTTP_LOG="$OUT_DIR/http_health.tsv" # ts\thttp_code\tconnect_ms\ttotal_ms (deprecated; use network.tsv)
OUT_NET="$OUT_DIR/network.tsv"        # ts\ttype\tstatus\thttp_code\tconnect_ms\ttotal_ms
SSE_LOG="$OUT_DIR/sse.log"
SERIAL_LOG="$OUT_DIR/serial.log"
SUMMARY="$OUT_DIR/summary.txt"

echo "Diagnostics starting for $DEVICE_IP (duration ${DURATION}s, interval ${INTERVAL}s)"
echo "Logs in: $OUT_DIR"

# Try serial if available (non-fatal)
(
  python3 - << 'PY' "$SERIAL_LOG"
import sys, time
port_candidates=['/dev/cu.usbmodem101','/dev/tty.usbmodem101']
port=None
for p in port_candidates:
  try:
    import os
    if os.path.exists(p):
      port=p;break
  except Exception:
    pass
if not port:
  sys.exit(0)
try:
  import serial
  ser=serial.Serial(port,115200,timeout=1)
except Exception:
  sys.exit(0)
start=time.time()
f=open(sys.argv[1],'w')
f.write('# serial monitor started\n')
f.flush()
try:
  while True:
    if ser.in_waiting:
      line=ser.readline().decode('utf-8','replace').rstrip('\n')
      ts=time.strftime('%Y-%m-%dT%H:%M:%S')
      f.write(f'[{ts}] {line}\n'); f.flush()
    time.sleep(0.05)
except KeyboardInterrupt:
  pass
PY
) >/dev/null 2>&1 & SERIAL_PID=$! || true

# SSE in background
(
  echo "# SSE stream from /api/events started"
  $CURL -N "http://$DEVICE_IP/api/events"
) > "$SSE_LOG" 2>&1 & SSE_PID=$!

START=$(date +%s)
NEXT_HEALTH=0

printf "ts\thttp_code\tconnect_ms\ttotal_ms\n" > "$P_HTTP_LOG"
printf "ts\thttp_code\tconnect_ms\ttotal_ms\n" > "$H_HTTP_LOG"
printf "ts\ttype\tstatus\thttp_code\tconnect_ms\ttotal_ms\n" > "$OUT_NET"

while :; do
  NOW=$(date +%s)
  ELAPSED=$(( NOW - START ))
  if [ $ELAPSED -ge $DURATION ]; then break; fi

  # ICMP (also record in consolidated network.tsv)
  if ping -c 1 -W 1 "$DEVICE_IP" >/dev/null 2>&1; then 
    echo "[$(date '+%H:%M:%S')] ping ok" >> "$PING_LOG";
    printf "%s\ticmp\tok\t-\t-\t-\n" "$(date '+%H:%M:%S')" >> "$OUT_NET";
  else 
    echo "[$(date '+%H:%M:%S')] ping fail" >> "$PING_LOG";
    printf "%s\ticmp\tfail\t-\t-\t-\n" "$(date '+%H:%M:%S')" >> "$OUT_NET";
  fi

  # /ping
  raw=$($CURL -o /dev/null -w '%{http_code}\t%{time_connect}\t%{time_total}' --connect-timeout 2 --max-time 4 "http://$DEVICE_IP/ping" || true)
  code=$(printf "%s" "$raw" | awk -F '\t' '{print $1}')
  c_s=$(printf "%s" "$raw" | awk -F '\t' '{print $2}')
  t_s=$(printf "%s" "$raw" | awk -F '\t' '{print $3}')
  # Convert seconds->ms with 1-decimal precision
  c_ms=$(awk -v v="$c_s" 'BEGIN{ if(v=="") v=0; printf "%.1f", v*1000 }')
  t_ms=$(awk -v v="$t_s" 'BEGIN{ if(v=="") v=0; printf "%.1f", v*1000 }')
  [ -z "$code" ] && code=000
  printf "%s\t%s\t%s\n" "$(date '+%H:%M:%S')" "$code" "$c_ms\t$t_ms" >> "$P_HTTP_LOG"
  status=$([ "$code" = "200" ] && echo ok || echo fail)
  printf "%s\thttp_ping\t%s\t%s\t%s\t%s\n" "$(date '+%H:%M:%S')" "$status" "$code" "$c_ms" "$t_ms" >> "$OUT_NET"

  # /health every 30s
  if [ $ELAPSED -ge $NEXT_HEALTH ]; then
    raw=$($CURL -o /dev/null -w '%{http_code}\t%{time_connect}\t%{time_total}' --connect-timeout 2 --max-time 5 "http://$DEVICE_IP/health" || true)
    code=$(printf "%s" "$raw" | awk -F '\t' '{print $1}')
    c_s=$(printf "%s" "$raw" | awk -F '\t' '{print $2}')
    t_s=$(printf "%s" "$raw" | awk -F '\t' '{print $3}')
    c_ms=$(awk -v v="$c_s" 'BEGIN{ if(v=="") v=0; printf "%.1f", v*1000 }')
    t_ms=$(awk -v v="$t_s" 'BEGIN{ if(v=="") v=0; printf "%.1f", v*1000 }')
    [ -z "$code" ] && code=000
    printf "%s\t%s\t%s\n" "$(date '+%H:%M:%S')" "$code" "$c_ms\t$t_ms" >> "$H_HTTP_LOG"
    status=$([ "$code" = "200" ] && echo ok || echo fail)
    printf "%s\thttp_health\t%s\t%s\t%s\t%s\n" "$(date '+%H:%M:%S')" "$status" "$code" "$c_ms" "$t_ms" >> "$OUT_NET"
    NEXT_HEALTH=$(( ELAPSED + 30 ))
  fi

  sleep "$INTERVAL"
done

# Cleanup
kill $SSE_PID >/dev/null 2>&1 || true
kill $SERIAL_PID >/dev/null 2>&1 || true

# Summary
P_OK=$(awk 'NR>1 && $2==200 {ok++} END{print ok+0}' "$P_HTTP_LOG")
P_ALL=$(($(wc -l < "$P_HTTP_LOG")-1))
H_OK=$(awk 'NR>1 && $2==200 {ok++} END{print ok+0}' "$H_HTTP_LOG")
H_ALL=$(($(wc -l < "$H_HTTP_LOG")-1))
{
  echo "Diagnostics summary @ $(date '+%Y-%m-%dT%H:%M:%S')"
  echo "Device: $DEVICE_IP"
  echo "Duration: ${DURATION}s"
  echo "/ping: $P_OK/$P_ALL 200s"
  echo "/health: $H_OK/$H_ALL 200s"
  echo "Logs: $OUT_DIR"
} | tee "$SUMMARY"

exit 0


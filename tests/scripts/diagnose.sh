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
OUT_NET="$OUT_DIR/network.tsv"        # ts\telapsed_s\ticmp_status\ticmp_rtt_ms\thp_status\thp_code\thp_connect_ms\thp_total_ms\thl_status\thl_code\thl_connect_ms\thl_total_ms\thealth_age_s\tsse_bytes\twifi_rssi_dbm\twifi_disc\twifi_reconn\theap_kb\tuptime_s
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
printf "ts\telapsed_s\ticmp_status\ticmp_rtt_ms\thp_status\thp_code\thp_connect_ms\thp_total_ms\thl_status\thl_code\thl_connect_ms\thl_total_ms\thealth_age_s\tsse_bytes\twifi_rssi_dbm\twifi_disc\twifi_reconn\theap_kb\tuptime_s\n" > "$OUT_NET"

LAST_HEALTH_JSON=""
LAST_WIFI_RSSI="-"; LAST_WIFI_DISC="-"; LAST_WIFI_RECONN="-"; LAST_HEAP_KB="-"; LAST_UPTIME_S="-"; LAST_HEALTH_AGE=999999

while :; do
  NOW=$(date +%s)
  ELAPSED=$(( NOW - START ))
  if [ $ELAPSED -ge $DURATION ]; then break; fi

  TS=$(date '+%H:%M:%S')

  # ICMP
  POUT=$(mktemp)
  if ping -c 1 -W 1 "$DEVICE_IP" > "$POUT" 2>&1; then 
    echo "[$TS] ping ok" >> "$PING_LOG";
    ICMP_STATUS=ok
    ICMP_RTT=$(awk -F'time=' '/ time=/{split($2,a," "); print a[1]}' "$POUT")
    [ -z "$ICMP_RTT" ] && ICMP_RTT="-"
  else 
    echo "[$TS] ping fail" >> "$PING_LOG";
    ICMP_STATUS=fail
    ICMP_RTT="-"
  fi
  rm -f "$POUT"

  # /ping
  raw=$($CURL -o /dev/null -w '%{http_code}\t%{time_connect}\t%{time_total}' --connect-timeout 2 --max-time 4 "http://$DEVICE_IP/ping" || true)
  code=$(printf "%s" "$raw" | awk -F '\t' '{print $1}')
  c_s=$(printf "%s" "$raw" | awk -F '\t' '{print $2}')
  t_s=$(printf "%s" "$raw" | awk -F '\t' '{print $3}')
  # Convert seconds->ms with 1-decimal precision
  c_ms=$(awk -v v="$c_s" 'BEGIN{ if(v=="") v=0; printf "%.1f", v*1000 }')
  t_ms=$(awk -v v="$t_s" 'BEGIN{ if(v=="") v=0; printf "%.1f", v*1000 }')
  [ -z "$code" ] && code=000
  printf "%s\t%s\t%s\n" "$TS" "$code" "$c_ms\t$t_ms" >> "$P_HTTP_LOG"
  HP_STATUS=$([ "$code" = "200" ] && echo ok || echo fail)
  HP_CODE=$code; HP_CONNECT_MS=$c_ms; HP_TOTAL_MS=$t_ms

  # /health every 30s
  if [ $ELAPSED -ge $NEXT_HEALTH ]; then
    # Fetch /health body and timings
    HJSON="$OUT_DIR/last_health.json"
    raw=$($CURL -o "$HJSON" -w '%{http_code}\t%{time_connect}\t%{time_total}' --connect-timeout 2 --max-time 5 "http://$DEVICE_IP/health" || true)
    code=$(printf "%s" "$raw" | awk -F '\t' '{print $1}')
    c_s=$(printf "%s" "$raw" | awk -F '\t' '{print $2}')
    t_s=$(printf "%s" "$raw" | awk -F '\t' '{print $3}')
    c_ms=$(awk -v v="$c_s" 'BEGIN{ if(v=="") v=0; printf "%.1f", v*1000 }')
    t_ms=$(awk -v v="$t_s" 'BEGIN{ if(v=="") v=0; printf "%.1f", v*1000 }')
    [ -z "$code" ] && code=000
    printf "%s\t%s\t%s\n" "$TS" "$code" "$c_ms\t$t_ms" >> "$H_HTTP_LOG"

    HL_STATUS=$([ "$code" = "200" ] && echo ok || echo fail)
    HL_CODE=$code; HL_CONNECT_MS=$c_ms; HL_TOTAL_MS=$t_ms
    LAST_HEALTH_AGE=0
    LAST_HEALTH_JSON="$HJSON"

    # Defaults for consolidated health fields
    WIFI_RSSI="-"; WIFI_DISC="-"; WIFI_RECONN="-"; HEAP_KB="-"; UPTIME_S="-"
    if [ "$code" = "200" ] && [ -s "$HJSON" ]; then
      PYOUT=$(python3 - "$HJSON" << 'PY' 2>/dev/null
import sys, json
try:
  with open(sys.argv[1]) as f:
    d=json.load(f)
  wifi=d.get('wifi',{})
  rssi=wifi.get('rssi_dbm','-')
  disc=wifi.get('disconnects','-')
  reconn=wifi.get('reconnects','-')
  heap=d.get('free_heap','-')
  up=d.get('uptime_seconds','-')
  if isinstance(heap,int):
    heap//=1024
  print(rssi, disc, reconn, heap, up)
except Exception:
  print('-', '-', '-', '-', '-')
PY
)
      LAST_WIFI_RSSI=$(echo "$PYOUT" | awk '{print $1}')
      LAST_WIFI_DISC=$(echo "$PYOUT" | awk '{print $2}')
      LAST_WIFI_RECONN=$(echo "$PYOUT" | awk '{print $3}')
      LAST_HEAP_KB=$(echo "$PYOUT" | awk '{print $4}')
      LAST_UPTIME_S=$(echo "$PYOUT" | awk '{print $5}')
    fi
    NEXT_HEALTH=$(( ELAPSED + 30 ))
  else
    # Increment age since last health update
    if [ $LAST_HEALTH_AGE -lt 999999 ]; then LAST_HEALTH_AGE=$(( LAST_HEALTH_AGE + INTERVAL )); fi
    HL_STATUS="-"; HL_CODE="-"; HL_CONNECT_MS="-"; HL_TOTAL_MS="-"
  fi

  # SSE progress sampling (bytes written since last check)
  if [ -f "$SSE_LOG" ]; then
    SZ=$(stat -f%z "$SSE_LOG" 2>/dev/null || echo 0)
    LAST_SZ=${LAST_SZ:-0}
    DELTA=$(( SZ - LAST_SZ ))
    if [ $DELTA -lt 0 ]; then DELTA=$SZ; fi
    LAST_SZ=$SZ
    SSE_BYTES=$DELTA
  else
    SSE_BYTES="-"
  fi

  # Emit one consolidated row for this tick
  printf "%s\t%d\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%d\t%s\t%s\t%s\t%s\t%s\t%s\n" \
    "$TS" "$ELAPSED" \
    "$ICMP_STATUS" "$ICMP_RTT" \
    "$HP_STATUS" "$HP_CODE" "$HP_CONNECT_MS" "$HP_TOTAL_MS" \
    "$HL_STATUS" "$HL_CODE" "$HL_CONNECT_MS" "$HL_TOTAL_MS" "$LAST_HEALTH_AGE" \
    "$SSE_BYTES" "$LAST_WIFI_RSSI" "$LAST_WIFI_DISC" "$LAST_WIFI_RECONN" "$LAST_HEAP_KB" "$LAST_UPTIME_S" \
    >> "$OUT_NET"

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


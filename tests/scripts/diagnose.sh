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
OUT_NET_PRETTY="$OUT_DIR/network_aligned.txt"
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

# SSE in background (can be disabled via NO_SSE=1)
if [ "${NO_SSE:-0}" != "1" ]; then
  (
    echo "# SSE stream from /api/events started"
    $CURL -N "http://$DEVICE_IP/api/events"
  ) > "$SSE_LOG" 2>&1 & SSE_PID=$!
else
  SSE_PID=""
fi

START=$(date +%s)
NEXT_HEALTH=0

printf "ts\thttp_code\tconnect_ms\ttotal_ms\n" > "$P_HTTP_LOG"
printf "ts\thttp_code\tconnect_ms\ttotal_ms\n" > "$H_HTTP_LOG"
printf "ts\telapsed_s\ticmp_status\ticmp_rtt_ms\thp_status\thp_code\thp_connect_ms\thp_total_ms\thl_status\thl_code\thl_connect_ms\thl_total_ms\thealth_age_s\tsse_bytes\twifi_rssi_dbm\twifi_disc\twifi_reconn\theap_kb\tuptime_s\n" > "$OUT_NET"

# Human-friendly aligned header
{
  printf "%-8s %-8s | %-11s %-11s | %-9s %-7s %-13s %-13s | %-9s %-7s %-13s %-13s %-12s | %-10s | %-13s %-10s %-12s | %-8s %-8s\n" \
    "ts" "elapsed" \
    "icmp_status" "icmp_rtt" \
    "hp_status" "hp_code" "hp_connect_ms" "hp_total_ms" \
    "hl_status" "hl_code" "hl_connect_ms" "hl_total_ms" "health_age_s" \
    "sse_bytes" \
    "wifi_rssi_dbm" "wifi_disc" "wifi_reconn" \
    "heap_kb" "uptime_s"
  printf "%-8s %-8s | %-11s %-11s | %-9s %-7s %-13s %-13s | %-9s %-7s %-13s %-13s %-12s | %-10s | %-13s %-10s %-12s | %-8s %-8s\n" \
    "--------" "--------" \
    "-----------" "-----------" \
    "---------" "-------" "-------------" "-------------" \
    "---------" "-------" "-------------" "-------------" "------------" \
    "----------" \
    "-------------" "----------" "------------" \
    "--------" "--------"
} > "$OUT_NET_PRETTY"

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

  # /ping (stagger +1s)
  sleep 1
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

  # /health every 30s (stagger +2s on first eligible tick)
  if [ $ELAPSED -ge $NEXT_HEALTH ]; then
    sleep 1
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

  # Emit human-friendly aligned row
  printf "%-8s %-8d | %-11s %-11s | %-9s %-7s %-13s %-13s | %-9s %-7s %-13s %-13s %-12d | %-10s | %-13s %-10s %-12s | %-8s %-8s\n" \
    "$TS" "$ELAPSED" \
    "$ICMP_STATUS" "$ICMP_RTT" \
    "$HP_STATUS" "$HP_CODE" "$HP_CONNECT_MS" "$HP_TOTAL_MS" \
    "$HL_STATUS" "$HL_CODE" "$HL_CONNECT_MS" "$HL_TOTAL_MS" "$LAST_HEALTH_AGE" \
    "$SSE_BYTES" \
    "$LAST_WIFI_RSSI" "$LAST_WIFI_DISC" "$LAST_WIFI_RECONN" \
    "$LAST_HEAP_KB" "$LAST_UPTIME_S" \
    >> "$OUT_NET_PRETTY"

  sleep "$INTERVAL"
done

# Cleanup
if [ -n "$SSE_PID" ]; then kill $SSE_PID >/dev/null 2>&1 || true; fi
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

# Detailed stats from consolidated network.tsv
python3 - "$OUT_NET" >> "$SUMMARY" << 'PY'
import sys, statistics as stats
from math import floor

path = sys.argv[1]
def f(x):
  try:
    return float(x)
  except Exception:
    return None

icmp_ok=icmp_fail=0; icmp_rtts=[]
hp_ok=hp_fail=0; hp_conn=[]; hp_total=[]
hl_ok=hl_fail=0; hl_conn=[]; hl_total=[]
sse_bytes=[]; sse_active=0
wifi_rssi=[]; last_disc='-'; last_reconn='-'

with open(path) as fh:
  header=True
  for line in fh:
    if header:
      header=False; continue
    cols=line.rstrip('\n').split('\t')
    if len(cols) < 19: continue
    # columns per script
    icmp_status, icmp_rtt = cols[2], f(cols[3])
    hp_status, hp_code, hp_c, hp_t = cols[4], cols[5], f(cols[6]), f(cols[7])
    hl_status, hl_code, hl_c, hl_t = cols[8], cols[9], f(cols[10]), f(cols[11])
    sse = f(cols[13])
    rssi = f(cols[14])
    last_disc = cols[15]
    last_reconn = cols[16]

    if icmp_status=='ok':
      icmp_ok+=1
      if icmp_rtt is not None: icmp_rtts.append(icmp_rtt)
    elif icmp_status=='fail': icmp_fail+=1

    if hp_status=='ok':
      hp_ok+=1
      if hp_c is not None: hp_conn.append(hp_c)
      if hp_t is not None: hp_total.append(hp_t)
    elif hp_status=='fail': hp_fail+=1

    if hl_status=='ok':
      hl_ok+=1
      if hl_c is not None: hl_conn.append(hl_c)
      if hl_t is not None: hl_total.append(hl_t)
    elif hl_status=='fail': hl_fail+=1

    if sse is not None:
      sse_bytes.append(sse)
      if sse>0: sse_active+=1

    if rssi is not None:
      wifi_rssi.append(rssi)

def pct_ok(ok, fail):
  n=ok+fail
  return (ok*100.0/n) if n else 0.0

def describe(lst):
  if not lst: return '-'
  s=sorted(lst)
  n=len(s)
  p50=s[floor((n-1)*0.5)]
  p95=s[floor((n-1)*0.95)]
  avg=sum(s)/n
  return f"avg={avg:.1f} p50={p50:.1f} p95={p95:.1f}"

print("\n=== Detailed summary ===")
print(f"ICMP: ok={icmp_ok} fail={icmp_fail} ok%={pct_ok(icmp_ok,icmp_fail):.1f} {describe(icmp_rtts)} ms")
print(f"HTTP /ping: ok={hp_ok} fail={hp_fail} ok%={pct_ok(hp_ok,hp_fail):.1f} connect_ms[{describe(hp_conn)}] total_ms[{describe(hp_total)}]")
print(f"HTTP /health: ok={hl_ok} fail={hl_fail} ok%={pct_ok(hl_ok,hl_fail):.1f} connect_ms[{describe(hl_conn)}] total_ms[{describe(hl_total)}]")
if sse_bytes:
  n=len(sse_bytes); active=sse_active
  print(f"SSE: active_ticks={active}/{n} ({active*100.0/n:.1f}%) bytes[{describe(sse_bytes)}]")
if wifi_rssi:
  print(f"WiFi RSSI: {describe(wifi_rssi)} dBm last_disc={last_disc} last_reconn={last_reconn}")
PY

exit 0


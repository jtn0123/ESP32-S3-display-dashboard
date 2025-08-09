#!/usr/bin/env bash
# Soak test: poll /health periodically and keep SSE connection open.
# Usage: ./tests/scripts/soak-web.sh <device-ip> [duration_seconds] [interval_seconds]

set -euo pipefail

DEVICE_IP=${1:-}
DURATION_SECS=${2:-3600}
INTERVAL_SECS=${3:-60}
CURL="curl -sS"
MAX_TIME=${MAX_TIME:-10}
RETRIES=${RETRIES:-2}

if [ -z "$DEVICE_IP" ]; then
  echo "Usage: $0 <device-ip> [duration_seconds] [interval_seconds]"
  exit 1
fi

START_TS=$(date +%s)
STAMP=$(date +%Y%m%d-%H%M%S)
OUT_DIR="soak-logs/${STAMP}-${DEVICE_IP}"
mkdir -p "$OUT_DIR"
HEALTH_LOG="$OUT_DIR/health.log"
SSE_LOG="$OUT_DIR/sse.log"
ANOMALY_LOG="$OUT_DIR/anomalies.log"
SUMMARY_LOG="$OUT_DIR/summary.log"

echo "Starting soak test for $DEVICE_IP for ${DURATION_SECS}s (interval ${INTERVAL_SECS}s)"
echo "Logs: $OUT_DIR"

# Start SSE stream in background
(
  echo "# SSE stream from /api/events started"
  while IFS= read -r line; do
    printf '[%s] %s\n' "$(date '+%Y-%m-%dT%H:%M:%S')" "$line"
  done < <($CURL -N "http://$DEVICE_IP/api/events" 2>&1)
) > "$SSE_LOG" &
SSE_PID=$!

cleanup() {
  echo "Stopping soak test..." | tee -a "$SUMMARY_LOG"
  if ps -p $SSE_PID >/dev/null 2>&1; then
    kill $SSE_PID >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT INT TERM

# Health polling
INITIAL_HEAP=""
MIN_HEAP=999999999
MAX_HEAP=0
WARN_COUNT=0
ITER=0

while true; do
  NOW=$(date '+%Y-%m-%dT%H:%M:%S')
  ELAPSED=$(( $(date +%s) - START_TS ))
  if [ $ELAPSED -ge $DURATION_SECS ]; then
    break
  fi

  RESP=""
  for attempt in $(seq 0 $RETRIES); do
    RESP=$($CURL --max-time "$MAX_TIME" "http://$DEVICE_IP/health" || true)
    [ -n "$RESP" ] && break
    sleep 1
  done
  if [ -z "$RESP" ]; then
    echo "$NOW health: ERROR no response" | tee -a "$ANOMALY_LOG" >> "$HEALTH_LOG"
    WARN_COUNT=$((WARN_COUNT+1))
  else
    FREE_HEAP=$(echo "$RESP" | grep -o '"free_heap":[0-9]*' | head -1 | cut -d: -f2)
    STATUS=$(echo "$RESP" | grep -o '"status":"[^"]*"' | head -1 | cut -d: -f2 | tr -d '"')
    RESET_REASON=$(echo "$RESP" | grep -o '"reset_reason":"[^"]*"' | head -1 | cut -d: -f2 | tr -d '"')
    RESET_CODE=$(echo "$RESP" | grep -o '"reset_code":[0-9-]*' | head -1 | cut -d: -f2)

  echo "$NOW health: status=$STATUS heap=$FREE_HEAP reset_reason=${RESET_REASON:-n/a} code=${RESET_CODE:-n/a}" >> "$HEALTH_LOG"

    # Track heap stats
    if [ -n "$FREE_HEAP" ]; then
      [ -z "$INITIAL_HEAP" ] && INITIAL_HEAP=$FREE_HEAP
      [ $FREE_HEAP -lt $MIN_HEAP ] && MIN_HEAP=$FREE_HEAP
      [ $FREE_HEAP -gt $MAX_HEAP ] && MAX_HEAP=$FREE_HEAP

      # Anomaly: non-healthy status
      if [ "$STATUS" != "healthy" ]; then
        echo "$NOW anomaly: non-healthy status ($STATUS)" | tee -a "$ANOMALY_LOG"
        WARN_COUNT=$((WARN_COUNT+1))
      fi
    else
      echo "$NOW anomaly: failed to parse free_heap from /health" | tee -a "$ANOMALY_LOG"
      WARN_COUNT=$((WARN_COUNT+1))
    fi
  fi

  ITER=$((ITER+1))
  sleep $INTERVAL_SECS
done

# Summary
DELTA_HEAP=$(( ${INITIAL_HEAP:-0} - ${MIN_HEAP:-0} ))
{
  echo "Soak summary @ $(date '+%Y-%m-%dT%H:%M:%S')"
  echo "Device: $DEVICE_IP"
  echo "Duration: ${ELAPSED}s, interval: ${INTERVAL_SECS}s, samples: ${ITER}"
  echo "Heap: initial=${INITIAL_HEAP:-n/a} min=${MIN_HEAP:-n/a} max=${MAX_HEAP:-n/a} delta_drop=${DELTA_HEAP}"
  echo "Warnings: $WARN_COUNT"
  echo "Logs in: $OUT_DIR"
} | tee -a "$SUMMARY_LOG"

exit 0

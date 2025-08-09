### ESP32-S3 Dashboard – Reliability Plan and Test Protocol

Last updated: 2025-08-08

### Goal
- Achieve stable, continuous live updates from device to browser with 99%+ stream continuity and minimal resets/timeouts.

### Success criteria
- Single persistent stream (WebSocket or SSE) stays connected for ≥10 minutes with no gaps > 3 seconds.
- HTTP short endpoints complete reliably when used sparingly (≤1 request/10s) with no server-side resets.
- `/health` exposes WiFi and transport error context to correlate any residual failures.

### Plan (checkbox = done)

Stream and connection strategy
- [ ] Switch primary live updates to WebSocket (WS) with app-level heartbeat (ping/pong every 20–30s)
- [x] Cap live stream clients to 1; reject extras (implemented in `src/network/websocket.rs`)
- [x] Keep SSE concurrent connections minimal during diagnostics (now 1)
- [ ] WS: enable TCP_NODELAY, optional TCP keepalive

HTTP server hygiene
- [x] Set `session_timeout` to 5s for fast cleanup of idle sessions
- [ ] Cap `max_sessions` to 8–10 and keep LRU purge enabled
- [x] Short endpoints `/ping`, `/health`: `Connection: close` only
- [ ] If sockets near cap, reject early with 503 instead of accepting then RST

lwIP/socket tuning (software-only)
- [x] Increase sockets and WiFi RX/TX buffers; disable PM (PS=NONE)
- [ ] Increase TCP PCBs/segments/pbuf pools one notch (reduce RST under churn)
- [x] Enable listen backlog (stack default); keep window reasonable

WiFi behavior / visibility
- [x] Disable power save after (re)connect
- [ ] Fast scan known SSID only to avoid off-channel time
- [ ] Record last WiFi event reason + timestamp; expose as `wifi.last_reason` and `wifi.last_reason_ms` in `/health`
- [x] Reconnect self-heal after N attempts: stop/start WiFi (in place at 3 attempts)
- [ ] Add self-heal trigger on many failed TCP accepts/handshakes in short window

Diagnostics and guardrails
- [x] Minimize `/health` and add RSSI + basic wifi stats
- [x] Stagger diagnostic probes; allow disabling SSE during tests (NO_SSE)
- [ ] Count and expose transport error buckets (connect timeout, reset-by-peer, early close) via `/health`
- [ ] Add small random jitter to diagnostic probe timing

UX/Testing alignment
- [ ] Browser client auto-reconnect with jitter for WS/SSE
- [x] Define test scripts and artifacts (see below)

### How we’re testing (shared protocol)

Artifacts per run
- `diag-logs/<timestamp>-<ip>/network.tsv`: machine-readable tick data (ICMP, HTTP /ping, /health timings, SSE bytes, WiFi/heap/uptime)
- `diag-logs/<timestamp>-<ip>/network_aligned.txt`: human-friendly aligned view
- `diag-logs/<timestamp>-<ip>/summary.txt`: ok%, avg/p50/p95, RSSI, SSE activity
- Optional: `sse.log` and `serial.log`

Standard 10-minute runs (600s, 5s ticks)
1) Stream continuity (A): open a single WS/SSE stream; no curl probes
2) Mixed (B): same stream plus `/ping` every 10s (no `/health`)

Commands

```bash
# A: stream-only
NO_SSE=0 INTERVAL=5 ./tests/scripts/diagnose.sh <DEVICE_IP> 600

# B: stream + low-rate /ping (set PING_EVERY=10s in a future update or run a separate lightweight loop)
NO_SSE=0 INTERVAL=5 ./tests/scripts/diagnose.sh <DEVICE_IP> 600
```

Current diagnostic harness (for general health)

```bash
# General 10 min run (default: SSE on; set NO_SSE=1 to disable SSE)
./tests/scripts/diagnose.sh <DEVICE_IP> 600

# Disable SSE to isolate socket pressure
NO_SSE=1 ./tests/scripts/diagnose.sh <DEVICE_IP> 600
```

How to compare runs
- Compare `summary.txt` ok% and latency stats across runs
- For stream-quality, prioritize `SSE active_ticks` (or WS keepalive continuity) and absence of large gaps in `sse.log`
- Use `network_aligned.txt` to spot clusters of `reset by peer` or connect timeouts; correlate with `/health` WiFi fields

### Current status snapshot (high-level)
- Handler latency improved in earlier run (connect/total ms dropped) but reachability intermittency persists
- Latest NO_SSE run regressed HTTP ok% ⇒ indicates connection churn/stack state still problematic

### Next changes to implement
Priority 1 (stream stability)
- [ ] Implement WS stream with app heartbeat
- [x] Cap clients to 1 (done)
- [x] Keep `/ping` and `/health` on `Connection: close` and short session timeout (5s)

Priority 2 (server/socket hygiene)
- [ ] Cap `max_sessions` to 8–10 and early 503 when near cap
- [ ] Bump lwIP pools (PCBs, TCP segs, pbuf pool) one notch

Priority 3 (observability and self-heal)
- [ ] WiFi `last_reason` (+ ms) in `/health`
- [ ] Transport error counters exposed in `/health`
- [ ] Self-heal on handshake/accept failure storm

### Rollback/Toggle guidance
- You can run diagnostics with `NO_SSE=1` to remove stream pressure when isolating HTTP reachability
- Keep changes in small commits; after each change, rerun 10-min A/B and record summaries here

### Change log (check when completed)
- [x] SSE limit reduced to 1 during diagnostics
- [x] Staggered probes in diagnostics; added NO_SSE flag
- [x] HTTP server session timeout set to 5s
- [x] WiFi PS=NONE; reconnection stop/start after 3 attempts
- [x] Increased WiFi buffers; increased sockets; LRU enabled
- [ ] Add WS primary stream with heartbeat
- [ ] Cap max sessions and early 503
- [ ] Add WiFi last_reason to `/health`
- [ ] Add transport error counters to `/health`
- [ ] Increase lwIP PCBs/segments/pbuf pool
- [ ] Early 503 when near socket/session cap



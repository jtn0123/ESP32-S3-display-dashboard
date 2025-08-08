# Debugging & Stability Roadmap (Current and Next)

This document tracks the strongest-impact improvements for reliability and debuggability, with a prioritized plan and concrete action items.

## Completed (baseline)
- Single enhanced logger with colored serial, telnet forwarding, and in-memory ring buffer (`LogStreamer`).
- Runtime log-level control endpoint: POST `/api/v1/debug/log-level` (`{"level":"debug"}` or `?level=`).
- Recent logs and live stream: `GET /api/v1/logs/recent?count=N` and SSE `GET /sse/logs`.
- Panic hook dumps diagnostics and memory state before restart.
- Replaced panic-prone `unwrap()`s in critical paths (SSE/WS/web APIs, time conversions, locks) with safe handling.
- Standardized JSON error responses in web server using helpers.
- Replaced unsafe globals with `OnceLock` + `Mutex` patterns.

## Highest impact next (implement in order)

1) Persisted crash diagnostics (forensics)
- Goal: Survive reboot with actionable info.
- Actions:
  - Store last panic reason, timestamp, heap stats, uptime, and a short recent-log excerpt to NVS.
  - Endpoint: `GET /api/v1/diagnostics/last-crash` (JSON), and surface in `/logs` UI.
  - Add a tiny ring buffer snapshot function callable from panic hook.

2) Request tracing and timing
- Goal: End-to-end visibility per HTTP request.
- Actions:
  - Middleware to assign `request_id`, record start/end timestamps, compute duration.
  - Include `request_id` in all logs and JSON error responses.
  - Add summary counters: per-route 2xx/4xx/5xx, average/percentile latency.

3) Memory backpressure and graceful degradation
- Goal: Avoid instability under low heap/PSRAM.
- Actions:
  - Periodic heap monitor with thresholds; when critical:
    - Temporarily lower log level (e.g., to WARN), shrink log buffer, and pause SSE/WS broadcasts.
  - Expose current memory status in `/health` and `/metrics`.

4) Watchdog coverage on long operations
- Goal: Prevent false watchdog resets during heavy work.
- Actions:
  - Feed watchdog inside long handlers (OTA, gzip, file uploads, scans).
  - Log watchdog feeds with task names using `diagnostics::log_watchdog_feed`.

## Secondary improvements (soon after)
- Task/stack monitoring: read FreeRTOS high-watermarks, expose `/api/v1/diagnostics/tasks`.
- Rate limits and connection hygiene: per-IP caps and timeouts for HTTP/SSE/WS; clean teardown on errors.
- Brownout/voltage resilience: enable brownout detector; emit alerts and reduce nonessential work.
- Security for debug endpoints: token-gate `/api/v1/debug/*` and `/api/v1/logs/*` or compile-gate for release.
- Build and runtime metadata: include git hash, build time, feature flags in `/metrics`.

## Testing plan (to lock behavior)
- Crash persistence E2E: induce a panic, reboot, fetch `/api/v1/diagnostics/last-crash`.
- Request tracing: assert `request_id` propagation and latency logging for success and error paths.
- Low-memory behavior: simulate heap pressure; verify backpressure triggers (reduced logs/SSE pause) and recovery.
- Watchdog: long operations do not reset device; feeds logged at expected cadence.
- Rate limiting: concurrent clients receive proper 429/503 with no panics.

## Milestones and acceptance criteria
- M1: Crash persistence implemented; endpoint returns correct JSON; unit/integration tests passing.
- M2: Request tracing visible in logs and responses; per-route metrics exposed.
- M3: Low-memory backpressure active; device remains responsive under stress tests.
- M4: Watchdog feed added to long operations; no unintended resets during OTA and large transfers.

## References
- Logging: `src/logging.rs`, `src/network/log_streamer.rs`
- Web/API: `src/network/{web_server.rs,api_routes.rs,sse_v2.rs,websocket.rs}`
- Diagnostics: `src/{crash_diagnostics.rs,memory_diagnostics.rs,diagnostics.rs}`
- Power/Voltage: `src/power/voltage_monitor.rs`

## Notes
- Keep default log level at DEBUG during active development; allow runtime adjustment for field testing.
- Log buffer policy: bounded (~2000 entries), drop-oldest, non-blocking appends to avoid stalls.

---

## Design: Persisted Crash Diagnostics (M1)

Goal: After a panic/reboot, surface actionable crash info via API/UI for post-mortem debugging.

- Data to persist (compact JSON or key-value):
  - panic_reason: short string from panic hook (location + message)
  - timestamp_unix: seconds since epoch (best-effort)
  - uptime_seconds: session uptime at crash
  - heap: { free, min_free, psram_free }
  - log_excerpt: last N log entries (e.g., 50 lines) from `LogStreamer`

- Storage:
  - NVS namespace: `crash`
  - Keys: `reason`, `ts`, `uptime`, `heap_free`, `heap_min`, `psram_free`, `excerpt`
  - Size limits: excerpt ~2–4KB; truncate safely if needed

- Write path (panic-safe):
  1) Panic hook logs error and memory state (already in place)
  2) Try to snapshot recent logs: `log_streamer.get_recent_logs(50)` with `try_lock` semantics inside helper
  3) Store minimal fields to NVS using short writes; ignore failures; no unwraps
  4) Delay briefly to flush outputs; restart

- Read path:
  - Endpoint: `GET /api/v1/diagnostics/last-crash` → JSON with fields above
  - Clear endpoint: `DELETE /api/v1/diagnostics/last-crash` → removes keys
  - UI: show on logs page with “last crash” card; copy-to-clipboard button

- Security/operational notes:
  - Gate under debug build flag or require a token in production
  - Keep payload small; avoid heavy serialization in hook
  - Works even if telnet/serial logs weren’t captured

- Tests:
  - Inject synthetic panic path in a controlled handler, verify persistence and endpoint output post-reboot (integration test harness)

Implementation sketch:
- New module `src/diagnostics/crash_persist.rs` with `save_last_crash()` and `read_last_crash()`
- Extend panic hook in `main.rs` to call `crash_persist::save_last_crash()` after `dump_diagnostics()`
- Add endpoints in web server or `api_routes.rs`

---

## README updates to incorporate (when M1 lands)

- Add “Last Crash Diagnostics” to Debugging & Diagnostics section:
  - Describe what is captured and where it’s stored (NVS, best-effort, small excerpt)
  - Endpoints:
    - `GET /api/v1/diagnostics/last-crash`
    - `DELETE /api/v1/diagnostics/last-crash`
  - Example usage:
    ```bash
    curl http://<device-ip>/api/v1/diagnostics/last-crash | jq
    curl -X DELETE http://<device-ip>/api/v1/diagnostics/last-crash
    ```
  - UI note: visible on Logs page as a “Last Crash” panel if present
  - Security note: optionally token-gated or debug-only

- Mention known limits:
  - Log excerpt truncated, timestamps best-effort, persistence best-effort on severe faults
  - Safe fallback if NVS unavailable

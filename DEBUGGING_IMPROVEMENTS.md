# Debugging Improvements Plan

This document lists concrete areas to improve debugging, with priorities and references.

## Top priorities (actionable)

1) Consolidate and upgrade logging
- Problem: Two loggers exist: `src/logging.rs` and `src/logging_enhanced.rs`, but `main.rs` initializes `logging::init_logger()` which maps to `src/logging.rs` (basic). Enhanced logger with timestamps/colors isn’t used consistently.
  - Evidence:
    - `main.rs` uses `logging::init_logger()`.
    - Both modules define `init_logger()`.
- Risks: Inconsistent formatting; `println!` used in multiple places bypasses telnet/filters.
- Actions:
  - Replace `mod logging;` with the enhanced logger module (or merge both into `logging.rs`).
  - Remove direct `println!` in code paths and use `log::*` macros only. Hook enhanced logger to Telnet and to a ring buffer.
  - Add runtime knob (telnet command or config) to set `log::set_max_level` at runtime.
  - Ensure ANSI colors are suppressed on telnet.

2) Wire logs to streamed endpoints and bound memory usage
- Problem: `src/network/log_streamer.rs` stores logs but logger doesn’t push entries into it. `MAX_LOG_LINES = 10000` is likely too high for DRAM.
  - Evidence: The log streamer has no append API used by logger.
- Actions:
  - Add non-blocking append in the logger to push to `LogStreamer` (use `try_lock`, drop-on-full policy, PSRAM if available).
  - Reduce default buffer (e.g., 1–2K entries) or place buffer in PSRAM.
  - Provide `/logs/recent?count=N` and keep SSE logs endpoint using this buffer.

3) Replace fragile `.unwrap()` and mutex panics in critical paths
- Problem: Many `lock().unwrap()` and other `unwrap()` usages in network/UI paths can crash the device.
  - Examples (non-exhaustive):
    - Mutex locks in SSE/WebSocket/web server routes.
    - Time conversions with `.unwrap()`.
- Actions:
  - Introduce small helpers: `safe_lock(mutex, context)` returning `Result<Guard, Error>` and logging on poison.
  - Replace `unwrap()` with `expect("...context...")` or proper `?` propagation where feasible.
  - Prioritize network modules: `src/network/{sse_v2.rs,sse_broadcaster.rs,websocket.rs,web_server.rs,api_routes.rs}`.

4) Unify HTTP error handling and responses
- Problem: Two related modules exist: `src/network/error_wrapper.rs` and `src/network/error_handler.rs` (structured JSON). Not used uniformly across handlers.
- Actions:
  - Wrap all handlers with `wrap_handler()` and use `ErrorResponse` for JSON errors.
  - Ensure request IDs appear in logs alongside responses (include in log lines).

5) Panic/crash diagnostics consistency (release-safe)
- Observations:
  - Custom panic hook in `main.rs` logs and restarts; release profile uses `panic = "abort"` which still calls the hook, but there is no unwind.
  - Diagnostics module exists (`src/crash_diagnostics.rs`, `src/memory_diagnostics.rs`).
- Actions:
  - Call `crash_diagnostics::dump_diagnostics()` in the panic hook in addition to memory logs.
  - Confirm minimal allocations in hook and delay long enough for serial/telnet flush.
  - Consider enabling ESP-IDF backtrace printing (via `esp-idf-sys`/sdkconfig) and documenting how to decode.

6) Remove `static mut` for shared error state
- Problem: `static mut WEB_SERVER_ERROR: Option<String>` in `main.rs` is unsound and not synchronized.
- Action:
  - Replace with `OnceLock<String>` or `Mutex<Option<String>>`.

## Additional improvements

- Normalize timestamps and module tagging in all logs
  - Ensure every handler logs with a consistent prelude: request ID, path, start/stop, duration, memory delta.

- Watchdog and heap health visibility
  - Use `diagnostics::log_watchdog_feed` (already gated for debug) in long operations.
  - Add a periodic low-impact health log at INFO every 30–60s.

- Telnet commands for debugging controls
  - Add commands: `log level <trace|debug|info|warn|error>`, `diag dump`, `mem`, `tasks`.

- Testing and validation
  - Add Python tests that:
    - Induce handler errors and assert structured JSON + server stability.
    - Stress SSE/WebSocket while fetching `/health` and recent logs.
    - Verify no panics on poisoned mutex simulation (if feasible via fault injection wrappers).

## Concrete references (for quick navigation)

- Logging modules:
  - `src/logging.rs` and `src/logging_enhanced.rs`
  - `src/psram.rs` uses `println!` for info output.
  - `src/core1_tasks/mod.rs` prints directly.

- Panic/crash diagnostics:
  - Hook in `src/main.rs`.
  - Modules: `src/crash_diagnostics.rs`, `src/memory_diagnostics.rs`, `src/diagnostics.rs`.

- Error handling:
  - Structured errors: `src/network/error_handler.rs`.
  - Handler wrapper: `src/network/error_wrapper.rs`.

- Risky `unwrap()` hotspots (examples to start with):
  - `src/network/{sse_v2.rs,sse_broadcaster.rs,websocket.rs,api_routes.rs,web_server_enhanced.rs}`
  - `src/ui/mod.rs`, `src/sensors/history.rs`, `src/power/voltage_monitor.rs`

## Proposed sequence (increments)

1) Merge loggers and remove direct `println!` in non-logger code; wire `LogStreamer` + reduce buffer.
2) Replace critical `.unwrap()` in network modules with safe locking/propagation.
3) Adopt `wrap_handler` and `ErrorResponse` across HTTP endpoints, include request IDs in logs.
4) Harden panic hook and integrate `crash_diagnostics::dump_diagnostics()`.
5) Replace `static mut WEB_SERVER_ERROR` with `OnceLock` or `Mutex<Option<String>>`.
6) Add telnet log-level controls and minimal periodic health logs.

## Open questions

- Preferred default log level in production? Keep DEBUG or switch to INFO with runtime override?
- Acceptable log buffer size and PSRAM usage for logs?
- Should we keep colored logs on serial by default, or only in debug builds?

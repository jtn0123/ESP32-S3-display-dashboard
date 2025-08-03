# Rust Compilation Warnings to Fix

## Summary
The codebase has 30 warnings that need to be addressed. Most are unused code that should be removed rather than suppressed with `#[allow(dead_code)]`.

## Warnings by Category

### 1. Unused Imports (1)
- `src/network/telnet.rs`: `ShutdownHandler` is imported but never used

### 2. Unused Methods/Functions (17)
- `src/network/wifi_manager.rs`:
  - `increment_disconnect_count` - never used
  - `increment_reconnect_count` - never used
  
- `src/network/web.rs`:
  - `try_start_web_server_with_retries` - never used
  - `spawn_web_server_retry_task` - never used
  - `create_enhanced_http_config` - never used
  
- `src/network/streaming.rs`:
  - All streaming functionality is unused:
    - constant `CHUNK_SIZE`
    - struct `StreamingResponse`
    - methods: `new`, `write_str`, `write_bytes`, `write_fmt`, `flush`, `finish`
  - Helper functions all unused:
    - `stream_template_header`
    - `stream_template_footer`
    - `stream_card`
    - `format_value`
    
- `src/network/streaming_handlers.rs`:
  - `handle_home_streaming` - never used
  
- `src/network/telnet.rs`:
  - `wait_for_completion` - never used
  
- `src/system/shutdown.rs`:
  - `register_service` - never used
  - `setup_shutdown_handler` - never used
  - Multiple `new` methods for shutdown types
  
- `src/metrics.rs`:
  - `update_http_connections` - never used
  
- `src/templates/mod.rs`:
  - `HOME_PAGE_WITH_THEME` constant - never used
  - `render_home_page` - never used
  - `render_home_page_with_theme` - never used
  - `format_uptime` - never used
  
- `src/power/mod.rs`:
  - `update` - never used
  - `should_update_display` - never used

### 3. Unused Fields (9)
- `src/system/shutdown.rs`:
  - `shutdown_signal` field is never read
  
- `src/memory_diagnostics.rs`:
  - `psram_largest_kb` - never read
  - `stack_remaining` - never read
  
- `src/power/mod.rs`:
  - `dim_timeout` - never read
  - `power_save_timeout` - never read
  - `sleep_timeout` - never read
  - `low_battery_threshold` - never read

### 4. Unused Variants (1)
- `src/power/mod.rs`:
  - `PowerMode::Dimmed` variant is never constructed

### 5. Naming Convention (1)
- `src/main.rs`:
  - `portTICK_PERIOD_MS` should be `PORT_TICK_PERIOD_MS` (uppercase)

## Recommended Actions

### Remove Completely
1. All streaming code in `src/network/streaming.rs` and `streaming_handlers.rs` - appears to be unused experimental code
2. Unused template functions in `src/templates/mod.rs`
3. Unused shutdown handler code that's not being called
4. Unused retry functions in web server

### Fix or Implement
1. Power management methods (`update`, `should_update_display`) - these seem important
2. WiFi manager disconnect/reconnect counters - useful for monitoring
3. Memory diagnostic fields - these provide useful debug info

### Quick Fixes
1. Remove unused import in telnet.rs
2. Rename `portTICK_PERIOD_MS` to uppercase

## Priority Order
1. **High**: Remove unused imports and fix naming (easy wins)
2. **Medium**: Remove completely unused modules (streaming code)
3. **Low**: Decide whether to implement or remove partially used code
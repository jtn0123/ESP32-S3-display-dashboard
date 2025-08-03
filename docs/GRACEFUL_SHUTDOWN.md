# Graceful Shutdown System

## Overview

The ESP32-S3 Dashboard implements a comprehensive graceful shutdown system that ensures all services are properly stopped before the device restarts or powers down. This prevents data corruption, ensures clean network disconnections, and provides user feedback during the shutdown process.

## Architecture

### Components

1. **ShutdownManager** (`src/system/shutdown.rs`)
   - Central coordinator for shutdown operations
   - Maintains list of services to shutdown
   - Provides shutdown signal for async operations

2. **ShutdownSignal**
   - Shared signal that can be checked by long-running operations
   - Allows services to detect shutdown request and clean up

3. **ShutdownHandler Trait**
   - Interface for services that need cleanup
   - Each service implements its own shutdown logic

### Shutdown Handlers

- **WebServerShutdown**: Stops HTTP server and closes connections
- **TelnetServerShutdown**: Notifies clients and stops telnet server
- **WiFiShutdown**: Disconnects WiFi cleanly
- **DisplayShutdown**: Shows shutdown message and optionally powers off display

## Triggering Shutdown

### Button Combination
Hold both buttons (GPIO0 and GPIO14) simultaneously for 1 second to trigger shutdown.

```rust
// In button manager
if event == ButtonEvent::BothButtonsLongPress {
    shutdown_manager.shutdown()?;
}
```

### Programmatic Shutdown
```rust
// Request shutdown
shutdown_signal.request_shutdown();

// Or trigger full shutdown
shutdown_manager.lock().unwrap().shutdown()?;
```

## Shutdown Sequence

1. **Signal Phase**
   - Shutdown signal is set
   - Long-running operations check signal and begin cleanup

2. **Service Shutdown**
   - Services are shut down in reverse order of registration
   - Each service has its own cleanup logic
   - Errors are logged but don't stop the sequence

3. **Display Notification**
   - "Shutting down..." message displayed
   - Display remains on to show status

4. **Final Cleanup**
   - Memory state logged
   - 1 second delay for services to fully stop
   - Device restarts or powers down

## Testing

### Manual Test
1. Connect to device telnet: `telnet <device-ip> 23`
2. Hold both buttons for 1 second
3. Observe shutdown sequence in telnet output
4. Verify all services stop cleanly

### Automated Test
```bash
cd tests/python
./test_graceful_shutdown.py <device-ip>
```

The test script:
- Monitors telnet for shutdown messages
- Tracks service availability during shutdown
- Verifies all services stop
- Checks if device recovers after restart

## Integration Guide

### Adding a New Service

1. Implement the ShutdownHandler trait:
```rust
struct MyServiceShutdown {
    service: Option<MyService>,
}

impl ShutdownHandler for MyServiceShutdown {
    fn name(&self) -> &str {
        "MyService"
    }
    
    fn shutdown(&mut self) -> Result<()> {
        if let Some(service) = self.service.take() {
            service.stop()?;
        }
        Ok(())
    }
}
```

2. Register with shutdown manager:
```rust
shutdown_manager.register_service(
    Box::new(MyServiceShutdown::new(service))
);
```

### Using Shutdown Signal

For long-running operations:
```rust
loop {
    if shutdown_signal.is_shutdown_requested() {
        break;
    }
    
    // Do work...
}
```

## Monitoring

### Telnet Output
During shutdown, the following messages appear:
```
🛑 Shutdown requested
🛑 Beginning graceful shutdown sequence...
✅ TelnetServer shutdown complete
✅ WebServer shutdown complete
✅ WiFi shutdown complete
🛑 All services shut down
Shutdown complete
```

### Metrics
- Shutdown events can be tracked via metrics endpoint
- Memory state is logged before and after shutdown
- Service stop times are logged

## Best Practices

1. **Quick Cleanup**: Keep shutdown handlers fast (<1 second)
2. **Error Handling**: Log errors but don't fail shutdown
3. **Resource Release**: Ensure all resources are properly released
4. **Network Cleanup**: Close all network connections gracefully
5. **User Feedback**: Show clear shutdown status on display

## Troubleshooting

### Services Not Stopping
- Check if service is checking shutdown signal
- Verify service is registered with shutdown manager
- Look for blocking operations in service

### Shutdown Hangs
- Add timeout to shutdown operations
- Check for deadlocks in service cleanup
- Use non-blocking operations where possible

### Device Doesn't Restart
- Verify esp_restart() is called after shutdown
- Check for panic during shutdown
- Monitor serial output for errors
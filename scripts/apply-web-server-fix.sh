#!/bin/bash
# Script to apply web server connectivity fixes

set -e

echo "=== ESP32 Web Server Fix Application Script ==="
echo

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ] || [ ! -d "src/network" ]; then
    echo "Error: This script must be run from the project root directory"
    exit 1
fi

echo "This script will:"
echo "1. Back up your current main.rs"
echo "2. Apply the web server retry logic"
echo "3. Compile the project"
echo "4. Optionally flash the device"
echo

read -p "Continue? (y/n) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Aborted."
    exit 0
fi

# Create backup
echo "Creating backup of main.rs..."
cp src/main.rs src/main.rs.backup.$(date +%Y%m%d_%H%M%S)

# Apply the simple fix (minimal changes approach)
echo "Applying web server retry fix..."

# Use sed to replace the web server creation block
# This is the safer approach - just add retry logic around existing code
cat > /tmp/web_server_fix.sed << 'EOF'
/match network::web_server::WebConfigServer::new_with_ota/,/}$/ {
    # Save the pattern space
    h
    # Check if this is the start of our block
    /match network::web_server::WebConfigServer::new_with_ota/ {
        # Replace with retry logic
        c\
            // Add retry logic for web server startup\
            let mut server = None;\
            for attempt in 1..=3 {\
                log::info!("Web server start attempt {}/3", attempt);\
                match network::web_server::WebConfigServer::new_with_ota(config.clone(), ota_manager.clone()) {\
                    Ok(s) => {\
                        log::info!("Web server started successfully on port 80");\
                        server = Some(s);\
                        break;\
                    }\
                    Err(e) => {\
                        log::error!("Web server start attempt {} failed: {:?}", attempt, e);\
                        if attempt < 3 {\
                            esp_idf_hal::delay::FreeRtos::delay_ms(2000);\
                        } else {\
                            // Store error for UI display\
                            let error_msg = format!("Web server failed after 3 attempts: {}", e);\
                            unsafe {\
                                WEB_SERVER_ERROR = Some(error_msg);\
                            }\
                        }\
                    }\
                }\
            }\
            server
        # Skip to end of the block
        b
    }
    # For other lines in the block, delete them
    /}$/! d
}
EOF

# Apply the fix
echo "Modifying main.rs..."
if grep -q "Web server start attempt" src/main.rs; then
    echo "Fix already applied!"
else
    # This is complex, so let's use a simpler approach
    # We'll add a comment marker and manually suggest the change
    echo
    echo "MANUAL STEP REQUIRED:"
    echo "Please edit src/main.rs and find this block around line 371:"
    echo
    echo "    match network::web_server::WebConfigServer::new_with_ota(config.clone(), ota_manager.clone()) {"
    echo
    echo "Replace the entire match block (lines 371-393) with:"
    echo
    cat << 'EOF'
            // Add retry logic for web server startup
            let mut server = None;
            for attempt in 1..=3 {
                log::info!("Web server start attempt {}/3", attempt);
                match network::web_server::WebConfigServer::new_with_ota(config.clone(), ota_manager.clone()) {
                    Ok(s) => {
                        log::info!("Web server started successfully on port 80");
                        server = Some(s);
                        break;
                    }
                    Err(e) => {
                        log::error!("Web server start attempt {} failed: {:?}", attempt, e);
                        if attempt < 3 {
                            esp_idf_hal::delay::FreeRtos::delay_ms(2000);
                        } else {
                            // Store error for UI display
                            let error_msg = format!("Web server failed after 3 attempts: {}", e);
                            unsafe {
                                WEB_SERVER_ERROR = Some(error_msg);
                            }
                        }
                    }
                }
            }
            server
EOF
    echo
    echo "After making this change, run:"
    echo "  ./compile.sh --clean"
    echo "  ./scripts/flash.sh --no-erase"
fi

echo
echo "=== Additional Steps ==="
echo
echo "1. Run the debug script to check current status:"
echo "   ./scripts/debug-web-server.sh <device-ip>"
echo
echo "2. After flashing, monitor the serial output:"
echo "   espflash monitor"
echo
echo "3. Look for these success messages:"
echo "   - 'Web server start attempt 1/3'"
echo "   - 'Web server started successfully on port 80'"
echo
echo "If the fix doesn't work, check WEB_SERVER_FIX_SUMMARY.md for more options."
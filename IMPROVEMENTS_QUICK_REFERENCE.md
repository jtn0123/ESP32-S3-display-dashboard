# ESP32-S3 Dashboard - Quick Reference Guide

## 🚨 Critical Actions (Do First!)

### 1. Fix Partition Layout
```bash
# Choose one CSV and update all configs
# Current issue: Multiple CSVs with different offsets
```

### 2. Add OTA Authentication
```rust
// In OTA handler - 5 minute fix!
if req.headers().get("X-OTA-Token") != Some("your-secret") {
    return Err(StatusCode::Unauthorized);
}
```

### 3. Secure Telnet
```rust
#[cfg(not(feature = "debug"))]
const TELNET_ENABLED: bool = false;
```

## 📊 Current Status

### ✅ Completed (8 items)
- ESP_LCD DMA driver (55-65 FPS achieved!)
- Dirty rectangle tracking
- FPS counter accuracy
- Telnet server implementation
- OTA update functionality
- Dual-core architecture
- Real sensor monitoring
- Performance baseline capture

### 🔧 In Progress
- WiFi connection debugging
- Security hardening

### ⏳ Priority Queue (Top 10)
1. Unauthenticated OTA (CRITICAL)
2. Telnet without auth (CRITICAL)
3. Partition inconsistency (CRITICAL)
4. WiFi auto-reconnect (HIGH)
5. Health check endpoint (HIGH)
6. OTA rollback mechanism (HIGH)
7. Flash size verification (HIGH)
8. Rate limiting (MEDIUM)
9. mDNS support (LOW)
10. CI/CD pipeline (MEDIUM)

## 🛠️ Quick Fixes (<30 minutes each)

### Basic Auth for OTA
```bash
# Update scripts/ota.sh
curl -H "X-OTA-Token: your-secret" ...
```

### Health Endpoint Stub
```rust
HttpResponse::Ok().json(json!({
    "status": "ok",
    "version": DISPLAY_VERSION,
    "uptime_ms": get_time_ms()
}))
```

### Fix Partition Reference
```ini
# In sdkconfig.defaults.ota
CONFIG_PARTITION_TABLE_CUSTOM_FILENAME="partition_table/partitions_ota.csv"
```

## 📈 Performance Targets

| Metric | Current | Target | Status |
|--------|---------|--------|--------|
| Display FPS | 55-65 | 60+ | ✅ Achieved |
| Core 0 CPU | ~80% | <50% | 🔄 In Progress |
| Core 1 CPU | ~20% | 20-30% | ✅ On Track |
| Free Heap | 300KB | 300KB+ | ✅ Good |
| Web Response | Unknown | <100ms | 📊 Measure |

## 🗓️ Weekly Plan

### Week 1-2: Security Sprint
- [ ] OTA authentication
- [ ] SHA256 validation
- [ ] Telnet security
- [ ] Rate limiting

### Week 3-4: Stability Sprint  
- [ ] Health monitoring
- [ ] WiFi reconnection
- [ ] OTA rollback
- [ ] Crash recovery

### Week 5-6: Production Sprint
- [ ] CI/CD setup
- [ ] Documentation
- [ ] Performance tuning
- [ ] Testing suite

## 🎯 Definition of Done

### Security
- ✅ All endpoints authenticated
- ✅ Firmware validation implemented
- ✅ No hardcoded credentials

### Reliability
- ✅ Auto-recovery from failures
- ✅ 30+ day uptime achievable
- ✅ OTA rollback working

### Developer Experience
- ✅ CI/CD pipeline active
- ✅ <5 minute test cycle
- ✅ Comprehensive docs

## 📞 Get Help

- **Performance Issues**: Check dirty rect implementation
- **Network Issues**: Verify power management disabled
- **OTA Failures**: Check partition alignment
- **Build Issues**: Use espflash@3.3.0, not v4

## 🔗 Related Files

- `IMPROVEMENTS.md` - Full detailed roadmap
- `KNOWN_ISSUES.md` - Current bugs and workarounds
- `scripts/README.md` - Tool documentation
- `CLAUDE.md` - AI assistant context

---
*Use this guide for quick lookups. See IMPROVEMENTS.md for full details.*
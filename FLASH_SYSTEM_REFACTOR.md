# Flash System Refactor Plan

## Goal
Un-brick every board in one shot and keep day-to-day flashing painless.

## Implementation Steps

### 1. Create Scripts Folder Structure
```
scripts/
├── flash.sh           # USB flash - always works
├── ota.sh            # WiFi/HTTP OTA updates  
├── check-partition.sh # Diagnostic tool
└── README.md         # Scripts documentation
```

### 2. New flash.sh Features
- Always flashes to BOTH factory (0x20000) and ota_0 (0x1A0000)
- Initializes otadata to point to factory partition
- Optional `--no-erase` flag for faster development
- Converts ELF to binary automatically
- Clear success messages showing what was flashed

### 3. OTA Data Initial File
- Pre-generated binary that points to factory partition
- 8192 bytes with proper CRC32
- Stored in `firmware/ota_data_initial.bin`
- Never changes unless partition layout changes

### 4. Partition Layout (Fixed)
```
nvs,      data, nvs,     0x9000,   0x6000,   # 24KB
otadata,  data, ota,     0xf000,   0x2000,   # 8KB  
factory,  app,  factory, 0x20000,  0x180000, # 1.5MB
ota_0,    app,  ota_0,   0x1A0000, 0x180000, # 1.5MB
ota_1,    app,  ota_1,   0x320000, 0x180000, # 1.5MB
```

### 5. Workflow
| Task | Command |
|------|---------|
| Initial flash / un-brick | `./scripts/flash.sh` |
| Fast USB iteration | `./scripts/flash.sh --no-erase` |
| OTA update | `./scripts/ota.sh 192.168.1.100` |
| Check status | `./scripts/check-partition.sh` |

## Progress Tracking

- [x] Create scripts directory
- [x] Generate ota_data_initial.bin
- [x] Create new flash.sh
- [x] Move and update ota.sh
- [x] Update check-partition.sh
- [x] Delete old scripts
- [x] Update main README
- [ ] Test complete workflow

## Implementation Complete!

All scripts have been refactored and organized. The new system:
- `scripts/flash.sh` - Always flashes to both factory and ota_0
- `scripts/ota.sh` - Unchanged, works perfectly for wireless updates
- `scripts/check-partition.sh` - New diagnostic tool
- `firmware/ota_data_initial.bin` - Pre-generated, points to factory

### Next Step: Test
Run `./scripts/flash.sh` to un-brick any device!
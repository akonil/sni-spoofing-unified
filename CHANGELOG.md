# Changelog

All notable changes to SNI Spoof are documented here.

---

## [0.2.2] — 2026-04-18

### Documentation

- **README.md**: Reorganized into two-part structure
  - PART 1: For Regular Users (quick start, basic setup, simple troubleshooting)
  - PART 2: For Advanced Users (technical details, advanced features, monitoring)
  
- **SETUP.md**: Reorganized into two-part structure
  - PART 1: For Regular Users (30-second setup, binary download, basic verification)
  - PART 2: For Advanced Users (build from source, advanced configuration, systemd service)

### Improved

- Documentation now clearly separates beginner and expert guidance
- Both sections are understandable to all user types, not just their intended audience
- Better navigation between quick start and advanced features
- Added systemd service example for Linux production deployments
- Enhanced troubleshooting section with root cause analysis and advanced solutions

---

## [0.2.1] — 2026-04-17

### Fixed

- **Launcher Scripts**: Fixed flag handling in `run.sh` and `run.bat`
  - Scripts now properly detect `--wizard` and `--preset` flags
  - Flags are passed directly to binary without config file validation
  - Resolves "Config file not found" error when using interactive setup

### Updated

- **SETUP.md**: Added documentation for `--wizard` and `--preset` flag support in launcher scripts

---

## [0.2.0] — 2026-04-15

### Added

#### DPI Evasion Features

- **SNI Pool Rotation**: Rotate through multiple fake SNIs per connection
  - New `fake_sni_pool` config field (array of domains)
  - Backward compatible with legacy single `fake_sni` field
  
- **Payload Padding**: Add random extra bytes to fake ClientHello
  - `advanced.payload_padding.min_extra_bytes` and `max_extra_bytes`
  - Defeats fixed-size packet fingerprinting
  
- **Fragmentation**: Split fake ClientHello into 2-3 TCP segments
  - `advanced.fragmentation.enabled`, `fragments`, `delay_ms`
  - Confuses DPI reassembly logic
  
- **Rate Limiting**: Throttle incoming connections per listener
  - `max_connections_per_sec` field (0 = unlimited)
  - Token-bucket algorithm prevents connection floods
  
- **SNI Tracking**: Per-SNI success/failure counters
  - Logged every 60 seconds with top 10 SNIs by success
  - Helps identify which domains work best in your region

#### User-Friendly Features

- **`--wizard` Flag**: Interactive first-run setup
  - Prompts for upstream IP, listen port, SNI selection, DPI evasion mode
  - Generates `config.json` automatically
  
- **`--preset` Flag**: Pre-configured templates
  - `hcaptcha`: hCaptcha pool (6 SNIs), no fragmentation (recommended)
  - `cloudflare`: Cloudflare pool (4 SNIs), no fragmentation
  - `stealth`: hCaptcha pool + fragmentation + padding 0-128 bytes

#### Launcher Scripts

- **`run.sh`** (Linux/macOS): Automated build, permissions, and launch
  - Auto-detects flags and config files
  - Handles `sudo` permissions intelligently
  
- **`run.bat`** (Windows): Automated build and admin check
  - Auto-detects flags and config files
  - Validates Administrator privileges

### Updated

- **Cargo.toml**: Bumped version from 0.1.0 to 0.2.0
- **README.md**: Added comprehensive documentation for new DPI evasion features and user-friendly options
- **SETUP.md**: Added setup instructions for wizard, presets, and advanced configurations

### Technical Changes

- **src/config.rs**
  - New `PayloadPaddingConfig` and `FragmentationConfig` structs
  - New `AdvancedConfig` containing both above
  - `ListenerConfig`: changed `fake_sni` to `Option<String>`, added `fake_sni_pool` and `max_connections_per_sec`
  - Added `resolved_sni_pool()` method for backward compatibility
  - Enhanced validation for SNI length and fragment count

- **src/packet/tls.rs**
  - Modified `build_client_hello(sni, extra_padding)` to support variable-length payloads
  - TLS and Handshake length fields patched to account for padding
  - New `build_client_hello_padded()` helper with random padding
  - Fixed `parse_sni()` guard to work with padded ClientHellos

- **src/proto.rs**
  - `Registration.fake_payload` → `fake_payloads: Vec<Vec<u8>>` for fragment support
  - Added `fake_sni: String` for tracking which SNI was chosen
  - Added `fragment_delay_ms: u64` for inter-fragment delay
  - `SnifferResult::FakeConfirmed` now carries `{ sni: String }`

- **src/stats.rs**
  - New `SniEntry` struct with atomic success/failure counters
  - New `SniTracker` for thread-safe SNI statistics
  - `snapshot()` method returns sorted top SNIs

- **src/handler.rs**
  - SNI rotation logic: picks random SNI from pool
  - Fragment generation via `split_payload()` helper
  - Tracks SNI success/failure via `SniTracker`

- **src/sniffer/mod.rs**
  - Support for multiple fragments per connection
  - Fragment injection loop with inter-fragment delays
  - Correct sequence number arithmetic for fragmented payloads

- **src/listener.rs**
  - Token-bucket rate limiter per listener
  - Passes new parameters to `handle_connection()`

- **src/main.rs**
  - 60-second SNI statistics logging task
  - Wizard and preset support with flag handling

- **src/wizard.rs** (NEW)
  - Interactive configuration wizard
  - Named preset configs (hcaptcha, cloudflare, stealth)

---

## [0.1.0] — Initial Release

### Features

- Cross-platform SNI spoofing proxy (Linux, macOS, Windows)
- Inject fake TLS ClientHello with decoy SNI at wrong sequence number
- Transparent bidirectional data relay
- Configurable timeouts and jitter for DPI evasion
- Connection statistics and logging
- Gaming mode for latency-sensitive applications
- Debounced logging to prevent log flooding

### Requirements

- Linux: `sudo` or `CAP_NET_RAW` capability
- macOS: `sudo` (BPF device access)
- Windows: Administrator privileges (WinDivert)


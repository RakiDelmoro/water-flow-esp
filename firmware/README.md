# Water Flow ESP32 Firmware

A clean-architecture embedded Rust firmware for monitoring water flow using ESP32 and publishing data via MQTT.

## Features
- **Testable Design**: Core logic runs on host without hardware via dependency inversion
- **Separation of Concerns**: Business logic separated from hardware abstractions
- **Real-time Publishing**: Publishes flow measurements every second when MQTT connected
- **Robust Connectivity**: WiFi and MQTT auto-reconnection with state management
- **Multi-threaded**: WiFi and MQTT run in separate threads
- **Unit Test Coverage**: Comprehensive tests for each module with mocks

## Architecture

### Single Crate Structure
```
firmware/
├── Cargo.toml              (single package)
├── src/
│   ├── lib.rs             (orchestrator, module declarations)
│   ├── main.rs            (entry point)
│   ├── config.rs          (WiFi/MQTT constants)
│   ├── traits.rs          (trait abstractions)
│   ├── flow_monitor.rs    (core monitoring logic)
│   ├── wifi.rs            (WiFi adapter + step_wifi)
│   ├── mqtt.rs            (MQTT publisher + step_mqtt)
│   ├── sensor.rs          (FlowCounter + ISR setup)
│   └── time.rs            (TimeSource + Delay)
```

### Key Modules

| Module | Responsibility |
|--------|----------------|
| **config** | Configuration constants (WiFi, MQTT, pins) |
| **traits** | Abstractions: `TimeSource`, `FlowCounter`, `WifiAdapter`, `MqttPublisher`, `Delay` |
| **flow_monitor** | Core monitoring loop, JSON building, tick processing |
| **wifi** | WiFi connection management, `step_wifi()` for testable logic |
| **mqtt** | MQTT client lifecycle, `step_mqtt()` for testable logic |
| **sensor** | GPIO interrupt setup, pulse counting via `EspFlowCounter` |
| **time** | ESP timer and FreeRTOS delay wrappers |
| **lib** | Module declarations and `run()` orchestrator |
| **main** | Minimal entry point calling `firmware::run()` |

## Testing Strategy

### Unit Tests (Inside Each Module)

All tests are **unit tests** co-located with the code they test. They run on the host without requiring ESP32 hardware.

```bash
# Run all unit tests (host)
cargo test

# Run library tests only
cargo test --lib

# Build tests without running
cargo test --no-run
```

### Test Approach

- **Mocking**: Uses `mockall` to mock ESP-IDF dependencies
- **Testable design**: Infinite loops refactored into `step_*` functions for deterministic testing
- **Pure functions**: Core logic (e.g., `build_payload`, `EspFlowCounter::swap`) tested directly
- **Integration avoided**: Focus on fast, isolated unit tests; no integration tests in `tests/` directory

### Test Coverage

| Module | Coverage |
|--------|----------|
| `flow_monitor.rs` | `build_payload()` serialization, `process_tick()` with various states |
| `wifi.rs` | `step_wifi()`: connection flag updates, reconnection, error handling |
| `mqtt.rs` | `step_mqtt_manager()`: WiFi dependency, state clearing |
| `sensor.rs` | `EspFlowCounter::swap()`: atomic operations, reset behavior |
| `time.rs` | Platform-specific wrappers (ESP-only) |

### Example Test

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mockall::mock;

    mock! {
        TestWifi {}
        impl WifiAdapter for TestWifi {
            fn is_connected(&self) -> Result<bool>;
            fn connect(&mut self) -> Result<()>;
        }
    }

    #[test]
    fn test_step_wifi_sets_connected_flag_when_connected() {
        let mut wifi = MockTestWifi::new();
        let connected_flag = Arc::new(AtomicBool::new(false));

        wifi.expect_is_connected().returning(|| Ok(true));

        step_wifi(&mut wifi, &connected_flag).unwrap();

        assert!(connected_flag.load(Ordering::Relaxed));
    }
}
```

## Hardware Setup

### Connections
- **Flow Sensor**: Connect to GPIO pin specified in `config.rs` (default: GPIO4)
- **Power**: 3.3V or 5V depending on sensor (use appropriate level shifting)

### Configuration
Edit `src/config.rs` before building:

```rust
// WiFi credentials
pub const WIFI_SSID: &str = "your-ssid";
pub const WIFI_PASSWORD: &str = "your-password";

// MQTT broker
pub const MQTT_URL: &str = "mqtt://broker.example.com:1883";
pub const MQTT_USERNAME: &str = "username";
pub const MQTT_PASSWORD: &str = "password";
pub const MQTT_TOPIC: &str = "water/flow";

// Flow sensor GPIO pin
pub const FLOW_SENSOR_PIN: u8 = 4;
```

## Building & Flashing

### Prerequisites
- Rust toolchain with `esp` target: `rustup target add xtensa-esp32-espidf`
- ESP-IDF environment (automatic via `espflash`)
- Connected ESP32 device

### Build for ESP32
```bash
# Build library (host)
cargo build --lib

# Build firmware for ESP32
cargo build --release --target xtensa-esp32-espidf

# Or use cargo flash (requires espflash)
cargo flash --release --target xtensa-esp32-espidf
```

### Run Directly (Build + Flash)
```bash
cargo run --release --target xtensa-esp32-espidf
```

### Monitor Serial Output
```bash
cargo run --release --target xtensa-esp32-espidf -- --monitor
```

## Design Principles Applied

1. **Dependency Inversion**: High-level logic depends on trait abstractions, not concrete ESP types
2. **Single Responsibility**: Each module has one reason to change
3. **Testability**: Pure functions and extracted step functions enable host testing
4. **Zero-cost Abstractions**: Generics provide compile-time dispatch; no runtime overhead in release
5. **Separation of Concerns**: Hardware initialization isolated from business logic

## Future Improvements

- [ ] Add configuration validation tests
- [ ] Implement graceful shutdown on signal
- [ ] Add OTA update capability
- [ ] Support multiple flow sensors
- [ ] Add metrics and health checks
- [ ] Expansion of unit test coverage for edge cases

## License

[Your License Here]

## Acknowledgments

Built with [esp-idf-svc](https://github.com/esp-rs/esp-idf-svc) and the Rust embedded ecosystem.

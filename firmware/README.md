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
│   ├── main.rs            (ESP32 entry point)
│   ├── engine.rs          (core monitoring loop)
│   └── platform/          (platform abstractions & implementations)
│       ├── traits.rs      (trait abstractions)
│       ├── esp32/         (ESP32-specific implementations)
│       └── host/          (host mocks for testing)
```

### Key Modules

| Module | Responsibility |
|--------|----------------|
| **engine** | `FlowMonitor` - main monitoring loop with timing, interrupt handling, and conditional publishing |
| **platform::traits** | Abstractions: `PulseCounter`, `Clock`, `DataSink`, `ConnectionGuard`, `Delay` |
| **platform::esp32** | Concrete ESP32 implementations: `Esp32PulseCounter`, `Esp32Clock`, `MqttDataSink`, `Esp32ConnectionGuard`, `Esp32Delay`, `JsonPayloadBuilder`, `HardwarePayloadSampler`, `Esp32WifiManager`, `Esp32MqttManager` |
| **platform::host** | Mock implementations for host-based unit testing |
| **lib** | Exposes modules; provides `run()` orchestrator |
| **main** | Wires concrete ESP32 components and starts the system |

## Testing Strategy

### Unit Tests (Inside Each Module)

All tests are **unit tests** co-located with the code they test. They run on the host without requiring ESP32 hardware, using mock implementations from `platform/host/`.

```bash
# Run all unit tests (host)
cargo test

# Run library tests only
cargo test --lib

# Build tests without running
cargo test --no-run
```

### Test Approach

- **Host mocks**: The `platform/host/` module provides mock implementations of all traits (`HostPulseCounter`, `HostDataSink`, `HostConnectionGuard`, `HostClock`, `HostDelay`, etc.)
- **Deterministic testing**: The `HostClock` can be advanced manually; `HostPulseCounter` allows injecting pulse counts
- **Pure logic**: Core logic in `engine.rs` and `platform/host/` is pure Rust with no ESP-IDF dependencies
- **No hardware required**: All tests run on the host machine

### Test Coverage

| Module | Coverage |
|--------|----------|
| `platform::host::clock` | Deterministic time advancement and elapsed calculation |
| `platform::host::pulse_counter` | Pulse accumulation, reset, and thread-safe operations |
| `platform::host::connection` | WiFi/MQTT ready flag combination logic |
| `platform::host::mqtt` | MQTT manager behavior (waits for WiFi, slot population/clearing) |
| `platform::host::payload` | JSON payload format verification |
| `platform::host::sampler` | FIFO draining of samples |
| `platform::host::sink` | Send recording and failure injection/recovery |

### Example Test

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_publish_sequence() {
        let mut sink = HostDataSink::new();
        let sample = PayloadSample { pulse_delta: 5, time_delta_ms: 1000, accumulative_pulse: 10 };

        sink.send(&sample).unwrap();
        assert_eq!(sink.sent_count(), 1);
        assert_eq!(sink.total_pulses_sent(), 5);
    }
}
```

## Hardware Setup

### Connections
- **Flow Sensor**: Connect to GPIO pin specified by `FLOW_SENSOR_PIN` (default: GPIO4)
- **Power**: 3.3V or 5V depending on sensor (use appropriate level shifting)

### Configuration

Configuration is provided via environment variables. A template is available in `.env.example`.

**Required:**
- `WIFI_SSID` - WiFi network name
- `WIFI_PASS` - WiFi password
- `MQTT_BROKER_URL` - MQTT broker URL (e.g., `mqtt://broker.example.com:1883`)
- `MQTT_CLIENT_ID` - Unique client ID for MQTT connection

**Optional (with defaults):**
- `MQTT_TOPIC` - MQTT topic to publish to (default: `water/flow`)
- `DEVICE_ID` - Device identifier in payload (default: `esp32-flow`)
- `FLOW_SENSOR_PIN` - GPIO pin number for flow sensor (default: `4`)
- `MQTT_USERNAME` - MQTT username (if broker requires authentication)
- `MQTT_PASSWORD` - MQTT password (if broker requires authentication)

Example build with cargo:

```bash
WIFI_SSID="my-network" \
WIFI_PASS="my-password" \
MQTT_BROKER_URL="mqtt://broker.local:1883" \
MQTT_CLIENT_ID="esp32-001" \
cargo build --release --target xtensa-esp32-espidf
```

You can also set optional variables:

```bash
WIFI_SSID="my-network" \
WIFI_PASS="my-password" \
MQTT_BROKER_URL="mqtt://broker.local:1883" \
MQTT_CLIENT_ID="esp32-001" \
MQTT_TOPIC="water/flow/custom" \
DEVICE_ID="sensor-01" \
FLOW_SENSOR_PIN=5 \
cargo build --release --target xtensa-esp32-espidf
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
- [ ] Support multiple flow sensors (or configurable GPIO pin)
- [ ] Add metrics and health checks
- [ ] Add unit tests for `engine::FlowMonitor` with host mocks
- [ ] Expand edge case coverage in existing tests

## License

[Your License Here]

## Acknowledgments

Built with [esp-idf-svc](https://github.com/esp-rs/esp-idf-svc) and the Rust embedded ecosystem.

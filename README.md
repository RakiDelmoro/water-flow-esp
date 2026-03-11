# Water Flow Sensor for ESP32

An event-driven water flow monitoring system built for ESP32 microcontrollers. Measures water flow using a Hall-effect sensor and publishes data via MQTT with efficient power usage for self-powered applications.

## Features

- **Event-Driven Architecture**: No busy-waiting, efficient power usage with hardware timer-based sampling
- **Non-Blocking Sensor Reading**: Hardware interrupt counts pulses automatically
- **Self-Powered Design**: Optimized for devices powered by water flow (resets when water stops, fresh start when flows)
- **Fast-Fail Error Handling**: No complex retry logic - device resets with water flow
- **MQTT Publishing**: Publishes flow data every second via MQTT with QoS 1
- **Clean Modular Design**: Separated concerns across modules for maintainability

## Architecture

```
Water Flow → Hall Sensor → GPIO Interrupt → PULSE_COUNT (atomic)
                                              │
                                   1s Hardware Timer
                                              │
                                   TimerTick Event
                                              │
                                   Sample & Publish via MQTT
```

## Hardware Requirements

- **ESP32** microcontroller (ESP32-WROOM-32 or similar)
- **Hall-effect flow sensor** (e.g., YF-S201, FS300A)
- **WiFi network** for MQTT connectivity
- **MQTT broker** (local or cloud)

### Wiring

| Component | ESP32 Pin |
|-----------|-----------|
| Flow Sensor VCC | 3.3V or 5V |
| Flow Sensor GND | GND |
| Flow Sensor Signal | GPIO 25 |

**Note**: GPIO pin is currently hardcoded to 25 in `main.rs`. The `FLOW_SENSOR_GPIO` constant in `main_config.rs` is for documentation only.

## Software Architecture

### Modules

| File | Purpose |
|------|---------|
| `main.rs` | Entry point, initializes all modules, spawns threads |
| `sensor.rs` | GPIO interrupt setup and pulse counting |
| `timer.rs` | Hardware timer (1-second intervals) |
| `wifi_manager.rs` | WiFi connection management |
| `mqtt_manager.rs` | MQTT client connection |
| `app.rs` | Main event loop and publishing logic |
| `main_config.rs` | Configuration constants |

### Thread Model

1. **Main Thread**: Event loop blocks on `event_rx.recv()`, processes events
2. **WiFi Thread**: Monitors WiFi state, sends `WifiState` events
3. **MQTT Thread**: Manages MQTT connection, sends `MqttState` events
4. **Timer ISR**: Hardware interrupt sends `TimerTick` events every 1 second
5. **GPIO ISR**: Counts pulses on each sensor edge

### Events

```rust
enum AppEvent {
    TimerTick,        // Every 1 second
    WifiState(bool),  // WiFi connected/disconnected
    MqttState(bool),  // MQTT connected/disconnected
}
```

## Configuration

Edit `src/main_config.rs`:

```rust
// WiFi
pub const WIFI_SSID: &str = "your-wifi-ssid";
pub const WIFI_PASSWORD: &str = "your-wifi-password";

// MQTT
pub const MQTT_TOPIC: &str = "water/flow";
pub const MQTT_USERNAME: &str = "mqtt-user";
pub const MQTT_PASSWORD: &str = "mqtt-pass";
pub const MQTT_URL: &str = "mqtt://broker-ip:1883";

## Building

### Prerequisites

- [Rust](https://rustup.rs/) (1.77 or later)
- [ESP-IDF](https://docs.espressif.com/projects/esp-idf/en/latest/esp32/get-started/index.html)
- [espflash](https://github.com/esp-rs/espflash): `cargo install espflash`

### Build

```bash
# Development build
cargo build

# Release build (optimized for size)
cargo build --release
```

### Flash to ESP32

```bash
# Flash and monitor
cargo espflash flash --monitor

# Or manually:
espflash /dev/ttyUSB0 target/xtensa-esp32-espidf/release/water-flow-esp
```

Replace `/dev/ttyUSB0` with your ESP32's serial port (e.g., `/dev/ttyACM0` on Linux, `COM3` on Windows).

## MQTT Payload Format

Published every second when connected:

```json
{
  "pulse_delta": 42,
  "time_ms": 1000,
  "accumulative_pulses": 1500
}
```

- `pulse_delta`: Pulses since last successful publish
- `time_ms`: Time elapsed since last publish (ms)
- `accumulative_pulses`: Total pulses at start of this interval

## Power Characteristics

- **Sleep Behavior**: Main thread sleeps between events (`event_rx.recv()` blocks)
- **No Busy-Waiting**: CPU yields when no events pending
- **Self-Powered**: Device resets when water stops (power loss), fresh state on restart
- **Interrupt-Driven**: Sensor counting uses hardware interrupts (no polling)

## Error Handling

- **Fast-Fail**: No retry delays; errors are logged and operation continues
- **Buffering**: Pulses accumulate in global counter during network outages
- **Reset Recovery**: Full state reset on power loss (when water stops)

## Development

### Running Tests

```bash
# Check compilation
cargo check

# Clippy (linting)
cargo clippy

# Format code
cargo fmt
```

### Project Structure

```
water-flow-esp/
├── src/
│   ├── main.rs          # Entry point
│   ├── main_config.rs   # Configuration
│   ├── app.rs           # Event loop & publishing
│   ├── sensor.rs        # GPIO & pulse counting
│   ├── timer.rs         # Hardware timer
│   ├── wifi_manager.rs  # WiFi connection
│   └── mqtt_manager.rs  # MQTT connection
├── Cargo.toml
├── build.rs             # ESP-IDF build script
└── sdkconfig.defaults   # ESP-IDF config
```

## Troubleshooting

### WiFi Connection Fails
- Check SSID and password in `main_config.rs`
- Ensure WiFi network is 2.4GHz (ESP32 doesn't support 5GHz)
- Check signal strength near the device

### MQTT Connection Fails
- Verify broker URL and port (default: 1883)
- Check username/password if broker requires authentication
- Ensure broker is accessible from ESP32's network

### No Flow Data
- Check sensor wiring (VCC, GND, Signal to GPIO 25)
- Verify sensor outputs pulses (test with multimeter)
- Check logs: `pulse_delta` should be > 0 when water flows

### High Memory Usage
- Reduce thread stack sizes in `main.rs` (currently 4096 bytes each)
- Remove unused logging with `RUST_LOG=warn` or `RUST_LOG=error`

## License

MIT License - See LICENSE file for details

## Contributing

Contributions welcome! Please ensure:
- Code passes `cargo clippy`
- Code is formatted with `cargo fmt`
- No new warnings introduced

## Acknowledgments

- Built with [esp-idf-svc](https://github.com/esp-rs/esp-idf-svc) for ESP-IDF integration
- Uses [Rust ESP32 ecosystem](https://esp-rs.github.io/)

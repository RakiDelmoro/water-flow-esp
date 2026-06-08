# Water Flow ESP32

Water flow monitoring for ESP32. Reads a Hall-effect flow sensor via GPIO interrupt and publishes pulse data to an MQTT broker every second.

Designed for self-powered operation — the device is powered by water flow itself, so it boots fresh each time water starts and shuts down when it stops.

## Hardware

| Component | ESP32 Pin |
|-----------|-----------|
| Flow Sensor VCC | 3.3V or 5V |
| Flow Sensor GND | GND |
| Flow Sensor Signal | GPIO 25 |

## Configuration

Edit `src/main_config.rs` before building:

```rust
pub const WIFI_SSID: &str = "your-wifi";
pub const WIFI_PASSWORD: &str = "your-password";
pub const STATIC_IP: [u8; 4] = [192, 168, 1, 100];  // your static IP
pub const GATEWAY: [u8; 4] = [192, 168, 1, 1];       // your gateway
pub const NETMASK: u8 = 24;                           // CIDR prefix
pub const MQTT_URL: &str = "mqtt://broker-ip:1883";
pub const MQTT_USERNAME: &str = "user";
pub const MQTT_PASSWORD: &str = "pass";
pub const MQTT_TOPIC: &str = "esp/water-flow";
```

## Building

Builds are done inside the dev container. Flashing is done from the host machine using [espflash](https://github.com/esp-rs/espflash) standalone (not via cargo).

### Debug Build

```bash
cargo build
```

Output: `target/xtensa-esp32-espidf/debug/water-flow-esp`

### Release Build (optimized for size)

```bash
cargo build --release
```

Output: `target/xtensa-esp32-espidf/release/water-flow-esp`

## Flashing

From the host machine (outside the container), using espflash standalone:

```bash
# Flash debug build
espflash flash target/xtensa-esp32-espidf/debug/water-flow-esp --monitor

# Flash release build
espflash flash target/xtensa-esp32-espidf/release/water-flow-esp --monitor
```

Replace the serial port if needed (espflash auto-detects on most systems).

## MQTT Payload

Published every second when connected:

**Normal (1-second window):**
```json
{
  "total_pulses": 12,
  "Time_ms": 1000,
  "accumulative_pulses": 156
}
```

**First connect (accumulated data since boot):**
```json
{
  "total_pulses": 45,
  "Time_ms": 3000,
  "accumulative_pulses": 45,
  "first_connect": true
}
```

- `total_pulses` — pulses since last publish
- `Time_ms` — milliseconds since last publish
- `accumulative_pulses` — total pulse count
- `first_connect` — present only on the first publish after boot or reconnect

## Architecture

```
  MAIN THREAD                          CONNECTION THREAD
  ───────────                          ──────────────────
  GPIO ISR counts pulses               WiFi connect (static IP)
  Blocks on FreeRTOS queue             MQTT connect
  Publishes every 1s                   Monitor + auto-reconnect
```

- **ISR** increments an atomic counter on every pulse and signals a FreeRTOS queue
- **Main thread** blocks on the queue with 1s timeout — no polling, no busy loop
- **Connection thread** handles WiFi + MQTT in the background with reconnection logic
- Pulses are counted from boot regardless of WiFi/MQTT state — no data lost

## Troubleshooting

| Problem | Check |
|---------|-------|
| WiFi won't connect | SSID/password, 2.4GHz only, signal strength |
| MQTT won't connect | Broker URL/port, credentials, network reachability |
| No flow data | Sensor wiring to GPIO 25, sensor outputs pulses |
| Boot loop | Check serial monitor for panic messages |

use crate::traits::{Delay, FlowCounter, MqttPublisher, TimeSource};
use anyhow::Result;
use log::error;
use serde_json::json;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;

/// Build JSON payload from flow measurement
pub fn build_payload(pulses: u32, timestamp: u64) -> serde_json::Result<Vec<u8>> {
    serde_json::to_vec(&json!({"time_it_takes": timestamp, "pulses": pulses}))
}

/// Process a single tick of the flow monitor
///
/// Called from the main loop to handle one measurement cycle.
pub fn process_tick<C, T, P>(counter: &C, time: &T, mqtt_client: &mut P, mqtt_connected: bool, mqtt_topic: &str, qos: i8) -> Result<()>
where C: FlowCounter, T: TimeSource, P: MqttPublisher {
    if !mqtt_connected { return Ok(()); }

    let pulse_count = counter.swap(true);
    let timestamp = time.now_millis();

    if pulse_count == 0 { return Ok(());}

    let payload = build_payload(pulse_count, timestamp)?;
    mqtt_client.publish(mqtt_topic, &payload, qos, false)?;
    Ok(())
}

/// Core flow monitoring loop - generic over dependencies
pub fn runner<C, T, D, P>(counter: C, time: T, delay: D, mqtt_client: Arc<Mutex<Option<P>>>, mqtt_connected: Arc<AtomicBool>, mqtt_topic: &str, qos: i8) -> Result<()>
where C: FlowCounter, T: TimeSource, D: Delay, P: MqttPublisher {
    loop {
        delay.delay_ms(1000);

        let connected = mqtt_connected.load(Ordering::Relaxed);
        let mut client_guard = match mqtt_client.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                error!("MQTT client mutex poisoned");
                std::mem::drop(poisoned.into_inner());
                continue;
            }
        };

        if let Some(client) = client_guard.as_mut() {
            if let Err(e) = process_tick(&counter, &time, client, connected, mqtt_topic, qos) {
                error!("Tick error: {:?}", e);
            }
        } else {
            error!("MQTT client not available despite connected flag");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn test_build_payload() {
        let pulses = 42;
        let timestamp = 123456789;
        let payload = build_payload(pulses, timestamp).unwrap();
        let json_str = String::from_utf8(payload).unwrap();
        let value: Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(value["pulses"], pulses);
        assert_eq!(value["time_it_takes"], timestamp);
    }

    #[test]
    fn test_build_payload_serialization() {
        let payload = build_payload(0, 0).unwrap();
        assert!(!payload.is_empty());
    }

    #[cfg(test)]
    mod mock_tests {
        use super::*;
        use anyhow::Result;
        use mockall::mock;

        mock! {
            FlowCounter {}
            impl FlowCounter for FlowCounter {
                fn swap(&self, reset: bool) -> u32;
            }
        }

        mock! {
            TimeSource {}
            impl TimeSource for TimeSource {
                fn now_millis(&self) -> u64;
            }
        }

        mock! {
            MqttPublisher {}
            impl MqttPublisher for MqttPublisher {
                fn publish(&mut self, topic: &str, data: &[u8], qos: i8, retain: bool) -> Result<()>;
            }
        }

        #[test]
        fn publishes_when_connected_pulses() {
            let mut counter = MockFlowCounter::new();
            let mut time = MockTimeSource::new();
            let mut publisher = MockMqttPublisher::new();
            let topic = "test/topic";
            let qos = 0;

            counter.expect_swap().with(mockall::predicate::eq(true)).return_const(5u32);
            time.expect_now_millis().return_const(1000u64);
            let expected_payload = build_payload(5, 1000).unwrap();
            publisher.expect_publish().with(
                    mockall::predicate::eq(topic),
                    mockall::predicate::eq(expected_payload),
                    mockall::predicate::eq(qos),
                    mockall::predicate::eq(false),
                ).return_once(|_, _, _, _| Ok(()));

            let result = process_tick(&counter, &time, &mut publisher, true, topic, qos);

            assert!(result.is_ok());
        }

        #[test]
        fn publish_when_not_connected() {
            let mut counter = MockFlowCounter::new();
            let mut time = MockTimeSource::new();
            let mut publisher = MockMqttPublisher::new();

            counter.expect_swap().never();
            time.expect_now_millis().never();
            publisher.expect_publish().never();

            let result = process_tick(&counter, &time, &mut publisher, false, "test/topic", 0);

            assert!(result.is_ok());
        }

        #[test]
        fn publish_when_zero_pulses() {
            let mut counter = MockFlowCounter::new();
            let mut time = MockTimeSource::new();
            let mut publisher = MockMqttPublisher::new();

            counter.expect_swap().return_const(0u32);
            time.expect_now_millis().return_const(1000u64);
            publisher.expect_publish().never();

            let result = process_tick(&counter, &time, &mut publisher, true, "test/topic", 0);

            assert!(result.is_ok());
        }
    }
}

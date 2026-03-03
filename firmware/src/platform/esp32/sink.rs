use crate::platform::traits::{DataSink, MqttPublisher, PayloadBuilder, PayloadSample};
use std::sync::{Arc, Mutex};

pub struct MqttDataSink<P: MqttPublisher, B: PayloadBuilder> {
    client_slot: Arc<Mutex<Option<P>>>,
    builder: B,
    topic: String,
}

impl<P: MqttPublisher, B: PayloadBuilder> MqttDataSink<P, B> {
    pub fn new(client_slot: Arc<Mutex<Option<P>>>, builder: B, topic: String) -> Self {
        Self {
            client_slot,
            builder,
            topic,
        }
    }
}

impl<P: MqttPublisher, B: PayloadBuilder> DataSink for MqttDataSink<P, B> {
    fn send(&mut self, sample: &PayloadSample) -> anyhow::Result<()> {
        self.builder
            .build(
                sample.pulse_delta,
                sample.time_delta_ms,
                sample.accumulative_pulse,
            )
            .and_then(|payload| {
                self.client_slot
                    .lock()
                    .unwrap()
                    .as_mut()
                    .ok_or_else(|| anyhow::anyhow!("MQTT client not ready"))
                    .and_then(|c| c.publish(self.topic, false, &payload))
            })
    }
}

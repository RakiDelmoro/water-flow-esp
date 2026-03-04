use crate::platform::traits::PayloadBuilder;

pub struct JsonPayloadBuilder {
    pub device_id: String,
}

impl PayloadBuilder for JsonPayloadBuilder {
    fn build(
        &self,
        pulse_delta: u32,
        time_delta_ms: u64,
        accumulative_pulse: u32,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(format!(
            r#"{{"device_id":"{}","pulse_delta":{},"time_delta_ms":{},"accumulative_pulse":{}}}"#,
            self.device_id, pulse_delta, time_delta_ms, accumulative_pulse
        )
        .into_bytes())
    }
}

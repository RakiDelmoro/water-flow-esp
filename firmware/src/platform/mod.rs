pub mod traits;

#[cfg(target_os = "espidf")]
pub mod esp32;

#[cfg(not(target_os = "espidf"))]
pub mod host;

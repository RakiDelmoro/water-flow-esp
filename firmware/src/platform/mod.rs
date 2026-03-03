pub mod traits;

#[cfg(target_os = "espidf")]
pub mod hardware;

#[cfg(not(target_os = "espidf"))]
pub mod host;

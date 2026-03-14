#[cfg(target_os = "espidf")]
fn main() -> anyhow::Result<()> {
    firmware::run()
}
#[cfg(not(target_os = "espidf"))]
fn main() {
    // Host builds don't need a main; they only run unit tests
    println!("This firmware only runs on ESP32. Use `cargo test` for unit tests.");
}
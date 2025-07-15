fn main() -> anyhow::Result<()> {
    // Necessary for ESP-IDF
    embuild::espidf::sysenv::output();
    Ok(())
}
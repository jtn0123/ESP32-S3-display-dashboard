fn main() -> anyhow::Result<()> {
    // Necessary for ESP-IDF
    embuild::espidf::sysenv::output();
    
    // Add crash log helper for better panic diagnostics
    println!("cargo:rustc-link-arg=-Wl,--undefined=esp_backtrace_print_app_description");
    
    Ok(())
}
use std::env;

fn main() {
    // Necessary for esp-idf-sys
    if env::var("CARGO_FEATURE_STD").is_ok() {
        embuild::espidf::sysenv::output();
    }
}
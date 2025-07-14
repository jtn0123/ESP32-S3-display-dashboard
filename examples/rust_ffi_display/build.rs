// build.rs - Compile C display driver for FFI

use std::env;
use std::path::PathBuf;

fn main() {
    // Tell cargo to rerun if C files change
    println!("cargo:rerun-if-changed=src/display_driver.c");
    println!("cargo:rerun-if-changed=display_ffi.h");
    
    // Get ESP-IDF paths from environment
    let idf_path = env::var("IDF_PATH").expect("IDF_PATH not set");
    
    // Compile the C display driver
    cc::Build::new()
        .file("src/display_driver.c")
        .include(&idf_path)
        .include(&format!("{}/components/freertos/include", idf_path))
        .include(&format!("{}/components/driver/include", idf_path))
        .include(&format!("{}/components/soc/esp32s3/include", idf_path))
        .include(&format!("{}/components/hal/include", idf_path))
        .include(&format!("{}/components/esp_common/include", idf_path))
        .flag("-mlongcalls")
        .flag("-Wno-frame-address")
        .compile("display_driver");
    
    // Tell rustc where to find the library
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    println!("cargo:rustc-link-search=native={}", out_path.display());
    println!("cargo:rustc-link-lib=static=display_driver");
}
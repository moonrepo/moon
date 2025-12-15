fn main() {
    // Required for Rust v1.91+ builds to pass
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-lib=framework=CoreServices");
        println!("cargo:rustc-link-lib=framework=AppKit");
    }
}

use std::env;

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    // FreeBSD and MidnightBSD store X11 and Vulkan libraries in /usr/local/lib
    if target_os == "freebsd" || target_os == "midnightbsd" {
        println!("cargo:rustc-link-search=native=/usr/local/lib");
    }

    if target_os == "linux" || target_os == "freebsd" || target_os == "midnightbsd" {
        println!("cargo:rustc-link-lib=X11");
    }

    println!("cargo:rerun-if-changed=build.rs");
}

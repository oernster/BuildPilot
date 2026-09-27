//! Build script: compiles the Slint UI, hands the version from VERSION to the code so no version
//! literal lives in the source (CON-005), puts the icon and the version details on every
//! executable (INST-006); it also tells the setup program where its payload is.

use std::env;
use std::fs;

/// What Windows shows as the executables' name and description.
const PRODUCT_NAME: &str = "BuildPilot";
/// The icon made from assets/application-icon.png by tools/genicons.py.
const ICON: &str = "assets/application-icon.ico";
/// Names the built application for the setup program to carry; set by build.ps1.
const PAYLOAD_VARIABLE: &str = "BUILDPILOT_PAYLOAD";
/// Set when the setup program is built with a payload.
const PAYLOAD_CFG: &str = "buildpilot_payload";

fn main() {
    println!("cargo:rerun-if-changed=VERSION");
    println!("cargo:rerun-if-changed={ICON}");
    let version = fs::read_to_string("VERSION").unwrap_or_else(|_| "0.0.0-dev".to_owned());
    println!("cargo:rustc-env=BUILDPILOT_VERSION={}", version.trim());
    slint_build::compile("ui/main.slint").expect("the Slint UI compiles");

    println!("cargo:rerun-if-env-changed={PAYLOAD_VARIABLE}");
    println!("cargo::rustc-check-cfg=cfg({PAYLOAD_CFG})");
    if let Some(payload) = env::var_os(PAYLOAD_VARIABLE) {
        println!("cargo:rustc-cfg={PAYLOAD_CFG}");
        println!(
            "cargo:rustc-env={PAYLOAD_VARIABLE}={}",
            payload.to_string_lossy()
        );
        println!("cargo:rerun-if-changed={}", payload.to_string_lossy());
    }

    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_icon(ICON)
            .set("ProductName", PRODUCT_NAME)
            .set("FileDescription", PRODUCT_NAME)
            .compile()
            .expect("the icon and version details compile");
    }
}

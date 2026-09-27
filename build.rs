//! Build script: compiles the Slint UI and hands the version from VERSION to the code, so no
//! version literal lives in the source (CON-005).

use std::fs;

fn main() {
    println!("cargo:rerun-if-changed=VERSION");
    let version = fs::read_to_string("VERSION").unwrap_or_else(|_| "0.0.0-dev".to_owned());
    println!("cargo:rustc-env=BUILDPILOT_VERSION={}", version.trim());
    slint_build::compile("ui/main.slint").expect("the Slint UI compiles");
}

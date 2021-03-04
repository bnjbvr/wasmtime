use std::env;

fn main() {
    println!("cargo:rerun-if-changed=src/helpers.c");

    let mut build = cc::Build::new();

    build
        .warnings(true)
        .define(
            &format!("CFG_TARGET_OS_{}", env::var("CARGO_CFG_TARGET_OS").unwrap()),
            None,
        )
        .file("src/helpers.c");

    if env::var("CARGO_CFG_TARGET_OS").unwrap() == "macos"
        && env::var("CARGO_CFG_TARGET_ARCH").unwrap() == "aarch64"
    {
        println!("cargo:rerun-if-changed=src/traphandlers/macos_aarch64.S");
        build.file("src/traphandlers/macos_aarch64.S");
    }

    build.compile("helpers");
}

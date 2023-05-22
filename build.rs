use std::env;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=go-lib/lassie.go");

    let out_dir = env::var("OUT_DIR").unwrap();
    eprintln!("Building {out_dir}/libgolassie.a");

    let status = Command::new("go")
        .current_dir("go-lib")
        .args([
            "build",
            "-o",
            &format!("{}/libgolassie.a", out_dir),
            "-buildmode=c-archive",
            "lassie-ffi.go",
        ])
        .status()
        .unwrap();
    assert!(status.success());

    println!("cargo:rustc-link-search=native={}", out_dir);
}

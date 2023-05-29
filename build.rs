use std::env;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=go-lib/lassie.go");

    let out_dir = env::var("OUT_DIR").unwrap();

    #[cfg(not(target_os = "windows"))]
    let out_file = &format!("{out_dir}/libgolassie.a");
    //On windows platforms it's a `.lib` and there's no leading `lib`
    #[cfg(target_os = "windows")]
    let out_file = &format!("{out_dir}/golassie.lib");

    eprintln!("Building {out_file}");

    let status = Command::new("go")
        .current_dir("go-lib")
        .args([
            "build",
            "-tags",
            "netgo",
            "-o",
            out_file,
            "-buildmode=c-archive",
            "lassie-ffi.go",
        ])
        .status()
        .unwrap();
    assert!(status.success(), "`go build` failed");

    println!("cargo:rustc-link-search=native={}", out_dir);
    println!("cargo:rustc-link-lib=static={}", "golassie");

    let status = Command::new("go")
        .current_dir("go-lib")
        .args([
            "tool",
            "cgo",
            "-exportheader",
            "../target/lassie-ffi.h",
            "lassie-ffi.go",
        ])
        .status()
        .unwrap();
    assert!(status.success(), "`cgo -exportheader` failed");

    add_platform_specific_link_flags();
}

#[cfg(target_os = "macos")]
fn add_platform_specific_link_flags() {
    // See https://github.com/golang/go/issues/11258
    println!("cargo:rustc-link-arg=-framework");
    println!("cargo:rustc-link-arg=CoreFoundation");
    println!("cargo:rustc-link-arg=-framework");
    println!("cargo:rustc-link-arg=Security");
    // See https://github.com/golang/go/issues/58159
    // println!("cargo:rustc-link-lib=resolv");
    // ^^ Replaced with `-tags netgo`
}

#[cfg(not(target_os = "macos"))]
fn add_platform_specific_link_flags() {
    // no-op
}

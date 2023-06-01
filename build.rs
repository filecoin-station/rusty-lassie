use std::env;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=go-lib/lassie.go");

    build_lassie();
}

#[cfg(not(all(target_os = "windows", target_env = "msvc")))]
fn build_lassie() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_file = &format!("{out_dir}/libgolassie.a");

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
    // println!("cargo:rustc-link-lib=static={}", "golassie");

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

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn add_platform_specific_link_flags() {
    // no-op
}

#[cfg(all(target_os = "windows", target_env = "msvc"))]
fn build_lassie() {
    let out_dir = env::var("OUT_DIR").unwrap();

    //On windows platforms it's a `.dll` and there's no leading `lib`
    let out_file = format!("{out_dir}\\golassie.dll");
    eprintln!("Building {out_file}");

    let status = Command::new("go")
        .current_dir("go-lib")
        .args([
            "build",
            "-tags",
            "netgo",
            "-o",
            &out_file,
            "-buildmode=c-shared",
            "lassie-ffi.go",
        ])
        .status()
        .unwrap();
    assert!(status.success(), "`go build` failed");

    eprintln!("Building {out_file}.lib");

    let def_file = format!("{out_dir}\\golassie.def");
    std::fs::copy("go-lib\\golassie.def", &def_file).expect("cannot copy golassie.def to OUTDIR");
    println!("cargo:rerun-if-changed=go-lib/golassie.def");

    // let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    // assert_eq!(
    //     target_arch, "x86_64",
    //     "Unsupported Windows architecture: {target_arch}"
    // );

    let mut lib_cmd = cc::windows_registry::find(&env::var("TARGET").unwrap(), "lib.exe")
        .expect("cannot find the path to MSVC link.exe");

    let status = lib_cmd
        .args([
            format!("/def:{def_file}"),
            format!("/out:{out_file}.lib"),
            // "/machine:X64".to_string(),
        ])
        .status()
        .unwrap();
    assert!(status.success(), "`link.exe` failed");

    println!("cargo:rustc-link-search=native={}", out_dir);
    // println!("cargo:rustc-link-lib={}", "golassie.dll");
}

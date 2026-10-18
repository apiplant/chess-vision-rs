// The prebuilt ONNX Runtime for macOS calls `__isPlatformVersionAtLeast` (CoreML
// availability checks), which lives in clang's compiler-rt, not in libSystem.
// Rust's linker invocation does not pull that runtime in, so link it explicitly.
fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }
    let out = std::process::Command::new("xcrun").args(["clang", "--print-resource-dir"]).output();
    if let Ok(out) = out {
        if out.status.success() {
            let dir = String::from_utf8_lossy(&out.stdout).trim().to_string();
            println!("cargo:rustc-link-search=native={dir}/lib/darwin");
            println!("cargo:rustc-link-lib=static=clang_rt.osx");
        }
    }
    println!("cargo:rerun-if-changed=build.rs");
}

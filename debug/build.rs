#[path = "../tools/windows_resource.rs"] mod windows_resource;
fn main() {
    println!("cargo:rerun-if-changed=../linux/debug/wsl_clock.c");
    println!("cargo:rerun-if-changed=../linux/debug/wsl_audio.c");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu") {
        let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("wsl-clock.so");
        let cc = std::env::var_os("CC").unwrap_or_else(|| "cc".into());
        assert!(std::process::Command::new(cc)
            .args(["-shared", "-fPIC", "-O2", "-Wall", "-Wextra", "-Werror", "-pthread",
                "../linux/debug/wsl_clock.c", "-ldl", "-o"])
            .arg(output).status().expect("compile WSL clock compatibility library").success());
        let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("wsl-audio.so");
        let cc = std::env::var_os("CC").unwrap_or_else(|| "cc".into());
        assert!(std::process::Command::new(cc)
            .args(["-shared", "-fPIC", "-O2", "-Wall", "-Wextra", "-Werror", "-pthread",
                "../linux/debug/wsl_audio.c", "-ldl", "-o"])
            .arg(output).status().expect("compile WSL audio adapter").success());
    }
    println!("cargo:rerun-if-changed=../windows/debug/app.rc");
    println!("cargo:rerun-if-changed=../GUI/Windows/assets/app.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows")
        || std::env::var_os("CARGO_FEATURE_DESK").is_none() { return; }
    let directory=std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../windows/debug");
    windows_resource::compile(&directory,"app.rc",None,"a865r-debug");
}

fn main() {
    println!("cargo:rerun-if-changed=app.rc");
    println!("cargo:rerun-if-changed=../player/assets/app.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows")
        || std::env::var_os("CARGO_FEATURE_DESK").is_none() { return; }
    let manifest = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("app.res");
    let rc = std::env::var_os("RC").map(std::path::PathBuf::from).unwrap_or_else(|| {
        let kits = std::path::PathBuf::from(std::env::var_os("ProgramFiles(x86)").expect("Windows SDK required"))
            .join("Windows Kits/10/bin");
        let mut candidates: Vec<_> = std::fs::read_dir(kits).expect("Windows SDK required").flatten()
            .map(|e| e.path().join("x64/rc.exe")).filter(|p| p.exists()).collect();
        candidates.sort(); candidates.pop().expect("rc.exe required")
    });
    assert!(std::process::Command::new(rc).current_dir(manifest).arg("/nologo")
        .arg("/fo").arg(&output).arg("app.rc").status().expect("compile icon resource").success());
    println!("cargo:rustc-link-arg-bin=a865r-debug={}", output.display());
}

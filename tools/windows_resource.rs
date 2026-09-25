// Shared resource compiler for native MSVC and Linux -> MinGW builds.
use std::{env, path::Path, process::Command};
pub fn compile(directory:&Path, resource:&str, manifest:Option<&str>, binary:&str) {
    let out=std::path::PathBuf::from(env::var_os("OUT_DIR").unwrap());
    if env::var("CARGO_CFG_TARGET_ENV").as_deref()==Ok("gnu") {
        let prefix=if env::var("CARGO_CFG_TARGET_ARCH").as_deref()==Ok("x86"){"i686"}else{"x86_64"};
        let compiler=env::var("RC").unwrap_or_else(|_|format!("{prefix}-w64-mingw32-windres"));
        let input=out.join("windows-resources.rc");
        let mut source=format!("#include \"{}\"\n",directory.join(resource).display());
        if let Some(manifest)=manifest{source+=&format!("1 24 \"{}\"\n",directory.join(manifest).display());}
        std::fs::write(&input,source).unwrap();
        let output=out.join("windows-resources.o");
        assert!(Command::new(compiler).current_dir(directory).arg("-I").arg(directory)
            .arg("-i").arg(input).arg("-O").arg("coff").arg("-o").arg(&output)
            .status().expect("run MinGW resource compiler").success());
        println!("cargo:rustc-link-arg-bin={binary}={}",output.display());
    }else{
        let compiler=env::var_os("RC").map(std::path::PathBuf::from).unwrap_or_else(||{
            let kits=std::path::PathBuf::from(env::var_os("ProgramFiles(x86)").expect("Windows SDK required"))
                .join("Windows Kits/10/bin");
            let mut candidates:Vec<_>=std::fs::read_dir(kits).expect("Windows SDK required").flatten()
                .map(|e|e.path().join("x64/rc.exe")).filter(|p|p.exists()).collect();
            candidates.sort();candidates.pop().expect("rc.exe required")
        });
        let output=out.join("app.res");
        assert!(Command::new(compiler).current_dir(directory).arg("/nologo").arg("/fo")
            .arg(&output).arg(resource).status().expect("compile Windows resources").success());
        println!("cargo:rustc-link-arg-bin={binary}={}",output.display());
        if let Some(manifest)=manifest{
            println!("cargo:rustc-link-arg-bin={binary}=/MANIFEST:EMBED");
            println!("cargo:rustc-link-arg-bin={binary}=/MANIFESTINPUT:{}",directory.join(manifest).display());
        }
    }
}

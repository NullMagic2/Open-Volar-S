fn main() {
    build_captions();
    println!("cargo:rerun-if-changed=app.manifest");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref()==Ok("windows") {
        println!("cargo:rerun-if-changed=app.rc");
        println!("cargo:rerun-if-changed=assets/app.ico");
        let manifest=std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
        let output=std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("app.res");
        let kits=std::path::PathBuf::from(std::env::var_os("ProgramFiles(x86)").unwrap()).join("Windows Kits/10/bin");
        let mut compilers:Vec<_>=std::fs::read_dir(kits).expect("Windows SDK required").flatten()
            .map(|entry|entry.path().join("x64/rc.exe")).filter(|path|path.exists()).collect();
        compilers.sort();
        let rc=std::env::var_os("RC").map(std::path::PathBuf::from).or_else(||compilers.pop()).expect("rc.exe required");
        assert!(std::process::Command::new(rc).current_dir(&manifest).arg("/nologo").arg("/fo").arg(&output).arg("app.rc").status().expect("compile icon resource").success());
        println!("cargo:rustc-link-arg-bin=live-tv={}",output.display());
        let path=std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("app.manifest");
        println!("cargo:rustc-link-arg-bin=live-tv=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg-bin=live-tv=/MANIFESTINPUT:{}",path.display());
    }
}

fn build_captions() {
    use std::{path::PathBuf,fs};
    let root=PathBuf::from("../third-party/libaribcaption");
    println!("cargo:rerun-if-changed={}",root.display());
    println!("cargo:rerun-if-changed=caption_bridge.cpp");
    println!("cargo:rerun-if-changed=caption_layout.hpp");
    let out=PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    fs::write(out.join("aribcc_config.h"),"#pragma once\n#define ARIBCC_VERSION \"1.1.2\"\n#define ARIBCC_VERSION_MAJOR 1\n#define ARIBCC_VERSION_MINOR 1\n#define ARIBCC_VERSION_PATCH 2\n#define ARIBCC_USE_DIRECTWRITE 1\n").unwrap();
    let mut build=cc::Build::new();
    build.cpp(true).std("c++17").flag("/utf-8").flag("/EHsc").warnings(false)
        .define("ARIBCC_IMPLEMENTATION",None).define("NOMINMAX",None).define("UNICODE",None).define("_UNICODE",None)
        .include(root.join("include")).include(root.join("src")).include(&out).file("caption_bridge.cpp");
    for dir in ["base","common","decoder","renderer"] {
        for entry in fs::read_dir(root.join("src").join(dir)).unwrap().flatten(){
            let p=entry.path();let n=p.file_name().unwrap().to_string_lossy();
            if p.extension().is_some_and(|e|e=="cpp") && !["android","coretext","fontconfig","freetype","font_provider_gdi","open_type_gsub","tinyxml2"].iter().any(|x|n.contains(x)){build.file(p);}
        }
    }
    build.compile("a865r_captions");
    cc::Build::new().warnings(false).file(root.join("src/base/md5.c")).compile("arib_md5");
    for lib in ["ole32","d2d1","dwrite","windowscodecs"] {println!("cargo:rustc-link-lib={lib}");}
}

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref()!=Ok("linux"){return;}
    use std::{path::PathBuf,fs};
    let root=PathBuf::from("../../third-party/libaribcaption");
    println!("cargo:rerun-if-changed={}",root.display());
    println!("cargo:rerun-if-changed=../Windows/caption_bridge.cpp");
    println!("cargo:rerun-if-changed=../Windows/caption_layout.hpp");
    let out=PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    fs::write(out.join("aribcc_config.h"),"#pragma once\n#define ARIBCC_VERSION \"1.1.2\"\n#define ARIBCC_VERSION_MAJOR 1\n#define ARIBCC_VERSION_MINOR 1\n#define ARIBCC_VERSION_PATCH 2\n#define ARIBCC_USE_FONTCONFIG 1\n#define ARIBCC_USE_FREETYPE 1\n").unwrap();
    let mut build=cc::Build::new();
    build.cpp(true).std("c++17").flag_if_supported("/utf-8").flag_if_supported("/EHsc").warnings(false)
        .define("ARIBCC_IMPLEMENTATION",None).define("NOMINMAX",None).define("UNICODE",None).define("_UNICODE",None)
        .include(root.join("include")).include(root.join("src")).include(&out).file("../Windows/caption_bridge.cpp");
    let flags=std::process::Command::new("pkg-config").args(["--cflags","fontconfig","freetype2"]).output().expect("Install pkg-config, libfontconfig1-dev and libfreetype-dev");
    assert!(flags.status.success(),"Install fontconfig and FreeType development packages");
    for flag in String::from_utf8(flags.stdout).unwrap().split_whitespace(){if let Some(path)=flag.strip_prefix("-I"){build.include(path);}else{build.flag(flag);}}
    for dir in ["base","common","decoder","renderer"] {
        for entry in fs::read_dir(root.join("src").join(dir)).unwrap().flatten(){
            let p=entry.path();let n=p.file_name().unwrap().to_string_lossy();
            if p.extension().is_some_and(|e|e=="cpp") && !["android","coretext","directwrite","font_provider_gdi","tinyxml2"].iter().any(|x|n.contains(x)){build.file(p);}
        }
    }
    build.compile("a865r_captions");
    cc::Build::new().warnings(false).file(root.join("src/base/md5.c")).compile("arib_md5");
    for lib in ["fontconfig","freetype"] {println!("cargo:rustc-link-lib={lib}");}
}

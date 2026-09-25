fn main() {
    println!("cargo:rerun-if-changed=src/media.c");
    println!("cargo:rerun-if-changed=src/audio.cpp");
    println!("cargo:rerun-if-env-changed=OVS_FFMPEG_SDK");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") { return; }
    let sdk = std::path::PathBuf::from(std::env::var_os("OVS_FFMPEG_SDK")
        .expect("Set OVS_FFMPEG_SDK to a Windows shared FFmpeg SDK (include/lib/bin)"));
    cc::Build::new().file("src/media.c").include(sdk.join("include"))
        .define("_CRT_SECURE_NO_WARNINGS", None).compile("ovs_host_media");
    cc::Build::new().cpp(true).file("src/audio.cpp").flag_if_supported("/std:c++17")
        .compile("ovs_host_audio");
    println!("cargo:rustc-link-lib=ole32");
    println!("cargo:rustc-link-lib=ksuser");
    println!("cargo:rustc-link-search=native={}", sdk.join("lib").display());
    for lib in ["avformat", "avcodec", "avutil"] {
        println!("cargo:rustc-link-lib={lib}");
    }
}

use std::path::{Path, PathBuf};

fn configured(root: &Path, parser: &Path) -> cc::Build {
    let mut build = cc::Build::new();
    build.cpp(true)
        .std("c++17")
        .warnings(false)
        .include("headers")
        .include(root.join("common/include"))
        .include(root.join("common/libs"))
        .include(root.join("vk_video_decoder/include"))
        .include(root.join("vk_video_decoder/include/vkvideo_parser"))
        .include(root.join("vk_video_decoder/include/NvVideoParser"))
        .include(parser.join("include"));
    build
}

fn main() {
    let root = PathBuf::from("upstream");
    let parser = root.join("vk_video_decoder/libs/NvVideoParser");
    let sources = parser.join("src");
    let optimized = [
        ("NextStartCodeSSSE3.cpp", &["-mssse3"][..], "ssse3"),
        ("NextStartCodeAVX2.cpp", &["-mavx2"][..], "avx2"),
        (
            "NextStartCodeAVX512.cpp",
            &["-mavx512f", "-mavx512bw"][..],
            "avx512",
        ),
    ];
    let separate_simd = std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("x86_64")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc");

    let mut base = configured(&root, &parser);
    for name in [
        "VulkanVideoDecoder.cpp",
        "VulkanH264Parser.cpp",
        "nvVulkanh264ScalingList.cpp",
        "cpudetect.cpp",
        "NextStartCodeC.cpp",
    ] {
        base.file(sources.join(name));
    }
    if !separate_simd {
        for (name, _, _) in optimized {
            base.file(sources.join(name));
        }
    }
    base.file("parser.cpp").compile("broadcast_parser");

    if separate_simd {
        for (name, flags, label) in optimized {
            let mut build = configured(&root, &parser);
            for flag in flags {
                build.flag(flag);
            }
            build.file(sources.join(name))
                .compile(&format!("broadcast_parser_{label}"));
        }
    }
    println!("cargo:rerun-if-changed=parser.cpp");
}

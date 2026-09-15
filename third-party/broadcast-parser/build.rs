fn main() {
    let root = std::path::Path::new("upstream");
    let parser=root.join("vk_video_decoder/libs/NvVideoParser");
    let mut build=cc::Build::new();
    build.cpp(true).std("c++17").warnings(false)
        .include("headers")
        .include(root.join("common/include"))
        .include(root.join("common/libs"))
        .include(root.join("vk_video_decoder/include"))
        .include(root.join("vk_video_decoder/include/vkvideo_parser"))
        .include(root.join("vk_video_decoder/include/NvVideoParser"))
        .include(parser.join("include"));
    for f in ["VulkanVideoDecoder.cpp","VulkanH264Parser.cpp","nvVulkanh264ScalingList.cpp","cpudetect.cpp","NextStartCodeC.cpp","NextStartCodeSSSE3.cpp","NextStartCodeAVX2.cpp","NextStartCodeAVX512.cpp"] {
        build.file(parser.join("src").join(f));
    }
    build.file("parser.cpp").compile("broadcast_parser");
    println!("cargo:rerun-if-changed=parser.cpp");
}

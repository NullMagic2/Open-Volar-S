use ash::{vk,Entry};
use std::ffi::CStr;
fn main(){unsafe{
    let entry=Entry::load().unwrap();
    let app=vk::ApplicationInfo::builder().api_version(vk::API_VERSION_1_3);
    let instance=entry.create_instance(&vk::InstanceCreateInfo::builder().application_info(&app),None).unwrap();
    let video=vk::KhrVideoQueueFn::load(|name|entry.get_instance_proc_addr(instance.handle(),name.as_ptr()).map_or(std::ptr::null(),|f|f as *const _));
    let mut devices=Vec::new();
    for physical in instance.enumerate_physical_devices().unwrap(){
        let props=instance.get_physical_device_properties(physical);
        let extensions:Vec<_>=instance.enumerate_device_extension_properties(physical).unwrap().iter().map(|e|CStr::from_ptr(e.extension_name.as_ptr()).to_string_lossy().into_owned()).filter(|e|e.contains("video")).collect();
        let mut profiles=Vec::new();
        if extensions.iter().any(|e|e=="VK_KHR_video_decode_h264"){
            for (name,layout) in [("progressive",0),("interlaced_interleaved_lines",1),("interlaced_separate_planes",2)] {
                let mut h264=vk::VideoDecodeH264ProfileInfoKHR::builder().std_profile_idc(100).picture_layout(vk::VideoDecodeH264PictureLayoutFlagsKHR::from_raw(layout));
                let profile=vk::VideoProfileInfoKHR::builder().video_codec_operation(vk::VideoCodecOperationFlagsKHR::DECODE_H264).chroma_subsampling(vk::VideoChromaSubsamplingFlagsKHR::TYPE_420).luma_bit_depth(vk::VideoComponentBitDepthFlagsKHR::TYPE_8).chroma_bit_depth(vk::VideoComponentBitDepthFlagsKHR::TYPE_8).push_next(&mut h264);
                let mut decode=vk::VideoDecodeCapabilitiesKHR::default();let mut hcaps=vk::VideoDecodeH264CapabilitiesKHR::default();
                let mut caps=vk::VideoCapabilitiesKHR::builder().push_next(&mut decode).push_next(&mut hcaps).build();
                let result=(video.get_physical_device_video_capabilities_khr)(physical,&*profile,&mut caps);
                profiles.push(serde_json::json!({"layout":name,"result":format!("{result:?}"),"max_coded_extent":[caps.max_coded_extent.width,caps.max_coded_extent.height],"max_dpb_slots":caps.max_dpb_slots,"max_active_references":caps.max_active_reference_pictures}));
            }
        }
        devices.push(serde_json::json!({"device":CStr::from_ptr(props.device_name.as_ptr()).to_string_lossy(),"driver_version":props.driver_version,"api_version":props.api_version,"video_extensions":extensions,"h264_high_8bit_420_profiles":profiles}));
    }
    println!("{}",serde_json::to_string_pretty(&devices).unwrap());instance.destroy_instance(None);
}}

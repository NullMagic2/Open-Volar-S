use std::{ptr::NonNull, sync::Arc};

use ash::{Entry, vk};

mod command;
mod debug;
mod mem;
#[cfg(feature = "transcoder")]
mod pipeline;
mod sync;
mod video;
mod vk_extensions;

pub(crate) use command::*;
pub(crate) use debug::*;
pub(crate) use mem::*;
#[cfg(feature = "transcoder")]
pub(crate) use pipeline::*;
pub(crate) use sync::*;
pub(crate) use video::*;
pub(crate) use vk_extensions::*;

use crate::VulkanCommonError;

pub(crate) struct Instance {
    pub(crate) instance: ash::Instance,
    pub(crate) _entry: Arc<Entry>,
    pub(crate) video_queue_instance_ext: ash::khr::video_queue::Instance,
    pub(crate) video_encode_queue_instance_ext: ash::khr::video_encode_queue::Instance,
    pub(crate) debug_utils_instance_ext: ash::ext::debug_utils::Instance,
}

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe { self.destroy_instance(None) };
    }
}

impl std::ops::Deref for Instance {
    type Target = ash::Instance;

    fn deref(&self) -> &Self::Target {
        &self.instance
    }
}

pub(crate) struct Device {
    pub(crate) device: ash::Device,
    pub(crate) lost: std::sync::atomic::AtomicBool,
    pub(crate) video_queue_ext: ash::khr::video_queue::Device,
    pub(crate) video_decode_queue_ext: ash::khr::video_decode_queue::Device,
    pub(crate) video_encode_queue_ext: ash::khr::video_encode_queue::Device,
    #[cfg(feature = "vk-validation")]
    pub(crate) debug_utils_ext: ash::ext::debug_utils::Device,
    pub(crate) _instance: Arc<Instance>,
}

impl Device {
    #[cfg(feature = "vk-validation")]
    pub(crate) fn set_label<T: vk::Handle>(
        &self,
        object: T,
        label: Option<&str>,
    ) -> Result<(), VulkanCommonError> {
        use std::ffi::CStr;

        if let Some(label) = label {
            let mut text = [0; 64];
            let mut long_text = Vec::new();

            let label = if label.len() < text.len() {
                text[..label.len()].copy_from_slice(label.as_bytes());
                CStr::from_bytes_until_nul(&text).unwrap()
            } else {
                long_text.extend_from_slice(label.as_bytes());
                long_text.push(0);
                CStr::from_bytes_until_nul(&long_text).unwrap()
            };

            unsafe {
                self.debug_utils_ext.set_debug_utils_object_name(
                    &vk::DebugUtilsObjectNameInfoEXT::default()
                        .object_handle(object)
                        .object_name(label),
                )?
            }
        }

        Ok(())
    }

    #[cfg(not(feature = "vk-validation"))]
    pub(crate) fn set_label<T: vk::Handle>(
        &self,
        _object: T,
        _label: Option<&str>,
    ) -> Result<(), VulkanCommonError> {
        Ok(())
    }
}

impl std::ops::Deref for Device {
    type Target = ash::Device;

    fn deref(&self) -> &Self::Target {
        &self.device
    }
}

// A lost AMD device can block indefinitely inside vkDestroyDevice. Keep the
// instance/library alive on a detached cleanup thread; never wait for it on the
// playback owner. Recovery creates a separate Vulkan device, never reuses this one.
fn retire_lost_device(cleanup:impl FnOnce()+Send+'static)->std::io::Result<std::thread::JoinHandle<()>> {
    std::thread::Builder::new().name("lost-vulkan-device-cleanup".into()).spawn(cleanup)
}
impl Drop for Device {
    fn drop(&mut self) {
        if self.lost.load(std::sync::atomic::Ordering::Acquire) {
            let raw=self.device.clone();let instance=self._instance.clone();
            if retire_lost_device(move || {unsafe {raw.destroy_device(None)};drop(instance);}).is_err() {
                // If no thread can be created, retain the loader until process exit.
                std::mem::forget(self._instance.clone());
            }
        } else {unsafe {self.destroy_device(None)};}
    }
}
#[cfg(test)] mod retirement_tests {
    #[test] fn blocked_driver_cleanup_does_not_block_the_caller() {
        let (release,wait)=std::sync::mpsc::channel();let (done,finished)=std::sync::mpsc::channel();
        let worker=super::retire_lost_device(move || {wait.recv().unwrap();done.send(()).unwrap();}).unwrap();
        assert!(finished.try_recv().is_err());release.send(()).unwrap();
        finished.recv_timeout(std::time::Duration::from_secs(1)).unwrap();worker.join().unwrap();
    }
}

unsafe impl<'a> Send for ProfileInfo<'a> {}
unsafe impl<'a> Sync for ProfileInfo<'a> {}

pub(crate) struct ProfileInfo<'a> {
    pub(crate) profile_info: vk::VideoProfileInfoKHR<'a>,
    additional_infos_ptr: Vec<NonNull<dyn vk::ExtendsVideoProfileInfoKHR + Send + Sync + 'a>>,
}

impl<'a> ProfileInfo<'a> {
    pub(crate) fn new(
        mut profile_info: vk::VideoProfileInfoKHR<'a>,
        additional_info: Vec<Box<dyn vk::ExtendsVideoProfileInfoKHR + Send + Sync + 'a>>,
    ) -> Self {
        let (refs, ptrs) = additional_info
            .into_iter()
            .map(|i| {
                let r = Box::leak(i);
                let p = NonNull::from(&mut *r);
                (r, p)
            })
            .unzip::<_, _, Vec<_>, Vec<_>>();

        for r in refs {
            profile_info = profile_info.push_next(r);
        }

        Self {
            profile_info,
            additional_infos_ptr: ptrs,
        }
    }
}

impl Drop for ProfileInfo<'_> {
    fn drop(&mut self) {
        unsafe {
            for ptr in self.additional_infos_ptr.drain(..) {
                let _ = Box::from_raw(ptr.as_ptr());
            }
        }
    }
}

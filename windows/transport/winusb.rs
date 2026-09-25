//! Summary: Implements the Windows 11 USB backend using the inbox `winusb.sys` kernel driver.
//!
//! No third-party Rust Windows crate is required: the small FFI surface used here is declared
//! directly and linked against SetupAPI, Kernel32, and WinUSB from the Windows SDK.

use super::{BulkPipe, Transport};
use crate::error::{Error, Result};
use std::ffi::c_void;
use std::mem::{size_of, zeroed};
use std::ptr::{null, null_mut};

// Development interface GUID registered by winusb/A865R-WinUSB.inf.
const A865R_INTERFACE_GUID: Guid = Guid {
    data1: 0xF384CE54,
    data2: 0xA804,
    data3: 0x4C22,
    data4: [0xA1, 0x69, 0x56, 0x8D, 0x5C, 0xA0, 0xC5, 0x40],
};

// Signed AverMedia A865R libusbK interface, from its installed INF.
const A865R_LIBUSBK_GUID: Guid = Guid { data1: 0xA56BAC45, data2: 0xEBD3, data3: 0x4D26,
    data4: [0x96, 0x69, 0x89, 0x2D, 0xB8, 0xB7, 0xF3, 0x36] };

// GUID_DEVINTERFACE_USB_HUB, published by the Windows USB stack.
const USB_HUB_INTERFACE_GUID: Guid = Guid {
    data1: 0xF18A0E88,
    data2: 0xC30C,
    data3: 0x11D0,
    data4: [0x88, 0x15, 0x00, 0xA0, 0xC9, 0x06, 0xBE, 0xD8],
};

const DIGCF_PRESENT: u32 = 0x0000_0002;
const DIGCF_DEVICEINTERFACE: u32 = 0x0000_0010;
const ERROR_INVALID_HANDLE: u32 = 6;
const ERROR_ACCESS_DENIED: u32 = 5;
const ERROR_GEN_FAILURE: u32 = 31;
const ERROR_NOT_SUPPORTED: u32 = 50;
const ERROR_NO_MORE_ITEMS: u32 = 259;
const ERROR_SEM_TIMEOUT: u32 = 121;
const GENERIC_READ: u32 = 0x8000_0000;
const GENERIC_WRITE: u32 = 0x4000_0000;
const FILE_SHARE_READ: u32 = 0x0000_0001;
const FILE_SHARE_WRITE: u32 = 0x0000_0002;
const OPEN_EXISTING: u32 = 3;
const FILE_ATTRIBUTE_NORMAL: u32 = 0x0000_0080;
const FILE_FLAG_OVERLAPPED: u32 = 0x4000_0000;
const SPDRP_ADDRESS: u32 = 0x0000_001C;
const CR_SUCCESS: u32 = 0;
// CTL_CODE(FILE_DEVICE_USB=0x22, USB_HUB_CYCLE_PORT=273, METHOD_BUFFERED, FILE_ANY_ACCESS).
const IOCTL_USB_HUB_CYCLE_PORT: u32 = 0x0022_0444;
const PIPE_TRANSFER_TIMEOUT: u32 = 3;
const USBD_PIPE_TYPE_BULK: i32 = 2;
const INVALID_HANDLE_VALUE: Handle = -1isize as Handle;
// SetupAPI's Unicode detail header uses the packed SDK size on x86 (6),
// while x64 requires 8. Rust repr(C) adds padding on x86, so size_of is wrong.
const DEVICE_INTERFACE_DETAIL_SIZE: u32 = if cfg!(target_pointer_width = "64") {
    8
} else {
    6
};

type Bool = i32;
type Handle = *mut c_void;
type Hdevinfo = *mut c_void;
type WinUsbHandle = *mut c_void;

#[repr(C)]
#[derive(Clone, Copy)]
struct Guid {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}

#[repr(C)]
struct SpDeviceInterfaceData {
    cb_size: u32,
    interface_class_guid: Guid,
    flags: u32,
    reserved: usize,
}

#[repr(C)]
struct SpDeviceInterfaceDetailDataW {
    cb_size: u32,
    device_path: [u16; 1],
}

#[repr(C)]
struct SpDevinfoData {
    cb_size: u32,
    class_guid: Guid,
    dev_inst: u32,
    reserved: usize,
}

#[repr(C)]
struct UsbCyclePortParams {
    connection_index: u32,
    status_returned: u32,
}

struct FoundDevice {
    device_path: Vec<u16>,
    hub_path: Vec<u16>,
    port_number: u32,
    hub_resolution_error: Option<String>,
}

#[repr(C)]
struct UsbInterfaceDescriptor {
    b_length: u8,
    b_descriptor_type: u8,
    b_interface_number: u8,
    b_alternate_setting: u8,
    b_num_endpoints: u8,
    b_interface_class: u8,
    b_interface_sub_class: u8,
    b_interface_protocol: u8,
    i_interface: u8,
}

#[repr(C)]
struct WinUsbPipeInformation {
    pipe_type: i32,
    pipe_id: u8,
    maximum_packet_size: u16,
    interval: u8,
}

#[link(name = "setupapi")]
extern "system" {
    fn SetupDiGetClassDevsW(
        class_guid: *const Guid,
        enumerator: *const u16,
        hwnd_parent: Handle,
        flags: u32,
    ) -> Hdevinfo;
    fn SetupDiEnumDeviceInterfaces(
        device_info_set: Hdevinfo,
        device_info_data: *mut c_void,
        interface_class_guid: *const Guid,
        member_index: u32,
        device_interface_data: *mut SpDeviceInterfaceData,
    ) -> Bool;
    fn SetupDiGetDeviceInterfaceDetailW(
        device_info_set: Hdevinfo,
        device_interface_data: *mut SpDeviceInterfaceData,
        device_interface_detail_data: *mut SpDeviceInterfaceDetailDataW,
        device_interface_detail_data_size: u32,
        required_size: *mut u32,
        device_info_data: *mut c_void,
    ) -> Bool;
    fn SetupDiGetDeviceRegistryPropertyW(
        device_info_set: Hdevinfo,
        device_info_data: *mut SpDevinfoData,
        property: u32,
        property_reg_data_type: *mut u32,
        property_buffer: *mut u8,
        property_buffer_size: u32,
        required_size: *mut u32,
    ) -> Bool;
    fn SetupDiDestroyDeviceInfoList(device_info_set: Hdevinfo) -> Bool;
}

#[link(name = "cfgmgr32")]
extern "system" {
    fn CM_Get_Parent(parent: *mut u32, child: u32, flags: u32) -> u32;
}

#[link(name = "kernel32")]
extern "system" {
    fn CreateFileW(
        file_name: *const u16,
        desired_access: u32,
        share_mode: u32,
        security_attributes: *mut c_void,
        creation_disposition: u32,
        flags_and_attributes: u32,
        template_file: Handle,
    ) -> Handle;
    fn CloseHandle(object: Handle) -> Bool;
    fn DeviceIoControl(
        device: Handle,
        io_control_code: u32,
        input_buffer: *mut c_void,
        input_buffer_size: u32,
        output_buffer: *mut c_void,
        output_buffer_size: u32,
        bytes_returned: *mut u32,
        overlapped: *mut c_void,
    ) -> Bool;
    fn GetLastError() -> u32;
    fn LoadLibraryExW(name: *const u16, file: Handle, flags: u32) -> Handle;
    fn GetProcAddress(module: Handle, name: *const u8) -> *mut c_void;
    fn FreeLibrary(module: Handle) -> Bool;
}

#[link(name = "shell32")]
extern "system" {
    fn ShellExecuteW(hwnd: Handle, operation: *const u16, file: *const u16,
        parameters: *const u16, directory: *const u16, show: i32) -> Handle;
}

#[link(name = "winusb")]
extern "system" {
    fn WinUsb_Initialize(device_handle: Handle, interface_handle: *mut WinUsbHandle) -> Bool;
    fn WinUsb_Free(interface_handle: WinUsbHandle) -> Bool;
    fn WinUsb_QueryInterfaceSettings(
        interface_handle: WinUsbHandle,
        alternate_interface_number: u8,
        usb_alt_interface_descriptor: *mut UsbInterfaceDescriptor,
    ) -> Bool;
    fn WinUsb_QueryPipe(
        interface_handle: WinUsbHandle,
        alternate_interface_number: u8,
        pipe_index: u8,
        pipe_information: *mut WinUsbPipeInformation,
    ) -> Bool;
    fn WinUsb_SetPipePolicy(
        interface_handle: WinUsbHandle,
        pipe_id: u8,
        policy_type: u32,
        value_length: u32,
        value: *mut c_void,
    ) -> Bool;
    fn WinUsb_AbortPipe(interface_handle: WinUsbHandle, pipe_id: u8) -> Bool;
    fn WinUsb_FlushPipe(interface_handle: WinUsbHandle, pipe_id: u8) -> Bool;
    fn WinUsb_ReadPipe(
        interface_handle: WinUsbHandle,
        pipe_id: u8,
        buffer: *mut u8,
        buffer_length: u32,
        length_transferred: *mut u32,
        overlapped: *mut c_void,
    ) -> Bool;
    fn WinUsb_WritePipe(
        interface_handle: WinUsbHandle,
        pipe_id: u8,
        buffer: *mut u8,
        buffer_length: u32,
        length_transferred: *mut u32,
        overlapped: *mut c_void,
    ) -> Bool;
}

type InitFn = unsafe extern "system" fn(Handle, *mut WinUsbHandle) -> Bool;
type FreeFn = unsafe extern "system" fn(WinUsbHandle) -> Bool;
type QuerySettingsFn = unsafe extern "system" fn(WinUsbHandle, u8, *mut UsbInterfaceDescriptor) -> Bool;
type QueryPipeFn = unsafe extern "system" fn(WinUsbHandle, u8, u8, *mut WinUsbPipeInformation) -> Bool;
type SetPolicyFn = unsafe extern "system" fn(WinUsbHandle, u8, u32, u32, *mut c_void) -> Bool;
type PipeFn = unsafe extern "system" fn(WinUsbHandle, u8) -> Bool;
type TransferFn = unsafe extern "system" fn(WinUsbHandle, u8, *mut u8, u32, *mut u32, *mut c_void) -> Bool;

struct LibUsbKApi {
    module: Handle, initialize: InitFn, free: FreeFn, query_settings: QuerySettingsFn,
    query_pipe: QueryPipeFn, set_policy: SetPolicyFn, abort_pipe: PipeFn,
    flush_pipe: PipeFn, read_pipe: TransferFn, write_pipe: TransferFn,
}
impl LibUsbKApi {
    fn load() -> Result<Self> {
        // Search only System32, where the signed driver installs its user-mode DLL.
        let name: Vec<u16> = "libusbK.dll\0".encode_utf16().collect();
        let module = unsafe { LoadLibraryExW(name.as_ptr(), null_mut(), 0x800) };
        if module.is_null() { return Err(WinUsbTransport::last_error("LoadLibraryExW(libusbK.dll) failed")); }
        unsafe fn symbol<T: Copy>(module: Handle, name: &'static [u8]) -> Result<T> {
            let address = GetProcAddress(module, name.as_ptr());
            if address.is_null() { return Err(WinUsbTransport::last_error("GetProcAddress(libusbK) failed")); }
            Ok(std::mem::transmute_copy(&address))
        }
        let result = (|| -> Result<Self> { Ok(Self {
            module,
            initialize: unsafe { symbol(module, b"UsbK_Initialize\0")? },
            free: unsafe { symbol(module, b"UsbK_Free\0")? },
            query_settings: unsafe { symbol(module, b"UsbK_QueryInterfaceSettings\0")? },
            query_pipe: unsafe { symbol(module, b"UsbK_QueryPipe\0")? },
            set_policy: unsafe { symbol(module, b"UsbK_SetPipePolicy\0")? },
            abort_pipe: unsafe { symbol(module, b"UsbK_AbortPipe\0")? },
            flush_pipe: unsafe { symbol(module, b"UsbK_FlushPipe\0")? },
            read_pipe: unsafe { symbol(module, b"UsbK_ReadPipe\0")? },
            write_pipe: unsafe { symbol(module, b"UsbK_WritePipe\0")? },
        }) })();
        if result.is_err() { unsafe { FreeLibrary(module); } }
        result
    }
}
impl Drop for LibUsbKApi { fn drop(&mut self) { unsafe { FreeLibrary(self.module); } } }

/// Owns a SetupAPI device-information set and releases it when enumeration ends.
struct DeviceInfoSet(Hdevinfo);

impl Drop for DeviceInfoSet {
    /// Releases the SetupAPI enumeration handle.
    fn drop(&mut self) {
        if self.0 != INVALID_HANDLE_VALUE {
            // SAFETY: self.0 was returned by SetupDiGetClassDevsW and is released exactly once.
            unsafe {
                let _ = SetupDiDestroyDeviceInfoList(self.0);
            }
        }
    }
}

/// Implements synchronous command and stream transfers through Microsoft's WinUSB API.
pub struct WinUsbTransport {
    device_handle: Handle,
    interface_handle: WinUsbHandle,
    libusbk: Option<LibUsbKApi>,
    device_path: String,
    hub_path: Vec<u16>,
    port_number: u32,
    hub_resolution_error: Option<String>,
    bulk_pipes: Vec<BulkPipe>,
    command_out_pipe: u8,
    command_in_pipe: u8,
}

impl WinUsbTransport {
    /// Creates an unopened WinUSB transport.
    pub fn new() -> Self {
        Self {
            device_handle: INVALID_HANDLE_VALUE,
            interface_handle: null_mut(),
            libusbk: None,
            device_path: String::new(),
            hub_path: Vec::new(),
            port_number: 0,
            hub_resolution_error: None,
            bulk_pipes: Vec::new(),
            command_out_pipe: 0,
            command_in_pipe: 0,
        }
    }

    /// Converts a captured Win32 error code into a readable diagnostic.
    fn error_from_code(context: &str, code: u32) -> Error {
        let text = std::io::Error::from_raw_os_error(code as i32);
        Error::Transport(format!("{context}: {text} (Windows error {code})"))
    }

    /// Converts the current Win32 last-error value into a readable diagnostic.
    fn last_error(context: &str) -> Error {
        // SAFETY: GetLastError has no preconditions.
        let code = unsafe { GetLastError() };
        Self::error_from_code(context, code)
    }

    /// Explains WinUSB's special invalid-handle result without blaming the installed driver.
    fn winusb_initialize_error(code: u32) -> Error {
        if code == ERROR_INVALID_HANDLE {
            return Error::Transport(
                "WinUsb_Initialize returned ERROR_INVALID_HANDLE. WinUSB requires the device \
                 handle to be opened with FILE_FLAG_OVERLAPPED (Windows error 6)"
                    .to_string(),
            );
        }
        Self::error_from_code("WinUsb_Initialize failed", code)
    }

    unsafe fn usb_free(&self) {
        if let Some(a) = &self.libusbk { (a.free)(self.interface_handle); }
        else { WinUsb_Free(self.interface_handle); }
    }
    unsafe fn usb_query_settings(&self, d: *mut UsbInterfaceDescriptor) -> Bool {
        if let Some(a) = &self.libusbk { (a.query_settings)(self.interface_handle, 0, d) }
        else { WinUsb_QueryInterfaceSettings(self.interface_handle, 0, d) }
    }
    unsafe fn usb_query_pipe(&self, i: u8, p: *mut WinUsbPipeInformation) -> Bool {
        if let Some(a) = &self.libusbk { (a.query_pipe)(self.interface_handle, 0, i, p) }
        else { WinUsb_QueryPipe(self.interface_handle, 0, i, p) }
    }
    unsafe fn usb_set_policy(&self, p: u8, value: *mut c_void) {
        if let Some(a) = &self.libusbk { (a.set_policy)(self.interface_handle, p, PIPE_TRANSFER_TIMEOUT, 4, value); }
        else { WinUsb_SetPipePolicy(self.interface_handle, p, PIPE_TRANSFER_TIMEOUT, 4, value); }
    }
    unsafe fn usb_abort(&self, p: u8) {
        if let Some(a) = &self.libusbk { (a.abort_pipe)(self.interface_handle, p); }
        else { WinUsb_AbortPipe(self.interface_handle, p); }
    }
    unsafe fn usb_flush(&self, p: u8) {
        if let Some(a) = &self.libusbk { (a.flush_pipe)(self.interface_handle, p); }
        else { WinUsb_FlushPipe(self.interface_handle, p); }
    }
    unsafe fn usb_read(&self, p: u8, b: *mut u8, n: u32, done: *mut u32) -> Bool {
        if let Some(a) = &self.libusbk { (a.read_pipe)(self.interface_handle, p, b, n, done, null_mut()) }
        else { WinUsb_ReadPipe(self.interface_handle, p, b, n, done, null_mut()) }
    }
    unsafe fn usb_write(&self, p: u8, b: *mut u8, n: u32, done: *mut u32) -> Bool {
        if let Some(a) = &self.libusbk { (a.write_pipe)(self.interface_handle, p, b, n, done, null_mut()) }
        else { WinUsb_WritePipe(self.interface_handle, p, b, n, done, null_mut()) }
    }

    /// Sets a per-pipe synchronous transfer timeout.
    fn set_pipe_timeout(&self, pipe: u8, timeout_ms: u32) {
        if self.interface_handle.is_null() {
            return;
        }
        let mut timeout = timeout_ms;
        // SAFETY: interface_handle is live, timeout points to a valid u32, and WinUSB copies the value.
        unsafe {
            self.usb_set_policy(pipe, (&mut timeout as *mut u32).cast());
        }
    }

    /// Enumerates bulk endpoints from alternate setting zero.
    fn enumerate_bulk_pipes(&mut self) -> Result<()> {
        // SAFETY: zero is a valid bit pattern for the descriptor and WinUSB fills it on success.
        let mut descriptor: UsbInterfaceDescriptor = unsafe { zeroed() };
        // SAFETY: interface_handle is initialized and descriptor is writable.
        let ok =
            unsafe { self.usb_query_settings(&mut descriptor) };
        if ok == 0 {
            return Err(Self::last_error("WinUsb_QueryInterfaceSettings failed"));
        }

        self.bulk_pipes.clear();
        for index in 0..descriptor.b_num_endpoints {
            // SAFETY: zero is a valid initialization and WinUSB writes the full structure on success.
            let mut pipe: WinUsbPipeInformation = unsafe { zeroed() };
            // SAFETY: index is bounded by bNumEndpoints and pipe is writable.
            let ok = unsafe { self.usb_query_pipe(index, &mut pipe) };
            if ok == 0 {
                return Err(Self::last_error("WinUsb_QueryPipe failed"));
            }
            if pipe.pipe_type == USBD_PIPE_TYPE_BULK {
                self.bulk_pipes.push(BulkPipe {
                    id: pipe.pipe_id,
                    input: (pipe.pipe_id & 0x80) != 0,
                    maximum_packet_size: pipe.maximum_packet_size,
                });
            }
        }
        Ok(())
    }

    /// Releases only the handles that become invalid when a parent hub cycles the port.
    fn close_device_handles(&mut self) {
        // SAFETY: both handles are owned by this object and are invalidated immediately after release.
        unsafe {
            if !self.interface_handle.is_null() {
                self.usb_free();
                self.interface_handle = null_mut();
            }
            if self.device_handle != INVALID_HANDLE_VALUE {
                let _ = CloseHandle(self.device_handle);
                self.device_handle = INVALID_HANDLE_VALUE;
            }
        }
        self.libusbk = None;
        self.bulk_pipes.clear();
        self.command_out_pipe = 0;
        self.command_in_pipe = 0;
    }

    /// Retrieves an interface path and its associated device-information element.
    fn interface_detail(
        raw_set: Hdevinfo,
        interface_data: &mut SpDeviceInterfaceData,
    ) -> Result<(Vec<u16>, SpDevinfoData)> {
        let mut required_size = 0u32;
        // SAFETY: this documented first call requests the required detail buffer size.
        unsafe {
            let _ = SetupDiGetDeviceInterfaceDetailW(
                raw_set,
                interface_data,
                null_mut(),
                0,
                &mut required_size,
                null_mut(),
            );
        }
        if required_size < size_of::<SpDeviceInterfaceDetailDataW>() as u32 {
            return Err(Error::Transport(
                "SetupAPI returned an invalid device-interface detail size".to_string(),
            ));
        }

        let mut storage = vec![0u8; required_size as usize];
        let detail = storage.as_mut_ptr().cast::<SpDeviceInterfaceDetailDataW>();
        // SAFETY: storage is large enough for the header and device path returned by SetupAPI.
        unsafe {
            (*detail).cb_size = DEVICE_INTERFACE_DETAIL_SIZE;
        }
        let mut device_info = SpDevinfoData {
            cb_size: size_of::<SpDevinfoData>() as u32,
            class_guid: Guid {
                data1: 0,
                data2: 0,
                data3: 0,
                data4: [0; 8],
            },
            dev_inst: 0,
            reserved: 0,
        };
        // SAFETY: both output buffers are writable and correctly sized.
        let ok = unsafe {
            SetupDiGetDeviceInterfaceDetailW(
                raw_set,
                interface_data,
                detail,
                required_size,
                null_mut(),
                (&mut device_info as *mut SpDevinfoData).cast(),
            )
        };
        if ok == 0 {
            return Err(Self::last_error("SetupDiGetDeviceInterfaceDetailW failed"));
        }

        // DevicePath begins immediately after cbSize at byte offset four in this Unicode structure.
        let path_ptr = unsafe { storage.as_ptr().add(size_of::<u32>()).cast::<u16>() };
        let max_units = (storage.len() - size_of::<u32>()) / size_of::<u16>();
        let mut length = 0usize;
        // SAFETY: path_ptr points inside storage and length never exceeds max_units.
        unsafe {
            while length < max_units && *path_ptr.add(length) != 0 {
                length += 1;
            }
            if length == 0 {
                return Err(Error::Transport(
                    "SetupAPI returned an empty device-interface path".to_string(),
                ));
            }
            let mut path = std::slice::from_raw_parts(path_ptr, length).to_vec();
            path.push(0);
            Ok((path, device_info))
        }
    }

    /// Finds the USB-hub interface whose devnode is the A865R's immediate parent.
    fn find_parent_hub_path(parent_dev_inst: u32) -> Result<Vec<u16>> {
        // SAFETY: all pointers are valid constants or null as permitted by SetupAPI.
        let raw_set = unsafe {
            SetupDiGetClassDevsW(
                &USB_HUB_INTERFACE_GUID,
                null(),
                null_mut(),
                DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
            )
        };
        if raw_set == INVALID_HANDLE_VALUE {
            return Err(Self::last_error("SetupDiGetClassDevsW(USB hubs) failed"));
        }
        let _hub_set = DeviceInfoSet(raw_set);

        for index in 0u32.. {
            let mut interface_data = SpDeviceInterfaceData {
                cb_size: size_of::<SpDeviceInterfaceData>() as u32,
                interface_class_guid: USB_HUB_INTERFACE_GUID,
                flags: 0,
                reserved: 0,
            };
            // SAFETY: raw_set is live and interface_data is writable.
            let ok = unsafe {
                SetupDiEnumDeviceInterfaces(
                    raw_set,
                    null_mut(),
                    &USB_HUB_INTERFACE_GUID,
                    index,
                    &mut interface_data,
                )
            };
            if ok == 0 {
                // SAFETY: GetLastError has no preconditions.
                let code = unsafe { GetLastError() };
                if code == ERROR_NO_MORE_ITEMS {
                    break;
                }
                return Err(Self::error_from_code(
                    "SetupDiEnumDeviceInterfaces(USB hubs) failed",
                    code,
                ));
            }
            if let Ok((path, device_info)) = Self::interface_detail(raw_set, &mut interface_data) {
                if device_info.dev_inst == parent_dev_inst {
                    return Ok(path);
                }
            }
        }

        Err(Error::Transport(
            "the A865R parent devnode has no present GUID_DEVINTERFACE_USB_HUB interface"
                .to_string(),
        ))
    }

    /// Return the receiver from WSL only when no process holds the Linux device.
    /// A forced usbipd share also needs an elevated unbind to restore Windows PnP.
    fn release_idle_wsl_receiver() -> bool {
        use std::process::Command;
        let Some(program_files) = std::env::var_os("ProgramFiles") else { return false; };
        let usbipd = std::path::PathBuf::from(program_files).join("usbipd-win").join("usbipd.exe");
        let state = || -> Option<String> {
            let out = Command::new(&usbipd).arg("list").output().ok()?;
            String::from_utf8_lossy(&out.stdout).lines()
                .find(|line| line.to_ascii_lowercase().contains("07ca:b865"))
                .map(str::to_owned)
        };
        let Some(initial) = state() else { return false; };
        let attached = initial.to_ascii_lowercase().contains("attached");
        if attached {
            // The default distro must expose this device. If unavailable, keep the
            // WSL attachment rather than interrupting an unknown client.
            let Ok(activity) = Command::new("wsl.exe")
                .args(["--exec", "sh", "-lc",
                    "if ! command -v fuser >/dev/null || ! test -e /dev/open-volar-s0; then echo UNKNOWN; elif fuser /dev/open-volar-s* >/dev/null 2>&1; then echo BUSY; else echo IDLE; fi"])
                .output() else { return false; };
            if !activity.status.success() || String::from_utf8_lossy(&activity.stdout).trim() != "IDLE" {
                return false;
            }
            let Ok(detach) = Command::new(&usbipd)
                .args(["detach", "--hardware-id", "07ca:b865"]).status() else { return false; };
            if !detach.success() { return false; }
        }
        let forced = state().is_some_and(|line| line.to_ascii_lowercase().contains("shared (forced)"));
        if forced {
            // ShellExecuteW("runas") presents the standard administrator prompt.
            let wide = |value: &str| -> Vec<u16> { value.encode_utf16().chain([0]).collect() };
            let exe = wide(&usbipd.to_string_lossy());
            let operation = wide("runas");
            let args = wide("unbind --hardware-id 07ca:b865");
            let result = unsafe { ShellExecuteW(null_mut(), operation.as_ptr(), exe.as_ptr(),
                args.as_ptr(), null(), 0) } as usize;
            if result <= 32 { return false; }
        }
        attached || forced
    }

    /// Finds the first device interface registered by the development WinUSB INF.
    fn find_device() -> Result<(FoundDevice, bool)> {
        if let Ok(found) = Self::find_device_for(&A865R_INTERFACE_GUID) { return Ok((found, false)); }
        if let Ok(found) = Self::find_device_for(&A865R_LIBUSBK_GUID) { return Ok((found, true)); }
        Err(Error::Transport("no A865R WinUSB or libusbK interface was found".to_string()))
    }

    fn find_device_for(guid: &Guid) -> Result<FoundDevice> {
        // SAFETY: all pointers are either valid constants or null as permitted by SetupAPI.
        let raw_set = unsafe {
            SetupDiGetClassDevsW(
                guid,
                null(),
                null_mut(),
                DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
            )
        };
        if raw_set == INVALID_HANDLE_VALUE {
            return Err(Self::last_error("SetupDiGetClassDevsW failed"));
        }
        let _device_set = DeviceInfoSet(raw_set);

        for index in 0u32.. {
            let mut interface_data = SpDeviceInterfaceData {
                cb_size: size_of::<SpDeviceInterfaceData>() as u32,
                interface_class_guid: *guid,
                flags: 0,
                reserved: 0,
            };
            // SAFETY: raw_set is live and interface_data is correctly sized and writable.
            let ok = unsafe {
                SetupDiEnumDeviceInterfaces(
                    raw_set,
                    null_mut(),
                    guid,
                    index,
                    &mut interface_data,
                )
            };
            if ok == 0 {
                // SAFETY: GetLastError has no preconditions.
                let code = unsafe { GetLastError() };
                if code == ERROR_NO_MORE_ITEMS {
                    break;
                }
                return Err(Self::last_error("SetupDiEnumDeviceInterfaces failed"));
            }

            let Ok((device_path, mut device_info)) =
                Self::interface_detail(raw_set, &mut interface_data)
            else {
                continue;
            };

            // Hub metadata is optional for ordinary WinUSB operation. Preserve a precise error for
            // reset-cold instead of making diagnose depend on this additional enumeration path.
            let hub = (|| -> Result<(Vec<u16>, u32)> {
                let mut port_number = 0u32;
                let mut required_size = 0u32;
                // SAFETY: device_info belongs to raw_set and port_number is a writable DWORD buffer.
                let ok = unsafe {
                    SetupDiGetDeviceRegistryPropertyW(
                        raw_set,
                        &mut device_info,
                        SPDRP_ADDRESS,
                        null_mut(),
                        (&mut port_number as *mut u32).cast(),
                        size_of::<u32>() as u32,
                        &mut required_size,
                    )
                };
                if ok == 0 {
                    return Err(Self::last_error(
                        "could not read the A865R USB port number (SPDRP_ADDRESS)",
                    ));
                }
                if port_number == 0 {
                    return Err(Error::Transport(
                        "SetupAPI reported USB port number zero for the A865R".to_string(),
                    ));
                }

                let mut parent_dev_inst = 0u32;
                // SAFETY: both DEVINST values refer to the local device tree and flags must be zero.
                let status =
                    unsafe { CM_Get_Parent(&mut parent_dev_inst, device_info.dev_inst, 0) };
                if status != CR_SUCCESS {
                    return Err(Error::Transport(format!(
                        "CM_Get_Parent failed while resolving the A865R hub (CONFIGRET 0x{status:08X})"
                    )));
                }
                Ok((Self::find_parent_hub_path(parent_dev_inst)?, port_number))
            })();
            let (hub_path, port_number, hub_resolution_error) = match hub {
                Ok((path, port)) => (path, port, None),
                Err(error) => {
                    let message = error.to_string();
                    let message = message
                        .strip_prefix("USB transport error: ")
                        .unwrap_or(&message)
                        .to_string();
                    (Vec::new(), 0, Some(message))
                }
            };
            return Ok(FoundDevice {
                device_path,
                hub_path,
                port_number,
                hub_resolution_error,
            });
        }

        Err(Error::Transport("no A865R interface was found for this driver".to_string()))
    }
}

impl Drop for WinUsbTransport {
    /// Releases the WinUSB interface and underlying device handle exactly once.
    fn drop(&mut self) {
        self.close_device_handles();
    }
}

impl Transport for WinUsbTransport {
    /// Opens the A865R interface registered by our WinUSB development INF and enumerates bulk pipes.
    fn open(&mut self, _vendor_id: u16, _product_id: u16) -> Result<()> {
        self.close_device_handles();
        let found = Self::find_device().or_else(|error| {
            if Self::release_idle_wsl_receiver() {
                for _ in 0..80 {
                    std::thread::sleep(std::time::Duration::from_millis(250));
                    if let Ok(found) = Self::find_device() { return Ok(found); }
                }
            }
            Err(error)
        })?;
        let (found, use_libusbk) = found;
        self.device_path = String::from_utf16_lossy(
            &found.device_path[..found.device_path.len().saturating_sub(1)],
        );
        self.hub_path = found.hub_path;
        self.port_number = found.port_number;
        self.hub_resolution_error = found.hub_resolution_error;

        // SAFETY: path is NUL-terminated and the device handle is opened with the overlapped flag
        // required by WinUsb_Initialize. Individual transfers may still be synchronous by passing
        // a null OVERLAPPED pointer to WinUsb_ReadPipe or WinUsb_WritePipe.
        let handle = unsafe {
            CreateFileW(
                found.device_path.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                0, // Exclusive device ownership prevents another process interleaving commands.
                null_mut(),
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL | FILE_FLAG_OVERLAPPED,
                null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            return Err(Self::last_error(
                "found the A865R interface but CreateFileW failed",
            ));
        }

        let api = if use_libusbk {
            match LibUsbKApi::load() {
                Ok(api) => Some(api),
                Err(error) => { unsafe { CloseHandle(handle); } return Err(error); }
            }
        } else { None };
        let mut interface_handle: WinUsbHandle = null_mut();
        let ok = unsafe { if let Some(api) = &api {
            (api.initialize)(handle, &mut interface_handle)
        } else { WinUsb_Initialize(handle, &mut interface_handle) } };
        if ok == 0 {
            // Capture the WinUSB failure before CloseHandle can change the thread's last-error value.
            // SAFETY: GetLastError has no preconditions and is called immediately after the failure.
            let code = unsafe { GetLastError() };
            // SAFETY: handle is owned locally because WinUsb_Initialize failed.
            unsafe {
                let _ = CloseHandle(handle);
            }
            return Err(if use_libusbk { Self::error_from_code("UsbK_Initialize failed", code) }
                else { Self::winusb_initialize_error(code) });
        }

        self.device_handle = handle;
        self.interface_handle = interface_handle;
        self.libusbk = api;
        self.enumerate_bulk_pipes()
    }

    /// Closes the WinUSB handles and cycles the A865R's downstream port through its parent hub.
    fn cycle_port(&mut self) -> Result<()> {
        if self.hub_path.is_empty() || self.port_number == 0 {
            return Err(Error::Transport(
                self.hub_resolution_error.clone().unwrap_or_else(|| {
                    "the parent USB hub and port have not been resolved".to_string()
                }),
            ));
        }
        let hub_path = self.hub_path.clone();
        let port_number = self.port_number;
        self.close_device_handles();

        // SAFETY: hub_path is NUL-terminated and was returned for GUID_DEVINTERFACE_USB_HUB.
        let hub_handle = unsafe {
            CreateFileW(
                hub_path.as_ptr(),
                GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                null_mut(),
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                null_mut(),
            )
        };
        if hub_handle == INVALID_HANDLE_VALUE {
            return Err(Self::last_error("CreateFileW(parent USB hub) failed"));
        }

        let mut parameters = UsbCyclePortParams {
            connection_index: port_number,
            status_returned: 0,
        };
        let parameters_buffer: *mut c_void = (&mut parameters as *mut UsbCyclePortParams).cast();
        let mut bytes_returned = 0u32;
        // SAFETY: hub_handle is live and parameters is the documented buffered in/out structure.
        // Although older IOCTL documentation labels the output buffer as absent, current Windows
        // hub drivers return StatusReturned through the same USB_CYCLE_PORT_PARAMS buffer and reject
        // an output length of zero with ERROR_INSUFFICIENT_BUFFER.
        let ok = unsafe {
            DeviceIoControl(
                hub_handle,
                IOCTL_USB_HUB_CYCLE_PORT,
                parameters_buffer,
                size_of::<UsbCyclePortParams>() as u32,
                parameters_buffer,
                size_of::<UsbCyclePortParams>() as u32,
                &mut bytes_returned,
                null_mut(),
            )
        };
        let code = if ok == 0 {
            // SAFETY: read immediately after DeviceIoControl fails.
            Some(unsafe { GetLastError() })
        } else {
            None
        };
        // SAFETY: hub_handle is owned locally and released exactly once.
        unsafe {
            let _ = CloseHandle(hub_handle);
        }

        if let Some(code) = code {
            if code == ERROR_ACCESS_DENIED
                || code == ERROR_GEN_FAILURE
                || code == ERROR_NOT_SUPPORTED
            {
                return Err(Error::Transport(format!(
                    "IOCTL_USB_HUB_CYCLE_PORT failed for hub port {port_number}. Run an elevated Command Prompt; Windows can report access denial as error {code} for this request"
                )));
            }
            return Err(Self::error_from_code(
                &format!("IOCTL_USB_HUB_CYCLE_PORT failed for hub port {port_number}"),
                code,
            ));
        }
        if parameters.status_returned != 0 {
            return Err(Error::Transport(format!(
                "the hub accepted IOCTL_USB_HUB_CYCLE_PORT for port {port_number}, but returned USBD status 0x{:08X}",
                parameters.status_returned
            )));
        }
        Ok(())
    }

    /// Returns the endpoints discovered when the device was opened.
    fn bulk_pipes(&self) -> &[BulkPipe] {
        &self.bulk_pipes
    }

    /// Selects the framed-command endpoint pair and clears stale host-side I/O.
    fn set_command_pipes(&mut self, output_pipe: u8, input_pipe: u8) -> Result<()> {
        if self.interface_handle.is_null() {
            return Err(Error::Transport("WinUSB device is not open".to_string()));
        }
        self.command_out_pipe = output_pipe;
        self.command_in_pipe = input_pipe;
        self.set_pipe_timeout(output_pipe, 2_000);
        self.set_pipe_timeout(input_pipe, 2_000);
        // SAFETY: the selected endpoint IDs came from the active interface descriptor.
        unsafe {
            self.usb_abort(output_pipe);
            self.usb_abort(input_pipe);
            self.usb_flush(input_pipe);
        }
        // Do not call WinUsb_ResetPipe here. Physical A865R A/B testing showed that
        // resetting either command pipe during ordinary endpoint selection causes
        // the cold device's next firmware query to time out. AbortPipe cancels stale
        // host transfers and FlushPipe discards cached input without changing the
        // device-side endpoint state.
        Ok(())
    }

    /// Writes one complete command frame and synchronously reads one complete response frame.
    fn exchange(
        &mut self,
        request: &[u8],
        expected_response_size: usize,
        timeout_ms: u32,
    ) -> Result<Vec<u8>> {
        if self.interface_handle.is_null()
            || self.command_out_pipe == 0
            || self.command_in_pipe == 0
        {
            return Err(Error::Transport(
                "command endpoints have not been selected".to_string(),
            ));
        }
        let request_len: u32 = request.len().try_into().map_err(|_| {
            Error::InvalidArgument("command request exceeds WinUSB's 32-bit length".to_string())
        })?;
        let response_len: u32 = expected_response_size.try_into().map_err(|_| {
            Error::InvalidArgument("command response exceeds WinUSB's 32-bit length".to_string())
        })?;

        self.set_pipe_timeout(self.command_out_pipe, timeout_ms);
        self.set_pipe_timeout(self.command_in_pipe, timeout_ms);

        let mut written = 0u32;
        // WinUsb_WritePipe takes a mutable pointer even though it does not modify request bytes.
        let mut request_copy = request.to_vec();
        // SAFETY: all buffers are valid and synchronous operation uses a null OVERLAPPED pointer.
        let ok = unsafe {
            self.usb_write(self.command_out_pipe, request_copy.as_mut_ptr(), request_len, &mut written)
        };
        if ok == 0 {
            return Err(Self::last_error("WinUsb_WritePipe(command) failed"));
        }
        if written != request_len {
            return Err(Error::Transport(format!(
                "short command write: expected {request_len} bytes, wrote {written}"
            )));
        }

        let mut response = vec![0u8; expected_response_size];
        let mut received = 0u32;
        // SAFETY: response owns response_len writable bytes and synchronous operation uses a null OVERLAPPED pointer.
        let ok = unsafe {
            self.usb_read(self.command_in_pipe, response.as_mut_ptr(), response_len, &mut received)
        };
        if ok == 0 {
            return Err(Self::last_error("WinUsb_ReadPipe(command) failed"));
        }
        response.truncate(received as usize);
        Ok(response)
    }

    /// Reads raw bytes from a bulk IN endpoint and treats a transfer timeout as no data.
    fn read_bulk(
        &mut self,
        input_pipe: u8,
        requested_size: usize,
        timeout_ms: u32,
    ) -> Result<Vec<u8>> {
        if self.interface_handle.is_null() {
            return Err(Error::Transport("WinUSB device is not open".to_string()));
        }
        let buffer_len: u32 = requested_size.try_into().map_err(|_| {
            Error::InvalidArgument("stream read exceeds WinUSB's 32-bit length".to_string())
        })?;
        self.set_pipe_timeout(input_pipe, timeout_ms);

        let mut buffer = vec![0u8; requested_size];
        let mut received = 0u32;
        // SAFETY: buffer owns buffer_len writable bytes and input_pipe came from the USB descriptor.
        let ok = unsafe {
            self.usb_read(input_pipe, buffer.as_mut_ptr(), buffer_len, &mut received)
        };
        if ok == 0 {
            // SAFETY: GetLastError has no preconditions and is read immediately after the failing WinUSB call.
            let code = unsafe { GetLastError() };
            if code == ERROR_SEM_TIMEOUT {
                return Ok(Vec::new());
            }
            let text = std::io::Error::from_raw_os_error(code as i32);
            return Err(Error::Transport(format!(
                "WinUsb_ReadPipe(stream) failed: {text} (Windows error {code})"
            )));
        }
        buffer.truncate(received as usize);
        Ok(buffer)
    }

    /// Returns the selected SetupAPI device-interface path.
    fn description(&self) -> String {
        if self.device_path.is_empty() {
            "WinUSB (not open)".to_string()
        } else {
            format!("{}: {}", if self.libusbk.is_some() { "libusbK" } else { "WinUSB" }, self.device_path)
        }
    }
}

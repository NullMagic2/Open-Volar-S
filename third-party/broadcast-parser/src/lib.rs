//! H.264 bitstream metadata from the Khronos Vulkan Video sample parser.
//! Callbacks contain compressed data and picture descriptions, never decoded pixels.
#![deny(unsafe_op_in_unsafe_fn)]
use ash::vk::native::{StdVideoH264PictureParameterSet, StdVideoH264SequenceParameterSet};
use std::ffi::c_void;
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Reference {
    pub id: u64,
    pub frame_num: i32,
    pub flags: u32,
    pub poc: [i32; 2],
}
#[repr(C)]
pub struct RawPicture {
    pub id: u64,
    pub config: u64,
    pub field: u32,
    pub bottom: u32,
    pub second: u32,
    pub progressive: u32,
    pub top_first: u32,
    pub repeat: u32,
    pub reference: u32,
    pub intra: u32,
    pub idr: u32,
    pub coded_width: u32,
    pub coded_height: u32,
    pub width: u32,
    pub height: u32,
    pub rate_num: u32,
    pub rate_den: u32,
    pub matrix: u32,
    pub full: u32,
    pub frame_num: u32,
    pub poc: [i32; 2],
    pub reference_count: u32,
    pub references: [Reference; 17],
    sps: *const StdVideoH264SequenceParameterSet,
    pps: *const StdVideoH264PictureParameterSet,
    bytes: *const u8,
    size: usize,
    slices: *const u32,
    slice_count: u32,
    pub idr_pic_id: u32,
}
type Callback = unsafe extern "C" fn(*mut c_void, u32, *const c_void, i64) -> i32;
unsafe extern "C" {
    fn broadcast_create() -> *mut c_void;
    fn broadcast_destroy(parser: *mut c_void);
    fn broadcast_parse(
        parser: *mut c_void,
        bytes: *const u8,
        size: usize,
        pts: i64,
        valid: i32,
        eos: i32,
        context: *mut c_void,
        callback: Callback,
    ) -> i32;
}

// The C++ adapter uses only synchronous callbacks. No borrowed parser memory
// escapes a callback, and a Parser remains on its creating thread.
pub struct Parser {
    native: *mut c_void,
}
pub struct Picture<'a> {
    raw: &'a RawPicture,
    pub sps: &'a StdVideoH264SequenceParameterSet,
    pub pps: &'a StdVideoH264PictureParameterSet,
    pub vui: Option<&'a ash::vk::native::StdVideoH264SequenceParameterSetVui>,
    pub bytes: &'a [u8],
    pub slices: &'a [u32],
}
impl std::ops::Deref for Picture<'_> {
    type Target = RawPicture;
    fn deref(&self) -> &RawPicture {
        self.raw
    }
}
pub enum Event<'a> {
    Picture(Picture<'a>),
    Display { id: u64, pts: i64 },
}
struct Delivery<F> {
    callback: F,
    error: Option<String>,
}
const _: () = assert!(std::mem::size_of::<RawPicture>() == 560);
const _: () = assert!(std::mem::size_of::<Reference>() == 24);
const _: () = assert!(std::mem::size_of::<StdVideoH264SequenceParameterSet>() == 88);
const _: () = assert!(std::mem::size_of::<StdVideoH264PictureParameterSet>() == 24);
impl Parser {
    pub fn new() -> Result<Self, String> {
        // SAFETY: creates an owned parser; null reports allocation/init failure.
        let native = unsafe { broadcast_create() };
        if native.is_null() {
            Err("Could not initialize the broadcast parser".into())
        } else {
            Ok(Self { native })
        }
    }
    pub fn parse<F>(
        &mut self,
        bytes: &[u8],
        pts: Option<i64>,
        eos: bool,
        callback: F,
    ) -> Result<(), String>
    where
        F: for<'a> FnMut(Event<'a>) -> Result<(), String>,
    {
        let mut delivery = Delivery {
            callback,
            error: None,
        };
        // SAFETY: the input and stack callback context remain alive throughout this
        // synchronous call. Exclusive &mut self prevents reentrant parser use.
        let ok = unsafe {
            broadcast_parse(
                self.native,
                bytes.as_ptr(),
                bytes.len(),
                pts.unwrap_or(0),
                pts.is_some().into(),
                eos.into(),
                &mut delivery as *mut Delivery<F> as *mut c_void,
                deliver::<F>,
            )
        };
        if let Some(error) = delivery.error {
            return Err(error);
        }
        if ok == 0 {
            return Err("Broadcast parser rejected the bitstream".into());
        }
        Ok(())
    }
}
impl Drop for Parser {
    fn drop(&mut self) {
        // SAFETY: this object owns the non-null handle; no parse call can coexist
        // with Drop, and the C++ destructor terminates no asynchronous work.
        unsafe { broadcast_destroy(self.native) };
    }
}
unsafe extern "C" fn deliver<F>(
    context: *mut c_void,
    kind: u32,
    data: *const c_void,
    pts: i64,
) -> i32
where
    F: for<'a> FnMut(Event<'a>) -> Result<(), String>,
{
    if context.is_null() || data.is_null() {
        return 0;
    }
    // SAFETY: context was created by Parser::parse for this monomorphized callback
    // and is valid exclusively for the duration of that synchronous call.
    let delivery = unsafe { &mut *context.cast::<Delivery<F>>() };
    if delivery.error.is_some() {
        return 0;
    }
    let result =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Result<(), String> {
            if kind == 1 {
                // SAFETY: the adapter passes an aligned stack RawPicture for event 1.
                let raw = unsafe { &*data.cast::<RawPicture>() };
                if raw.sps.is_null()
                    || raw.pps.is_null()
                    || raw.bytes.is_null()
                    || raw.slices.is_null()
                    || raw.size == 0
                    || raw.size > 16 * 1024 * 1024
                    || raw.slice_count == 0
                    || raw.slice_count > 65536
                    || raw.reference_count > 17
                {
                    return Err("Invalid parser picture metadata".into());
                }
                // SAFETY: C++ verifies both buffer lengths before calling us. Its SPS,
                // PPS and nested VUI belong to retained parameter objects and remain
                // valid until this callback returns. The public view cannot outlive it.
                let (sps, pps, bytes, slices) = unsafe {
                    (
                        &*raw.sps,
                        &*raw.pps,
                        std::slice::from_raw_parts(raw.bytes, raw.size),
                        std::slice::from_raw_parts(raw.slices, raw.slice_count as usize),
                    )
                };
                let vui = unsafe { sps.pSequenceParameterSetVui.as_ref() };
                if slices.iter().any(|v| *v as usize >= bytes.len())
                    || slices.windows(2).any(|v| v[0] >= v[1])
                {
                    return Err("Slice offsets exceed the compressed picture".into());
                }
                (delivery.callback)(Event::Picture(Picture {
                    raw,
                    sps,
                    pps,
                    vui,
                    bytes,
                    slices,
                }))?;
            } else if kind == 2 {
                // SAFETY: event 2 carries an aligned uint64_t on the adapter stack.
                let id = unsafe { *data.cast::<u64>() };
                (delivery.callback)(Event::Display { id, pts })?;
            } else {
                return Err("Unknown broadcast parser event".into());
            }
            Ok(())
        }));
    match result {
        Ok(Ok(())) => 1,
        Ok(Err(error)) => {
            delivery.error = Some(error);
            0
        }
        Err(_) => {
            delivery.error = Some("Parser callback panicked".into());
            0
        }
    }
}

/// Startup is safe at an IDR or an independent intra picture with an empty DPB.
/// A complementary second field always needs the first field to have been decoded.
pub fn independent_start(idr: bool, intra: bool, second_field: bool, reference_count: usize) -> bool {
    !second_field && (idr || (intra && reference_count == 0))
}
#[cfg(test)] mod startup_tests {
    #[test] fn accepts_independent_intra_but_never_missing_dependencies() {
        use super::independent_start as start;
        assert!(start(true,true,false,0));
        assert!(start(false,true,false,0));
        assert!(!start(false,true,false,1));
        assert!(!start(false,false,false,0));
        assert!(!start(true,true,true,0));
        assert!(!start(false,true,true,0));
    }
}

/// H.264 VUI sample proportions converted to a reduced display ratio.
pub fn display_aspect(width:u32,height:u32,idc:u32,extended:(u16,u16))->(u32,u32) {
    let ratios=[(1u32,1u32),(1,1),(12,11),(10,11),(16,11),(40,33),(24,11),(20,11),(32,11),(80,33),(18,11),(15,11),(64,33),(160,99),(4,3),(3,2),(2,1)];
    let (sw,sh)=if idc==255 && extended.0>0 && extended.1>0 {(extended.0 as u32,extended.1 as u32)} else {ratios.get(idc as usize).copied().unwrap_or((1,1))};
    let (Some(w),Some(h))=(width.checked_mul(sw),height.checked_mul(sh)) else {return(width,height)};
    if w==0||h==0{return(16,9)}
    let(mut a,mut b)=(w,h);while b!=0{(a,b)=(b,a%b)}(w/a,h/a)
}
#[cfg(test)] mod aspect_tests {
    #[test] fn anamorphic_and_invalid_sar() {
        use super::display_aspect as aspect;
        assert_eq!(aspect(720,576,2,(0,0)),(15,11));
        assert_eq!(aspect(720,480,255,(32,27)),(16,9));
        assert_eq!(aspect(1920,1080,1,(0,0)),(16,9));
        assert_eq!(aspect(320,180,255,(0,0)),(16,9));
    }
}

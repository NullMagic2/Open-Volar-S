use crate::{filter::Pin_Impl, put, trace, unsupported};
use std::mem::size_of;
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Media::{DirectShow::*, KernelStreaming::*},
        System::Com::CoTaskMemAlloc,
    },
};
impl IBDA_PinControl_Impl for Pin_Impl {
    fn GetPinID(&self, p: *mut u32) -> Result<()> {
        trace(format!("GetPinID {}", self.index));
        put(p, self.index as u32)
    }
    fn GetPinType(&self, p: *mut u32) -> Result<()> {
        put(p, self.index as u32)
    }
    fn RegistrationContext(&self, p: *mut u32) -> Result<()> {
        put(p, 0)
    }
}
fn identifier(set: GUID, id: u32) -> KSIDENTIFIER {
    KSIDENTIFIER {
        Anonymous: KSIDENTIFIER_0 {
            Anonymous: KSIDENTIFIER_0_0 {
                Set: set,
                Id: id,
                Flags: 0,
            },
        },
    }
}
fn array(value: KSIDENTIFIER) -> Result<*mut KSMULTIPLE_ITEM> {
    unsafe {
        let size = size_of::<KSMULTIPLE_ITEM>() + size_of::<KSIDENTIFIER>();
        let p = CoTaskMemAlloc(size) as *mut KSMULTIPLE_ITEM;
        if p.is_null() {
            return Err(Error::from_hresult(E_OUTOFMEMORY));
        }
        p.write(KSMULTIPLE_ITEM {
            Size: size as u32,
            Count: 1,
        });
        (p.add(1) as *mut KSIDENTIFIER).write(value);
        Ok(p)
    }
}
impl IKsPin_Impl for Pin_Impl {
    fn KsQueryMediums(&self) -> Result<*mut KSMULTIPLE_ITEM> {
        trace(format!(
            "KsQueryMediums capture={} pin={}",
            self.shared.capture, self.index
        ));
        array(identifier(
            GUID::from_u128(0x2a5fc455_33c1_482c_9a83_4df863f0c610),
            if (self.shared.capture && self.index == 0) || (!self.shared.capture && self.index == 1)
            {
                1
            } else {
                0
            },
        ))
    }
    fn KsQueryInterfaces(&self) -> Result<*mut KSMULTIPLE_ITEM> {
        array(identifier(KSINTERFACESETID_Standard, 0))
    }
    fn KsCreateSinkPinHandle(&self, _: *const KSIDENTIFIER, _: *const KSIDENTIFIER) -> Result<()> {
        trace("KsCreateSinkPinHandle unsupported: COM streaming only");
        Err(unsupported())
    }
    fn KsGetCurrentCommunication(
        &self,
        p: *mut KSPIN_COMMUNICATION,
        i: *mut KSIDENTIFIER,
        m: *mut KSIDENTIFIER,
    ) -> Result<()> {
        put(
            p,
            if self.index == 0 {
                KSPIN_COMMUNICATION_SINK
            } else {
                KSPIN_COMMUNICATION_SOURCE
            },
        )?;
        if !i.is_null() {
            put(i, identifier(KSINTERFACESETID_Standard, 0))?
        }
        if !m.is_null() {
            put(m, identifier(KSMEDIUMSETID_Standard, 0))?
        }
        Ok(())
    }
    fn KsPropagateAcquire(&self) -> Result<()> {
        Ok(())
    }
    fn KsDeliver(&self, s: Option<&IMediaSample>, _: u32) -> Result<()> {
        self.Receive(s)
    }
    fn KsMediaSamplesCompleted(&self, _: *const KSSTREAM_SEGMENT) -> Result<()> {
        Err(unsupported())
    }
    fn KsPeekAllocator(&self, _: KSPEEKOPERATION) -> Option<IMemAllocator> {
        self.allocator.lock().unwrap().clone()
    }
    fn KsReceiveAllocator(&self, a: Option<&IMemAllocator>) -> Result<()> {
        self.NotifyAllocator(a, BOOL(0))
    }
    fn KsRenegotiateAllocator(&self) -> Result<()> {
        Err(unsupported())
    }
    fn KsIncrementPendingIoCount(&self) -> i32 {
        0
    }
    fn KsDecrementPendingIoCount(&self) -> i32 {
        0
    }
    fn KsQualityNotify(&self, _: u32, _: i64) -> Result<()> {
        Err(unsupported())
    }
}

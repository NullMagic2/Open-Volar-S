use crate::{filter::Filter_Impl, invalid, put, trace, unsupported};
use std::{ffi::c_void, sync::atomic::Ordering};
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Media::{
            DirectShow::Tv::*, DirectShow::*, KernelStreaming::IKsPropertySet_Impl,
            MediaFoundation::AM_MEDIA_TYPE,
        },
    },
};
// A857.dll: IsAVerMediaAP (614B.../0), SetPower (614B.../1), IsAVerAP (A1DF.../6).
const AVER_APP: GUID = GUID::from_u128(0x614b7b41_e128_49cb_bbc7_71d843ca5296);
const AVER_CONTROL: GUID = GUID::from_u128(0xa1dfc885_1dfb_4a70_8f99_a5cbaf52acc4);
fn cached_quality(status: &crate::backend::Status) -> Result<i32> {
    if !status.locked.load(Ordering::Relaxed) {
        return Ok(0);
    }
    match *status.quality.lock().unwrap() {
        Some(value) if value <= 100 => Ok(i32::from(value)),
        _ => Err(Error::from_hresult(HRESULT(0x8000000au32 as i32))), // E_PENDING
    }
}
impl IBDA_DeviceControl_Impl for Filter_Impl {
    fn StartChanges(&self) -> Result<()> {
        trace("StartChanges");
        let committed = *self.shared.committed.lock().unwrap();
        *self.shared.tune.lock().unwrap() = committed;
        Ok(())
    }
    fn CheckChanges(&self) -> Result<()> {
        self.shared
            .tune
            .lock()
            .unwrap()
            .khz()
            .map(|_| ())
            .map_err(|e| Error::new(E_INVALIDARG, e))
    }
    fn CommitChanges(&self) -> Result<()> {
        trace("CommitChanges");
        self.shared.commit()
    }
    fn GetChangeState(&self, p: *mut u32) -> Result<()> {
        put(
            p,
            if *self.shared.committed.lock().unwrap() == *self.shared.tune.lock().unwrap() {
                0
            } else {
                1
            },
        )
    }
}
impl IBDA_FrequencyFilter_Impl for Filter_Impl {
    fn SetAutotune(&self, _: u32) -> Result<()> {
        Err(unsupported())
    }
    fn Autotune(&self, _: *mut u32) -> Result<()> {
        Err(unsupported())
    }
    fn SetFrequency(&self, v: u32) -> Result<()> {
        trace(format!("SetFrequency {v}"));
        self.shared.tune.lock().unwrap().frequency = v;
        Ok(())
    }
    fn Frequency(&self, p: *mut u32) -> Result<()> {
        put(p, self.shared.tune.lock().unwrap().frequency)
    }
    fn SetPolarity(&self, v: Polarisation) -> Result<()> {
        if v.0 == 0 || v.0 == -1 {
            Ok(())
        } else {
            Err(unsupported())
        }
    }
    fn Polarity(&self, p: *mut Polarisation) -> Result<()> {
        put(p, Polarisation(0))
    }
    fn SetRange(&self, v: u32) -> Result<()> {
        if v == 0 {
            Ok(())
        } else {
            Err(unsupported())
        }
    }
    fn Range(&self, p: *mut u32) -> Result<()> {
        put(p, 0)
    }
    fn SetBandwidth(&self, v: u32) -> Result<()> {
        trace(format!("SetBandwidth {v}"));
        if v != 6 {
            return Err(invalid());
        }
        self.shared.tune.lock().unwrap().bandwidth = v;
        Ok(())
    }
    fn Bandwidth(&self, p: *mut u32) -> Result<()> {
        put(p, self.shared.tune.lock().unwrap().bandwidth)
    }
    fn SetFrequencyMultiplier(&self, v: u32) -> Result<()> {
        if v == 0 {
            return Err(invalid());
        }
        self.shared.tune.lock().unwrap().multiplier = v;
        Ok(())
    }
    fn FrequencyMultiplier(&self, p: *mut u32) -> Result<()> {
        put(p, self.shared.tune.lock().unwrap().multiplier)
    }
}
impl IBDA_SignalStatistics_Impl for Filter_Impl {
    fn SetSignalStrength(&self, _: i32) -> Result<()> {
        Err(unsupported())
    }
    fn SignalStrength(&self, _: *mut i32) -> Result<()> {
        Err(unsupported())
    }
    fn SetSignalQuality(&self, _: i32) -> Result<()> {
        Err(unsupported())
    }
    fn SignalQuality(&self, p: *mut i32) -> Result<()> {
        put(p, 0)?;
        put(p, cached_quality(&self.shared.status)?)
    }
    fn SetSignalPresent(&self, _: BOOLEAN) -> Result<()> {
        Err(unsupported())
    }
    fn SignalPresent(&self, p: *mut u8) -> Result<()> {
        put(p, self.shared.status.present.load(Ordering::Relaxed) as u8)
    }
    fn SetSignalLocked(&self, _: BOOLEAN) -> Result<()> {
        Err(unsupported())
    }
    fn SignalLocked(&self, p: *mut u8) -> Result<()> {
        put(p, self.shared.status.locked.load(Ordering::Relaxed) as u8)
    }
    fn SetSampleTime(&self, _: i32) -> Result<()> {
        Err(unsupported())
    }
    fn SampleTime(&self, p: *mut i32) -> Result<()> {
        put(p, 100)
    }
}
// The ISDB-T receiver obtains modulation/FEC from the broadcast TMCC. The
// current RF backend supports automatic acquisition, not forced DVB parameters.
fn automatic(value: i32, name: &str) -> Result<()> {
    trace(format!("Demodulator {name}={value}"));
    if value == -1 || value == 0 {
        Ok(())
    } else {
        Err(unsupported())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quality_is_measured_and_cleared_on_signal_loss() {
        let status = crate::backend::Status::default();
        assert_eq!(cached_quality(&status).unwrap(), 0);
        status.locked.store(true, Ordering::Relaxed);
        assert!(cached_quality(&status).is_err());
        *status.quality.lock().unwrap() = Some(73);
        assert_eq!(cached_quality(&status).unwrap(), 73);
        *status.quality.lock().unwrap() = Some(255);
        assert!(cached_quality(&status).is_err());
        status.locked.store(false, Ordering::Relaxed);
        assert_eq!(cached_quality(&status).unwrap(), 0);
    }
    #[test]
    fn rejected_tune_is_discarded_by_next_transaction() {
        unsafe {
            // Capture passthrough permits transaction tests without USB I/O.
            let filter = crate::create_filter(true);
            let control: IBDA_DeviceControl = filter.cast().unwrap();
            let frequency: IBDA_FrequencyFilter = filter.cast().unwrap();
            control.StartChanges().unwrap();
            frequency.SetFrequency(521143).unwrap();
            control.CommitChanges().unwrap();
            control.StartChanges().unwrap();
            frequency.SetFrequency(123350).unwrap();
            assert_eq!(control.CheckChanges().unwrap_err().code(), E_INVALIDARG);
            assert_eq!(control.CommitChanges().unwrap_err().code(), E_INVALIDARG);
            // Playback uses the committed configuration, never the rejected draft.
            filter.Run(0).unwrap();
            assert_eq!(filter.GetState(0).unwrap(), State_Running);
            let mut pending = 0;
            control.GetChangeState(&mut pending).unwrap();
            assert_eq!(pending, 1);
            control.StartChanges().unwrap();
            let mut current = 0;
            frequency.Frequency(&mut current).unwrap();
            assert_eq!(current, 521143);
            control.GetChangeState(&mut pending).unwrap();
            assert_eq!(pending, 0);
            // Stop must not erase the committed tuning configuration.
            filter.Stop().unwrap();
            control.StartChanges().unwrap();
            frequency.Frequency(&mut current).unwrap();
            assert_eq!(current, 521143);
        }
    }
    #[test]
    fn demodulator_exposes_auto_controls_and_rejects_forced_parameters() {
        unsafe {
            let filter = crate::create_filter(false);
            let demod: IBDA_DigitalDemodulator = filter.cast().unwrap();
            demod.SetModulationType(&BDA_MOD_NOT_SET).unwrap();
            demod
                .SetInnerFECMethod(&BDA_FEC_METHOD_NOT_DEFINED)
                .unwrap();
            demod
                .SetSpectralInversion(&BDA_SPECTRAL_INVERSION_AUTOMATIC)
                .unwrap();
            assert_eq!(
                demod.SetModulationType(&BDA_MOD_256QAM).unwrap_err().code(),
                E_NOTIMPL
            );
            assert_eq!(
                demod
                    .SetModulationType(std::ptr::null())
                    .unwrap_err()
                    .code(),
                E_POINTER
            );
            let topology: IBDA_Topology = filter.cast().unwrap();
            let mut count = 0;
            let mut ids = [GUID::zeroed(); 2];
            topology.GetNodeInterfaces(1, &mut count, &mut ids).unwrap();
            assert_eq!(count, 2);
            assert!(ids.contains(&IBDA_DigitalDemodulator::IID));
        }
    }
}
fn read_value<T: Copy>(p: *const T) -> Result<T> {
    if p.is_null() {
        Err(Error::from_hresult(E_POINTER))
    } else {
        Ok(unsafe { p.read_unaligned() })
    }
}
impl IBDA_DigitalDemodulator_Impl for Filter_Impl {
    fn SetModulationType(&self, p: *const ModulationType) -> Result<()> {
        automatic(read_value(p)?.0, "modulation")
    }
    fn ModulationType(&self, p: *mut ModulationType) -> Result<()> {
        put(p, BDA_MOD_NOT_SET)
    }
    fn SetInnerFECMethod(&self, p: *const FECMethod) -> Result<()> {
        automatic(read_value(p)?.0, "inner FEC")
    }
    fn InnerFECMethod(&self, p: *mut FECMethod) -> Result<()> {
        put(p, BDA_FEC_METHOD_NOT_SET)
    }
    fn SetInnerFECRate(&self, p: *const BinaryConvolutionCodeRate) -> Result<()> {
        automatic(read_value(p)?.0, "inner rate")
    }
    fn InnerFECRate(&self, p: *mut BinaryConvolutionCodeRate) -> Result<()> {
        put(p, BDA_BCC_RATE_NOT_SET)
    }
    fn SetOuterFECMethod(&self, p: *const FECMethod) -> Result<()> {
        automatic(read_value(p)?.0, "outer FEC")
    }
    fn OuterFECMethod(&self, p: *mut FECMethod) -> Result<()> {
        put(p, BDA_FEC_METHOD_NOT_SET)
    }
    fn SetOuterFECRate(&self, p: *const BinaryConvolutionCodeRate) -> Result<()> {
        automatic(read_value(p)?.0, "outer rate")
    }
    fn OuterFECRate(&self, p: *mut BinaryConvolutionCodeRate) -> Result<()> {
        put(p, BDA_BCC_RATE_NOT_SET)
    }
    fn SetSymbolRate(&self, p: *const u32) -> Result<()> {
        let v = read_value(p)?;
        if v == 0 || v == u32::MAX {
            Ok(())
        } else {
            Err(unsupported())
        }
    }
    fn SymbolRate(&self, p: *mut u32) -> Result<()> {
        put(p, 0)
    }
    fn SetSpectralInversion(&self, p: *const SpectralInversion) -> Result<()> {
        let v = read_value(p)?;
        if v == BDA_SPECTRAL_INVERSION_AUTOMATIC {
            Ok(())
        } else {
            automatic(v.0, "inversion")
        }
    }
    fn SpectralInversion(&self, p: *mut SpectralInversion) -> Result<()> {
        put(p, BDA_SPECTRAL_INVERSION_AUTOMATIC)
    }
}
fn list<T: Copy>(values: &[T], count: *mut u32, max: u32, out: *mut T) -> Result<()> {
    put(count, values.len() as u32)?;
    if max < values.len() as u32 {
        return Err(Error::from_hresult(HRESULT::from_win32(234)));
    }
    if !values.is_empty() && out.is_null() {
        return Err(Error::from_hresult(E_POINTER));
    }
    if !values.is_empty() {
        unsafe {
            std::ptr::copy_nonoverlapping(values.as_ptr(), out, values.len());
        }
    }
    Ok(())
}
impl IBDA_Topology_Impl for Filter_Impl {
    fn GetNodeTypes(&self, n: *mut u32, max: u32, out: *mut u32) -> Result<()> {
        trace("GetNodeTypes");
        list(&[0, 1], n, max, out)
    }
    fn GetNodeDescriptors(
        &self,
        n: *mut u32,
        max: u32,
        out: *mut BDANODE_DESCRIPTOR,
    ) -> Result<()> {
        list(
            &[
                BDANODE_DESCRIPTOR {
                    ulBdaNodeType: 0,
                    guidFunction: KSNODE_BDA_RF_TUNER,
                    guidName: KSNODE_BDA_RF_TUNER,
                },
                BDANODE_DESCRIPTOR {
                    ulBdaNodeType: 1,
                    guidFunction: KSNODE_BDA_COFDM_DEMODULATOR,
                    guidName: KSNODE_BDA_COFDM_DEMODULATOR,
                },
            ],
            n,
            max,
            out,
        )
    }
    fn GetNodeInterfaces(&self, node: u32, n: *mut u32, max: u32, out: *mut GUID) -> Result<()> {
        trace(format!("GetNodeInterfaces {node}"));
        match node {
            0 => list(
                &[IBDA_FrequencyFilter::IID, IBDA_SignalStatistics::IID],
                n,
                max,
                out,
            ),
            1 => list(
                &[IBDA_DigitalDemodulator::IID, IBDA_SignalStatistics::IID],
                n,
                max,
                out,
            ),
            _ => Err(invalid()),
        }
    }
    fn GetPinTypes(&self, n: *mut u32, max: u32, out: *mut u32) -> Result<()> {
        list(&[0, 1], n, max, out)
    }
    fn GetTemplateConnections(
        &self,
        n: *mut u32,
        max: u32,
        out: *mut BDA_TEMPLATE_CONNECTION,
    ) -> Result<()> {
        list(
            &[
                BDA_TEMPLATE_CONNECTION {
                    FromNodeType: u32::MAX,
                    FromNodePinType: 0,
                    ToNodeType: 0,
                    ToNodePinType: 0,
                },
                BDA_TEMPLATE_CONNECTION {
                    FromNodeType: 0,
                    FromNodePinType: 1,
                    ToNodeType: 1,
                    ToNodePinType: 0,
                },
                BDA_TEMPLATE_CONNECTION {
                    FromNodeType: 1,
                    FromNodePinType: 1,
                    ToNodeType: u32::MAX,
                    ToNodePinType: 1,
                },
            ],
            n,
            max,
            out,
        )
    }
    fn CreatePin(&self, kind: u32, id: *mut u32) -> Result<()> {
        if kind > 1 {
            return Err(invalid());
        }
        put(id, kind)
    }
    fn DeletePin(&self, _: u32) -> Result<()> {
        Err(unsupported())
    }
    fn SetMediaType(&self, _: u32, _: *const AM_MEDIA_TYPE) -> Result<()> {
        Err(unsupported())
    }
    fn SetMedium(&self, _: u32, _: *const REGPINMEDIUM) -> Result<()> {
        Err(unsupported())
    }
    fn CreateTopology(&self, input: u32, output: u32) -> Result<()> {
        trace(format!("CreateTopology {input} {output}"));
        if input == 0 && output == 1 {
            Ok(())
        } else {
            Err(invalid())
        }
    }
    fn GetControlNode(
        &self,
        input: u32,
        output: u32,
        node: u32,
        p: *mut Option<IUnknown>,
    ) -> Result<()> {
        trace(format!("GetControlNode {input} {output} {node}"));
        if p.is_null() {
            return Err(Error::from_hresult(E_POINTER));
        }
        unsafe { p.write(None) }
        if input != 0 || output != 1 || node > 1 {
            return Err(invalid());
        }
        unsafe { p.write(Some(self.to_interface())) }
        Ok(())
    }
}
impl IKsPropertySet_Impl for Filter_Impl {
    fn QuerySupported(&self, g: *const GUID, id: u32) -> Result<u32> {
        if g.is_null() {
            return Err(Error::from_hresult(E_POINTER));
        }
        let g = unsafe { *g };
        trace(format!("KS QuerySupported {g:?} id={id}"));
        if (g == AVER_APP && id <= 1) || (g == AVER_CONTROL && id == 6) {
            Ok(2)
        } else if g == KSPROPSETID_BdaFrequencyFilter && [0, 4, 5].contains(&id) {
            Ok(3)
        } else if g == KSPROPSETID_BdaSignalStats && [1, 2, 3, 4].contains(&id) {
            Ok(1)
        } else {
            Err(Error::from_hresult(HRESULT(0x80070490u32 as i32)))
        }
    }
    fn Set(
        &self,
        g: *const GUID,
        id: u32,
        _: *const c_void,
        _: u32,
        p: *const c_void,
        size: u32,
    ) -> Result<()> {
        if self.QuerySupported(g, id)? & 2 == 0 {
            return Err(unsupported());
        }
        if p.is_null() || size != 4 {
            return Err(invalid());
        }
        let v = unsafe { (p as *const u32).read_unaligned() };
        let g = unsafe { *g };
        trace(format!("KS Set {g:?} id={id} value={v}"));
        if g == AVER_APP || g == AVER_CONTROL {
            if v > 1 {
                return Err(invalid());
            }
            // A857 SetPower callers use 0 at activation and 1 at deactivation.
            if g == AVER_APP && id == 1 && v == 1 {
                self.shared.stop_stream();
            }
            // Activation initializes RF at the next BDA commit. This does not
            // claim device-wide suspend/resume support across separate clients.
            return Ok(());
        }
        match id {
            0 => self.SetFrequency(v),
            4 => self.SetBandwidth(v),
            5 => self.SetFrequencyMultiplier(v),
            _ => Err(unsupported()),
        }
    }
    fn Get(
        &self,
        g: *const GUID,
        id: u32,
        _: *const c_void,
        _: u32,
        p: *mut c_void,
        size: u32,
        returned: *mut u32,
    ) -> Result<()> {
        put(returned, 0)?;
        if self.QuerySupported(g, id)? & 1 == 0 {
            return Err(unsupported());
        }
        if p.is_null() || size < 4 {
            put(returned, 4)?;
            return Err(Error::from_hresult(HRESULT::from_win32(234)));
        }
        let g = unsafe { *g };
        let v = if g == KSPROPSETID_BdaFrequencyFilter {
            let t = *self.shared.tune.lock().unwrap();
            match id {
                0 => t.frequency,
                4 => t.bandwidth,
                5 => t.multiplier,
                _ => return Err(unsupported()),
            }
        } else {
            match id {
                1 => {
                    let mut quality = 0;
                    self.SignalQuality(&mut quality)?;
                    quality as u32
                }
                2 => self.shared.status.present.load(Ordering::Relaxed) as u32,
                3 => self.shared.status.locked.load(Ordering::Relaxed) as u32,
                4 => 100,
                _ => return Err(unsupported()),
            }
        };
        unsafe { (p as *mut u32).write_unaligned(v) }
        put(returned, 4)
    }
}

//! Uses the real Windows DVB-T Network Provider to verify the antenna connection.
use windows::Win32::Media::DirectShow::Tv::*;
use windows::{
    core::*,
    Win32::{
        Media::{
            DirectShow::*,
            MediaFoundation::{
                CLSID_DVBTNetworkProvider, CLSID_FileWriter, CLSID_FilterGraph, CLSID_InfTee,
                CLSID_MPEG2Demultiplexer, AM_MEDIA_TYPE,
            },
        },
        System::Com::*,
    },
};
unsafe fn free_pin(filter: &IBaseFilter, direction: PIN_DIRECTION) -> Result<IPin> {
    let pins = filter.EnumPins()?;
    loop {
        let mut next = [None];
        if pins.Next(&mut next, None).is_err() || next[0].is_none() {
            break;
        }
        let pin = next[0].take().unwrap();
        if pin.QueryDirection()? == direction && pin.ConnectedTo().is_err() {
            return Ok(pin);
        }
    }
    Err(Error::new(
        windows::Win32::Foundation::E_FAIL,
        "No unconnected pin",
    ))
}
fn main() -> Result<()> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
        let graph: IGraphBuilder =
            CoCreateInstance(&CLSID_FilterGraph, None, CLSCTX_INPROC_SERVER)?;
        let provider: IBaseFilter =
            CoCreateInstance(&CLSID_DVBTNetworkProvider, None, CLSCTX_INPROC_SERVER)?;
        let space: IDVBTuningSpace = CoCreateInstance(&DVBTuningSpace, None, CLSCTX_INPROC_SERVER)?;
        space.SetNetworkType(&BSTR::from("{216C62DF-6D7F-4E9A-8571-05F14EDB766A}"))?;
        space.SetSystemType(DVB_Terrestrial)?;
        let locator: IDVBTLocator = CoCreateInstance(&DVBTLocator, None, CLSCTX_INPROC_SERVER)?;
        locator.SetCarrierFrequency(521143)?;
        locator.SetBandwidth(6)?;
        space.SetDefaultLocator(&locator)?;
        let tv: ITuner = provider.cast()?;
        tv.SetTuningSpace(&space)?;
        let tuner = a865r_bda::create_filter(false);
        graph.AddFilter(&provider, w!("Network Provider"))?;
        graph.AddFilter(&tuner, w!("A865R combined tuner"))?;
        let pins = provider.EnumPins()?;
        let mut pin = [None];
        pins.Next(&mut pin, None).ok()?;
        let input = tuner.FindPin(w!("0"))?;
        let output = pin[0].as_ref().unwrap();
        println!(
            "Pin directions: {:?} -> {:?}",
            output.QueryDirection(),
            input.QueryDirection()
        );

        let result = graph.ConnectDirect(output, &input, None);
        println!("Windows Network Provider -> A865R antenna: {result:?}");
        if result.is_ok() {
            if let Some(path) = std::env::args().nth(1) {
                if std::path::Path::new(&path).exists() {
                    return Err(Error::new(
                        windows::Win32::Foundation::E_INVALIDARG,
                        "Output already exists",
                    ));
                }
                let writer: IBaseFilter =
                    CoCreateInstance(&CLSID_FileWriter, None, CLSCTX_INPROC_SERVER)?;
                writer
                    .cast::<IFileSinkFilter>()?
                    .SetFileName(&HSTRING::from(&path), None)?;
                graph.AddFilter(&writer, w!("TS recording"))?;
                let mut inputs = [None];
                writer.EnumPins()?.Next(&mut inputs, None).ok()?;
                let writer_input = inputs[0].as_ref().unwrap();
                let transport = tuner.FindPin(w!("1"))?;
                let tee: IBaseFilter = CoCreateInstance(&CLSID_InfTee, None, CLSCTX_INPROC_SERVER)?;
                let demux: IBaseFilter =
                    CoCreateInstance(&CLSID_MPEG2Demultiplexer, None, CLSCTX_INPROC_SERVER)?;
                graph.AddFilter(&tee, w!("TS branches"))?;
                graph.AddFilter(&demux, w!("System demultiplexer"))?;
                graph.ConnectDirect(&transport, &free_pin(&tee, PINDIR_INPUT)?, None)?;
                graph.ConnectDirect(&free_pin(&tee, PINDIR_OUTPUT)?, writer_input, None)?;
                graph.ConnectDirect(
                    &free_pin(&tee, PINDIR_OUTPUT)?,
                    &free_pin(&demux, PINDIR_INPUT)?,
                    None,
                )?;
                // The legacy DVB-T provider delegates locator interpretation to
                // its TIF. Without registration it can return S_OK without ever
                // forwarding a frequency (Locate internally returns E_NOINTERFACE).
                let tif: IBaseFilter = CoCreateInstance(
                    &GUID::from_u128(0xfc772ab0_0c7f_11d3_8ff2_00a0c9224cf4),
                    None,
                    CLSCTX_INPROC_SERVER,
                )?;
                graph.AddFilter(&tif, w!("Transport information"))?;
                let tif_input = free_pin(&tif, PINDIR_INPUT)?;
                let mut types: [*mut AM_MEDIA_TYPE; 1] = [std::ptr::null_mut()];
                tif_input.EnumMediaTypes()?.Next(&mut types, None).ok()?;
                if types[0].is_null() {
                    return Err(Error::from_hresult(windows::Win32::Foundation::E_FAIL));
                }
                let mpeg: IMpeg2Demultiplexer = demux.cast()?;
                let sections = mpeg.CreateOutputPin(types[0], w!("Sections"))?;
                if !(*types[0]).pbFormat.is_null() {
                    CoTaskMemFree(Some((*types[0]).pbFormat.cast()));
                }
                std::mem::ManuallyDrop::drop(&mut (*types[0]).pUnk);
                CoTaskMemFree(Some(types[0].cast()));
                let pids: IMPEG2PIDMap = sections.cast()?;
                let tables = [0, 1, 0x10, 0x11, 0x12, 0x14];
                pids.MapPID(tables.len() as u32, tables.as_ptr(), MEDIA_MPEG2_PSI)?;
                graph.ConnectDirect(&sections, &tif_input, None)?;
                let request = space.CreateTuneRequest()?;
                request.SetLocator(&locator)?;
                let control: IMediaControl = graph.cast()?;
                control.Pause()?;
                tv.SetTuneRequest(&request)?;
                let stats: IBDA_SignalStatistics = tuner.cast()?;
                let mut locked = 0;
                stats.SignalLocked(&mut locked)?;
                println!("Windows Network Provider tune 521143 kHz: locked={locked}");
                let run = control.Run();
                if run.is_ok() {
                    std::thread::sleep(std::time::Duration::from_secs(5));
                }
                let stop = control.Stop();
                run?;
                stop?;
                graph.Disconnect(&sections)?;
                graph.Disconnect(&tif_input)?;
                graph.RemoveFilter(&tif)?;
                graph.Disconnect(&transport)?;
                graph.Disconnect(writer_input)?;
                graph.RemoveFilter(&writer)?;
                let bytes = std::fs::metadata(&path)
                    .map_err(|e| Error::new(windows::Win32::Foundation::E_FAIL, e.to_string()))?
                    .len();
                println!("Network-provider recording: {bytes} bytes");
                if locked == 0 || bytes == 0 {
                    return Err(Error::new(
                        windows::Win32::Foundation::E_FAIL,
                        "No locked transport stream",
                    ));
                }
            }
        }
        if result.is_ok() {
            graph.Disconnect(output)?;
            graph.Disconnect(&input)?;
        }

        graph.RemoveFilter(&tuner)?;
        graph.RemoveFilter(&provider)?;
        result
    }
}

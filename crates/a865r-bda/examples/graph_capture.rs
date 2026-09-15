//! Independent DirectShow client: drives only COM TV interfaces, not Device/Receiver.
use windows::{
    core::*,
    Win32::{
        Media::{
            DirectShow::*,
            MediaFoundation::{CLSID_FileWriter, CLSID_FilterGraph},
        },
        System::Com::*,
    },
};
fn main() -> Result<()> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
        let args: Vec<_> = std::env::args().collect();
        if args.len() != 4 {
            println!("graph_capture <frequency-khz> <seconds> <new-recording.ts>");
            return Ok(());
        }
        let frequency: u32 = args[1]
            .parse()
            .map_err(|_| Error::from_hresult(windows::Win32::Foundation::E_INVALIDARG))?;
        let seconds: u64 = args[2]
            .parse()
            .map_err(|_| Error::from_hresult(windows::Win32::Foundation::E_INVALIDARG))?;
        if seconds == 0 || seconds > 120 || std::path::Path::new(&args[3]).exists() {
            return Err(Error::from_hresult(
                windows::Win32::Foundation::E_INVALIDARG,
            ));
        }
        let graph: IGraphBuilder =
            CoCreateInstance(&CLSID_FilterGraph, None, CLSCTX_INPROC_SERVER)?;
        let tuner = a865r_bda::create_filter(false);
        let capture = a865r_bda::create_filter(true);
        let writer: IBaseFilter = CoCreateInstance(&CLSID_FileWriter, None, CLSCTX_INPROC_SERVER)?;
        let file: IFileSinkFilter = writer.cast()?;
        file.SetFileName(&HSTRING::from(&args[3]), None)?;
        graph.AddFilter(&tuner, w!("Open A865R tuner"))?;
        graph.AddFilter(&capture, w!("Open A865R capture"))?;
        graph.AddFilter(&writer, w!("TS file"))?;
        let output = tuner.FindPin(w!("1"))?;
        let input = capture.FindPin(w!("0"))?;
        graph.ConnectDirect(&output, &input, None)?;
        let output2 = capture.FindPin(w!("1"))?;
        let pins = writer.EnumPins()?;
        let mut pin = [None];
        pins.Next(&mut pin, None).ok()?;
        let writer_input = pin[0].as_ref().unwrap();
        graph.ConnectDirect(&output2, writer_input, None)?;
        let topology: IBDA_Topology = tuner.cast()?;
        let mut node = None;
        topology.GetControlNode(0, 1, 0, &mut node)?;
        let frequency_control: IBDA_FrequencyFilter = node.as_ref().unwrap().cast()?;
        let controls: IBDA_DeviceControl = tuner.cast()?;
        // Reproduce AVerTV's preliminary unsupported tune, then discard it.
        controls.StartChanges()?;
        frequency_control.SetFrequency(123350)?;
        if controls.CheckChanges().is_ok() {
            return Err(Error::new(
                windows::Win32::Foundation::E_FAIL,
                "Unsupported draft was accepted",
            ));
        }
        controls.StartChanges()?;
        frequency_control.SetFrequencyMultiplier(1000)?;
        frequency_control.SetFrequency(frequency)?;
        frequency_control.SetBandwidth(6)?;
        controls.CheckChanges()?;
        controls.CommitChanges()?;
        let stats: IBDA_SignalStatistics = tuner.cast()?;
        let mut locked = 0;
        stats.SignalLocked(&mut locked)?;
        println!("BDA signal_locked={locked}");
        let media: IMediaControl = graph.cast()?;
        let result = (|| -> Result<()> {
            media.Run()?;
            std::thread::sleep(std::time::Duration::from_secs(seconds));
            Ok(())
        })();
        let stop = media.Stop();
        result?;
        stop?;
        graph.Disconnect(&output)?;
        graph.Disconnect(&input)?;
        graph.Disconnect(&output2)?;
        graph.Disconnect(writer_input)?;
        graph.RemoveFilter(&writer)?;
        graph.RemoveFilter(&capture)?;
        graph.RemoveFilter(&tuner)?;
        let bytes = std::fs::metadata(&args[3])
            .map_err(|e| Error::new(windows::Win32::Foundation::E_FAIL, e.to_string()))?
            .len();
        if bytes == 0 {
            return Err(Error::new(
                windows::Win32::Foundation::E_FAIL,
                "No transport stream was delivered",
            ));
        }
        println!("DirectShow recording complete: {} ({bytes} bytes)", args[3]);
        Ok(())
    }
}

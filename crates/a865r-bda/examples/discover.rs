use windows::{
    core::*,
    Win32::{Media::DirectShow::*, System::Com::*},
};
fn main() -> Result<()> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
        let dev: ICreateDevEnum = CoCreateInstance(
            &GUID::from_u128(0x62be5d10_60eb_11d0_bd3b_00a0c911ce86),
            None,
            CLSCTX_INPROC_SERVER,
        )?;
        for category in [
            GUID::from_u128(0x71985f48_1ca1_11d3_9cc8_00c04f7971e0),
            GUID::from_u128(0xfd0a5af4_b41d_11d2_9c95_00c04f7971e0),
        ] {
            let mut list = None;
            dev.CreateClassEnumerator(&category, &mut list, 0)?;
            println!("Category {category:?}");
            if let Some(list) = list {
                loop {
                    let mut monikers = [None];
                    if list.Next(&mut monikers, None) != windows::Win32::Foundation::S_OK {
                        break;
                    }
                    let m = monikers[0].as_ref().unwrap();
                    let ctx = CreateBindCtx(0)?;
                    let name = m.GetDisplayName(&ctx, None)?;
                    println!("  {}", name.to_string()?);
                    CoTaskMemFree(Some(name.0.cast()));
                    let filter: Result<IBaseFilter> = m.BindToObject(&ctx, None);
                    println!("  bind: {:?}", filter.as_ref().map(|_| "IBaseFilter"));
                }
            }
        }
        Ok(())
    }
}

//! Processing resolution is independent of the size of the viewing area.
pub fn geometry(source: (u32, u32), window: (u32, u32), cap: (u32, u32)) -> ([u32; 4], (u32, u32)) {
    let fit = (window.0 as f64 / source.0 as f64).min(window.1 as f64 / source.1 as f64);
    let width = (source.0 as f64 * fit).round().max(1.) as u32;
    let height = (source.1 as f64 * fit).round().max(1.) as u32;
    let processing = fit
        .min(cap.0 as f64 / source.0 as f64)
        .min(cap.1 as f64 / source.1 as f64);
    (
        [
            (window.0 - width) / 2,
            (window.1 - height) / 2,
            width,
            height,
        ],
        (
            (source.0 as f64 * processing).round().max(1.) as u32,
            (source.1 as f64 * processing).round().max(1.) as u32,
        ),
    )
}

#[cfg(test)] mod tests {
    use super::geometry;
    #[test] fn resolution_caps_preserve_proportions_in_every_window() {
        for ratio in [(4,3),(16,9),(16,10),(5,4)] {
            for window in [(977,671),(3840,2160),(1280,1024),(720,1280)] {
                for cap in [(1920,1080),(2560,1440),(3840,2160)] {
                    let ([x,y,w,h],(pw,ph))=geometry(ratio,window,cap);
                    assert!(x+w<=window.0 && y+h<=window.1);
                    assert!(pw<=cap.0 && ph<=cap.1);
                    // Only whole-pixel rounding may change the exact ratio.
                    assert!((w as i64*ratio.1 as i64-h as i64*ratio.0 as i64).abs() <= (ratio.0+ratio.1) as i64);
                    assert!((pw as i64*ratio.1 as i64-ph as i64*ratio.0 as i64).abs() <= (ratio.0+ratio.1) as i64);
                }
            }
        }
        assert_eq!(geometry((4,3),(1920,1080),(3840,2160)).0,[240,0,1440,1080]);
        assert_eq!(geometry((16,9),(1280,1024),(3840,2160)).0,[0,152,1280,720]);
    }
}

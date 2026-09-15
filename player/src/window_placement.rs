//! Initial placement in physical pixels. Both windows retain their own proportions.
#[derive(Debug, Clone, Copy)]
pub struct Rect { pub x: i32, pub y: i32, pub width: i32, pub height: i32 }

pub fn centered_pair(work: Rect, dpi: u32) -> [Rect; 2] {
    let d = dpi.max(96) as f64 / 96.0;
    let margin = (16.0 * d).round() as i32;
    let width = (work.width - 2 * margin).max(2);
    let height = (work.height - 2 * margin).max(3);
    let scale = d.min(width as f64 / 1120.0).min(height as f64 / (770.0 + 354.0 - 76.0));
    let px = |n: f64| (n * scale).floor().max(1.0) as i32;
    // Overlap the lower decorative margin, as in the approved startup arrangement.
    let (vw, vh, dw, dh, overlap) = (px(1120.0), px(770.0), px(1088.0), px(354.0), px(76.0));
    let top = work.y + (work.height - vh + overlap - dh) / 2;
    [Rect { x: work.x + (work.width - vw) / 2, y: top, width: vw, height: vh },
     Rect { x: work.x + (work.width - dw) / 2, y: top + vh - overlap, width: dw, height: dh }]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn centered_visible_and_slightly_overlapped_on_common_screens() {
        for (x, y, w, h, dpi) in [(0,0,1920,1040,96), (0,0,1366,728,96),
            (0,40,1920,1040,144), (-2560,-200,2560,1400,144), (0,0,3840,2080,192),
            (0,0,1280,680,192), (40,0,1040,1880,120)] {
            let work = Rect { x, y, width:w, height:h };
            let [v,d] = centered_pair(work,dpi);
            for r in [v,d] {
                assert!(r.x >= x && r.y >= y && r.x+r.width <= x+w && r.y+r.height <= y+h);
                assert!((2*r.x+r.width-(2*x+w)).abs() <= 1);
            }
            let overlap=v.y+v.height-d.y;
            assert!(overlap > 0 && overlap < v.height/8);
            assert!((overlap as f64/v.height as f64-76.0/770.0).abs()<0.01);
            assert!((v.y + d.y + d.height - (2*y+h)).abs() <= 1);
            assert!((v.width as f64/v.height as f64-1120.0/770.0).abs()<0.01);
            assert!((d.width as f64/d.height as f64-1088.0/354.0).abs()<0.02);
        }
    }
}

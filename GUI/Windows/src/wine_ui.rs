//! Modest readability adjustment for Wine; native Windows retains its sizing.
pub fn pixels(value:i32)->i32 {
    static WINE:std::sync::OnceLock<bool>=std::sync::OnceLock::new();
    if *WINE.get_or_init(a865r::transport::wine_bridge::is_wine) {(value*110+50)/100}else{value}
}
pub fn dpi(value:u32)->u32 {pixels(value as i32) as u32}

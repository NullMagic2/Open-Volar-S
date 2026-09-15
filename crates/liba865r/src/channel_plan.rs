//! Geography-independent scan lists for the supported 6 MHz ISDB-T UHF receiver.
use crate::{Error, Result};

pub const MIN_FREQUENCY_KHZ: u32 = 470_000;
pub const MAX_FREQUENCY_KHZ: u32 = 697_999;

pub fn validate_frequency(frequency: u32) -> Result<()> {
    if !(MIN_FREQUENCY_KHZ..=MAX_FREQUENCY_KHZ).contains(&frequency) {
        return Err(Error::InvalidArgument(format!("Supported ISDB-T UHF range is {MIN_FREQUENCY_KHZ}..{MAX_FREQUENCY_KHZ} kHz, at 6 MHz bandwidth")));
    }
    Ok(())
}

/// Custom centers are independent of country and channel numbering. Frequencies include any offset.
pub fn custom_scan(first_khz: u32, last_khz: u32, step_khz: u32) -> Result<Vec<u32>> {
    validate_frequency(first_khz)?;
    validate_frequency(last_khz)?;
    if first_khz > last_khz || step_khz == 0 {
        return Err(Error::InvalidArgument(
            "Scan requires first <= last and a positive step".into(),
        ));
    }
    let count = (last_khz - first_khz) / step_khz + 1;
    if count > 256 {
        return Err(Error::InvalidArgument(
            "A scan may contain at most 256 frequencies".into(),
        ));
    }
    Ok((0..count).map(|i| first_khz + i * step_khz).collect())
}

pub fn brazil_uhf() -> Vec<u32> {
    (0..38).map(|i| 473_143 + i * 6000).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn custom_centers_need_no_city_or_channel_number() {
        assert_eq!(
            custom_scan(500_000, 518_000, 6000).unwrap(),
            [500_000, 506_000, 512_000, 518_000]
        );
        assert_eq!(custom_scan(521_143, 521_143, u32::MAX).unwrap(), [521_143]);
        assert_eq!(brazil_uhf().last(), Some(&695_143));
    }
    #[test]
    fn invalid_and_unbounded_scans_are_rejected() {
        for (a, b, s) in [
            (1, 521143, 6000),
            (521143, 800000, 6000),
            (521143, 500000, 6000),
            (500000, 600000, 0),
            (470000, 697999, 1),
        ] {
            assert!(custom_scan(a, b, s).is_err());
        }
    }
}

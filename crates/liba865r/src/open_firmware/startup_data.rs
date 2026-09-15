//! Numeric ROM ABI tables reconstructed from the board's reference download.
//! These are calibration/lookup data, not executable vendor firmware. Units of
//! unidentified coefficient rows are intentionally not guessed.
pub(super) fn tables() -> Vec<u8> {
    let mut bytes = Vec::new();
    for v in [
        0x0085u16, 0x0716, 0x0CD4, 0x0280, 0x08BC, 0x0EB1, 0x0478, 0x0AEE,
    ] {
        bytes.extend_from_slice(&v.to_be_bytes());
    }
    for v in [
        0x9F59u16, 0x9F92, 0x9F95, 0x9F96, 0x9F97, 0x99F5, 0x9F98, 0x9F99, 0x9F9A, 0x9FA7, 0x9FFC,
        0x9A85, 0xA03B,
    ] {
        bytes.extend_from_slice(&v.to_be_bytes());
    }
    for bits in 1..=8 {
        bytes.push(((1u16 << bits) - 1) as u8);
    }
    bytes.extend_from_slice(&[0, 38, 68, 94, 119, 144, 168, 193]);
    bytes.extend_from_slice(&[0, 1, 2, 4, 6, 10, 14, 22]);
    bytes.extend_from_slice(&[32, 25, 20, 13, 8, 3, 1, 0]);
    bytes.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 9]);
    bytes.extend_from_slice(&[0, 1, 2, 3, 4, 5, 5, 3, 8, 1, 0, 1, 1, 2, 3, 3, 1, 1, 1, 1]);
    for v in [
        16000u16, 1600, 32000, 4800, 4800, 64000, 4800, 48000, 44000, 160,
    ] {
        bytes.extend_from_slice(&v.to_be_bytes());
    }
    bytes.extend_from_slice(&[0, 0, 1, 2, 2, 2, 2, 2, 1, 1]);
    let curves: &[&[u8]] = &[
        &[59, 60, 61, 63, 66, 70, 75, 80, 85, 89, 92, 95, 97, 98, 100],
        &[40, 45, 49, 54, 59, 65, 71, 77, 82, 87, 91, 94, 97, 98, 100],
        &[
            42, 43, 44, 45, 46, 47, 48, 49, 51, 53, 57, 61, 66, 71, 77, 83, 88, 91, 94, 96, 97, 98,
            100,
        ],
        &[
            49, 51, 53, 57, 61, 65, 70, 75, 80, 85, 89, 92, 95, 97, 98, 99,
        ],
    ];
    for curve in curves {
        bytes.extend_from_slice(curve);
        bytes.extend(std::iter::repeat(255).take(24 - curve.len()));
    }
    assert_eq!(bytes.len(), 230);
    bytes
}

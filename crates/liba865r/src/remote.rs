//! Infrared input from the tuner's firmware, independent of AVerRemote.
//! Protocol framing follows the documented ITE command interface; a decoded
//! word cannot prove support for arbitrary IR pulse protocols.
use crate::{protocol::Protocol, Result, Transport};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InfraredCode {
    pub raw: [u8; 4],
}
impl InfraredCode {
    /// Stable key for learned mappings. Preserve all address and command bytes.
    pub fn key(self) -> String {
        self.raw.iter().map(|b| format!("{b:02X}")).collect()
    }
    /// A candidate only: the firmware and a physical handset must confirm NEC.
    pub fn nec_candidate(self) -> Option<(u16, u8)> {
        let [a, b, c, d] = self.raw;
        if c ^ d != 0xff {
            return None;
        }
        Some((
            if a ^ b == 0xff {
                a as u16
            } else {
                u16::from_be_bytes([a, b])
            },
            c,
        ))
    }
}
pub struct Infrared<'a> {
    protocol: Protocol<'a>,
}
impl<'a> Infrared<'a> {
    pub(crate) fn new(transport: &'a mut dyn Transport) -> Self {
        Self {
            protocol: Protocol::new(transport),
        }
    }
    pub fn poll(&mut self) -> Result<Option<InfraredCode>> {
        self.protocol
            .poll_infrared()
            .map(|value| value.map(|raw| InfraredCode { raw }))
    }
}

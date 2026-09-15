//! Summary: Verifies protocol framing, checksum validation, register chunking, EEPROM parsing, and TS alignment.

use a865r::eeprom::{layout_for_chip, EepromInfo, EepromLayout};
use a865r::protocol::{Command, Protocol};
use a865r::transport::{BulkPipe, Transport};
use a865r::{
    execution_probe_image, Error, FirmwareCore, FirmwareImage, Result, TsAnalyzer, OPEN_LINK_ENTRY,
    OPEN_LINK_VERSION, OPEN_PROBE_ENTRY, OPEN_PROBE_MARKER_ADDRESS, OPEN_PROBE_MARKER_VALUE,
};
use std::collections::VecDeque;

/// Minimal deterministic transport used to test protocol code without physical hardware.
struct MockTransport {
    pipes: Vec<BulkPipe>,
    responses: VecDeque<Vec<u8>>,
    requests: Vec<Vec<u8>>,
}

impl MockTransport {
    /// Creates a mock with caller-supplied response frames.
    fn new(responses: Vec<Vec<u8>>) -> Self {
        Self {
            pipes: Vec::new(),
            responses: responses.into(),
            requests: Vec::new(),
        }
    }
}

impl Transport for MockTransport {
    /// Performs no real device opening.
    fn open(&mut self, _vendor_id: u16, _product_id: u16) -> Result<()> {
        Ok(())
    }

    fn cycle_port(&mut self) -> Result<()> {
        Ok(())
    }

    /// Returns the synthetic endpoint list.
    fn bulk_pipes(&self) -> &[BulkPipe] {
        &self.pipes
    }

    /// Accepts any synthetic endpoint pair.
    fn set_command_pipes(&mut self, _output_pipe: u8, _input_pipe: u8) -> Result<()> {
        Ok(())
    }

    /// Records requests and returns the next preloaded frame.
    fn exchange(
        &mut self,
        request: &[u8],
        _expected_response_size: usize,
        _timeout_ms: u32,
    ) -> Result<Vec<u8>> {
        self.requests.push(request.to_vec());
        self.responses
            .pop_front()
            .ok_or_else(|| Error::Transport("mock response queue is empty".to_string()))
    }

    /// Returns no stream data in protocol tests.
    fn read_bulk(
        &mut self,
        _input_pipe: u8,
        _requested_size: usize,
        _timeout_ms: u32,
    ) -> Result<Vec<u8>> {
        Ok(Vec::new())
    }

    /// Identifies this backend in diagnostics.
    fn description(&self) -> String {
        "mock".to_string()
    }
}

/// Builds a valid response frame matching the protocol's checksum rules.
fn response_frame(sequence: u8, status: u8, payload: &[u8]) -> Vec<u8> {
    let mut frame = vec![(payload.len() + 4) as u8, sequence, status];
    frame.extend_from_slice(payload);
    let checksum = Protocol::checksum(&frame).unwrap();
    frame.extend_from_slice(&checksum.to_be_bytes());
    frame
}

#[test]
fn infrared_empty_replies_still_require_valid_checksum_and_sequence() {
    for payload in [&[][..], &[0, 0, 0, 0][..]] {
        let frame = response_frame(7, 1, payload);
        assert_eq!(Protocol::decode_infrared_response(&frame, 7).unwrap(), None);
        assert!(Protocol::decode_infrared_response(&frame, 8).is_err());
        let mut damaged = frame.clone();
        *damaged.last_mut().unwrap() ^= 1;
        assert!(Protocol::decode_infrared_response(&damaged, 7).is_err());
    }
    assert!(Protocol::decode_infrared_response(&response_frame(0, 2, &[]), 0).is_err());
    assert!(Protocol::decode_infrared_response(&response_frame(0, 1, &[1]), 0).is_err());
}

#[test]
fn infrared_poll_preserves_codes_and_serializes_sequence() {
    let code = [0x12, 0xed, 0x34, 0xcb];
    let mut transport =
        MockTransport::new(vec![response_frame(0, 0, &code), response_frame(1, 1, &[])]);
    let mut protocol = Protocol::new(&mut transport);
    assert_eq!(protocol.poll_infrared().unwrap(), Some(code));
    assert_eq!(protocol.poll_infrared().unwrap(), None);
    assert_eq!(
        transport.requests[0],
        Protocol::encode_request(0, Command::InfraredGet, 0, &[]).unwrap()
    );
    assert_eq!(transport.requests[1][3], 1);
    let key = a865r::remote::InfraredCode { raw: code };
    assert_eq!(key.key(), "12ED34CB");
    assert_eq!(key.nec_candidate(), Some((0x12, 0x34)));
    assert_eq!(
        a865r::remote::InfraredCode { raw: [1, 2, 3, 4] }.nec_candidate(),
        None
    );
}

#[test]
fn firmware_query_encoding_matches_reference_frame() {
    let frame = Protocol::encode_request(0, Command::FirmwareQueryInfo, 0, &[1]).unwrap();
    assert_eq!(frame, vec![0x06, 0x00, 0x22, 0x00, 0x01, 0xFF, 0xDC]);
}

#[test]
fn response_checksum_and_payload_are_validated() {
    let frame = response_frame(7, 0, &[1, 2, 3, 4]);
    assert_eq!(
        Protocol::decode_response(&frame, 4).unwrap(),
        vec![1, 2, 3, 4]
    );
}

#[test]
fn ofdm_firmware_query_uses_the_forwarded_mailbox() {
    let response = response_frame(0, 0, &[0, 1, 0, 0]);
    let mut transport = MockTransport::new(vec![response]);
    {
        let mut protocol = Protocol::new(&mut transport);
        assert_eq!(
            protocol
                .query_processor_firmware_version(0x80, 400)
                .unwrap(),
            [0, 1, 0, 0]
        );
    }
    assert_eq!(transport.requests[0][1], 0x80);
    assert_eq!(transport.requests[0][2], Command::FirmwareQueryInfo as u8);
    assert_eq!(transport.requests[0][4], 1);
}

#[test]
fn bad_status_is_rejected() {
    let frame = response_frame(0, 5, &[]);
    assert!(Protocol::decode_response(&frame, 0).is_err());
}

#[test]
fn short_error_response_preserves_device_status() {
    let frame = response_frame(0, 5, &[]);
    let err = Protocol::decode_response(&frame, 4).unwrap_err();
    assert!(err.to_string().contains("status 0x05"));
}

#[test]
fn oversized_response_is_rejected_before_usb_io() {
    let mut transport = MockTransport::new(vec![]);
    let result = Protocol::new(&mut transport).transact(0, Command::MemoryRead, &[], usize::MAX, 1);
    assert!(result.is_err());
    assert!(transport.requests.is_empty());
    assert!(Protocol::decode_response(&[], usize::MAX).is_err());
}

#[test]
fn register_transfers_split_at_mailbox_boundary() {
    let mut transport =
        MockTransport::new(vec![response_frame(0, 0, &[1]), response_frame(1, 0, &[2])]);
    assert_eq!(
        Protocol::new(&mut transport)
            .read_registers(0xffff, 2)
            .unwrap(),
        vec![1, 2]
    );
    assert_eq!(transport.requests[0][1], 0);
    assert_eq!(&transport.requests[0][4..10], &[1, 2, 0, 0, 0xff, 0xff]);
    assert_eq!(transport.requests[1][1], 1);
    assert_eq!(&transport.requests[1][4..10], &[1, 2, 0, 0, 0, 0]);
    let mut transport =
        MockTransport::new(vec![response_frame(0, 0, &[]), response_frame(1, 0, &[])]);
    Protocol::new(&mut transport)
        .write_registers(0x80ffff, &[1, 2])
        .unwrap();
    assert_eq!(transport.requests[0][1], 0x80);
    assert_eq!(transport.requests[1][1], 0x81);
    assert_eq!(transport.requests[0][4], 1);
}

#[test]
fn scatter_parser_rejects_wrapping_core_address() {
    assert!(FirmwareImage::from_scatter_bytes(vec![3, 0, 0, 1, 0xff, 0xff, 2, 0, 0]).is_err());
}

#[test]
fn register_read_uses_expected_mailbox_and_address_bytes() {
    let response = response_frame(0, 0, &[0xAA, 0xBB, 0xCC]);
    let mut transport = MockTransport::new(vec![response]);
    {
        let mut protocol = Protocol::new(&mut transport);
        assert_eq!(
            protocol.read_registers(0x123456, 3).unwrap(),
            vec![0xAA, 0xBB, 0xCC]
        );
    }
    let request = &transport.requests[0];
    assert_eq!(request[1], 0x12);
    assert_eq!(request[2], Command::MemoryRead as u8);
    assert_eq!(&request[4..10], &[3, 2, 0, 0, 0x34, 0x56]);
}

#[test]
fn register_read_chunks_responses_to_the_64_byte_command_buffer() {
    let mut responses = Vec::new();
    for sequence in 0..5u8 {
        let length = if sequence == 4 { 20 } else { 59 };
        responses.push(response_frame(sequence, 0, &vec![sequence; length]));
    }
    let mut transport = MockTransport::new(responses);
    {
        let mut protocol = Protocol::new(&mut transport);
        let bytes = protocol.read_registers(0x004994, 256).unwrap();
        assert_eq!(bytes.len(), 256);
    }
    assert_eq!(transport.requests.len(), 5);
    assert_eq!(transport.requests[0][4], 59);
    assert_eq!(transport.requests[4][4], 20);
}

#[test]
fn register_write_chunks_payloads_to_the_64_byte_command_buffer() {
    let responses = vec![
        response_frame(0, 0, &[]),
        response_frame(1, 0, &[]),
        response_frame(2, 0, &[]),
    ];
    let mut transport = MockTransport::new(responses);
    {
        let mut protocol = Protocol::new(&mut transport);
        protocol.write_registers(0x001000, &[0x55; 120]).unwrap();
    }
    assert_eq!(transport.requests.len(), 3);
    assert_eq!(transport.requests[0][4], 52);
    assert_eq!(transport.requests[1][4], 52);
    assert_eq!(transport.requests[2][4], 16);
    assert!(transport.requests.iter().all(|request| request.len() <= 64));
}

#[test]
fn eeprom_summary_extracts_known_offsets() {
    let mut bytes = vec![0u8; 256];
    bytes[0x31] = 3;
    bytes[0x3C] = 0x60;
    bytes[0x38] = 0x34;
    bytes[0x39] = 0x12;
    let info = EepromInfo::from_bytes(EepromLayout::It9135, bytes).unwrap();
    assert!(info.summary.dual_mode);
    assert_eq!(info.summary.tuner_id, 0x60);
    assert_eq!(info.summary.tuner_if_khz, 0x1234);
}

#[test]
fn it9175_uses_the_it9135_eeprom_layout() {
    assert_eq!(layout_for_chip(0x9175), Some(EepromLayout::It9135));
}

#[test]
fn extracted_it9175_firmware_uses_the_avermedia_descriptor_boundaries() {
    let mut bytes = Vec::new();
    for index in 0..123 {
        let record_length = match index {
            10 => 35,
            122 => 20,
            _ => 48,
        };
        let data_length = record_length - 7;
        bytes.extend_from_slice(&[
            0x03,
            (index & 1) as u8,
            0x00,
            0x01,
            0x40 + ((index >> 8) as u8),
            index as u8,
            data_length as u8,
        ]);
        bytes.extend(std::iter::repeat(0xAA).take(data_length));
    }
    let firmware = FirmwareImage::from_scatter_bytes(bytes).unwrap();
    assert_eq!(firmware.bytes().len(), 5_863);
    assert_eq!(firmware.segment_count(), 123);
    assert_eq!(firmware.segments().nth(10).unwrap().len(), 35);
    assert_eq!(firmware.segments().last().unwrap().len(), 20);
}

#[test]
fn scatter_parser_decodes_multi_region_records_without_marker_scanning() {
    let bytes = vec![
        0x03, 0x01, 0x00, 0x02, 0x41, 0x00, 0x03, 0x60, 0x00, 0x02, 0x02, 0x60, 0x00, 0x80, 0xFE,
    ];
    let firmware = FirmwareImage::from_scatter_bytes(bytes).unwrap();
    let regions: Vec<_> = firmware.regions().collect();
    assert_eq!(firmware.segment_count(), 1);
    assert_eq!(regions.len(), 2);
    assert_eq!(regions[0].core, FirmwareCore::Ofdm);
    assert_eq!(regions[0].address, 0x4100);
    assert_eq!(regions[0].data, &[0x02, 0x60, 0x00]);
    assert_eq!(regions[1].address, 0x6000);
    assert_eq!(regions[1].data, &[0x80, 0xFE]);
}

#[test]
fn open_firmware_probe_is_small_reproducible_and_independently_generated() {
    let firmware = execution_probe_image().unwrap();
    assert!(firmware.segment_count() > 28);
    assert!(firmware.bytes().len() < 5863);
    assert_eq!(OPEN_LINK_ENTRY, 0x4193);
    assert_eq!(OPEN_LINK_VERSION, [0, 1, 4, 0]);
    assert_eq!(OPEN_PROBE_ENTRY, 0x4870);
    assert_eq!(OPEN_PROBE_MARKER_ADDRESS, 0x8046FF);
    assert_eq!(OPEN_PROBE_MARKER_VALUE, 0xA5);
    let regions: Vec<_> = firmware.regions().collect();
    assert!(regions.len() > 28);
    assert_eq!(regions[0].core, FirmwareCore::Link);
    assert_eq!(regions[0].address, 0x4100);
    assert_eq!(regions[0].data, &[0x02, 0x12, 0xBF]);
    assert_eq!(regions[1].core, FirmwareCore::Link);
    assert_eq!(regions[1].address, 0x4180);
    assert_eq!(&regions[1].data[..3], &[0x02, 0x41, 0x93]);
    assert_eq!(&regions[1].data[6..], &OPEN_LINK_VERSION);
    assert_eq!(regions[2].address, OPEN_LINK_ENTRY);
    assert_eq!(regions[4].address as usize + regions[4].data.len(), 0x41FB);
    assert_eq!(regions[5].core, FirmwareCore::Link);
    assert_eq!(regions[5].address, 0x4204);
    assert_eq!(
        regions[10].address as usize + regions[10].data.len(),
        0x4310
    );
    assert_eq!(regions[11].core, FirmwareCore::Ofdm);
    assert_eq!(regions[11].address, 0x4100);
    assert_eq!(regions[12].address, OPEN_PROBE_ENTRY);
    assert_eq!(
        regions[16].address as usize + regions[16].data.len(),
        0x495B
    );
    let find = |address| {
        regions
            .iter()
            .find(|region| region.core == FirmwareCore::Ofdm && region.address == address)
            .unwrap()
    };
    let startup: Vec<_> = regions
        .iter()
        .filter(|r| r.core == FirmwareCore::Ofdm && (0x4700..0x47E6).contains(&r.address))
        .flat_map(|r| r.data.iter().copied())
        .collect();
    assert_eq!(startup.len(), 230);
    assert_eq!(&startup[16..20], &[0x9F, 0x59, 0x9F, 0x92]);
    assert_eq!(find(0x495B).data.len(), 3);
    assert_eq!(find(0x4960).data.len(), 11);
    assert_eq!(find(0x4970).data.len(), 16);
    let vector: Vec<_> = regions
        .iter()
        .filter(|r| r.core == FirmwareCore::Ofdm && r.address >= 0x6680)
        .flat_map(|r| r.data.iter().copied())
        .collect();
    assert_eq!(vector.len(), 384);
    assert_eq!(
        &vector[(0x67be - 0x6680)..(0x67c1 - 0x6680)],
        &[0x02, 0xa0, 0xa6]
    );
}

#[test]
fn ts_analyzer_recovers_alignment_across_chunks() {
    let mut packet = vec![0xFFu8; 188];
    packet[0] = 0x47;
    packet[1] = 0x1F;
    packet[2] = 0xFF;
    packet[3] = 0x10;
    let mut data = vec![0x00, 0x01, 0x02];
    data.extend_from_slice(&packet);
    data.extend_from_slice(&packet);
    data.extend_from_slice(&packet);
    let mut analyzer = TsAnalyzer::new();
    analyzer.push(&data[..100]);
    analyzer.push(&data[100..]);
    let stats = analyzer.finish();
    assert_eq!(stats.packets, 3);
    assert!(stats.sync_losses >= 1);
}

#[test]
fn i2c_write_read_uses_public_bridge_frame_shape() {
    let response = response_frame(0, 0, &[0x12, 0x34]);
    let mut transport = MockTransport::new(vec![response]);
    {
        let mut protocol = Protocol::new(&mut transport);
        assert_eq!(
            protocol.i2c_write_read(0x60, &[0x01, 0x02], 2).unwrap(),
            vec![0x12, 0x34]
        );
    }
    let request = &transport.requests[0];
    assert_eq!(request[1], 0x00);
    assert_eq!(request[2], Command::I2cRead as u8);
    assert_eq!(&request[4..11], &[2, 0xC0, 0, 0, 0, 0x01, 0x02]);
}

#[test]
fn i2c_transfer_rejects_more_than_conservative_family_limit() {
    let mut transport = MockTransport::new(Vec::new());
    let mut protocol = Protocol::new(&mut transport);
    assert!(protocol.i2c_read(0x60, 41).is_err());
    assert!(protocol.i2c_write(0x60, &[0u8; 41]).is_err());
}

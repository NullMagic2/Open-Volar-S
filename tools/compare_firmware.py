#!/usr/bin/env python3
"""Verify the user's reference against the supported x64 driver, without executing it."""
import argparse
import hashlib
import json
from pathlib import Path
import struct

REFERENCE_SHA256 = "4b066157d0eb1a088e55daaf339418fc6596b5f76c4583d06d5eddb3a272e921"
DRIVER_SHA256 = "80dfe4cde9d5e06fc8663b68ef351404138443a0f11606caf963f038f0534b30"


def parse_scatter(data):
    records, regions, offset = [], [], 0
    if not data:
        raise ValueError("empty firmware")
    occupied = [set(), set()]
    while offset < len(data):
        start = offset
        if offset + 4 > len(data) or data[offset] != 3 or data[offset+1] not in (0, 1) or data[offset+2] != 0:
            raise ValueError(f"invalid record at 0x{offset:x}")
        core, count = data[offset+1], data[offset+3]
        offset += 4
        if not count or offset + 3*count > len(data):
            raise ValueError("invalid descriptor count")
        descriptors = [(int.from_bytes(data[offset+3*i:offset+3*i+2], 'big'), data[offset+3*i+2]) for i in range(count)]
        offset += 3*count
        for address, length in descriptors:
            if not length or address + length > 65536 or offset + length > len(data):
                raise ValueError("invalid region bounds")
            overlap = bool(occupied[core].intersection(range(address, address+length)))
            occupied[core].update(range(address, address+length))
            r = dict(core=core, address=address, length=length, payload_offset=offset, overlaps_previous=overlap)
            if address == 0x4100 and length >= 3 and data[offset] == 2:
                r['reset_ljmp_target'] = int.from_bytes(data[offset+1:offset+3], 'big')
            regions.append(r)
            offset += length
        if offset - start > 58:
            raise ValueError("scatter record exceeds 58 bytes")
        records.append(offset - start)
    ranges = []
    for core in (0, 1):
        current = None
        for address in sorted(occupied[core]):
            if current is None or current['end_exclusive'] != address:
                current = dict(core=core, start=address, end_exclusive=address+1)
                ranges.append(current)
            else:
                current['end_exclusive'] += 1
    return dict(size=len(data), sha256=hashlib.sha256(data).hexdigest(), record_count=len(records),
                region_count=len(regions), record_lengths=records, regions=regions, ranges=ranges)


def compare(driver, firmware):
    raw, reference = Path(driver).read_bytes(), Path(firmware).read_bytes()
    if hashlib.sha256(raw).hexdigest() != DRIVER_SHA256:
        raise ValueError("driver differs from the verified AVer857BDA.sys 12.6.64.12; fixed offsets are not applicable")
    if raw[0x1c99f] != 123:
        raise ValueError("unexpected descriptor count")
    lengths = []
    for i in range(123):
        pos = 0x1f5a0 + 8*i
        kind, size = raw[pos], struct.unpack_from('<I', raw, pos+4)[0]
        if kind != 1 or not 7 <= size <= 58:
            raise ValueError("unexpected firmware descriptor")
        lengths.append(size)
    embedded = raw[0x1deb0:0x1deb0+sum(lengths)]
    report = parse_scatter(reference)
    embedded_report = parse_scatter(embedded)
    if lengths != embedded_report['record_lengths']:
        raise ValueError("descriptor table does not agree with embedded record boundaries")
    report.update(driver_name=Path(driver).name, driver_size=len(raw), driver_sha256=DRIVER_SHA256,
                  embedded_offset='0x1deb0', embedded_size=len(embedded),
                  byte_identical_to_driver=(reference == embedded),
                  known_reference=(report['sha256'] == REFERENCE_SHA256),
                  evidence='Static file analysis only; no device boot or reception test.')
    return report


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('driver', type=Path)
    p.add_argument('firmware', type=Path)
    p.add_argument('--output', type=Path)
    args = p.parse_args()
    try:
        report = compare(args.driver, args.firmware)
        text = json.dumps(report, indent=2) + '\n'
        if args.output:
            if args.output.resolve() in (args.driver.resolve(), args.firmware.resolve()):
                raise ValueError('output would overwrite an input')
            args.output.write_text(text, encoding='utf-8')
        else:
            print(text)
        if not report['byte_identical_to_driver']:
            p.exit(1, 'Firmware differs from the embedded image.\n')
    except (OSError, ValueError) as e:
        p.exit(2, f'error: {e}\n')


if __name__ == '__main__':
    main()

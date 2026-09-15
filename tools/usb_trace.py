#!/usr/bin/env python3
"""Offline USBPcap analyzer, standard library only. Never opens or writes USB devices.

Format references: https://desowin.org/usbpcap/captureformat.html
https://www.ietf.org/archive/id/draft-ietf-opsawg-pcapng-04.html
Protocol reference: Linux drivers/media/usb/dvb-usb-v2/af9035.c (GPL-2.0-or-later).
"""
import argparse
from collections import Counter
from dataclasses import dataclass
import hashlib
import json
from pathlib import Path
import struct

MAX_BLOCK = 32 * 1024 * 1024
USBPCAP = 249
SHB = b"\x0a\x0d\x0d\x0a"


def exact(f, n):
    if not 0 <= n <= MAX_BLOCK:
        raise ValueError(f"unsupported block/packet length {n}")
    data = f.read(n)
    if len(data) != n:
        raise ValueError("truncated capture; stop and save the capture before analysis")
    return data


def options(data, endian):
    pos = 0
    while pos < len(data):
        if pos + 4 > len(data):
            raise ValueError("truncated pcapng option")
        code, size = struct.unpack_from(endian + "HH", data, pos)
        pos += 4
        if code == 0:
            if size:
                raise ValueError("invalid end-of-options")
            return
        end = pos + ((size + 3) & ~3)
        if end > len(data):
            raise ValueError("truncated pcapng option value")
        yield code, data[pos:pos + size]
        pos = end


@dataclass
class Packet:
    number: int
    section: int
    interface: int
    linktype: int
    time_ns: int | None
    data: bytes
    truncated: bool


def packets(path):
    """Stream pcap and pcapng EPB/SPB/obsolete PB; honor section byte order and clocks."""
    with Path(path).open("rb") as f:
        magic = exact(f, 4)
        variants = {b"\xd4\xc3\xb2\xa1": ("<", 1000), b"\xa1\xb2\xc3\xd4": (">", 1000),
                    b"\x4d\x3c\xb2\xa1": ("<", 1), b"\xa1\xb2\x3c\x4d": (">", 1)}
        if magic in variants:
            endian, scale = variants[magic]
            major, minor, _, _, snap, link = struct.unpack(endian + "HHIIII", exact(f, 20))
            if (major, minor) != (2, 4):
                raise ValueError("unsupported pcap version")
            number = 0
            while header := f.read(16):
                if len(header) != 16:
                    raise ValueError("truncated pcap packet header")
                sec, frac, caplen, origlen = struct.unpack(endian + "IIII", header)
                if caplen > origlen or (snap and caplen > snap):
                    raise ValueError("invalid pcap packet lengths")
                number += 1
                yield Packet(number, 0, 0, link & 0xffff, sec * 10**9 + frac * scale,
                             exact(f, caplen), caplen < origlen)
            return
        if magic != SHB:
            raise ValueError("expected a .pcap or .pcapng file")
        f.seek(0)
        endian, interfaces, section, number = None, [], -1, 0
        while header := f.read(8):
            if len(header) != 8:
                raise ValueError("truncated pcapng block header")
            prefix = b""
            if header[:4] == SHB:
                prefix = exact(f, 4)
                endian = {b"\x4d\x3c\x2b\x1a": "<", b"\x1a\x2b\x3c\x4d": ">"}.get(prefix)
                if endian is None:
                    raise ValueError("invalid pcapng byte-order magic")
                interfaces, section = [], section + 1
            if endian is None:
                raise ValueError("missing pcapng section")
            kind, size = struct.unpack(endian + "II", header)
            if size < 12 + len(prefix) or size % 4 or size > MAX_BLOCK:
                raise ValueError("invalid pcapng block size")
            rest = prefix + exact(f, size - 8 - len(prefix))
            if struct.unpack(endian + "I", rest[-4:])[0] != size:
                raise ValueError("pcapng trailing length mismatch")
            body = rest[:-4]
            if kind == 0x0a0d0d0a:
                if len(body) < 16 or struct.unpack_from(endian + "HH", body, 4) != (1, 0):
                    raise ValueError("unsupported pcapng section version")
            elif kind == 1:
                if len(body) < 8:
                    raise ValueError("short interface block")
                link, _, snap = struct.unpack_from(endian + "HHI", body)
                denominator, offset = 10**6, 0
                for code, value in options(body[8:], endian):
                    if code == 9:
                        if len(value) != 1:
                            raise ValueError("invalid timestamp resolution")
                        denominator = (2 if value[0] & 128 else 10) ** (value[0] & 127)
                    elif code == 14:
                        if len(value) != 8:
                            raise ValueError("invalid timestamp offset")
                        offset = struct.unpack(endian + "q", value)[0]
                interfaces.append((link, snap, denominator, offset))
            elif kind in (2, 3, 6):
                number += 1
                if kind == 3:
                    if len(body) < 4 or not interfaces:
                        raise ValueError("invalid simple packet block")
                    interface, time_ns = 0, None
                    origlen = struct.unpack_from(endian + "I", body)[0]
                    caplen = min(origlen, interfaces[0][1] or origlen)
                    start = 4
                else:
                    if len(body) < 20:
                        raise ValueError("short packet block")
                    if kind == 6:
                        interface, hi, lo, caplen, origlen = struct.unpack_from(endian + "IIIII", body)
                    else:
                        interface, _, hi, lo, caplen, origlen = struct.unpack_from(endian + "HHIIII", body)
                    start = 20
                    if interface >= len(interfaces):
                        raise ValueError("packet references unknown interface")
                    _, _, denominator, offset = interfaces[interface]
                    time_ns = ((hi << 32) | lo) * 10**9 // denominator + offset * 10**9
                if caplen > origlen or start + ((caplen + 3) & ~3) > len(body):
                    raise ValueError("invalid pcapng packet lengths")
                link, snap, _, _ = interfaces[interface]
                if snap and caplen > snap:
                    raise ValueError("packet exceeds interface snapshot length")
                yield Packet(number, section, interface, link, time_ns,
                             body[start:start + caplen], caplen < origlen)


def usb_packet(packet):
    if packet.linktype != USBPCAP:
        return None
    if len(packet.data) < 27:
        raise ValueError(f"packet {packet.number}: short USBPcap header")
    hlen, irp, status, function, info, bus, device, ep, transfer, length = struct.unpack_from(
        "<HQIHBHHBBI", packet.data)
    if hlen < 27 or hlen > len(packet.data):
        raise ValueError(f"packet {packet.number}: invalid USBPcap header length")
    return dict(packet=packet.number, section=packet.section, interface=packet.interface,
                time_ns=packet.time_ns, irp=f"0x{irp:x}", usb_status=status,
                function=function, completion=bool(info & 1), bus=bus, device=device,
                endpoint=ep, transfer=transfer, requested_length=length,
                truncated=packet.truncated or length > len(packet.data) - hlen,
                data=packet.data[hlen:hlen + length])


def checksum(data):
    return (~sum(b << (8 if i & 1 else 0) for i, b in enumerate(data) if i)) & 0xffff


def frame(data, incoming):
    minimum = 5 if incoming else 6
    if not minimum <= len(data) <= 64 or data[0] != len(data) - 1:
        return None
    if checksum(data[:-2]) != int.from_bytes(data[-2:], "big"):
        return None
    if incoming:
        return dict(sequence=data[1], device_status=data[2], payload=data[3:-2].hex())
    command, payload = data[2], data[4:-2]
    result = dict(mailbox=data[1], command=command, sequence=data[3], payload=payload.hex())
    result["operation"] = {0: "memory_read", 1: "memory_write", 2: "i2c_read", 3: "i2c_write",
                           0x18: "infrared", 0x21: "firmware_download", 0x22: "firmware_query",
                           0x23: "firmware_boot_or_restart", 0x24: "download_begin",
                           0x25: "download_end", 0x29: "firmware_scatter"}.get(command, "unknown")
    if command in (0, 1) and len(payload) >= 6:
        result.update(address=(data[1] << 16) | int.from_bytes(payload[4:6], "big"),
                      count=payload[0], address_length=payload[1])
        if command == 1:
            result["write_data"] = payload[6:].hex()
    elif command in (2, 3) and len(payload) >= 5:
        result.update(i2c_address_7bit=payload[1] >> 1, count=payload[0],
                      register_length=payload[2], register=payload[3:5].hex(),
                      extra_data=payload[5:].hex())
    elif command == 0x29:
        # Keep the digest and structure; omit proprietary uploaded code from reports.
        result.pop("payload")
        result["payload_sha256"] = hashlib.sha256(payload).hexdigest()
        if len(payload) >= 4 and payload[0] == 3 and payload[1] in (0, 1) and payload[2] == 0:
            n = payload[3]
            if n and 4 + 3 * n <= len(payload):
                regions = [dict(address=int.from_bytes(payload[4+i*3:6+i*3], "big"),
                                length=payload[6+i*3]) for i in range(n)]
                if all(r["length"] and r["address"] + r["length"] <= 65536 for r in regions) and 4 + 3*n + sum(r["length"] for r in regions) == len(payload):
                    result.update(core=payload[1], regions=regions)
                else:
                    result["scatter_error"] = "invalid region lengths"
            else:
                result["scatter_error"] = "invalid descriptor count"
        else:
            result["scatter_error"] = "unknown scatter header"
    return result


def inventory(path):
    devices, links = {}, Counter()
    for p in packets(path):
        links[p.linktype] += 1
        u = usb_packet(p)
        if u is None:
            continue
        key = (u["section"], u["interface"], u["bus"], u["device"])
        d = devices.setdefault(key, dict(section=key[0], interface=key[1], bus=key[2], device=key[3],
                                        packets=0, endpoints=Counter(), descriptor_ids=[]))
        d["packets"] += 1
        d["endpoints"][f"0x{u['endpoint']:02x}"] += 1
        data = u["data"]
        # GET_DESCRIPTOR device responses start with bLength=18, bDescriptorType=1.
        if u["transfer"] == 2 and u["completion"] and not u["usb_status"] and not u["truncated"] and len(data) >= 18 and data[:2] == b"\x12\x01":
            identity = f"{int.from_bytes(data[8:10], 'little'):04x}:{int.from_bytes(data[10:12], 'little'):04x}"
            if identity not in d["descriptor_ids"]:
                d["descriptor_ids"].append(identity)
    return dict(linktypes=dict(links), devices=list(devices.values()))


def analyze(path, target, output, command_out=None, command_in=None):
    counts, pending, operations = Counter(), {}, Counter()
    out_path = Path(output)
    with out_path.open("w", encoding="utf-8") as out:
        for p in packets(path):
            u = usb_packet(p)
            if u is None or (u["section"], u["interface"], u["bus"], u["device"]) != target:
                continue
            counts["usb_packets"] += 1
            data = u.pop("data")
            if u["completion"] and u["usb_status"]:
                counts["usb_errors"] += 1
                for seq in [s for s, req in pending.items() if req["irp"] == u["irp"]]:
                    pending.pop(seq)
                u["kind"] = "usb_error"
            elif u["transfer"] != 3:
                continue
            elif u["truncated"]:
                counts["truncated_bulk_packets"] += 1
                u["kind"] = "truncated_bulk"
            else:
                incoming = bool(u["endpoint"] & 0x80)
                # OUT data is on submission; IN data is on completion. Avoid counting twice.
                if incoming != u["completion"] or not data:
                    continue
                counts["bulk_payload_bytes"] += len(data)
                endpoint_filter = command_in if incoming else command_out
                decoded = frame(data, incoming) if endpoint_filter in (None, u["endpoint"]) else None
                if decoded is None:
                    counts["unclassified_bulk_transfers"] += 1
                    counts["unclassified_bulk_bytes"] += len(data)
                    continue
                u.update(decoded)
                if incoming:
                    u["kind"] = "candidate_response"
                    req = pending.get(u["sequence"])
                    # Sequence is only 8 bits. Require one outstanding request, matching size,
                    # and a bounded timestamp interval. This is a candidate association, not proof.
                    if req and len(pending) == 1:
                        age = None if u["time_ns"] is None or req["time_ns"] is None else u["time_ns"] - req["time_ns"]
                        expected = req.get("count") if req["command"] in (0, 2) else (4 if req["command"] == 0x22 else 0)
                        size_ok = u["device_status"] != 0 or len(bytes.fromhex(u["payload"])) == expected
                        if age is not None and 0 <= age <= 5_000_000_000 and size_ok:
                            u.update(request_packet=req["packet"], latency_ns=age, operation=req["operation"])
                            pending.pop(u["sequence"])
                            counts["candidate_pairs"] += 1
                    if u["device_status"]:
                        counts["device_error_responses"] += 1
                else:
                    u["kind"] = "candidate_request"
                    operations[u["operation"]] += 1
                    if u["sequence"] in pending:
                        counts["unanswered_sequence_reuse"] += 1
                    # Expire outstanding candidates without treating them as successful commands.
                    for seq in [s for s, r in pending.items() if u["time_ns"] is not None and r["time_ns"] is not None and u["time_ns"] - r["time_ns"] > 5_000_000_000]:
                        pending.pop(seq)
                        counts["expired_requests"] += 1
                    pending[u["sequence"]] = u.copy()
                counts[u["kind"]] += 1
            out.write(json.dumps(u, sort_keys=True) + "\n")
    counts["outstanding_at_end"] = len(pending)
    summary = dict(target=dict(zip(("section", "interface", "bus", "device"), target)),
                   counts=dict(counts), operations=dict(operations),
                   note="Offline candidates only. No USB writes, tune replay, or hardware compatibility claim. "
                        "Pairing requires timestamps and one outstanding command; split command URBs are unclassified.")
    out_path.with_suffix(".summary.json").write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    return summary


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("capture", type=Path)
    p.add_argument("--inventory", action="store_true")
    for name in ("section", "interface", "bus", "device", "command-out", "command-in"):
        p.add_argument("--" + name, type=lambda s: int(s, 0))
    p.add_argument("--output", type=Path)
    args = p.parse_args()
    try:
        info = inventory(args.capture)
        if args.inventory:
            print(json.dumps(info, indent=2))
            return
        matches = [d for d in info["devices"] if all(getattr(args, k) is None or d[k] == getattr(args, k)
                   for k in ("section", "interface", "bus", "device"))]
        if args.bus is None or args.device is None:
            matches = [d for d in matches if d["descriptor_ids"] == ["07ca:b865"]]
        if len(matches) != 1:
            p.error("select one tuner with --inventory then --section N --interface N --bus N --device N; "
                    "automatic selection requires an unambiguous 07ca:b865 device descriptor")
        if args.output is None:
            p.error("--output commands.jsonl is required")
        if args.output.resolve() == args.capture.resolve() or args.output.with_suffix(".summary.json").resolve() == args.capture.resolve():
            p.error("output must differ from the input capture")
        if args.command_out is not None and not 1 <= args.command_out <= 15:
            p.error("--command-out must be an OUT endpoint 1..15")
        if args.command_in is not None and not 0x81 <= args.command_in <= 0x8f:
            p.error("--command-in must be an IN endpoint 0x81..0x8f")
        target = tuple(matches[0][k] for k in ("section", "interface", "bus", "device"))
        print(json.dumps(analyze(args.capture, target, args.output, args.command_out, args.command_in), indent=2))
    except (OSError, ValueError, struct.error) as e:
        p.exit(2, f"error: {e}\n")


if __name__ == "__main__":
    main()

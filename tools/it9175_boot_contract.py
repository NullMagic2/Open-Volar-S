#!/usr/bin/env python3
"""Analyze IT9175 scatter firmware boot contracts without embedding firmware bytes.

The tool is intentionally evidence-oriented: it reconstructs each core's downloaded
code address space, follows MCS-51 control flow from the reset vector, records calls
across the downloaded/ROM boundary, and reports callback/vector tables used during
startup.  It never writes reconstructed firmware payloads to its report.
"""

from __future__ import annotations

import argparse
import hashlib
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class Region:
    core: int
    address: int
    data: bytes


@dataclass(frozen=True)
class Instruction:
    address: int
    length: int
    text: str
    target: int | None = None
    call: bool = False
    conditional: bool = False
    terminal: bool = False


def parse_scatter(image: bytes) -> tuple[list[bytes], list[Region]]:
    records: list[bytes] = []
    regions: list[Region] = []
    offset = 0
    while offset < len(image):
        start = offset
        if offset + 7 > len(image) or image[offset] != 3 or image[offset + 1] not in (0, 1):
            raise ValueError(f"invalid scatter header at 0x{offset:X}")
        core = image[offset + 1]
        if image[offset + 2] != 0:
            raise ValueError(f"unknown reserved field at 0x{offset + 2:X}")
        count = image[offset + 3]
        offset += 4
        descriptors: list[tuple[int, int]] = []
        for _ in range(count):
            if offset + 3 > len(image):
                raise ValueError("truncated descriptor")
            address = int.from_bytes(image[offset : offset + 2], "big")
            length = image[offset + 2]
            if length == 0:
                raise ValueError("zero-length region")
            descriptors.append((address, length))
            offset += 3
        for address, length in descriptors:
            if offset + length > len(image):
                raise ValueError("truncated region")
            regions.append(Region(core, address, image[offset : offset + length]))
            offset += length
        record = image[start:offset]
        if len(record) > 58:
            raise ValueError(f"record {len(records)} exceeds 58 bytes")
        records.append(record)
    return records, regions


def memory_map(regions: list[Region], core: int) -> tuple[bytearray, bytearray]:
    memory = bytearray(0x10000)
    present = bytearray(0x10000)
    for region in regions:
        if region.core != core:
            continue
        end = region.address + len(region.data)
        if end > 0x10000:
            raise ValueError("region crosses core address boundary")
        for address in range(region.address, end):
            if present[address]:
                raise ValueError(f"overlapping core {core} region at 0x{address:04X}")
        memory[region.address:end] = region.data
        present[region.address:end] = b"\x01" * len(region.data)
    return memory, present


def signed(value: int) -> int:
    return value - 256 if value >= 128 else value


def decode(memory: bytes, pc: int) -> Instruction:
    op = memory[pc]
    b1 = memory[(pc + 1) & 0xFFFF]
    b2 = memory[(pc + 2) & 0xFFFF]
    rel2 = (pc + 2 + signed(b1)) & 0xFFFF
    rel3 = (pc + 3 + signed(b2)) & 0xFFFF
    rn = op & 7

    if op == 0x00:
        return Instruction(pc, 1, "NOP")
    if op & 0x1F == 0x01:
        target = ((pc + 2) & 0xF800) | ((op & 0xE0) << 3) | b1
        return Instruction(pc, 2, f"AJMP 0x{target:04X}", target, terminal=True)
    if op == 0x02:
        target = b1 << 8 | b2
        return Instruction(pc, 3, f"LJMP 0x{target:04X}", target, terminal=True)
    if op == 0x03:
        return Instruction(pc, 1, "RR A")
    if op == 0x04:
        return Instruction(pc, 1, "INC A")
    if op == 0x05:
        return Instruction(pc, 2, f"INC 0x{b1:02X}")
    if op in (0x06, 0x07):
        return Instruction(pc, 1, f"INC @R{op & 1}")
    if 0x08 <= op <= 0x0F:
        return Instruction(pc, 1, f"INC R{rn}")
    if op == 0x10:
        return Instruction(pc, 3, f"JBC 0x{b1:02X},0x{rel3:04X}", rel3, conditional=True)
    if op & 0x1F == 0x11:
        target = ((pc + 2) & 0xF800) | ((op & 0xE0) << 3) | b1
        return Instruction(pc, 2, f"ACALL 0x{target:04X}", target, call=True)
    if op == 0x12:
        target = b1 << 8 | b2
        return Instruction(pc, 3, f"LCALL 0x{target:04X}", target, call=True)
    if op == 0x13:
        return Instruction(pc, 1, "RRC A")
    if op == 0x14:
        return Instruction(pc, 1, "DEC A")
    if op == 0x15:
        return Instruction(pc, 2, f"DEC 0x{b1:02X}")
    if op in (0x16, 0x17):
        return Instruction(pc, 1, f"DEC @R{op & 1}")
    if 0x18 <= op <= 0x1F:
        return Instruction(pc, 1, f"DEC R{rn}")
    if op in (0x20, 0x30):
        name = "JB" if op == 0x20 else "JNB"
        return Instruction(pc, 3, f"{name} 0x{b1:02X},0x{rel3:04X}", rel3, conditional=True)
    if op == 0x22:
        return Instruction(pc, 1, "RET", terminal=True)
    if op == 0x23:
        return Instruction(pc, 1, "RL A")
    if 0x24 <= op <= 0x2F:
        operands = [f"#0x{b1:02X}", f"0x{b1:02X}", "@R0", "@R1"] + [f"R{x}" for x in range(8)]
        idx = op - 0x24
        return Instruction(pc, 2 if idx < 2 else 1, f"ADD A,{operands[idx]}")
    if op == 0x32:
        return Instruction(pc, 1, "RETI", terminal=True)
    if op == 0x33:
        return Instruction(pc, 1, "RLC A")
    if 0x34 <= op <= 0x3F:
        operands = [f"#0x{b1:02X}", f"0x{b1:02X}", "@R0", "@R1"] + [f"R{x}" for x in range(8)]
        idx = op - 0x34
        return Instruction(pc, 2 if idx < 2 else 1, f"ADDC A,{operands[idx]}")
    if op in (0x40, 0x50, 0x60, 0x70):
        name = {0x40: "JC", 0x50: "JNC", 0x60: "JZ", 0x70: "JNZ"}[op]
        return Instruction(pc, 2, f"{name} 0x{rel2:04X}", rel2, conditional=True)
    if op in (0x42, 0x52, 0x62):
        name = {0x42: "ORL", 0x52: "ANL", 0x62: "XRL"}[op]
        return Instruction(pc, 2, f"{name} 0x{b1:02X},A")
    if op in (0x43, 0x53, 0x63):
        name = {0x43: "ORL", 0x53: "ANL", 0x63: "XRL"}[op]
        return Instruction(pc, 3, f"{name} 0x{b1:02X},#0x{b2:02X}")
    if op in (0x44, 0x54, 0x64):
        name = {0x44: "ORL", 0x54: "ANL", 0x64: "XRL"}[op]
        return Instruction(pc, 2, f"{name} A,#0x{b1:02X}")
    if op in (0x45, 0x55, 0x65):
        name = {0x45: "ORL", 0x55: "ANL", 0x65: "XRL"}[op]
        return Instruction(pc, 2, f"{name} A,0x{b1:02X}")
    if op in (0x46, 0x47, 0x56, 0x57, 0x66, 0x67):
        name = {0x4: "ORL", 0x5: "ANL", 0x6: "XRL"}[op >> 4]
        return Instruction(pc, 1, f"{name} A,@R{op & 1}")
    if 0x48 <= op <= 0x4F or 0x58 <= op <= 0x5F or 0x68 <= op <= 0x6F:
        name = {0x4: "ORL", 0x5: "ANL", 0x6: "XRL"}[op >> 4]
        return Instruction(pc, 1, f"{name} A,R{rn}")
    if op in (0x72, 0x82, 0xA0, 0xB0):
        name = {0x72: "ORL C", 0x82: "ANL C", 0xA0: "ORL C,/", 0xB0: "ANL C,/"}[op]
        return Instruction(pc, 2, f"{name}0x{b1:02X}")
    if op == 0x73:
        return Instruction(pc, 1, "JMP @A+DPTR", terminal=True)
    if op == 0x74:
        return Instruction(pc, 2, f"MOV A,#0x{b1:02X}")
    if op == 0x75:
        return Instruction(pc, 3, f"MOV 0x{b1:02X},#0x{b2:02X}")
    if op in (0x76, 0x77):
        return Instruction(pc, 2, f"MOV @R{op & 1},#0x{b1:02X}")
    if 0x78 <= op <= 0x7F:
        return Instruction(pc, 2, f"MOV R{rn},#0x{b1:02X}")
    if op == 0x80:
        return Instruction(pc, 2, f"SJMP 0x{rel2:04X}", rel2, terminal=True)
    if op == 0x83:
        return Instruction(pc, 1, "MOVC A,@A+PC")
    if op == 0x84:
        return Instruction(pc, 1, "DIV AB")
    if op == 0x85:
        return Instruction(pc, 3, f"MOV 0x{b2:02X},0x{b1:02X}")
    if op in (0x86, 0x87):
        return Instruction(pc, 2, f"MOV 0x{b1:02X},@R{op & 1}")
    if 0x88 <= op <= 0x8F:
        return Instruction(pc, 2, f"MOV 0x{b1:02X},R{rn}")
    if op == 0x90:
        return Instruction(pc, 3, f"MOV DPTR,#0x{b1 << 8 | b2:04X}")
    if op == 0x92:
        return Instruction(pc, 2, f"MOV 0x{b1:02X},C")
    if op == 0x93:
        return Instruction(pc, 1, "MOVC A,@A+DPTR")
    if 0x94 <= op <= 0x9F:
        operands = [f"#0x{b1:02X}", f"0x{b1:02X}", "@R0", "@R1"] + [f"R{x}" for x in range(8)]
        idx = op - 0x94
        return Instruction(pc, 2 if idx < 2 else 1, f"SUBB A,{operands[idx]}")
    if op == 0xA2:
        return Instruction(pc, 2, f"MOV C,0x{b1:02X}")
    if op == 0xA3:
        return Instruction(pc, 1, "INC DPTR")
    if op == 0xA4:
        return Instruction(pc, 1, "MUL AB")
    if op in (0xA6, 0xA7):
        return Instruction(pc, 2, f"MOV @R{op & 1},0x{b1:02X}")
    if 0xA8 <= op <= 0xAF:
        return Instruction(pc, 2, f"MOV R{rn},0x{b1:02X}")
    if op == 0xB2:
        return Instruction(pc, 2, f"CPL 0x{b1:02X}")
    if op == 0xB3:
        return Instruction(pc, 1, "CPL C")
    if op == 0xB4:
        return Instruction(pc, 3, f"CJNE A,#0x{b1:02X},0x{rel3:04X}", rel3, conditional=True)
    if op == 0xB5:
        return Instruction(pc, 3, f"CJNE A,0x{b1:02X},0x{rel3:04X}", rel3, conditional=True)
    if op in (0xB6, 0xB7):
        return Instruction(pc, 3, f"CJNE @R{op & 1},#0x{b1:02X},0x{rel3:04X}", rel3, conditional=True)
    if 0xB8 <= op <= 0xBF:
        return Instruction(pc, 3, f"CJNE R{rn},#0x{b1:02X},0x{rel3:04X}", rel3, conditional=True)
    if op == 0xC0:
        return Instruction(pc, 2, f"PUSH 0x{b1:02X}")
    if op == 0xC2:
        return Instruction(pc, 2, f"CLR 0x{b1:02X}")
    if op == 0xC3:
        return Instruction(pc, 1, "CLR C")
    if op == 0xC4:
        return Instruction(pc, 1, "SWAP A")
    if op == 0xC5:
        return Instruction(pc, 2, f"XCH A,0x{b1:02X}")
    if op in (0xC6, 0xC7):
        return Instruction(pc, 1, f"XCH A,@R{op & 1}")
    if 0xC8 <= op <= 0xCF:
        return Instruction(pc, 1, f"XCH A,R{rn}")
    if op == 0xD0:
        return Instruction(pc, 2, f"POP 0x{b1:02X}")
    if op == 0xD2:
        return Instruction(pc, 2, f"SETB 0x{b1:02X}")
    if op == 0xD3:
        return Instruction(pc, 1, "SETB C")
    if op == 0xD4:
        return Instruction(pc, 1, "DA A")
    if op == 0xD5:
        return Instruction(pc, 3, f"DJNZ 0x{b1:02X},0x{rel3:04X}", rel3, conditional=True)
    if op in (0xD6, 0xD7):
        return Instruction(pc, 1, f"XCHD A,@R{op & 1}")
    if 0xD8 <= op <= 0xDF:
        return Instruction(pc, 2, f"DJNZ R{rn},0x{rel2:04X}", rel2, conditional=True)
    if op == 0xE0:
        return Instruction(pc, 1, "MOVX A,@DPTR")
    if op in (0xE2, 0xE3):
        return Instruction(pc, 1, f"MOVX A,@R{op & 1}")
    if op == 0xE4:
        return Instruction(pc, 1, "CLR A")
    if op == 0xE5:
        return Instruction(pc, 2, f"MOV A,0x{b1:02X}")
    if op in (0xE6, 0xE7):
        return Instruction(pc, 1, f"MOV A,@R{op & 1}")
    if 0xE8 <= op <= 0xEF:
        return Instruction(pc, 1, f"MOV A,R{rn}")
    if op == 0xF0:
        return Instruction(pc, 1, "MOVX @DPTR,A")
    if op in (0xF2, 0xF3):
        return Instruction(pc, 1, f"MOVX @R{op & 1},A")
    if op == 0xF4:
        return Instruction(pc, 1, "CPL A")
    if op == 0xF5:
        return Instruction(pc, 2, f"MOV 0x{b1:02X},A")
    if op in (0xF6, 0xF7):
        return Instruction(pc, 1, f"MOV @R{op & 1},A")
    if 0xF8 <= op <= 0xFF:
        return Instruction(pc, 1, f"MOV R{rn},A")
    return Instruction(pc, 1, f"DB 0x{op:02X}", terminal=op == 0xA5)


def reset_target(memory: bytes, present: bytes) -> int:
    if not all(present[0x4100:0x4103]) or memory[0x4100] != 0x02:
        raise ValueError("missing MCS-51 reset LJMP at 0x4100")
    return memory[0x4101] << 8 | memory[0x4102]


def walk(memory: bytes, present: bytes, entries: list[int]) -> tuple[dict[int, Instruction], set[int], set[int]]:
    decoded: dict[int, Instruction] = {}
    rom_calls: set[int] = set()
    missing_targets: set[int] = set()
    work = list(entries)
    while work:
        pc = work.pop()
        while 0 <= pc < 0x10000 and present[pc] and pc not in decoded:
            insn = decode(memory, pc)
            if not all(present[pc : pc + insn.length]):
                missing_targets.add(pc)
                break
            decoded[pc] = insn
            fallthrough = (pc + insn.length) & 0xFFFF
            if insn.target is not None:
                if present[insn.target]:
                    work.append(insn.target)
                elif insn.call:
                    rom_calls.add(insn.target)
                else:
                    missing_targets.add(insn.target)
            if insn.terminal and not insn.call:
                break
            pc = fallthrough
    return decoded, rom_calls, missing_targets


def immediate_xdata_writes(memory: bytes, start: int, end: int) -> list[tuple[int, int, int]]:
    """Evaluate immediate A/DPTR state in a straight-line initialization block."""
    found: list[tuple[int, int, int]] = []
    accumulator: int | None = None
    dptr: int | None = None
    pc = start
    while pc < end:
        op = memory[pc]
        insn = decode(memory, pc)
        if op == 0x74:  # MOV A,#imm
            accumulator = memory[pc + 1]
        elif op == 0x90:  # MOV DPTR,#imm16
            dptr = memory[pc + 1] << 8 | memory[pc + 2]
        elif op == 0xA3 and dptr is not None:  # INC DPTR
            dptr = (dptr + 1) & 0xFFFF
        elif op == 0xF0 and dptr is not None and accumulator is not None:  # MOVX @DPTR,A
            found.append((pc, dptr, accumulator))
        elif op in (0xE0, 0xE4, 0xE5, 0xE6, 0xE7) or 0xE8 <= op <= 0xEF:
            accumulator = 0 if op == 0xE4 else None
        pc += insn.length
    return found


def vector_table(memory: bytes, present: bytes, start: int, end: int) -> list[tuple[int, int]]:
    result: list[tuple[int, int]] = []
    for address in range(start, end, 3):
        if all(present[address : address + 3]) and memory[address] == 0x02:
            result.append((address, memory[address + 1] << 8 | memory[address + 2]))
    return result


def contiguous_ranges(present: bytes) -> list[tuple[int, int]]:
    ranges: list[tuple[int, int]] = []
    start = None
    for address, value in enumerate(present + b"\x00"):
        if value and start is None:
            start = address
        elif not value and start is not None:
            ranges.append((start, address))
            start = None
    return ranges


def make_report(path: Path, image: bytes, records: list[bytes], regions: list[Region]) -> str:
    lines = [
        "# IT9175 coordinated boot-contract analysis",
        "",
        f"Input: `{path.name}`  ",
        f"SHA-256: `{hashlib.sha256(image).hexdigest()}`  ",
        f"Scatter records: {len(records)}  ",
        f"Decoded regions: {len(regions)}",
        "",
        "This report records addresses and behavior but does not reproduce firmware payloads.",
        "",
    ]
    for core, name in ((0, "link"), (1, "OFDM")):
        memory, present = memory_map(regions, core)
        reset = reset_target(memory, present)
        decoded, rom_calls, missing = walk(memory, present, [0x4100])
        lines.extend([f"## Core {core}: {name}", "", f"- Reset target: `0x{reset:04X}`"])
        lines.append(f"- Downloaded bytes reachable from reset by static flow: {sum(x.length for x in decoded.values())}")
        lines.append("- Downloaded ranges: " + ", ".join(f"`0x{a:04X}..0x{b - 1:04X}`" for a, b in contiguous_ranges(present)))
        if rom_calls:
            lines.append("- Direct calls from reachable downloaded code into ROM: " + ", ".join(f"`0x{x:04X}`" for x in sorted(rom_calls)))
        if missing:
            lines.append("- Non-call flow targets outside the downloaded image: " + ", ".join(f"`0x{x:04X}`" for x in sorted(missing)))
        lines.append("")

        if core == 1:
            vectors = vector_table(memory, present, 0x6680, 0x6800)
            downloaded = [(a, t) for a, t in vectors if present[t]]
            rom = [(a, t) for a, t in vectors if not present[t]]
            lines.extend(
                [
                    "### OFDM downloaded vector veneer",
                    "",
                    f"- Entries at `0x6680..0x67FF`: {len(vectors)}",
                    f"- Targets in downloaded code: {len(downloaded)}",
                    f"- Targets in immutable ROM: {len(rom)}",
                ]
            )
            if any(a == 0x67BE for a, _ in vectors):
                target = dict(vectors)[0x67BE]
                lines.append(f"- Startup veneer: `0x67BE -> 0x{target:04X}`")
            lines.append("")

            writes = immediate_xdata_writes(memory, 0x48B3, 0x4950)
            values = {address: value for _, address, value in writes}
            lines.extend(["### OFDM startup ROM patch map", "", "| Slot | Original entry | Replacement entry |", "| ---: | ---: | ---: |"])
            for slot in range(6):
                primary_address = 0xFB3B + slot * 2
                secondary_address = 0xFB5B + slot * 2
                primary = values[primary_address] << 8 | values[primary_address + 1]
                secondary = values[secondary_address] << 8 | values[secondary_address + 1]
                secondary_kind = "downloaded" if present[secondary] else "ROM"
                lines.append(f"| {slot} | `0x{primary:04X}` | `0x{secondary:04X}` ({secondary_kind}) |")
            lines.extend(
                [
                    "",
                    "The two 32-word halves pair an original ROM entry with its replacement. The table",
                    "is initialized to `0xFF`; only these six redirects are enabled by the reference image.",
                    "Redirecting an unsupported entry to a RET stub is therefore not a neutral fallback:",
                    "the correct open fallback is to leave that slot unused so mask ROM remains reachable.",
                    "",
                ]
            )
            lines.append("")

            timer_decoded, _, _ = walk(memory, present, [0x4A1B])
            timer_returns = sorted(
                address for address, instruction in timer_decoded.items() if instruction.text == "RET"
            )
            acknowledged_returns = [
                address
                for address in timer_returns
                if address >= 2 and memory[address - 2 : address] == bytes((0xC2, 0xCF))
            ]
            lines.extend(
                [
                    "### OFDM Timer-2 callback contract",
                    "",
                    "- Reference replacement entry: `0x4A1B` (original ROM entry `0x99F5`).",
                    f"- Reachable return sites: {len(timer_returns)}.",
                    f"- Return sites immediately preceded by `CLR 0xCF`: {len(acknowledged_returns)}.",
                    "- Direct bit `0xCF` is the MCS-51 Timer-2 overflow flag `TF2`.",
                    "",
                    "The minimum independently authored acknowledgement contract is therefore",
                    "`CLR TF2; RET`; this does not reproduce the callback's maintenance body.",
                    "",
                ]
            )

            lines.extend(
                [
                    "### OFDM reset-path semantics",
                    "",
                    "- `0x4873`: clear internal RAM `0x01..0x7F`.",
                    "- `0x4879`: set stack pointer to `0x26`.",
                    "- `0x487F..0x489E`: publish four version bytes at XDATA `0x4191..0x4194`.",
                    "- `0x489F..0x492A`: clear and populate the ROM patch map.",
                    "- `0x492B..0x4949`: establish serial/interrupt SFR state.",
                    "- `0x494A..0x494F`: publish ready value `1` at XDATA `0xFB3A`.",
                    "- `0x4950`: call veneer `0x67BE`, which transfers to immutable ROM `0xA0A6`.",
                    "- `0x4953`: return to the coordinated boot caller after ROM startup returns.",
                    "",
                ]
            )
    lines.extend(
        [
            "## Consequence for an open probe",
            "",
            "A valid probe must preserve the coordinated boot ABI for both MCS-51 cores. In particular,",
            "the OFDM core cannot be replaced by a marker-and-loop program: the reference reset path",
            "builds a callback table, configures serial/interrupt state, sets a ready mailbox, and calls",
            "the immutable-ROM startup service through the downloaded `0x67BE` veneer. Omitting that",
            "handoff prevents a link-side command timeout from identifying which link hook is correct.",
            "",
        ]
    )
    return "\n".join(lines)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("firmware", type=Path)
    parser.add_argument("--report", type=Path)
    args = parser.parse_args()
    image = args.firmware.read_bytes()
    records, regions = parse_scatter(image)
    report = make_report(args.firmware, image, records, regions)
    if args.report:
        args.report.write_text(report, encoding="utf-8", newline="\n")
    else:
        print(report)


if __name__ == "__main__":
    main()

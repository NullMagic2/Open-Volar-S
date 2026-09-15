> Historical design/research snapshot from 0.4.0. Current measured reception and firmware status are in RECEPTION.md and OPEN_FIRMWARE.md; API.md describes the 0.5.0 interface.

# Evidence from the supplied A865R files

Analysis date: 2026-09-10. The original installer was unpacked with 7-Zip as archives; none of its host executables was run. The supplied firmware was compared to the embedded image and subsequently loaded into the connected tuner's volatile memory. No original-driver USB capture or reception result was available. The new Rust tools recorded their own diagnostic USB transactions.

## Source identity

| Source | SHA-256 |
|---|---|
| `A865R_AP6.9.1.5_Drv_12.6.x.12_150820.exe` | `d00336b6fc04878fd79808a7d2f0ce6ac59aba4e4b8fe414eb3592d52c1208b6` |
| Extracted x64 `AVer857BDA.sys`, 146,560 bytes | `80dfe4cde9d5e06fc8663b68ef351404138443a0f11606caf963f038f0534b30` |
| Supplied `a865r-it9175.fw`, 5,863 bytes | `4b066157d0eb1a088e55daaf339418fc6596b5f76c4583d06d5eddb3a272e921` |

Extraction route: outer RAR self-extracting archive → `Drivers/Default/Win10_x64_V12.6.64.12_Install.exe` → nested NSIS archive → `AVer857BDA.sys` / `AVer865R.inf`.

INF lines 34–45 include whole-device and MI_00 forms of `USB\VID_07ca&PID_b865`. Line 14 specifies `11/29/2013,12.6.64.12`. The description at line 122 identifies A865R USB Pure ISDB-Tb. The live unit identified as chip IT9175, revision 1, prechip 0x83.

## Firmware comparison

The supplied firmware exactly equals the driver's bytes `[0x1deb0, 0x1f597)`. The count at `0x1c99f` is 123. The table at `0x1f5a0` contains 123 eight-byte descriptors; type 1 and the little-endian lengths agree with every parsed scatter boundary. The image contains 128 regions and no overlapping destination ranges. `tools/compare_firmware.py` reproduces these checks and rejects other driver fingerprints before using fixed offsets.

Each scatter record contains a four-byte header (`3`, core, reserved zero, region count), then three bytes per region (big-endian address and one-byte length), then concatenated payloads. The record ceiling is 58 bytes. All range endpoints below are inclusive.

| Core selector | Downloaded ranges | Reset vector interpretation |
|---|---|---|
| 0 / LINK | `4100–4102`, `4180–4189`, `4193–4335` | 8051 `LJMP 12BF` |
| 1 / OFDM | `4100–4102`, `4700–47E5`, `4870–57D1`, `6680–67FF` | 8051 `LJMP 4870` |

These are observed downloaded ranges, not a complete map of executable RAM or mask ROM. The instruction encoding and separate image selectors support the inherited dual-MCS-51 interpretation. They do not establish that every missing address is ROM, nor that a boot target is safe to invoke without its initialization contract.

## Static host tuning entry points

The PE image base is `0x10000`. GNU objdump disassembly was cross-referenced against the binary's diagnostic strings. The following names are inferred from nearby diagnostic strings and call sites; the binary has no corresponding public source symbols.

| Inferred role | Preferred VA | RVA | Evidence |
|---|---:|---:|---|
| `DRV_SetFreqBw` entry | `0x1BB34` | `0xBB34` | Function loads the diagnostic-string address `0x29320` at `0x1BB72` |
| Channel acquisition | `0x23DCC` | `0x13DCC` | Called from `0x1BD15`; failure path references acquisition error string at `0x29460` |
| Bandwidth selection | `0x1F618` | `0xF618` | Called at `0x23E1C`; error path uses `Standard_selectBandwidth` diagnostic at `0x2A540` |
| Frequency programming | `0x1FACC` | `0xFACC` | Called at `0x23E73`; failure diagnostic at `0x2A5C0` names `Standard_setFrequency` |
| Lock check | `0x216F4` | `0x116F4` | Called at `0x1BD92` after acquisition; associated lock diagnostic follows |

At the acquisition call, RCX receives the demodulator object, DL the slave index, R8D the saved bandwidth, and R9D the saved frequency. The high-level path caches prior tuning state, skips some repeated requests, and checks tuner-power state before acquisition. Therefore blindly replaying one write list is insufficient to reproduce the driver state machine. Further analysis must resolve the called routines, object-specific tuner dispatch, register tables, polling conditions, and timing. This release does not execute these routines or invent their register sequences.

## Inherited claims and unresolved items

The adjacent v0.3.24 archive reports OFDM timeouts with its open probe; those open-image tests were not repeated here. Its generator was reproduced offline: 1,168 bytes in 27 records, SHA-256 `0577b70b9f5db5607f320b3c60750c53d402fc8cc4c4e4b7eb1d503d29f01582`. The generator still places compatibility stubs at `0x6100`, outside the ranges present in the reference download. That is an unsupported memory-placement assumption worth testing, not proof of the timeout's cause. It also stubs several downloaded routines and omits complete tuning behavior.

Live reference baseline: the existing WinUSB binding exposed command OUT `02`, IN `81`, and additional IN endpoints `84`/`85`, all max-packet 512. Cold LINK reported `0.0.0.0`. Uploading the exact 123-record reference succeeded and LINK returned `3.0.3.0`; a forwarded read of OFDM version address `0x804191` returned `03 00 04 05`. Endpoint 84 is only a TS candidate. EEPROM layout decoding yielded tuner ID `70`, TS mode 0 and IF field 0; those fields alone do not establish RF silicon identity or zero-IF operation. See debug exports and `POWER_MANAGEMENT.md` for additional measurements and the reconstructed power contract.

The supplied reference establishes payload structure and comparison, not a known-good open firmware implementation. Hardware measurements must resolve the tuner identity, RF calibration state, OFDM startup/interrupt ABI, original-driver initialization, tune/lock sequence, and TS start/stop behavior.

Public family references are used only where applicable: [Linux af9035 command transport](https://github.com/torvalds/linux/blob/master/drivers/media/usb/dvb-usb-v2/af9035.c) explains framing, mailbox register access and I2C shape. It does not establish IT9175 ISDB-Tb tuner compatibility. [Microsoft WinUSB API documentation](https://learn.microsoft.com/en-us/windows-hardware/drivers/usbcon/using-winusb-api-to-communicate-with-a-usb-device) supports the userspace USB transport architecture.

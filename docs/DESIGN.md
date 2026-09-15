> Historical design/research snapshot from 0.4.0. Current measured reception and firmware status are in RECEPTION.md and OPEN_FIRMWARE.md; API.md describes the 0.5.0 interface.

# Windows userspace and open firmware design

The first compatibility target is the original A865R board, its original RF tuner, and ISDB-Tb reception. This design does not substitute DVB-T tables or assume an interchangeable tuner. The current code implements diagnostics and parts of the host transport; the full acceptance criteria below remain outstanding.

## Architecture

```text
Rust debug GUI / Rust CLI
        |
liba865r: session, framing, bounded register/I2C access, firmware loader, TS analysis
        |
WinUSB userspace API -> Microsoft winusb.sys -> USB bulk endpoints
        |
IT9175 LINK processor <-> OFDM processor <-> original RF tuner

Original BDA driver + USBPcap -> saved capture -> Python decoder -> evidence/fixtures
```

Windows still needs its inbox kernel USB transport; the new device logic runs in a normal process. A future application can consume MPEG transport stream bytes through a file, named pipe, or localhost stream. This does not expose a BDA tuner to existing Windows TV applications. The earlier BDA kernel scaffold was excluded from this userspace deliverable.

## Host behavior to complete

Model the connection as Disconnected → Identified → FirmwareRunning → BoardInitialized → Tuned → Locked → Streaming, with explicit failure states. A nonzero firmware version proves neither board initialization nor lock. A TS endpoint with data proves neither valid transport packets nor successful reception.

The existing `Device` implementation covers discovery and reference loading but is not the complete state machine. Before implementing tuning, give the connection one serialized command owner and a sequence counter lasting for that connection, separate stream I/O from command I/O, classify retryable reads, and avoid retrying state-changing writes after ambiguous completion. Keep the original frequency/bandwidth in explicit units once the vendor call chain is resolved.

The next host implementation milestone is one documented original-board sequence: power/clock/GPIO setup, firmware boot/version checks, tuner calibration, ISDB-Tb bandwidth/frequency programming, bounded lock polling, TS enable, packet reception, and TS disable. Each table/operation should cite a trace packet range or a fingerprinted binary location. Unknown operations remain unknown; the offline decoder provides no automatic replay command.

## Open firmware workstream

The Rust code is the host implementation. Evidence points to MCS-51-compatible firmware instruction encoding, so an independently authored firmware image needs an 8051 assembler or a suitable C-to-8051 compiler, plus a scatter packager. Do not assume ordinary Rust targets can generate native firmware for these cores. Source-authored Rust generation of a small instruction sequence is possible and is how the inherited experimental probe is represented.

Keep the supplied firmware external as a comparison oracle. Record its regions, reset targets and observable services. Reconstruct only demonstrated ROM/RAM entry contracts: register preservation, stack, interrupt acknowledgement, scheduler state, memory access, and interprocessor routing. A replacement image must account for all required callbacks rather than replacing unknown behavior with RET and declaring success.

The current open generator is retained for research reproducibility; its generated machine code is unchanged from the supplied v0.3.24 archive. The historical OFDM failure remains unresolved. The GUI deliberately does not upload this image. `build-open-fw-probe` is an offline CLI operation; `probe-open-fw` is an explicit experimental hardware operation, not a reception test. The known-problematic `reset-cold` command remains disabled.

### Firmware acceptance gates

1. Both cores answer version queries after a truly cold start, repeatedly.
2. A known read-only OFDM mailbox/register checkpoint works and command forwarding survives repeated requests.
3. Interrupt/timer service and required callbacks remain functional over time.
4. The original tuner can initialize, tune and lock on a known signal using the same host path as the reference.
5. Valid 188-byte MPEG-TS packets arrive; PAT/PMT, continuity and transport-error statistics are credible.
6. Retune, stream stop/start, unplug/reconnect, failed-lock timeout and sustained reception pass.
7. The OFDM shutdown handshake, bounded failure handling and repeated suspend/resume restore tuning and valid TS reception. See `POWER_MANAGEMENT.md` for the recovered conditional host sequence.

A marker byte alone does not meet any reception gate. Every gate needs an exported hardware result. Unplug/reconnect is the inherited project's known recovery method; no software cold-reset behavior is assumed here.

## Changes in v0.4.0

- Rust GUI with background diagnostics, file selection, editable observations, copyable results and automatic export.
- Per-exchange request/reply/error recording for GUI probes.
- Offline Python PCAP/PCAPNG inventory and protocol decoding, with explicit ambiguity/error reporting.
- Fingerprinted firmware-to-driver comparison and derived region metadata.
- Short error responses now preserve the device status; oversized expected responses fail before USB I/O.
- Register chunks stop at 16-bit mailbox boundaries; parsed firmware regions cannot wrap core addresses.
- Reference firmware loading is restricted to chip `0x9175` for this target, rather than assuming IT9135 equivalence.

## Immediate next evidence

Reference boot and forwarded OFDM communication were measured on the connected unit and are exported. Next save original-driver USBPcap traffic including enumeration, initialization, attempted tune, stream stop and power transitions. Record RF frequency, bandwidth, action times and whether reception works. Use Power snapshot for read-only checkpoints while the Rust interface is available. No compatible open firmware or functioning receiver is claimed by this release.

> 0.5.0 update: the bounded reference-derived wake/calibration path now supports measured TV reception. Suspend/power-down transitions remain unverified. Open firmware 0.1.2.0 boots both cores but does not lock TV; see OPEN_FIRMWARE.md.

# A865R power-management reconstruction

## Finding and limits

The original driver implements separate demodulator, tuner, secondary-NIM and Windows USB power paths. Its shutdown path includes an OFDM request/acknowledgement handshake. Sending firmware command `0x23` alone is not an implementation of suspend/resume.

This identifies a missing compatibility contract for the experimental open firmware. It does **not** yet establish which missing callback caused its historical OFDM timeout. No shutdown, reboot, USB port cycle or experimental firmware upload was performed in this session. The only uploaded image was the supplied reference, to volatile device memory.

Evidence below is from the extracted x64 `AVer857BDA.sys`, SHA-256 `80dfe4cde9d5e06fc8663b68ef351404138443a0f11606caf963f038f0534b30`, preferred image base `0x10000`. Addresses are preferred virtual addresses; subtract the image base for RVAs. Function names are inferred from strings and call sites. Internal object type 2/5 and chip-count branches must be resolved for the actual board before executing this contract.

## Host call paths

| Path | Entry VA | Observed behavior |
|---|---:|---|
| Application power control (`DRV_ApCtrl`) | `1BFB8` | On: readiness queries, tuner power, then demodulator power. Off: demodulator power, then tuner power. Tracks active clients/slaves and has peer-active branches. |
| Demodulator power | `22F28` | Register sequence and bounded OFDM shutdown polling below. |
| Tuner power | `2388C` | On writes OFDM EC40=1; off disables and clears multiple analog-register blocks. |
| Secondary-chip leakage reduction | `2365C` | Similar to tuner off, but with a different EC02 mask. |
| Secondary NIM suspend | `1C390` | Single-chip branch returns without writes; otherwise D8BB bit 0 is set on suspend, cleared on resume. Waits 20 ms / 100 ms respectively. |
| Windows selective suspend | `1C4A8` | Checks host activity before calling `18C5C`; resume calls `18EF8`. These are host-side USB request/cancellation paths, not downloadable 8051 routines. |
| Firmware reboot | `2402C` | Queries LINK, dispatches a reboot operation through a vtable, waits 1 ms; object types 2/5 return without the other variants' subsequent polling. |

The wait helper at `10388` multiplies milliseconds by -10,000 for a relative kernel delay. The host idle-request path at `18D5A` constructs internal USB IOCTL `0x220027`. A userspace implementation must use supported WinUSB/Windows power facilities instead of copying kernel IRP handling into firmware.

## Demodulator sequence

Notation: LINK addresses map directly to the host's 24-bit register address; OFDM local `xxxx` maps to `0x80xxxx`. Bit operations are read/modify/write, preserving unrelated bits. Whole-byte writes are explicitly shown. These tables describe observed code, not an enabled replay program.

Wake, in call order:

1. For internal object types 2/5: clear OFDM FB24 bit 3, then write FBA8=0.
2. Clear OFDM FBB9 bit 5; write LINK E00C=0.
3. Pulse OFDM F84F with whole-byte writes 0, then 1.
4. For types 2/5 with one chip: set LINK D8C7 bit 0 and clear D8BB bit 0.
5. Restore additional configuration through calls at `231xx` to `21AF8` and `2220C` for applicable chip/slave branches. Their complete role and board-specific arguments remain to be reconstructed. Additional multi-chip routing branches follow.

Sleep, in call order:

1. Set LINK D8BF bit 0; clear OFDM FBB9 bit 5; write LINK E00C=1.
2. Pulse OFDM F84F with 0, then 1; set OFDM FBB9 bit 5.
3. For types 2/5 with one chip: clear LINK D8C7 bit 0 and set D8BB bit 0.
4. Write LINK D91B=1 and D91C=0.
5. For types 2/5: write OFDM **004C=1** (`234E8`), then **0000=0** (`2351F`). Read 004C (`2354E`) until it becomes zero. On nonzero reads, wait 10 ms, up to 150 iterations: approximately 1.5 seconds plus USB overhead.
6. Set OFDM FB24 bit 3 (`23597`). Additional board/chip branches follow.

The original loop falls through to step 6 even if all 150 reads remain nonzero. It exits on transport errors, but does not create a distinct handshake-timeout error at loop exhaustion. Our intended implementation should report an explicit timeout and stop that transition, retaining the evidence. The meaning of FB24 bit 3 as a clock/power gate is an inference from its placement; its complete silicon semantics are not established here.

The open firmware must service the actual shutdown request before publishing an acknowledgement. Clearing 004C unconditionally would hide missing shutdown work and would not demonstrate compatibility. Identify the responsible firmware/ROM service, scheduler conditions, register preservation and interrupts before replacing it.

## Tuner and NIM distinctions

Tuner-off (`2388C`) writes OFDM FBA8=0 and EC40=0, then:

| Start | Length | Data |
|---|---:|---|
| EC02 | 15 | `3F 1F 3F 3E`, followed by 11 zero bytes |
| EC12 | 4 | Zero bytes |
| EC17 | 9 | Zero bytes |
| EC22 | 10 | Zero bytes |
| EC20 | 1 | `00` |
| EC3F | 1 | `01` |

The leakage-reduction routine instead starts EC02 with `00 0C`, followed by 13 zero bytes. These routines must not be conflated. Tuner-on only writing EC40=1 does not prove that all tuning/calibration state survives a cold start or suspend.

Similarly, the separate NIM routine's D8BB toggle is conditional on multiple chips. It is not a universal reset procedure. Application-level power also tracks active consumers, so one client closing must not power down a still-used peer.

## Open firmware implementation plan

Keep `reset-cold` disabled. Treat firmware boot/reboot, application stop, analog power-off and USB suspend as distinct transitions. The current open generator contains incomplete maintenance/callback code and relies on inferred ROM entry contracts; its generated image was not changed or uploaded here.

Before enabling suspend in the userspace driver:

1. Resolve the board's internal object type, chip count and saved configuration from initialization and a vendor-driver trace.
2. Measure the reference firmware's complete power-on/off transaction stream, including 004C request/ack latency and the following FB24 operation.
3. Locate the OFDM service satisfying this handshake, then implement its actual quiesce/ack behavior and wake reinitialization in the open firmware. Do not substitute RET callbacks or a fabricated version/ack byte.
4. Serialize state-changing operations. Stop stream submissions and drain/cancel pending I/O before suspension. Reopen/revalidate the USB interface on resume; check both cores and decide from measured state whether a reference cold-load or board reinitialization is needed.
5. Validate timeout, disconnect and repeated suspend/resume cases against the reference. Reception, retuning and MPEG-TS continuity must recover, not just version queries.

The debug GUI now offers **Power snapshot**. It checks both cores, reads 13 named power checkpoints, records timestamps and raw USB requests/replies, and stops the register list at the first failure. It never writes the sequence above. Snapshots are sequential and may wake an idle USB link; they are not atomic measurements of a suspended device. Save notes about the action preceding each snapshot. No protocol decoder annotation or snapshot value alone proves successful suspension.

## Measured reference baseline

This session discovered the existing WinUSB-bound 07CA:B865 device: IT9175 revision 1, prechip 0x83, command OUT 02 / IN 81. The cold LINK query returned 0.0.0.0. Loading the exact supplied 5,863-byte reference in 123 records succeeded; LINK returned 3.0.3.0 and the forwarded OFDM version register at 0x804191 returned bytes 03 00 04 05. These measurements establish reference boot and forwarding, not reception or a working open image. See the supplied debug exports for subsequent power snapshots and errors.

The subsequent GUI snapshot successfully queried both versions and all 13 registers. Its raw values were LINK D8BF=00, E00C=01, D8C7=00, D8BB=00, D91B=00, D91C=00; OFDM 004C=00, FB24=00, FBB9=68, F84F=01, FBA8=00, EC40=01, EC3F=00. This is a warm reference-boot baseline before board initialization or tuning, not an observed suspend/resume transition. Windows device-inventory access was denied in this execution environment; that separate failure was preserved alongside successful USB results.

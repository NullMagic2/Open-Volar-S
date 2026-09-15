# OFDM service reconstruction — open firmware 0.1.3.0

The nine previously placeholder-backed services now have independently assembled implementations in `crates/liba865r/src/open_firmware/services.rs`. The generated image is experimental. Offline agreement with the downloaded reference is not evidence of TV reception, electrical timing, or complete suspend/resume support.

## Reconstructed contracts

Addresses identify reference entry points, not addresses to write from Windows. State bytes retain numeric names where the hardware meaning is unverified.

| Reference | Implemented behavior |
| --- | --- |
| `4DDA` | Event dispatcher: `1A` resets acquisition bookkeeping; `1B` advances/resets the `4570` retry counter and conditionally calls ROM `D210`; `1D` pulses `F0EB`; `21` triggers correction controls; `22` updates `F98F` from receiver flags; `65` acknowledges a pending state request. Other event values are no-ops. |
| `503F` | Calls the two preparation services, chooses one of six ROM receiver-state services from `4465/4466/44E3/44E9`, applies the selected `45F7..45FA` profile when enabled, and snapshots completion state. |
| `5153` | Runs the `F625..F633` measurement/gain search; chooses tracking-register settings; detects duplicate candidate IDs; handles candidates 4 and 5 using measured or fallback values; saves two correction configurations; issues the final ROM notification. Each ready poll is bounded. |
| `554E` | Clears serial-test controls, tests SBUF with `78` and `9A` separated by ROM waits, records the observed echo result, selects the reference's follow-up action and sends event `4F`. This is not a generic infrared decoder. |
| `559B` | Clears receiver XDATA from `4422` through `4611`, inclusive. It does not clear the whole device RAM. |
| `55B6` | Applies the two stored timing corrections with the reference's 32-bit addition and single-wrap adjustment around `4000`; writes the resulting low words and companion control bytes; triggers application. ROM comparison semantics are preserved by calling the same utility rather than guessing a replacement formula. |
| `578D` | Writes the six demonstrated defaults at `42FD..4302`. The reference's unsigned-byte comparison against zero cannot take its negative branch. |
| `4EF6` | Sets `459B` when the wrapping eight-bit difference `45EC - 4588` is less than five. This is not a signal-quality percentage. |
| `4F13` | Selects one of three seven-entry threshold tables, retaining the previous table for other selectors; applies the demonstrated first-threshold override when its three gates are satisfied. |

Timer-2 (`4A1B`) now includes periodic ROM maintenance at the ten- and fifty-tick boundaries, status aggregation and output control, in addition to incrementing the timebase and acknowledging TF2. The serial callback (`4DCF`) already matched its full eleven-byte downloaded behavior; it has been retained and relocated.

The five remaining no-op veneers (`57C3`, `57C4`, `57C5`, `57CC`, `57CD`) are RET in the reference itself. They are no longer being used as substitutes for the nine substantial services.

## Verification and its limits

`debug/exports/firmware-reconstruction/differential.json` records 4,129 passing reference/candidate comparisons. The cases exercise every decoded instruction in all nine services, Timer-2 and the serial callback. This is instruction coverage, not exhaustive state/path coverage.

Checks compare final XDATA, ordered MMIO writes, modeled ROM call arguments and the EA/TF2/RI/TI control bits. Cases include all event and threshold selectors, receiver-state combinations, counter rollover, both correction comparison models, acquisition exits, repeated candidates, and serial echo outcomes. A separate stuck-ready test verifies bounded exit and stack balance. Seven instruction-executor self-tests cover arithmetic, stack/SFR separation, calls, branches and unsupported-operation rejection.

The CPU executor is a limited validation tool, not a full tuner emulator. It assumes register bank zero, uses explicit deterministic oracles for unavailable ROM bodies, and does not model asynchronous interrupts, analog circuitry, MMIO side effects or real-time deadlines. Scratch CPU registers and unrelated arithmetic flags are not treated as function outputs. ROM bodies and their complete register-preservation requirements still need hardware confirmation.

The only intentional control-flow difference is the stuck-ready exit: after a bounded number of polls, the candidate disables `F625` and returns `R7=FF`. The reference loops indefinitely. No success flag or lock is fabricated. That return is a development diagnostic; propagation through every ROM caller has not been established.

The new program fits the reference-demonstrated executable RAM window. Rust tests check non-overlap, populated dispatch targets, startup ordering and scatter framing. A generated image contains 4,829 bytes in 95 records and reports 0.1.3.0 on both cores. Rebuilding does not read or require the proprietary payload. The comparison test separately requires a user-supplied reference with SHA-256 `4b066157d0eb1a088e55daaf339418fc6596b5f76c4583d06d5eddb3a272e921`.

## What remained incomplete in 0.1.3.0

This reconstruction does not implement the separate downloaded patches for ROM entries `7353`, `30F6` and `DC0F`; they remain unredirected. It does not reconstruct immutable ROM services, all startup data, complete acquisition integration, calibrated signal-quality reporting or device-wide suspend/resume. The receiver API continues to mark open-firmware reception as experimental. The normal viewing path still requires original OFDM firmware.

The serial/default/profile routines now have concrete behavior; the previous claim that all nine services still redirect to RET is obsolete. The broader claim that the firmware is not yet reception-ready remains valid until hardware tests demonstrate boot, tune, sustained transport and lifecycle recovery.

## Reproduction

From the project directory:

```text
cargo test -p liba865r --offline
cargo run -p a865rctl --offline -- build-open-fw-probe new-open-0.1.3.0.fw
python tools/test_mcs51_service_cpu.py
python tools/validate_ofdm_services.py path/to/reference.fw new-open-0.1.3.0.fw --output differential.json
```

Hardware tests use the newly built CLI, not an older installed Debug Desk:

```text
a865rctl probe-open-fw
a865rctl check-open-fw
a865rctl receive-open 521143 5 new-open-test.ts
```

The upload requires a cold device. The existing firmware must not be overwritten while running. No verified software operation substitutes for unplug/reconnect, and no EEPROM flash is involved.

Instruction and ABI references: [Keil MCS-51 instruction manual](https://www.keil.com/support/man/docs/is51/is51_instructions.asp), [Keil function-return conventions](https://www.keil.com/support/man/docs/c51/c51_ap_funcret.asp). Hardware-specific contracts above come from local analysis of the user-supplied firmware, not those general references.

## Hardware test of this image

On 2026-09-10, after the confirmed physical reconnect, 0.1.3.0 cold-booted on the A865R. Both cores reported the expected version; the scheduler advanced 101 -> 121 at boot, 9574 -> 9594 in the following health check, and 56804 -> 56825 after the reception attempt. Calibration returned 3755. RF22 (521143 kHz) remained channel status 0 / MPEG status 0 after 6022 ms. The receive command failed honestly with no MPEG lock and no transport recording. Both cores remained responsive afterward. This validates boot and timer liveness, not execution of every service on hardware or successful reception. The post-stop state snapshot is retained for comparison and must not be interpreted as a trace of acquisition itself.

Machine-readable results and raw logs are under `debug/exports/firmware-reconstruction/`. That historical test left 0.1.3.0 running; the current test state is open 0.1.4.0.

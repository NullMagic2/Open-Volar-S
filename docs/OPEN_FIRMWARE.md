# Open firmware test results, 2026-09-10

The independently assembled download is an open RAM patch/startup layer for the IT9175's existing mask ROM. It does not replace the immutable ROM. It contains derived hardware addresses and interface constants, source-authored 8051 instructions and explicit placeholders. It contains no bundled proprietary executable firmware payload. Open 0.1.4.0 now receives RF22 on the tested board; see [the current integration report](RF_INTEGRATION_0.1.4.md).

## Experiments on the original A865R

| Image | Result |
|---|---|
| 0.1.1.0, 1,168 bytes / 27 records | LINK recovered and reported 0.1.1.0; OFDM query timed out. Relocating compatibility handlers from unverified 0x6100 RAM to the demonstrated 0x4E00 window did not by itself fix OFDM boot. |
| Open LINK 0.1.1.0 + original OFDM | Both versions answered. RF22 locked in 1.53 seconds and five seconds of TS were recorded. This is a proprietary/open component-isolation experiment, not a fully open firmware. The mixed image was assembled only in memory and was not exported. |
| 0.1.2.0, 1,201 bytes / 28 records | Added the 13-entry ROM interrupt dispatch ABI table at OFDM 0x4710. Both cores reported 0.1.2.0; forwarded register reads worked. Scheduler timebase advanced from 52205 to 52225 during a 20 ms health check. |
| 0.1.2.0 reception experiment | Tuner calibration succeeded: ED23=3755, clock mode 0, increasing calibration boundaries. RF22 remained channel status 0 / MPEG status 0 after six seconds. No TV lock or recording was produced. |

The original boot test incorrectly required a marker written after the ROM scheduler startup call returned. The ROM can remain in its scheduler, so this test rejected a working command processor. `check-open-fw` now requires the current version on both cores, successful forwarded reads and a moving scheduler counter. The post-return marker remains diagnostic only; it is not asserted as proof of boot.

The interrupt table addition resolved the observed command-response failure in this run. This is hardware evidence for the required table, not proof that every table entry or every ROM callback is exercised.

## Historical 0.1.3.0 reconstruction

Open firmware **0.1.3.0** now contains source-assembled replacements for all nine previously missing downloaded services (4DDA, 503F, 5153, 554E, 559B, 55B6, 578D, 4EF6 and 4F13). Timer-2 now includes the downloaded periodic-maintenance contract. The serial callback already matched the reference's complete downloaded body. The remaining five no-op veneers correspond to RET entries in the reference itself.

The 4,129 offline comparison cases pass and exercise every decoded instruction in these services and the two callbacks. The comparisons use declared ROM oracles and do not model RF hardware, asynchronous interrupts or electrical timing. The new image is 4,829 bytes / 95 records. It has not yet demonstrated reception. See [OFDM_RECONSTRUCTION.md](OFDM_RECONSTRUCTION.md) for per-service semantics, test limits, bounded-wait behavior and remaining work.

Those remaining downloaded patches and startup data were completed in 0.1.4.0. RF22 reception, public quality reporting and explicit device standby/resume have now been exercised on hardware. The GUI accepts the tested open image for viewing. Windows system sleep and independent RF-level calibration remain unverified. See [the current report](RF_INTEGRATION_0.1.4.md) for evidence and limits.

## Reproducible commands

```text
a865rctl build-open-fw-probe open-0.1.4.0.fw
a865rctl probe-open-fw
a865rctl check-open-fw
a865rctl receive-open 521143 5 new-open-test.ts
a865rctl probe-components original.fw open-link
```

Only upload commands require a physically cold tuner. The component command uses one user-supplied proprietary core and is clearly identified as such. The normal `receive` command accepts the exact hardware-tested 0.1.4.0 version. `receive-open` remains available for explicit development tests.

There is no verified software cold-reset path. Restarting the userspace process clears its memory, not the tuner RAM. The tested hub-port cycle preserved running firmware; command 0x23 previously stranded the command processor. Neither is presented as a power-cycle substitute. Unplug/reconnect clears the volatile firmware. No EEPROM flash or global USB power policy is changed by these tests.

Detailed logs and each open image are retained under `debug/exports/reception-development/`. The original reference firmware was restored to RAM after the open-firmware experiments for normal reception testing.

## Hardware test of this image

On 2026-09-10, after the confirmed physical reconnect, 0.1.3.0 cold-booted on the A865R. Both cores reported the expected version; the scheduler advanced 101 -> 121 at boot, 9574 -> 9594 in the following health check, and 56804 -> 56825 after the reception attempt. Calibration returned 3755. RF22 (521143 kHz) remained channel status 0 / MPEG status 0 after 6022 ms. The receive command failed honestly with no MPEG lock and no transport recording. Both cores remained responsive afterward. This validates boot and timer liveness, not execution of every service on hardware or successful reception. The post-stop state snapshot is retained for comparison and must not be interpreted as a trace of acquisition itself.

Machine-readable results and raw logs are under `debug/exports/firmware-reconstruction/`. This historical test left 0.1.3.0 in RAM. Subsequent reference and open 0.1.4.0 tests are documented in the current integration report.

# Live TV! 0.8.0-alpha.32

## Receiver startup

General settings now includes Start receiver with Windows (Off/On), translated into Brazilian Portuguese, English, Spanish and Greek. It defaults to Off. Apply registers or removes a quoted, per-user Windows Run command. General now displays Apply. Language selection retains its existing immediate behavior.

At Windows sign-in, `live-tv.exe --receiver-startup` prepares the receiver and exits without creating the player UI, renderer or audio graph. It does not tune a channel or retain USB ownership. A cold receiver receives the existing firmware image in RAM; a warm receiver keeps its current firmware. No driver or firmware code was modified. An open player is checked before hardware access; exclusive USB ownership prevents taking a busy receiver. Absent or unavailable receivers exit quietly. Status is written to `%LOCALAPPDATA%/A865R/TV/receiver-startup.json`.

This is per-user sign-in preparation, not a service running before login. Windows may delay Run entries. Switching Off removes this app's entry. Uninstall removes a matching entry for the uninstalling user. See [Microsoft's Run-key documentation](https://learn.microsoft.com/en-us/windows/win32/setupapi/run-and-runonce-registry-keys).

## Validation

- Player tests: 78 passed, zero failed, two existing GPU tests intentionally ignored.
- New checks cover Unicode/quoted startup paths and the Windows command-length limit, isolated registry enable/disable round trips, preview isolation, and measured General-label fit in all four languages at 96, 120, 144 and 192 DPI.
- Release executable on the available physical A865R: exit code 0, status ready, 190 ms, firmware already 0.1.4.0, no firmware upload, no tuning, no GUI. Device handles were released on return. No running live-tv process was present when subsequently checked.
- Release preview command: exit code 0, preview-skipped-hardware, no GUI or tuning.
- The real Windows startup registration remained absent after validation. Tests used a disposable registry key and isolated profile directories.
- Source packaging checks ZIP CRC and every source-manifest SHA-256. Delivery also checks installer-copy and staged-player identity.

Cold-device initialization at actual Windows sign-in and the busy-player guard were not exercised in this release check. Native interactive settings inspection was unavailable; automated GDI text measurement verified the added labels fit. No reboot, installation or automatic-start opt-in was performed during validation.

## FM investigation

The separate FM experiment produced no usable FM audio. Requested frequencies reached the command shadow registers, but physical RF lock was not verified. The tested endpoint remained the digital-TV transport path. A different antenna could improve signal strength but cannot add a missing FM demodulator or raw-signal output mode. Hardware/firmware capability remains unproven; no speculative firmware patch is included. TV reception was successfully restored after that experiment.

Driver and Debug Desk remain 0.7.0. Earlier application validation is retained in ALPHA31-VALIDATION.md.

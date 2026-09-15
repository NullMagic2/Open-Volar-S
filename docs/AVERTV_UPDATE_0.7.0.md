# AVerTV compatibility follow-up to 0.7.0

Validated locally on September 10, 2026, with AVerTV 3D, the A865R bound to
WinUSB, and open LINK/OFDM firmware 0.1.4.0. This follow-up retains version
0.7.0; package SHA-256 hashes distinguish it from the earlier baseline.

## Playback and normal resolution

AVerTV displayed live broadcasts on RF50 and RF22, and channel switching
worked. The adapter delivers the original compressed transport stream without
decoding, deinterlacing, rescaling, or color conversion. AVerTV retains control
of its normal renderer and resolution. No 4K upscaling is applied to AVerTV.
Standalone player settings, including 4K and ICC processing, are independent.

AVerTV's saved country/channel settings were attached to its old device name.
`tools/migrate_avertv_profile.py` copies only missing settings to the active
A865R profile, preserving the existing channel database. It defaults to a
dry run; `--apply` requires AVerTV to be closed and creates an original backup.
It does not change the USB driver, vendor executables, or encrypted channel data.

AVerTV also submits an unsupported provisional frequency before its real tune.
The adapter now keeps pending and committed settings separately. StartChanges
discards uncommitted changes; Pause/Run use the last successful commit. The
unsupported request remains an error and cannot replace the committed channel.

## Measured signal quality

The receiver reads the firmware's public quality register at 0x800049, with
MPEG-lock checks before and after. Monitoring runs about every 500 ms on the
existing USB owner thread while transport delivery continues. The BDA adapter
caches the result for IBDA_SignalStatistics::get_SignalQuality and the matching
KS property. These queries do not open a second USB connection.

AVerTV visibly displayed `Qualidade: 100%`. The live log also contains lower
measurements such as 57%, 62%, and 97% during tuning/acquisition. No signal is
reported as 0%; a locked receiver without a valid measurement returns E_PENDING.
Retuning/stopping clears the previous measurement. Out-of-range values are not
clamped into an invented percentage. A monitoring read failure invalidates the
cached signal report while video streaming can continue.

This is the firmware's relative reception-quality percentage, not calibrated
RF power in dBm and not proof of error-free reception. Signal strength remains
unsupported.

## Validation and limits

- 51 x64 workspace tests and 43 x86 core/BDA tests passed, including rejected
  tune transactions, measured quality, invalid values, and loss of lock.
- Direct x86 and x64 BDA captures each delivered 7,267,328 bytes in five seconds
  before the quality follow-up. The latest x86 DLL was then tested in AVerTV
  with live picture and measured quality; x64 AVerTV UI playback was not tested.
- Windows' per-process meter detected nonzero audio in all 30 samples during
  the earlier successful playback run. A later short sample from the quality
  run was silent; that sample does not establish continuous audio or its cause.
- The generic Microsoft Network Provider example can connect filters but still
  fails to forward the requested tune and produce transport. This remains a
  separate compatibility issue despite successful AVerTV playback.
- Use one tuner client at a time. Full graph error events, shared ownership,
  vendor remote-property forwarding, and arbitrary infrared remotes remain open.
- The GUI installer does not manage the experimental BDA registration. This
  machine's user-registered x86/x64 adapters were updated separately. The older
  system/service registration was retained. A complete compatibility installer
  and uninstaller remain future work.

The follow-up source-and-tools archive includes the current buildable workspace,
both adapter DLLs, and the unchanged source-built firmware. It excludes local
TV recordings, screenshots, original vendor firmware, and personal profiles.
The original baseline installer is unchanged. The follow-up installer refreshes
the GUI, CLI, firmware, tools and documentation; BDA DLLs are supplied in the
source-and-tools archive and still require separate registration.
Close AVerTV before replacing loaded adapter DLLs; unpacking the
archive alone does not register them on a new machine.

Local evidence is under `debug/exports/avertv-070`: adapter logs, test output,
profile-migration report, screenshots, and separate audio-meter reports.

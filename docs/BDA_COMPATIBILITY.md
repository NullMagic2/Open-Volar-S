> Alpha.37 update: external BDA clients can now use persistent signal-processing presets. Original remains the default. See ALPHA37-VALIDATION.md; historical pass-through and installer limitations below describe earlier releases.

# Experimental userspace BDA adapter

`windows/a865r-bda` builds both 32-bit and 64-bit DirectShow COM adapters. The
physical device remains bound to WinUSB. This is development software, not a
complete replacement for the original Windows BDA driver.

Implemented: a combined tuner/demodulator source plus an optional capture pass-through, pin enumeration and transport
negotiation, frequency/bandwidth controls, tune transactions, actual RF lock
status, and MPEG transport delivery through the standard sample allocator.
Signal quality is measured from the receiver firmware and exposed through both
IBDA_SignalStatistics and the BDA signal-statistics property set. Unsupported
controls, including signal strength in dB, return errors.
The USB discovery detail header now has the Windows-required x86 size of 6 and
x64 size of 8. Both architectures have opened and streamed from the device.

The September 10 follow-up recorded 7,267,328 bytes in five seconds through
tuner -> capture -> Windows File Writer on each of x86 and x64.

AVerTV 3D now displays live TV and changes channels with the x86 adapter and
open firmware 0.1.4.0. Its quality display reached 100% and the underlying
measurement also reported lower values during acquisition and channel changes.
An earlier playback run had nonzero per-process audio output; a later short
audio sample was silent, so continuous audio is not established by that sample.
The missing device-specific country/channel profile was migrated, and tuning
transactions now discard rejected drafts instead of letting them break Run.
The unsupported provisional 123350 kHz request remains rejected.
See AVERTV_UPDATE_0.7.0.md for current evidence and limitations.

The adapter passes the original compressed MPEG transport stream to AVerTV.
It does not decode, deinterlace, upscale to 4K, or apply our player's color
processing. AVerTV uses its own normal renderer and resolution behavior.
The standalone player's upscaling settings do not affect AVerTV.

Both architectures pass the Windows Network Provider connection test, but
the generic provider's full tuning/streaming example remains unsuccessful.
Connection success alone does not establish a working provider playback path.
Automatic IBDA_DigitalDemodulator controls and the correct antenna subtype are implemented.
Vendor IR property forwarding,
shared ownership between applications/services, complete graph error events,
all network-provider methods and integration with the installer remain open.
Use one TV/diagnostic client at a time. Cold startup now uses the source-built open 0.1.4.0 image by default.
A865R_FIRMWARE optionally selects the matching reference image. Earlier BDA
capture failures below predate the successful 0.1.4.0 RF integration tests.

## Local development registration

Start Menu repair (September 10): a failing launch loaded an outdated system
adapter while successful development launches loaded the current user adapter.
Both installed architectures have now been backed up and synchronized using
`tools/repair-installed-bda.ps1`, with installed hashes verified and AVerRemote
restarted. This script repairs an existing matching installation; it is not a
general installer or a fresh device-registration tool. Keep both scopes aligned
when deploying updates. The original vendor kernel driver is not required.

The local test added BDA network-tuner and receiver interface GUIDs to the
existing WinUSB DeviceInterfaceGUIDs list, and associated each interface with
our own COM CLSID. It did not install the old AVerMedia kernel driver. Both COM
architectures were registered for the user and, to repair AVerRemote, for Windows
services under Program Files/A865R/compatibility. Software-category duplicates
were removed; discovery uses physical PnP monikers.

The Debug Desk installer deliberately does not enable or remove this separate
experimental registration. Its uninstall preserves the separate compatibility
directory and the current USB binding. Do not manually delete registered DLLs
while testing; AVerRemote needs its system registration. A complete transactional
compatibility install/uninstall is still required before general distribution.

Class IDs: tuner `{2A5FC455-33C1-482C-9A83-4DF863F0C601}`; capture
`{2A5FC455-33C1-482C-9A83-4DF863F0C602}`. The classes are openly identified as the
A865R Open Driver Project; only the legacy discovery display name is compatible
with AVerTV's expected device name.


# Open Volar S 0.9.5 — Linux channel switching

0.9.5 includes the control-socket recovery and automatic channel-selection fixes
validated below. All project versions, Windows resources, installers and DKMS
metadata have been bumped so this fix is distinguishable from original 0.9.4
packages. Artifacts are in `dist/release-0.9.5`.

Release checks on 2026-09-25: native release builds, Windows x64 application
and x64/x86 BDA cross-builds, Linux DEB/RPM and the Wine/Inno Windows installer
completed successfully. All 32 native-player unit tests and the three packaging
workflow/permissions tests passed. Windows Live TV and installer file/product
resources report 0.9.5.0; all ten workspace packages and lockfile entries report
0.9.5. External dependency versions are unchanged. Channel-switch playback
results below apply to the identical fix before this metadata-only version bump;
physical Windows playback has not been retested.

---

# Open Volar S 0.9.4 — Linux channel-switch fix

The refreshed packages are in `dist/release-0.9.4-channel-switch`.
The desktop journal identified the player exit as `Address already in use`
after a channel change: forced shutdown left the Unix control socket behind.
The native player now recovers a disconnected socket owned by the current user,
while preserving live listeners and ordinary files. GTK also ignores service
updates from the stopping job so they cannot overwrite the newly selected
channel. Selecting a channel automatically queues playback; another selection
during shutdown replaces that queued request.

Validation on 2026-09-25:

- The orphaned-socket regression failed with OS error 98 before the fix.
  All **32 native-player unit tests** now pass, including repeated recovery,
  active-listener protection and regular-file preservation.
- The GUI channel-selection regression passes: it queues Watch automatically,
  retains the selected frequency/service despite old metadata, and honors the
  latest selection during shutdown.
- The rebuilt player passed three forced stops and restarts on the same socket
  using a captured TV Gazeta broadcast, with video advancing automatically on
  all four starts. Final playback completed with 586 fields and zero scheduler
  drops. This was muted and did not open or retune the live tuner.
- Linux applications and DEB/RPM are rebuilt. Windows binaries and installer
  are unchanged from the verified 0.9.4 build below.

---

# Initial 0.9.4 version update (historical)

0.9.4 carries forward the native playback and aspect-ratio changes from the
0.9.3 native-player refresh. This is a version-only release: playback algorithms
and settings are unchanged. Cargo packages, lockfile, Windows version resources,
installer metadata, DKMS package/hooks and release filenames use 0.9.4.

Artifacts are in `dist/release-0.9.4`. Hardware requirements and remaining
Wine/WSL/recording-export limitations are unchanged; see NATIVE-LINUX-PLAYER.md.
Historical playback results are retained below with their original versions.

Checks performed for 0.9.4 on 2026-09-25:

- Native release builds and Windows x64 application/x64+x86 BDA cross-builds
  completed successfully. DEB and RPM packages were rebuilt.
- All 29 native-player unit tests and the three device-permissions, build-workflow
  and Wine-build-environment checks passed.
- DEB/RPM versions, DKMS source path and maintenance hooks report 0.9.4.
  The DEB's application binaries match the native release outputs.
- Windows Live TV file/product version resources report 0.9.4.0. The Windows
  installer compiled successfully using Inno Setup 7.0.2 under isolated Wine.
- Playback hardware tests were not repeated for this version-only change.
  Physical Windows playback and installation remain unverified.

---

# Open Volar S 0.9.3 — native playback validation (historical)

Build date: 2026-09-25. This refresh integrates the native player into the GTK
application and includes it in the DEB, RPM and standalone Linux archive.
Windows builds share the extracted deinterlacing, aspect-ratio and PCM sources.
The old release notes below describe earlier 0.9.3 binaries, not this backend.

Read [NATIVE-LINUX-PLAYER.md](NATIVE-LINUX-PLAYER.md) for supported hardware,
installation, new features and remaining scope. Native Linux playback is free
of FFmpeg/mpv/GStreamer; recording export and Wine/WSL bridges are not yet ported.

Checks performed for the 0.9.3 native playback refresh:

- Native release builds and Windows x64 application/x64+x86 BDA cross-builds.
- DEB and RPM include the native helper and required library dependencies. The
  helper extracted from the DEB matches the release binary SHA-256 and passed
  real-broadcast control/EOF plus forward/backward seek/audio-track tests.
- Windows installer rebuilt successfully with Inno Setup 7.0.2 under isolated
  Wine. Windows playback and installer execution on physical Windows remain
  unverified; the compiler itself completed successfully.
- Native player: **29 unit tests passed**, covering shared logic, transport,
  AAC, audio, clock, proportions and cache behavior.
- Actual GTK embedded playback: pause/resume controls, displayed-frame seek,
  picture settings, persistence and rendered snapshot.
- Automatic 16:9 and forced 4:3, UHD processing, resizing while paused; shared
  geometry tests additionally cover 16:10, 5:4, portrait windows and SD aperture.
- The user previously confirmed stereo audio and lip-sync in the native player.
- Fresh tuner capture: TV Gazeta HD at 521143 kHz, service 16960, native LATM
  AAC playback. Pause/resume, sound modes and deinterlacing changes passed;
  570 fields presented, zero dropped, clean EOF in 12.9 seconds. This test was
  muted; the earlier stereo lip-sync test was audible and confirmed by the user.
- Source and artifact manifests provide SHA-256 checksums.

The original Windows native playback and old Wine registration validations have
not been repeated on physical Windows for this refresh. Hardware tests here use
Ubuntu, Mesa 26.0.8 and Radeon RX 7900 XTX. Packages require glibc 2.43.

---

# Earlier 0.9.3 validation (historical)

Build date: 2026-09-24. Host: native Ubuntu x86_64, kernel 7.0.0-30-generic,
glibc 2.43. Native Rust 1.93; Windows GNU cross-build Rust 1.98.1 and MinGW-w64 GCC 13.

## Release contents

Version 0.9.3 includes all 0.9.1 fixes: automatic Linux permissions and DVB
registration, paired Linux window focus, larger Linux fonts, numeric channel
ordering, the larger Wine interface, automatic authenticated Wine helper startup,
and Wine-compatible BDA registration. The updated 0.9.3 also adds native Linux
ISDB caption rendering using the Windows decoder/layout, a persistent live EPG
with full event descriptions and channel filtering, and gpu-next playback with
verified double-rate/single-rate deinterlacing. It includes a complete ZIP of
current source and build artifacts.
Third-party dependency versions are unchanged.

The historical hardware, playback, GUI and Wine registration results are preserved
in [RELEASE-0.9.1-VALIDATION.md](RELEASE-0.9.1-VALIDATION.md). They were obtained with
0.9.1 and must not be interpreted as new hardware tests of 0.9.3. The user installed the initial 0.9.3 DEB on this host and DKMS completed
successfully. Later UI refinements below require the refreshed 0.9.3 package.

## 0.9.3 release checks

- Native GUI, CLI, debugger, package-builder GUI, WSL helper, DVB broker and kernel
  module build completed. DEB/RPM packages report version 0.9.3.
- Windows x64 applications, x64/x86 BDA adapters and x64 diagnostic probes rebuilt.
  The GUI's embedded Windows file/product version is 0.9.3.0.
- All nine project Cargo packages and lockfile entries report 0.9.3.
- The rebuilt 0.9.3 passed 28 receiver-library tests, 21 protocol tests,
  30 media tests, 32 GUI tests (including the isolated rerun described below),
  and one explicitly invoked broadcast-caption fixture test: 112 in total.
- Both permissions/build-workflow tests and shell/Python syntax checks passed.
- The Windows installer was compiled with Inno Setup 7.1.0 under Wine 10.0.
  Installation to a path containing spaces, automatic helper setup (0600 token),
  migration of old registration, and uninstall cleanup passed. The installed
  application, native helper and both adapters matched their build outputs.
- Both installed BDA categories enumerated under Wine and bound to IBaseFilter;
  uninstall removed their entries. This does not establish third-party playback.
- The complete ZIP, nested archives, source manifest and release checksums were
  checked after packaging. Broadcast hardware tests remain the separately labelled
  0.9.1 results linked above.

## Updated Linux captions, EPG and playback validation

- Native media regression suite: 30 passed, one optional recording-fixture test
  ignored by the ordinary run. Native GUI suite: 32 passed in isolated Xvfb
  processes using the rebuilt 0.9.3 binaries. The settings-window placement test
  failed its timing-sensitive synthetic-move assertion once while the cross-build
  was running, then passed when rerun alone; that test remains timing-sensitive.
  The separately invoked broadcast-caption test also passed.
- The caption renderer uses bundled libaribcaption, FreeType/fontconfig and the
  same layout helper as Windows. Synthetic Portuguese captions passed timed
  appearance/clear, pause, backward seek, resize and premultiplied-alpha tests.
  An mpv/Xvfb integration test verified actual overlay pixels, CC off/on, and
  that seeking backward removes a future caption.
- A separately invoked recording-fixture test decoded/rendered four caption
  images from the saved TV TRIBUNA HD transport stream and verified the recording
  reader resets on a backward seek and follows the selected video service in a
  file containing multiple services. This is replay of a prior capture, not a new
  live reception test. Its caption PES clock was unrelated to video; the parser
  now anchors gross clock mismatches to the video and preserves the resulting
  offset. Valid caption timestamps remain unchanged.
- A generated two-second interlaced clip produced 100 frames through mpv/bwdif
  Double rate and 50 through Single rate. The native settings/shader/OSD tests
  also passed with gpu-next. Double rate already existed; this update fixes the
  production renderer path and makes the native filter syntax consistent.
- Guide tests verify persistent updates without merging different multiplexes,
  live GTK refresh, numeric channel filtering and full accented descriptions.
  Broadcast event mapping is shared with Windows. Guide contents depend on the
  information received while tuning each multiplex.
- Caption extraction/history is bounded. Original 188-byte TS recordings have
  native ISDB support; converted recordings need their own subtitle track.
  Backward seeks reparse original recordings off the UI thread, so long files
  may require time to recover caption state. No new tuning test interrupted the
  user's running VLC session, and the new native package was not installed over it.

### VLC metadata investigation

Native VLC 3.0.23 replayed the saved multiplex with eight `PSI section too long`
warnings. A diagnostic copy omitting nine empty EIT stuffing packets produced
zero such warnings. The production driver keeps the original stream intact;
Open Volar S's EPG parser already ignores stuffing. This identifies the cause in
this capture, not every possible PSI error.

Auto/ARIB interpretation produced Japanese-looking Portuguese EPG titles.
`--ts-standard=dvb` decoded the sample's titles as `MELHOR DA TARDE` and
`BRASIL URGENTE`; the station label became a generic programme number. That flag
changes demux interpretation and can affect ISDB EPG/subtitles. Keep the tuner
URL `isdb-t://frequency=641143000:bandwidth=6`; in VLC's GUI use the Network tab,
Show more options, and append ` :ts-standard=dvb` in Edit options. The frequency
and programme 17056 are the locally tested station, not universal settings.

## Build outputs

| Artifact | Contents |
| --- | --- |
| Open-Volar-S-All-Files-0.9.3.zip | All release artifacts below, internal checksums and a readme |
| Open-Volar-S-Setup-0.9.3-x64.exe | Unsigned Windows installer, native Linux Wine helper, x64/x86 BDA adapters |
| Open-Volar-S-Windows-0.9.3.zip | Portable x64 application/CLI/debugger, x64/x86 adapters and diagnostic probes |
| open-volar-s_0.9.3_amd64.deb | Ubuntu/Debian applications, permissions, DVB broker and DKMS source |
| open-volar-s-0.9.3-1.x86_64.rpm | Equivalent RPM package |
| Open-Volar-S-Linux-Binaries-0.9.3.tar.gz | Native GUI, CLI, debugger, package-builder GUI, WSL helper, DVB broker and Wine launcher |
| open_volar_s_usb-7.0.0-30-generic.ko | Module for this exact kernel; use DKMS on other supported kernels |
| Open-Volar-S-Source-0.9.3-updated.zip | Complete source, build scripts, tests, documentation and source SHA-256 manifest |
| SHA256SUMS | Individual artifact checksums; the complete ZIP has a separate .sha256 file |

The complete ZIP excludes previous release archives and broadcast recordings.

## Compatibility limits

Prebuilt Linux programs require glibc 2.43. Rebuild from source on older Linux
systems. RPM installation on another distribution remains untested. Windows
BDA is a userspace compatibility adapter over WinUSB, not a signed kernel AVStream
driver. Native Windows discovery/playback and full playback in third-party
Windows TV applications under Wine remain unverified. Wine discovery and binding
are separate from complete third-party tuning/playback support.

The separate Windows WSL FFmpeg video host was not cross-built because the required
Windows FFmpeg development SDK is unavailable; its source is included. The normal
Windows application and the Linux WSL helper are included as binaries.

For Wine, install the native Linux package for permissions/driver/mpv, then run
the Windows installer as your desktop user. Authentication, helper startup and
prefix-local BDA registration are automatic. See [WINE.md](WINE.md).

## Reproduce

Native build: `bash build.sh`; packages: `bash linux/package.sh --format all`.
Windows x64: `bash windows/cross_build.sh`; x86 adapter:
`bash windows/cross_build.sh i686-pc-windows-gnu`.
Installer: `ISCC=/path/to/ISCC.exe bash windows/installer/build_linux.sh --no-build`.
After all builds, `python3 tools/package_deliverables.py` generates the individual
archives and complete release ZIP, with source manifest and checksums.

## 0.9.3 Linux guide and active controls refresh (2026-09-24)

- Linux EPG now uses the Windows settings-frame artwork, warm brown panels,
  cream text, gold borders and selected rows, and matching custom close control.
  Channel filtering, live descriptions, age ratings and double-click tuning are
  retained. The title bar supports dragging and double-click maximize/restore;
  the lower-right grip supports resizing, with a readable minimum window size.
- DAC CC has a persistent yellow border/text when enabled; REC has a persistent
  red border/text while recording. The viewer REC label turns red alongside its
  existing recording lamp. Windows appearance and interface dimensions are unchanged.
- Removed stale Wine build-test menu shortcuts that referred to a deleted /tmp
  prefix. Their backup is in dist/validation/wine-test-shortcut-cleanup. The Wine
  build helper now disables winemenubuilder and isolates XDG desktop integration
  inside its build prefix. Normal user Wine installations retain their shortcuts.
- Normalized the Linux Live TV desktop file line endings. The original 0.9.3 DEB
  installation and DKMS succeeded; the launcher error concerned a stale Wine
  test shortcut, not failure to install the Linux application.

Validation: native release build and DEB/RPM packaging passed. The existing guide
filter/live-details test passed, with a screenshot captured on an isolated Xvfb
screen. The existing language/layout/close test passed in its own process. These
GTK tests must run in separate processes: combining them initializes GTK from two
Rust test threads, even with --test-threads=1. Shell syntax, both desktop files,
and three packaging/environment tests passed. Rendered active/inactive DAC
controls for all three materials and inspected the active colours. No live tuner
access or interruption of the user's running playback was needed. Windows binaries
and installer are unchanged from the earlier validated 0.9.3 build.

Visual evidence: dist/validation/release-093-guide-preview.png and
release-093-active-buttons.png; focused test logs carry the release-093-ui prefix.

## 0.9.3 DVB-T compatibility alias (2026-09-24)

The Linux driver now accepts SYS_DVBT as an alias for the existing ISDB-T
controller. ISDB-T remains the first advertised delivery system. The alias is
on by default for VLC 3's capture dialog and can be disabled with the read-only
module option dvbt_compat=0. No new RF standard, broker ABI, transport-stream
processing, encoding, or buffering is involved. The tuning frequency range and
6 MHz bandwidth restrictions remain in force. Performance was not benchmarked;
the code adds only a tuning-time conditional, with no work per packet.

Hardware validation on the connected A865R:
- Built the module for 7.0.0-30-generic. Verified actual DTV_ENUM_DELSYS with
  compatibility disabled (ISDB-T only) and enabled (ISDB-T then DVB-T).
- VLC 3.0.23 successfully set delivery system 3 (DVB-T), acquired HAS_LOCK at
  521143000 Hz / 6 MHz, and captured TV GAZETA HD (program 16960).
- ffprobe identified 1920x1080 H.264 video and AAC audio. FFmpeg decoded a sample
  and a full-resolution frame was inspected. Native isdb-t reception on the same
  frequency also succeeded after the change.
- A separate 10-second direct VLC decoding run (dummy audio/video outputs,
  ts-standard=dvb) successfully created its H.264 video and audio outputs.
- Short remuxed captures on BOTH tuning paths contain VLC timestamp/packetizer
  and FFmpeg reference-frame/PES warnings; this is not a clean-stream claim.
  Direct playback also logged an initial buffer-deadlock-prevented warning.
- Native DEB/RPM rebuilt; three package/environment tests and shell syntax passed.

Logs and frame: dist/validation/release-093-dvbt/. Test capture files remain in
/tmp/ovs-dvbt-compat and /tmp/ovs-isdb-native, outside the release/source archives.
See TV-COMPATIBILITY.md for VLC's GUI fields and how to disable the alias.

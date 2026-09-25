#!/usr/bin/env python3
"""Package previously built Linux/MinGW outputs and a verified source manifest."""
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tarfile
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]
DIST = Path(os.environ.get('OPEN_VOLAR_S_DIST', ROOT / 'dist')).resolve()
LINUX_TARGET = Path(os.environ.get('OPEN_VOLAR_S_LINUX_TARGET', ROOT / 'target')).resolve()
WINDOWS_TARGET = Path(os.environ.get('OPEN_VOLAR_S_WINDOWS_TARGET', ROOT / 'target')).resolve()
WORK = Path(tempfile.mkdtemp(prefix='open-volar-s-release-'))
DIST.mkdir(parents=True, exist_ok=True)
WORK.mkdir(exist_ok=True)

def copy(source, destination):
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination)

def zip_tree(directory, output):
    with zipfile.ZipFile(output, 'w', zipfile.ZIP_DEFLATED, compresslevel=6) as archive:
        for path in sorted(directory.rglob('*')):
            if path.is_file():
                archive.write(path, path.relative_to(directory.parent))

windows = WORK / 'Open-Volar-S-Windows-0.9.5'
if windows.exists():
    shutil.rmtree(windows)
for arch, target, names in [
    ('x64', 'x86_64-pc-windows-gnu', ['live-tv.exe', 'a865r-debug.exe', 'a865rctl.exe', 'a865r_bda.dll']),
    ('x86', 'i686-pc-windows-gnu', ['a865r_bda.dll']),
]:
    for name in names:
        copy(WINDOWS_TARGET / target / 'release' / name, windows / arch / name)
for path in (WINDOWS_TARGET / 'x86_64-pc-windows-gnu/release/examples').glob('*.exe'):
    if not re.search(r'-[0-9a-f]{16}\.exe$', path.name):
        copy(path, windows / 'x64' / ('probe-' + path.name))
# Include the complete GNU runtime dependency closure; OS DLLs ship with Windows.
pending = list((windows / 'x64').glob('*.exe')) + list((windows / 'x64').glob('*.dll'))
seen = set()
while pending:
    binary = pending.pop()
    imports = subprocess.check_output(['x86_64-w64-mingw32-objdump', '-p', str(binary)], text=True)
    for name in re.findall(r'DLL Name:\s+(\S+)', imports):
        if not name.lower().startswith(('libstdc++', 'libgcc_', 'libwinpthread')) or name in seen:
            continue
        seen.add(name)
        runtime = Path(subprocess.check_output(['x86_64-w64-mingw32-g++', '-print-file-name=' + name], text=True).strip())
        if not runtime.is_file():
            raise RuntimeError('Missing GNU runtime: ' + name)
        destination = windows / 'x64' / name
        copy(runtime, destination)
        pending.append(destination)
for name in ['LICENSES.md', 'THIRD_PARTY.md']:
    copy(ROOT / name, windows / name)
copy('/usr/share/doc/gcc-mingw-w64-x86-64-posix-runtime/copyright', windows / 'licenses/MinGW-GCC-copyright.txt')
copy(ROOT / 'windows/register_tuner.ps1', windows / 'register_tuner.ps1')
copy(ROOT / 'docs/TV-COMPATIBILITY.md', windows / 'TV-COMPATIBILITY.md')
copy(ROOT / 'docs/WINE.md', windows / 'WINE.md')
copy(ROOT / 'docs/BUILD-AND-TEST-REPORT.md', windows / 'BUILD-AND-TEST-REPORT.md')
copy(ROOT / 'docs/RELEASE-0.9.1-VALIDATION.md', windows / 'RELEASE-0.9.1-VALIDATION.md')
(windows / 'README.txt').write_text('''Open Volar S 0.9.5 — Windows cross-build (x64 application; x64/x86 BDA adapters)
Extract to a permanent directory. Requires the existing A865R WinUSB binding.
Run PowerShell: powershell -NoProfile -ExecutionPolicy Bypass -File .\\register_tuner.ps1
Launch x64\\live-tv.exe. Restart VLC after registration.
Before deleting this folder, run the registration script with -Unregister.
The DLLs in x64 must stay beside the EXEs. Native Windows fonts are unchanged.
probe-discover.exe enumerates standard BDA categories. Other probe EXEs are
engineering diagnostics; some use hardcoded local frequencies (see source).
These binaries were cross-compiled on Linux, NOT playback-tested on Windows.
For Wine, install the Linux package and use open-volar-s-wine x64/live-tv.exe.
The Windows installer configures its Wine helper automatically; see WINE.md.
See TV-COMPATIBILITY.md and BUILD-AND-TEST-REPORT.md for limitations.
''')
zip_tree(windows, DIST / 'Open-Volar-S-Windows-0.9.5.zip')

linux = WORK / 'Open-Volar-S-Linux-0.9.5'
if linux.exists():
    shutil.rmtree(linux)
for name in ['a865rctl', 'a865r-debug', 'open-volar-s-live-tv', 'open-volar-s-installer', 'open-volar-s-wsl-player', 'open-volar-s-dvb-bridge', 'open-volar-s-player']:
    copy(LINUX_TARGET / 'release' / name, linux / 'bin' / name)
copy(ROOT / 'linux/open-volar-s-wine', linux / 'bin/open-volar-s-wine')
copy(ROOT / 'docs/WINE.md', linux / 'WINE.md')
copy(ROOT / 'docs/NATIVE-LINUX-PLAYER.md', linux / 'NATIVE-LINUX-PLAYER.md')
copy(ROOT / 'COMPATIBILITY.md', linux / 'COMPATIBILITY.md')
for name in ['LICENSES.md', 'THIRD_PARTY.md']:
    copy(ROOT / name, linux / name)
copy(ROOT / 'docs/BUILD-AND-TEST-REPORT.md', linux / 'BUILD-AND-TEST-REPORT.md')
copy(ROOT / 'docs/RELEASE-0.9.1-VALIDATION.md', linux / 'RELEASE-0.9.1-VALIDATION.md')
(linux / 'README.txt').write_text('''Native Linux x86_64 binaries built on Ubuntu with glibc 2.43.
Use the DEB/RPM for automatic DKMS, service and permissions installation.
Native playback requires GTK3, Vulkan Video H.264 support, FAAD2, PulseAudio,
X11/XWayland and Little CMS. WSL/Wine and recording export retain their earlier
backends; see NATIVE-LINUX-PLAYER.md for the exact scope and limitations.
The graphical package builder requires the source tree: run it from the
extracted source root or set OPEN_VOLAR_S_SOURCE to that directory.
The WSL player helper is a Linux binary, not the Windows FFmpeg video host.
See BUILD-AND-TEST-REPORT.md. Rebuild from source on older distributions.
''')
with tarfile.open(DIST / 'Open-Volar-S-Linux-Binaries-0.9.5.tar.gz', 'w:gz') as archive:
    archive.add(linux, arcname=linux.name)
copy(ROOT / 'linux/open_volar_s_usb.ko', DIST / ('open_volar_s_usb-' + os.uname().release + '.ko'))

excluded = {'.git', '.codex', '.agents', '.build-cache', 'target', 'dist', '__pycache__', '.package-work', '.tmp_versions'}
sources = []
for directory, dirs, files in os.walk(ROOT):
    dirs[:] = [name for name in dirs if name not in excluded]
    for name in files:
        path = Path(directory) / name
        relative = path.relative_to(ROOT)
        if name == 'SOURCE-MANIFEST.json' or name.endswith(('.pyc', '.o', '.ko', '.mod', '.mod.c', '.cmd')) or name in {'Module.symvers', 'modules.order'}:
            continue
        sources.append((relative, path))
manifest = {'project': 'Open Volar S', 'application': 'Live TV!', 'version': '0.9.5',
            'variant': 'Native Linux Vulkan Video playback, shared Windows deinterlacing and PCM, existing DVB/BDA and Wine support',
            'files': {str(relative): {'bytes': path.stat().st_size, 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()} for relative, path in sorted(sources)}}
(ROOT / 'SOURCE-MANIFEST.json').write_text(json.dumps(manifest, indent=2) + '\n')
sources.append((Path('SOURCE-MANIFEST.json'), ROOT / 'SOURCE-MANIFEST.json'))
with zipfile.ZipFile(DIST / 'Open-Volar-S-Source-0.9.5-updated.zip', 'w', zipfile.ZIP_DEFLATED, compresslevel=6) as archive:
    for relative, path in sorted(sources):
        archive.write(path, Path('open-volar-s') / relative)
copy(ROOT / 'docs/BUILD-AND-TEST-REPORT.md', DIST / 'BUILD-AND-TEST-REPORT.md')
copy(ROOT / 'COMPATIBILITY.md', DIST / 'COMPATIBILITY.md')
for name in ['NATIVE-LINUX-PLAYER.md', 'WINE.md', 'TV-COMPATIBILITY.md', 'RELEASE-0.9.1-VALIDATION.md']:
    copy(ROOT / 'docs' / name, DIST / name)
outputs = [DIST / name for name in [
    'BUILD-AND-TEST-REPORT.md',
    'NATIVE-LINUX-PLAYER.md',
    'COMPATIBILITY.md',
    'WINE.md',
    'TV-COMPATIBILITY.md',
    'RELEASE-0.9.1-VALIDATION.md',
    'Open-Volar-S-Linux-Binaries-0.9.5.tar.gz',
    'Open-Volar-S-Setup-0.9.5-x64.exe',
    'Open-Volar-S-Source-0.9.5-updated.zip',
    'Open-Volar-S-Windows-0.9.5.zip',
    'open-volar-s-0.9.5-1.x86_64.rpm',
    'open-volar-s_0.9.5_amd64.deb',
    'open_volar_s_usb-' + os.uname().release + '.ko',
]]
(DIST / 'SHA256SUMS').write_text(''.join(hashlib.sha256(path.read_bytes()).hexdigest() + '  ' + path.name + '\n' for path in outputs))
bundle = DIST / 'Open-Volar-S-All-Files-0.9.5.zip'
bundle_root = Path('Open-Volar-S-All-Files-0.9.5')
with zipfile.ZipFile(bundle, 'w', zipfile.ZIP_DEFLATED, compresslevel=1) as archive:
    for path in outputs + [DIST / 'SHA256SUMS']:
        archive.write(path, bundle_root / path.name)
    archive.writestr(str(bundle_root / 'README.txt'), '''Open Volar S 0.9.5 — native Linux playback update

Windows installer: Open-Volar-S-Setup-0.9.5-x64.exe
Windows portable applications and x64/x86 BDA adapters: Open-Volar-S-Windows-0.9.5.zip
Ubuntu/Debian installer: open-volar-s_0.9.5_amd64.deb
RPM installer: open-volar-s-0.9.5-1.x86_64.rpm
Linux standalone programs: Open-Volar-S-Linux-Binaries-0.9.5.tar.gz
Complete source: Open-Volar-S-Source-0.9.5-updated.zip

This update adds the native Linux player with the shared Windows deinterlacer.
Read NATIVE-LINUX-PLAYER.md for hardware requirements and remaining limitations.
Linux binaries require glibc 2.43; rebuild from source on older systems.
The standalone .ko is only for the kernel named in its filename. The Linux
packages include DKMS source for rebuilding the module on supported kernels.
For Wine, install the Linux package first, then run the Windows installer as
your desktop user. Helper startup/authentication and BDA registration are automatic.
Native Windows playback and third-party Windows TV playback under Wine remain
unverified. See BUILD-AND-TEST-REPORT.md for exact validation and limitations.

SHA256SUMS verifies the enclosed release artifacts. The source archive also
contains a per-file source manifest. Older releases and broadcast recordings
are excluded from this bundle.
''')
(DIST / (bundle.name + '.sha256')).write_text(hashlib.sha256(bundle.read_bytes()).hexdigest() + '  ' + bundle.name + '\n')
print('Complete release ZIP:', bundle.name)
print('Packaged:', ', '.join(path.name for path in outputs))
shutil.rmtree(WORK)

#!/usr/bin/env python3
"""Stage MinGW binaries and runtime imports for the shared Inno Setup script."""
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys

root = Path(__file__).resolve().parents[2]
stage = Path(sys.argv[1]).resolve()
stage.mkdir(parents=True, exist_ok=True)

def copy(source, destination):
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination)

target_dir = Path(os.environ.get('CARGO_TARGET_DIR', root / 'target')).resolve()
linux_target = Path(os.environ.get('OPEN_VOLAR_S_LINUX_TARGET', root / 'target')).resolve()
release = target_dir / 'x86_64-pc-windows-gnu/release'
for name, folder in [('live-tv.exe', 'player'), ('a865r-debug.exe', 'debug'), ('a865rctl.exe', 'bin')]:
    copy(release / name, stage / folder / name)
hashes = {}
for arch, target in [('x64', 'x86_64'), ('x86', 'i686')]:
    source = target_dir / f'{target}-pc-windows-gnu/release/a865r_bda.dll'
    copy(source, stage / f'adapter-update/{arch}/a865r_bda.dll')
    hashes[arch] = hashlib.sha256(source.read_bytes()).hexdigest().upper()
(stage / 'adapter-update/SHA256.json').write_text(json.dumps(hashes, indent=2) + '\n')
# Inspect each architecture's imports and copy the transitive GNU runtime closure.
for folder, prefix in [('player', 'x86_64'), ('debug', 'x86_64'), ('bin', 'x86_64'),
                       ('adapter-update/x64', 'x86_64'), ('adapter-update/x86', 'i686')]:
    pending = list((stage / folder).iterdir())
    seen = set()
    while pending:
        binary = pending.pop()
        imports = subprocess.check_output([prefix + '-w64-mingw32-objdump', '-p', str(binary)], text=True)
        for name in re.findall(r'DLL Name:\s+(\S+)', imports):
            if not name.lower().startswith(('libstdc++', 'libgcc_', 'libwinpthread')) or name in seen:
                continue
            seen.add(name)
            runtime = Path(subprocess.check_output([prefix + '-w64-mingw32-g++', '-print-file-name=' + name], text=True).strip())
            if not runtime.is_file():
                raise RuntimeError('Missing runtime: ' + name)
            destination = stage / folder / name
            copy(runtime, destination)
            pending.append(destination)
copy('/usr/share/doc/gcc-mingw-w64-x86-64-posix-runtime/copyright', stage / 'licenses/MinGW-GCC-copyright.txt')
copy('/usr/share/common-licenses/GPL-3', stage / 'licenses/GPL-3.txt')
print(stage)

copy(linux_target / "release/a865rctl", stage / "wine/a865rctl")

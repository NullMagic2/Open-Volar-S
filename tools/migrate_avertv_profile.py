"""Reuse the owner's A865R channel settings after the WinUSB display-name change.

Dry-run by default. Never modifies encrypted channel files or the USB binding.
"""
import argparse
import configparser
import csv
import hashlib
import io
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time

SOURCE = 'AVerMedia A865R USB Pure ISDB-Tb'
TARGET = 'AVerMedia A865R - liba865r WinUSB Whole-Device Development Binding'
KEYS = ('COUNTRY', 'REGION', 'PREVIEW_14_1_IDX', 'PREVIEW_CH_14_1_VIDEO',
        'PREVIEW_CH_LAST_14_1_VIDEO', 'PREVIEW_DIGITAL_SOURCETYPE',
        'MY_FAVORITE_CHANNEL_14_1_55_IDX')


def prepare(data):
    encoding = 'utf-16' if data.startswith((b'\xff\xfe', b'\xfe\xff')) else 'utf-8-sig'
    text = data.decode(encoding)
    cfg = configparser.ConfigParser(interpolation=None, strict=True)
    cfg.optionxform = str
    cfg.read_string(text)
    if cfg['ACTIVE']['ACTIVE_DRIVER_NAME'] != TARGET:
        raise ValueError('Active device is not the supported open A865R binding')
    if 'usb#vid_07ca&pid_b865#' not in cfg['ACTIVE']['ACTIVE_DEVICE_NAME'].lower():
        raise ValueError('Active USB identity is not A865R')
    changes = {k: cfg[SOURCE][k] for k in KEYS if k in cfg[SOURCE] and k not in cfg[TARGET]}
    for key, value in changes.items():
        if key.endswith('_IDX') and not Path(value).is_file():
            raise ValueError(f'Referenced channel list is missing: {value}')
    # Preserve unrelated settings and line endings exactly.
    newline = '\r\n' if '\r\n' in text else '\n'
    marker = f'[{TARGET}]'
    addition = ''.join(newline + key + '=' + value for key, value in changes.items())
    updated = text.replace(marker, marker + addition, 1).encode(encoding)
    return updated, changes


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('ini', type=Path)
    parser.add_argument('--apply', action='store_true')
    args = parser.parse_args()
    path = args.ini.resolve(strict=True)
    original = path.read_bytes()
    updated, changes = prepare(original)
    report = {'file': str(path), 'changes': changes, 'applied': False,
              'before_sha256': hashlib.sha256(original).hexdigest()}
    if args.apply and changes:
        tasklist = subprocess.run(['tasklist', '/FI', 'IMAGENAME eq AVerTV.exe', '/FO', 'CSV', '/NH'],
                                  check=True, capture_output=True, text=True)
        if any(row and row[0].lower() == 'avertv.exe' for row in csv.reader(io.StringIO(tasklist.stdout))):
            raise RuntimeError('Close AVerTV before applying its settings')
        backup = path.with_name(path.name + f'.a865r-backup-{time.time_ns()}')
        with backup.open('xb') as stream:
            stream.write(original)
        if path.read_bytes() != original:
            raise RuntimeError('Settings changed during migration; retry after closing AVerTV')
        with tempfile.NamedTemporaryFile(dir=path.parent, delete=False, suffix='.a865r.tmp') as stream:
            temporary = Path(stream.name)
            stream.write(updated)
        try:
            os.replace(temporary, path)
        finally:
            temporary.unlink(missing_ok=True)
        report.update(applied=True, backup=str(backup), after_sha256=hashlib.sha256(updated).hexdigest())
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()

"""Exercise package configuration without touching host devices or DKMS."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

POSTINST = Path(__file__).resolve().parents[1] / 'postinst.sh'


class DevicePermissions(unittest.TestCase):
    def test_reload_before_driver_setup_and_reapply_existing_devices(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            log = root / 'calls'
            for name in ('udevadm', 'dkms', 'modprobe', 'systemctl'):
                path = root / name
                path.write_text(f'#!/bin/sh\nprintf "%s\\n" "{name} $*" >> "$CALL_LOG"\n')
                path.chmod(0o755)
            uname = root / 'uname'
            uname.write_text('#!/bin/sh\necho open-volar-s-test-no-kernel-headers\n')
            uname.chmod(0o755)
            env = dict(os.environ, PATH=f'{root}:/usr/bin:/bin', CALL_LOG=str(log))
            subprocess.run(['/bin/sh', str(POSTINST)], env=env, check=True,
                           stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            calls = log.read_text().splitlines()
            self.assertEqual(calls[1], 'udevadm control --reload-rules')
            self.assertIn('dkms add -m open-volar-s -v 0.9.5', calls)
            self.assertEqual(calls[-4:], [
                "udevadm trigger --action=add --subsystem-match=usbmisc --sysname-match=open-volar-s[0-9]*",
                "udevadm trigger --action=add --subsystem-match=misc --sysname-match=open-volar-dvb[0-9]*",
                "udevadm trigger --action=add --subsystem-match=dvb",
                'udevadm settle --timeout=10',
            ])
            self.assertFalse(any(line.startswith('modprobe') for line in calls))


if __name__ == '__main__':
    unittest.main()

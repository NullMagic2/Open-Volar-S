"""Exercise automatic post-build setup without compiling or elevating."""
from pathlib import Path
import os
import subprocess
import tempfile
import unittest

BUILD = Path(__file__).resolve().parents[2] / 'build.sh'

class BuildWorkflow(unittest.TestCase):
    def test_permissions_follow_successful_build_and_can_be_disabled(self):
        for skip, fails in [(False, False), (True, False), (False, True)]:
            with self.subTest(skip=skip, fails=fails), tempfile.TemporaryDirectory() as folder:
                root = Path(folder)
                (root/'linux').mkdir()
                (root/'build.sh').write_bytes(BUILD.read_bytes())
                (root/'linux/install_permissions.sh').write_text('echo permissions >> "$CALL_LOG"\n')
                (root/'cargo').write_text('#!/bin/sh\necho cargo >> "$CALL_LOG"\nexit '+('1' if fails else '0')+'\n')
                (root/'cargo').chmod(0o755)
                log = root/'calls'
                args = ['/bin/bash',str(root/'build.sh'),'--userspace-only']
                if skip: args.append('--no-permissions')
                result = subprocess.run(args, env=dict(os.environ, PATH=f'{root}:/usr/bin:/bin', CALL_LOG=str(log)))
                self.assertEqual(result.returncode, 1 if fails else 0)
                self.assertEqual(log.read_text().splitlines(), ['cargo'] if skip or fails else ['cargo','permissions'])

if __name__ == '__main__': unittest.main()

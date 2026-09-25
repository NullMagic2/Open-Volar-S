"""Wine build/test menu writes must stay inside the disposable test prefix."""
from pathlib import Path
import json
import os
import subprocess
import tempfile
import unittest

HELPER = Path(__file__).resolve().parents[2] / 'windows/installer/wine_build_environment.sh'

class WineBuildEnvironment(unittest.TestCase):
    def test_desktop_data_is_isolated_without_changing_home(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            desktop = root / 'user-data'
            desktop.mkdir()
            prefix = root / 'prefix with spaces'
            code = '''set -eu
source "$1"
python3 - <<'PY'
from pathlib import Path
import json, os
Path(os.environ['XDG_DATA_HOME'], 'applications', 'test.desktop').write_text('test')
print(json.dumps({k:os.environ[k] for k in ['HOME','XDG_DATA_HOME','XDG_CONFIG_HOME','XDG_CACHE_HOME','WINEDLLOVERRIDES']}))
PY
'''
            env = dict(os.environ, WINEPREFIX=str(prefix), XDG_DATA_HOME=str(desktop),
                       XDG_CONFIG_HOME=str(root/'user-config'), XDG_CACHE_HOME=str(root/'user-cache'),
                       WINEDLLOVERRIDES='example=n')
            result = json.loads(subprocess.check_output(['bash','-c',code,'bash',str(HELPER)],env=env,text=True))
            self.assertEqual(result['HOME'],os.environ['HOME'])
            for key in ['XDG_DATA_HOME','XDG_CONFIG_HOME','XDG_CACHE_HOME']:
                self.assertTrue(Path(result[key]).is_relative_to(prefix))
            self.assertEqual(result['WINEDLLOVERRIDES'],'example=n;winemenubuilder.exe=d')
            self.assertEqual(list(desktop.iterdir()),[])
            self.assertTrue((prefix/'.host-integration/data/applications/test.desktop').is_file())

if __name__ == '__main__': unittest.main()

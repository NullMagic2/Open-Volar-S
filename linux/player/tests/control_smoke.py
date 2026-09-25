#!/usr/bin/env python3
"""Native playback control smoke test; requires desktop audio, Vulkan Video, TS.
Usage: control_smoke.py /absolute/path/to/open-volar-s-player capture.ts
The test is muted and never opens the tuner.
"""
import json
import os
from pathlib import Path
import socket
import subprocess
import sys
import tempfile
import time

player, fixture = map(lambda x: str(Path(x).resolve()), sys.argv[1:3])
with tempfile.TemporaryDirectory(prefix="ovs-native-controls-") as folder:
    ipc = str(Path(folder) / "control.sock")
    with open(Path(folder) / "player.log", "w+") as log:
        args = [player, fixture, "--volume=0", "--ipc=" + ipc]
        if os.environ.get("A865R_TEST_PROGRAM"):
            args.append("--program=" + os.environ["A865R_TEST_PROGRAM"])
        child = subprocess.Popen(args, stdout=log, stderr=subprocess.STDOUT)
        def command(*args):
            with socket.socket(socket.AF_UNIX) as client:
                client.settimeout(1)
                client.connect(ipc)
                client.sendall((json.dumps({"command": args, "request_id": 1}) + "\n").encode())
                response = json.loads(client.makefile().readline())
                assert response["error"] == "success", response
                return response.get("data")
        try:
            deadline = time.monotonic() + 6
            while not os.path.exists(ipc):
                assert child.poll() is None, "player exited before opening its control socket"
                assert time.monotonic() < deadline, "control socket startup timed out"
                time.sleep(.03)
            time.sleep(1.5)
            command("set_property", "pause", True)
            time.sleep(.12)
            start = command("get_property", "time-pos")
            time.sleep(.5)
            assert abs(command("get_property", "time-pos") - start) < .02, "pause clock moved"
            command("set_property", "pause", False)
            time.sleep(.5)
            assert command("get_property", "time-pos") > start + .3, "resume did not advance"
            for mode in range(5):
                command("set_property", "audio-mode", mode)
            for mode in [1, 0, 2]:
                command("set_property", "deinterlace-mode", mode)
                time.sleep(.2)
            command("set_property", "output-size", 1)
            assert child.wait(timeout=25) == 0, "native player failed"
            print("PASS: native pause/resume, sound modes, deinterlacing changes and EOF")
        finally:
            if child.poll() is None:
                child.terminate()
                child.wait(timeout=5)
            log.seek(0)
            print(log.read())

#!/usr/bin/env python3
"""Exercise native seek/restart using a >=10-second TS fixture; muted, no tuner."""
import json
from pathlib import Path
import socket
import subprocess
import sys
import tempfile
import time

with tempfile.TemporaryDirectory(prefix="ovs-native-seek-") as folder:
    ipc = str(Path(folder) / "ipc.sock")
    with open(Path(folder) / "player.log", "w+") as log:
        child = subprocess.Popen([str(Path(sys.argv[1]).resolve()), "--play", str(Path(sys.argv[2]).resolve()), "--mute", "--ipc=" + ipc], stdout=log, stderr=subprocess.STDOUT)
        def cmd(*args):
            with socket.socket(socket.AF_UNIX) as stream:
                stream.settimeout(1)
                stream.connect(ipc)
                stream.sendall((json.dumps({"command": args}) + "\n").encode())
                reply = json.loads(stream.makefile().readline())
                assert reply["error"] == "success", reply
                return reply.get("data")
        def wait_position(target):
            deadline = time.monotonic() + 5
            while time.monotonic() < deadline:
                assert child.poll() is None, "player exited during seek"
                value = cmd("get_property", "time-pos")
                video = cmd("get_property", "native-video-pos")
                if video is not None and target - .1 <= video <= target + 1 and target - .1 <= value <= target + 1:
                    return value
                time.sleep(.04)
            raise AssertionError(("seek failed", target, value))
        try:
            deadline = time.monotonic() + 6
            while not Path(ipc).exists():
                assert child.poll() is None and time.monotonic() < deadline
                time.sleep(.03)
            time.sleep(1.2)
            assert cmd("get_property", "duration") >= 10
            cmd("seek", 5, "absolute")
            print("Forward:", wait_position(5))
            cmd("set_property", "pause", True)
            cmd("seek", 1, "absolute")
            print("Paused backward:", wait_position(1))
            time.sleep(.2)
            position = cmd("get_property", "time-pos")
            time.sleep(.3)
            assert abs(cmd("get_property", "time-pos") - position) < .02
            cmd("set_property", "pause", False)
            time.sleep(.5)
            assert cmd("get_property", "time-pos") > 1.2
            before = cmd("get_property", "track-list")
            cmd("cycle", "audio")
            time.sleep(.5)
            after = cmd("get_property", "track-list")
            if sum(t["type"] == "audio" for t in before) > 1:
                assert before != after, "audio track did not change"
            assert child.wait(timeout=20) == 0
            print("PASS: native seek, paused seek, resume and track switch")
        finally:
            if child.poll() is None:
                child.terminate()
                child.wait(timeout=5)
            log.seek(0)
            print(log.read())

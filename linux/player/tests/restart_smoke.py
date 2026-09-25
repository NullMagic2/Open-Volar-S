#!/usr/bin/env python3
"""Replay a broadcast after forced stops, reusing the GUI's control socket.

Requires desktop Vulkan Video/audio and a >=10-second TS fixture; muted, no tuner.
Usage: restart_smoke.py /path/to/open-volar-s-player capture.ts [program_id]
"""
import json
from pathlib import Path
import socket
import subprocess
import sys
import tempfile
import time

player, fixture = [str(Path(p).resolve()) for p in sys.argv[1:3]]
with tempfile.TemporaryDirectory(prefix="ovs-native-restart-") as folder:
    ipc = str(Path(folder) / "control.sock")
    args = [player, fixture, "--volume=0", "--ipc=" + ipc]
    if len(sys.argv) > 3:
        args.append("--program=" + sys.argv[3])

    def command(*args):
        with socket.socket(socket.AF_UNIX) as client:
            client.settimeout(1)
            client.connect(ipc)
            client.sendall((json.dumps({"command": args}) + "\n").encode())
            reply = json.loads(client.makefile().readline())
            assert reply["error"] == "success", reply
            return reply.get("data")

    for attempt in range(4):
        with open(Path(folder) / f"player-{attempt}.log", "w+") as log:
            child = subprocess.Popen(args, stdout=log, stderr=subprocess.STDOUT)
            try:
                deadline = time.monotonic() + 10
                while True:
                    assert child.poll() is None, "replacement player exited"
                    assert time.monotonic() < deadline, "playback did not start automatically"
                    try:
                        position = command("get_property", "native-video-pos")
                        if position is not None and position > .25:
                            break
                    except OSError:
                        pass  # An old socket can remain while the new process starts.
                    time.sleep(.04)
                print(f"Session {attempt + 1}: video advanced to {position:.3f}s")
                if attempt < 3:
                    # Matches native stop_player: kill, wait, then spawn replacement.
                    child.kill()
                    child.wait(timeout=5)
                    assert Path(ipc).exists(), "expected orphaned socket after forced stop"
                else:
                    assert child.wait(timeout=25) == 0, "final playback failed"
                    assert not Path(ipc).exists(), "normal exit failed to clean up socket"
            finally:
                if child.poll() is None:
                    child.kill()
                    child.wait(timeout=5)
                log.seek(0)
                print(log.read())
    print("PASS: three forced channel-style restarts, automatic playback and clean EOF")

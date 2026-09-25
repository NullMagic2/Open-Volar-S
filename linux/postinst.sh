#!/bin/sh
set -e
if command -v systemctl >/dev/null 2>&1; then
    systemctl daemon-reload || true
fi
# Load the packaged rule before modprobe can create a device node.
if command -v udevadm >/dev/null 2>&1; then
    udevadm control --reload-rules || true
fi
if command -v dkms >/dev/null 2>&1; then
    dkms add -m open-volar-s -v 0.9.5 2>/dev/null || true
    if [ -d "/lib/modules/$(uname -r)/build" ]; then
        dkms build -m open-volar-s -v 0.9.5
        dkms install -m open-volar-s -v 0.9.5
        # Upgrade an idle loaded module as well. Never force-unload an in-use tuner.
        if [ -d /sys/module/open_volar_s_usb ]; then
            if command -v systemctl >/dev/null 2>&1; then
                systemctl stop 'open-volar-s-dvb@*.service' || true
            fi
            if ! modprobe -r open_volar_s_usb; then
                echo 'Open Volar S: tuner is in use; close TV applications and reboot to activate the updated module.' >&2
            fi
        fi
        modprobe open_volar_s_usb || true
    else
        echo 'Open Volar S: matching kernel headers are unavailable; DKMS source was installed.' >&2
    fi
fi
if command -v udevadm >/dev/null 2>&1; then
    # Also repair nodes created before installation (including an already loaded driver).
    udevadm trigger --action=add --subsystem-match=usbmisc --sysname-match='open-volar-s[0-9]*' || true
    udevadm trigger --action=add --subsystem-match=misc --sysname-match='open-volar-dvb[0-9]*' || true
    udevadm trigger --action=add --subsystem-match=dvb || true
    udevadm settle --timeout=10 || true
fi

# udev does not restart a manually stopped service for an already active device.
if command -v systemctl >/dev/null 2>&1; then
    for node in /dev/open-volar-dvb[0-9]*; do
        [ -c "$node" ] || continue
        systemctl start "open-volar-s-dvb@${node##*/}.service" || true
    done
fi

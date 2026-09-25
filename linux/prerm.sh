#!/bin/sh
if command -v systemctl >/dev/null 2>&1; then
    systemctl stop 'open-volar-s-dvb@*.service' || true
fi
if command -v dkms >/dev/null 2>&1; then
    dkms remove -m open-volar-s -v 0.9.5 --all 2>/dev/null || true
fi
exit 0
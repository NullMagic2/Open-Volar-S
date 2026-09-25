#!/usr/bin/env bash
# Source inside a subshell before any Wine build/test command, including Inno setup.
# WINEPREFIX alone does not isolate Wine's Linux desktop-menu integration.
: "${WINEPREFIX:?Choose a dedicated Wine build/test prefix first}"
export WINEPREFIX
export WINEDLLOVERRIDES="${WINEDLLOVERRIDES:-mscoree,mshtml=};winemenubuilder.exe=d"
export XDG_DATA_HOME="$WINEPREFIX/.host-integration/data"
export XDG_CONFIG_HOME="$WINEPREFIX/.host-integration/config"
export XDG_CACHE_HOME="$WINEPREFIX/.host-integration/cache"
mkdir -p "$XDG_DATA_HOME/applications" "$XDG_CONFIG_HOME" "$XDG_CACHE_HOME"

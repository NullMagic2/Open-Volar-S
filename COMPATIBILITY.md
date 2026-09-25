# VLC compatibility

These instructions apply to native Linux with the updated Open Volar S 0.9.5
driver. Tested with VLC 3.0.23 on Ubuntu using the AVerTV Volar S A865R.

## Open the tuner in VLC's GUI

1. Install the updated Linux package. If the previous driver was in use during
   installation, close TV applications and reboot to activate the update.
2. Close Live TV! and any other tuner application. Run VLC as your normal desktop
   user; only one application can control the physical tuner at a time.
3. Open **Media → Open Capture Device** (**Mídia → Abrir Dispositivo de Captura**,
   **Ctrl+C**). Do not use **Stream / Transmitir (Ctrl+S)** for local viewing.
4. Set **Capture mode / Modo de captura** to **TV - digital**.
5. Set **Tuner card / Placa sintonizadora** to `/dev/dvb/adapter0`. Use the actual
   adapter number if more than one tuner is installed.
6. Select **DVB-T** under **Delivery system / Sistema de entrega**.
7. Enter the channel's frequency in **kHz** and select **6 MHz** bandwidth.
   The tested TV GAZETA HD settings are **521143 kHz**, **6 MHz**, program **16960**.
   Frequencies and program numbers depend on your location.
8. Click **Play / Reproduzir**. If the button says **Stream / Transmissão**,
   cancel the dialog and reopen it with **Ctrl+C**. Local viewing does not need
   a streaming destination or conversion profile. Use **Playback → Program**
   to choose a service when the frequency carries several channels.

The equivalent command is:

```sh
vlc --sout= 'dvb-t://frequency=521143000:bandwidth=6' --dvb-adapter=0 --program=16960
```

The GUI frequency is in **kHz**; the URL frequency is in **Hz**. Native ISDB-T
access also works: replace `dvb-t://` with `isdb-t://`. To enter either URL through
the GUI, use **Media → Open Network Stream**, on the **Network / Rede** tab.

## What compatibility mode does

VLC 3's capture dialog omits ISDB-T. The driver therefore advertises DVB-T as an
alias and translates its tuning requests into the existing ISDB-T tuning path.
ISDB-T remains the preferred advertised standard. This does **not** add reception
of actual DVB-T broadcasts: the hardware still receives ISDB-T/ISDB-Tb within
the supported 470000–697999 kHz range and 6 MHz bandwidth.

The alias adds only a tuning-time check, with no extra packet processing,
transcoding or buffering. No meaningful performance penalty is expected, but
CPU differences have not been benchmarked.

Compatibility is enabled by default. To disable it, add
`options open_volar_s_usb dvbt_compat=0` to `/etc/modprobe.d/open-volar-s.conf`
and reboot. Set it to `1` to enable it again.

## Known VLC limitations

- `cannot start stream output instance`, `stream chain failed` or `cannot create
  chain` means VLC's streaming/conversion output failed. Cancel the streaming
  wizard and reopen the capture device with **Ctrl+C**, or use the command above;
  `--sout=` clears streaming output for that launch. These errors can abort
  playback before VLC even attempts to tune the device.
- If Brazilian programme titles appear as Japanese characters, enable
  **Show more options / Exibir mais opções** and append ` :ts-standard=dvb` to
  **Edit Options / Editar Opções**, preserving existing options. On the command
  line, use `--ts-standard=dvb`. This changes metadata parsing, not tuning, and
  may affect ISDB-specific captions or EPG handling.
- VLC may log `PSI section too long`, timestamp or decoder warnings even while
  video plays. Warnings occurred on both native ISDB-T and compatibility paths;
  the alias does not repair broadcast tables or VLC's demuxing/remuxing behavior.
- VLC does not automatically import Live TV!'s saved channel list. The Linux
  compatibility test received 1920×1080 H.264 video and AAC audio; it does not
  establish full caption/EPG parity with Live TV!.
- These Linux instructions do not establish Windows VLC or Wine BDA playback
  compatibility. See [standard TV application access](docs/TV-COMPATIBILITY.md)
  and [Wine support](docs/WINE.md) for those interfaces and their limitations.

## Native Linux player update (0.9.5)

The refreshed package includes our native Vulkan Video player for Live TV!.
VLC remains an external application and uses the DVB instructions above; it
does not use Live TV!'s deinterlacing or picture controls. The native player's
requirements and remaining Wine/WSL/export limitations are documented in
[docs/NATIVE-LINUX-PLAYER.md](docs/NATIVE-LINUX-PLAYER.md).

In Live TV!, leave aspect ratio on **Auto** to preserve broadcast proportions.
Native, QHD and UHD limit processing resolution independently of display aspect.
Resizing or fullscreen adds black borders as necessary; an explicit aspect
override intentionally changes the displayed ratio.

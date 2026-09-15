# Infrared remote development, 0.6.0

The tuner can be polled directly through WinUSB, independently of the AVerRemote
service. `Device::infrared()` borrows its USB command connection exclusively;
`Infrared::poll()` returns either a four-byte code or no key. Checksum, sequence
and reply length are validated. Only status 1 means no key. Other device errors,
corruption and transport failures remain errors.

Open Debug Desk manually and expand **Infrared remote through the tuner**.
Stop TV, enter the name of one button, then choose **Listen for 30 seconds**.
Point a handset at the tuner and press that button several times. Stop cancels
the session. `infrared.json` contains the raw bytes, stable code keys, timestamps,
label, firmware/configuration, empty-reply count and any error. The ordinary
diagnostic export also records the session. No keyboard events are injected.

For scripted collection:

```text
a865r-debug.exe --remote-seconds 30 --collect-export <new-export-folder>
```

The 2026-09-10 idle test received 30 valid no-key replies in three seconds from
the physical IT9175, reference LINK firmware 3.0.3.0. EEPROM IR mode/type both
reported 0. This verifies command communication, not reception from a handset.
No handset was available for a button test. Generic mappings, held-button repeat
behavior, concurrent TV/IR ownership and forwarding to AVerTV remain unfinished.
An NEC complement-byte interpretation is exported as a candidate, never as a
verified protocol. Arbitrary NEC, RC5, RC6, Sony or other handset compatibility
cannot be inferred from this idle test. No EEPROM or firmware settings are changed.

The command framing and empty-reply convention were checked against the public
[Linux ITE-family driver](https://github.com/torvalds/linux/blob/master/drivers/media/usb/dvb-usb-v2/af9035.c).
Raw four-byte firmware replies do not expose pulse timing needed for a universal
software decoder. The open firmware does not yet implement an IR decoder.

## AVerRemote startup warning

The installed helper crashed in GraphMaster.dll 2.3.0.11 at offset 0x1908a, with
exception 0xc0000005. Its caller dereferences a COM pointer without checking it.
The development BDA interfaces were visible machine-wide but their COM classes
were registered only for the interactive user. Registering both x86 and x64
adapters from the protected Program Files installation made the service start
and remain running, including after AVerTV was reopened. WinUSB remained bound.

This fixes the observed service failure. AVerTV's TV graph still fails to connect
the new adapter, and service startup does not prove remote button delivery.
The experimental BDA installation is separate from the Debug Desk installer;
see BDA_COMPATIBILITY.md for its current scope and registration state.

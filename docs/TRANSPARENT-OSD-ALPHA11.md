# AVerTV transparent OSD inspection

## Alpha 11: transparent green OSD

The channel and volume OSD now use transparent layered child windows instead of bronze panels, with 75% foreground opacity. Text uses the original AVerTV defaults recovered from the installed executable: RGB #00FF00 foreground, #646464 outline, Microsoft Sans Serif, weight 700, non-italic. The volume indicator uses green segments with transparent gaps; it has no opaque trough. Existing three-second expiry, fullscreen positioning and first-picture redisplay remain. The fullscreen exit button hides after two seconds without pointer movement and reappears on movement; F11 and Escape remain available.

Static evidence from AVerTV.exe (image base 0x400000): the OSD_TEXT configuration call at 0x48DC6C is preceded by `push 0x00FF00`; OSD_EDGE at 0x48DC8F uses `push 0x646464`; font weight at 0x48DD66 uses `push 700`. Font-size setting defaults to 10 in AVerTV's own sizing units; this implementation scales its OSD font with Windows DPI. The original application was not patched and no proprietary graphics are bundled.

Windows color-key composition uses WS_EX_LAYERED, LWA_COLORKEY and LWA_ALPHA. A Windows 8/10 compatibility manifest is embedded because layered child windows require that declaration. Text uses crisp GDI glyphs to prevent colored antialias fringes around transparent pixels. Only UI surfaces are composed; Vulkan decoded frames are not read back or copied.

API reference: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setlayeredwindowattributes


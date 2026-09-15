# Alpha.38 investigation — interim package, Vulkan repair incomplete

## Vulkan status
The custom Vulkan broadcast decoder still fails before its first decoded frame. The final Vulkan repair is incomplete; the user requested an interim source/installer package with Vulkan retained as default.

Working-tree changes propagate the actual parsed IDR picture identifier (the captured stream uses ID 2, while the bridge previously supplied 0), zero aligned bitstream-buffer padding, and wait for IDR startup for MBAFF streams. These are corrections/hardening, not an established explanation of the device loss. Opt-in Vulkan validation and a decode-completion diagnostic have been added. No video API validation error explained the loss. The known Naga shader-decoration warning retains its existing suppression.

Microsoft H.264 decoding with the same Vulkan presentation pipeline completed a 120-second live comparison and stopped cleanly (3,474 received frames, 6,953 presentations, zero reported dropped frames). That path remains available only through explicit diagnostic flags. Vulkan is the default; the user explicitly rejected substituting Microsoft decoding in the package. Existing graphics-failure recovery remains unchanged.

## Preserved-binary comparison
The preserved alpha.36 executable has SHA-256 `950F9DA458CA34C477FECB5D1AE9990CB0AA7E86C765354EF958F858CC04492D`, matching its original hardware report. Earlier alpha.35/36 hardware logs show successful Vulkan HD playback and seeking (over 600 displayed hardware-decoded frames in the HD run).

On September 13, the current build AND this unchanged alpha.36 binary now fail on the exact earlier HD fixture (`hel/work/alpha35-hardware/hd.ts`) with the earlier isolated settings. Both report device loss before the first decoded picture. This establishes that reverting recent source changes alone does not restore playback in the current machine state. It suggests a driver/device-state problem, but does not identify what originally triggered it or exclude an application defect that can trigger a driver hang.

Windows Application/WER records at 04:46:38–39 local time contain LiveKernelEvent 141 and AMD watchdog reports, with WATCHDOG dumps timestamped 04:45 and 04:46. Further GPU test launches were stopped. A Windows restart has been requested before repeating the controlled comparison. No driver reset, driver installation, firmware modification or installed-app replacement was performed.

## Caption case preservation
The requirement is exact character-case preservation for all text, not word-specific correction and not sentence casing. A native decoder regression checks 121 Latin letters in mixed-case context, comparing both decoded UTF-8 text and each glyph codepoint consumed by the renderer. It passes, including all ASCII letters, Latin extension letters, cedillas and accent case pairs. The native DirectWrite path forwards those codepoints without case conversion.

The captured examples `sÓ`, `tÁ` and `pÉ` already contain uppercase broadcast character bytes D3, C1 and C9. The same packet correctly encodes lowercase `ê` as EA. No speculative mapping/case-conversion change was made. This captured evidence does not independently identify the bytes from the user's specific cedilla occurrence.

Regression source: `player/tests/caption_case_regression.cpp`. Result: 121 Latin letters preserve case in decoded text and renderer glyph records; mixed-case phrase also passes. Further investigation of a specific visibly incorrect case should compare that caption's broadcast bytes, decoded codepoints and rendered output.

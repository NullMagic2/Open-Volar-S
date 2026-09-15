# Alpha.38 — interim Vulkan package

Packaged at the user's request while the Vulkan repair remains open. Vulkan Video decoding remains the normal default for live TV, recordings and files; explicit software diagnostic flags still take precedence. Existing graphics-failure recovery is retained. This package is not a claim that Vulkan device loss has been fixed.

## Changes
- Preserve parsed IDR picture identifiers instead of always submitting zero.
- Clear the entire aligned bitstream-buffer padding, including on buffer reuse.
- Require IDR startup for MBAFF broadcast decoding while retaining independent PAFF open-GOP startup.
- Add opt-in Vulkan validation and decode-completion diagnostics.
- Add exhaustive Latin caption case regression coverage, including per-character codepoints consumed by the renderer. No automatic case conversion or word-specific replacements were added.
- Preserve the unified alpha.37 features.

## Validation completed
- Workspace release regression suite: 170 passed, zero failed, two existing GPU fixture tests ignored. After restoring the requested Vulkan default, the decoder-selection regression was rerun and passed.
- Native broadcast-parser tests: three passed.
- Exact bitstream-padding helper and its source regression, compiled independently without creating a GPU device: one passed.
- Native caption test: 121 Latin letters preserve case in mixed-case context in both decoded UTF-8 and renderer glyph records; the mixed-case phrase fixture also passes.
- CPU broadcast parsing verifies 583 pictures with no missing references after independent startup and the actual IDR identifiers.

## Hardware evidence and remaining limit
The Microsoft-decoder comparison with Vulkan rendering ran for 120 seconds and stopped cleanly. That comparison does not validate Vulkan decoding and Microsoft decoding is not the packaged default.

Current Vulkan decoding still fails before its first output frame. Crucially, the preserved alpha.36 binary, verified against the exact SHA-256 in its original hardware report, now also fails on its previously passing HD fixture with the earlier settings. Windows recorded GPU watchdog/LiveKernelEvent 141 reports during the investigation. Further GPU test launches were stopped; a Windows restart was requested before a fresh old/new comparison. See ALPHA38-INVESTIGATION.md for evidence and uncertainty. The trigger of the original failure has not been established.

The installer is built from the release workspace and includes both existing BDA adapter architectures. Source packaging verifies ZIP CRC, every source-manifest hash, installer-copy identity and the staged player executable identity. No installation, firmware change or driver reset is performed by packaging. Hardware validation after reboot remains outstanding.

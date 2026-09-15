# Live TV! 0.8.0-alpha.29

Removed Live from the DAC receiver transport row. REC and Fullscreen now share the released space, with a consistent 12-DIP gap. The player Live control and DAC broadcast-status indicator remain.

Validation: 71 existing player tests passed; two hardware GPU tests remain intentionally ignored. Native preview checks passed at widths 1100, 1500 and 1679: receiver Live is hidden, player Live remains visible, and REC/Fullscreen have aligned bounds and no overlap. An initial preview ran the previous executable while compilation was still underway; the checks above were rerun successfully against the completed alpha.29 executable.

HDR rendering is unchanged. A custom HDR swap chain cannot bypass Windows' SDR display output: Windows clips HDR-range values in SDR mode. An SDR-compatible image enhancement is a separate option awaiting the user's choice. This release does not change Windows display settings.

Driver and Debug Desk remain 0.7.0. See ALPHA28-VALIDATION.md for the previous release's feature coverage and hardware limits.

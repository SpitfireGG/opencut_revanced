# Screenshot analysis — VN vs Concat (corrected 2026-09-18)

Pipeline: OpenRouter free VL vision (`inclusionai/ling-3.0-flash-vl:free`, one image per call,
serial driver, resumable). Ground truth from `/home/archbishop/Dev/Concat/screenshots/{VN,Concut_latest}`,
21 VN + 7 Concat images, read in capture-flow order.

> **Honest correction:** my first pass over-read the reference and claimed three gaps. Re-examining
> our own `.slint` after checking, two of the three are **already at parity** (see below). Only the
> export panel's visual detail and the text editor's parity work remain. Verified below in code.

## Already-parity (no work needed)
1. **Crop ratio tabs** — `phone-editor.slint:1204-1208` ships the identical set VN shows:
   `Free / 1:1 / 2:3 / 3:2 / 3:4`, yellow-highlight active tab, same order. ✅
2. **Export size options** — our export.slint grid matches VN's export screen: 4 sizes with
   4K/QHD/1080p/720p details + 30/60 fps + H.264/HEVC/AV1 codec labels. ✅
3. **Playback transport** — same control set as VN (skip-start/play/skip-end + undo/redo + dupe). ✅

## Real remaining gaps (below are what I will fix, in order)
1. **Text editor parity with VN's text overlay.** VN (reference #24) shows a text-overlay editor
   with: on-video preview text, a solid-text bounding box that is **moveable and resizable in
   preview**, per-text stroke+fill, font family/weight/size/colour, tracking, line-height, and a
   "max width" wrap via the stage grip. Our text panel covers font/size/colour/weight/italic/side
   but the **on-preview draggable bounding box / in-preview size + position grips** are set on the
   timeline via lanes and not yet equivalent to VN's stage grips.
2. **Export panel visual detail** — align the export sheet's option rows (detail text style,
   selected-state highlight, size summary) with VN's appearance rather than just the option set.

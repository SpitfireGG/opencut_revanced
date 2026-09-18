# Concat — handoff prompt for the next AI agent

You are continuing work on **Concat**, a Rust + Slint video editor that is
being turned into an **Android-only, VN-style editor for Instagram Reels
clips**. Read this whole file before doing anything.

## Repo and environment
- Local: `/home/archbishop/Dev/Concat` (NixOS desktop, fish shell). GitHub: `SpitfireGG/opencut_revanced`.
- Cargo workspace in `src/` (Rust 2024), Slint 1.17 UI, Skia/wgpu renderer. Android through android-activity + cargo-apk. FFmpeg 8.1 is built statically (MediaCodec, with the JNI VM set via `av_jni_set_java_vm`). whisper handles captions.
- CI (GitHub Actions): `.github/workflows/android.yml` builds the APK (artifact `Concat-android-arm64`), `ci.yml` runs checks, and `release.yml` runs on a `v*` tag to publish a release with the APK. The signing key lives in repo secrets, so every build updates the previous one in place.
- Test phone: Xiaomi 2201116SG (Redmi Note 11 Pro+ 5G), Android 13, 1080x2400. Package name: `app.concat.editor` (a release build, not debuggable, so `run-as` does not work).
- `scripts/phone.sh [--logs]` waits for HEAD's Android CI build, downloads it, runs `adb install -r` and launches the app. Downloading can be slow; you can also run `gh run download <run> -n Concat-android-arm64 -D dir` and then `adb install -r`.
- adb testing: `adb exec-out screencap -p > shot.png`, then view it. Screenshots are shown at 900x2000, so **multiply displayed coordinates by 1.2** before passing them to `adb shell input tap`. adb cannot simulate a pinch, so the user has to test pinch-zoom.

## User's workflow rules (strict)
1. **Never build locally** (no cargo build/check/test on the desktop). Push to GitHub and let CI build.
2. Release by bumping the version, tagging it and pushing the tag. The release should contain the **APK only**. **Delete the previous release** (`gh release delete vX -y`) but keep its tag.
3. Version bump: `src/Cargo.toml` line 6 `version = "x.y.z"`, plus every `name = "concat*"` entry in `src/Cargo.lock`. Leave third-party crates such as `paste` alone.
4. Work quickly: push often and don't make the user wait on long local work.
5. Commit messages end with `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`.
6. Match the existing code style, which uses plain prose comments.

## Current state (2026-09-18)
- `main` is at commit `a10deb8 "v1.0.16"`, and tag **v1.0.16** is pushed. The v1.0.15 release was deleted. The v1.0.16 release workflow had started, but I never confirmed it finished. **First step:** run `gh run list --workflow release.yml -L 1` and `gh release list`. If it failed, check the logs. If publishing hit a GitHub 5xx (this happened before), upload the APK by hand and publish the draft.
- PR #7 (`fix/on-device-round1`) was squash-merged into main.
- `PHONE.md` (a phone setup guide for the user's friend) is untracked and was never committed. Ask the user whether to commit it.

## Product spec the user gave (must hold)
- The layout matches the design images in `/home/archbishop/inspo` (arrangement only; the colours are our own).
- Timeline:
  - Its length is derived from the clips: the latest clip end across all tracks.
  - Tracks are independent. Nothing clamps them to the main track, and nothing is truncated automatically.
  - Imports are placed whole at the playhead.
  - The playhead is clamped to 0..timeline end, with a firm vibration when it reaches the end.
  - Lanes and the ruler are drawn from 0 to the timeline end (10 s minimum when the timeline is empty). Nothing is drawn past the end.
- Export creates the target directory if it is missing, and names files `CONCAT_<name>`. The default export resolution is 1080x1920, because MediaCodec on this phone cannot encode 4K. There is a fallback to 1080 if the encoder fails to open.
- The Instagram Reels 9:16 frame is the default.
- 9 text style presets: Title, Headline, Subtitle, Lower third, Caption, Elegant, Neon, Outline, Minimal.

## Key code map
- `src/crates/concat/src/studio.rs` holds the app state (`Studio`); `self.compact` means phone mode.
  - Lane roles: ROLE_FOOTAGE=0, WORDS=1, OVERLAY=2, SOUND=3, SPARE=4.
  - Placement and rows: `lane_roles`, `lane_for`, `place_by_role` (applied inside `apply()` when compact), `row_order`, `phone_row_height`.
  - Other functions: `seek` (clamps and sends haptics), `play_toggle`, `export_path`, `crop_to` + `centred_crop`, `delete_selected`, `selected()` → SelectedClipData, `add_title`, `restyle_title`.
  - Clip actions come through `menu-selected` strings such as "mirror", "rotate", "crop:1:1", "fit-width" and "delete".
- `src/crates/concat/src/lib.rs` wires the callbacks. The macros `on_window!` (full publish), `on_lanes!` and `on_moving!` (only the playhead and overlay).
- `src/crates/concat/src/presets.rs` holds the text presets.
- `src/crates/concat-text/src/lib.rs` is the title painter. For left-aligned text, the left edge sits on the clip's centre.
- `src/crates/concat/ui/phone/phone-editor.slint` is the phone editor:
  - Top bar, monitor and transport.
  - Tool rows (`main-tools`, `video-tools`, `text-tools`, and so on); the `run(tool)` function dispatches them.
  - Sheets: styles, crop, ratio ("Frame"), highlights, clip, library, project.
- `src/crates/concat/ui/timeline/lanes.slint` draws the timeline rows:
  - A fixed 44px icon column, the cover chip (deliberately on the overlay row), the per-clip sound row, and the playhead with its knob.
  - Gestures: a swipe always scrolls; a long press lifts a clip to move it.
- `src/crates/concat/ui/phone/phone-home.slint` is the home screen (projects list and the + FAB).
- `src/crates/concat/src/platform.rs` has the Android GPU backend and `haptic_tick` / `haptic_firm`. `src/crates/concat-android` holds the JNI gallery picker and haptics.

## Done in v1.0.16 (all verified by CI; not yet re-checked on the phone)
- The timeline swipe always scrolls; a long press moves a clip. Empty rows pan instead of swallowing the swipe.
- The preview no longer goes black at the end of the timeline (it is clamped to the last frame).
- Layout fixes: transport time overlap, timeline height, playhead knob, and the home-screen FAB position.
- Mirror on a clip rotated ±90° now toggles `flip_v`, so it looks left-to-right on screen (studio.rs, `"mirror"` arm).
- The crop sheet highlights the picked ratio, and its labels have a fixed height.
- The text-style card labels have `height: 18px`, so they no longer overlap the cards.
- The Lower third preset is centre-aligned, so it no longer runs off the right edge.
- The Back tool button is always present and hidden with `visible:` rather than created with `if`. Before, destroying the button during its own tap made Slint drop the next tap.

## Verified working on the phone (v1.0.15)
- Home screen and GPU preview.
- VN-style rows, icons and cover chip.
- Text styles sheet: adding a title, live typing, dragging a title on the preview, restyling while keeping the words.
- Crop: 1:1 is centred; Rotate, Free and undo all work.
- Ratio / Frame sheet: 1:1 makes a square canvas with the content fitted inside; undo works.
- Split at 0 s does nothing (correct).
- A title with empty content draws nothing and does not crash.
- 1080p export: H.264 1080x1920 30 fps + AAC, file `CONCAT_...mp4`.

## Open issues / next steps
1. **Confirm the v1.0.16 release was published** (see Current state).
2. **Delete on a selected text clip sometimes does nothing.** Taps at x≈980 physical on the text tool row failed several times; at x≈945 Delete worked. Possible causes: a dead zone on the right edge, or the stale-tap bug (the sheet's ✓ button is inside an `if root.sheet == ...` block and destroys itself during its own tap, just as Back did). Check the ✓ close button and any other button that removes itself on click; hide it with `visible:` instead of `if`. Then retest Delete on the phone slowly, one tap and one screenshot at a time.
3. **New title placed at the same time as an existing text clip.** It appeared on the same text row and hid the other clip ("Caption" at 0 s was hidden). `lane_for(ROLE_WORDS, ...)` should pick a free lane or a new one, so check whether a second text lane is created and drawn as its own row, or whether the clips overlap on one lane.
4. The first paint of a new title takes about 2 s.
5. Pinch-zoom on the preview monitor has to be tested by the user.
6. The test project on the phone still has leftover test text clips (an empty "Text" clip and a duplicate).
7. Optional plan, not done: make CI APK builds faster by adding `CARGO_PROFILE_RELEASE_LTO: "false"` and `CARGO_PROFILE_RELEASE_CODEGEN_UNITS: "16"` as env on the "Build the APK" step in `.github/workflows/android.yml`. The build step currently takes about 12.5 min.

## How to test a fix
Edit → commit → push main (or a branch/PR) → wait for "Build Android" (`gh run watch <id>`) → `scripts/phone.sh` → drive the app with adb taps and screenshots → tag and release when the user asks.

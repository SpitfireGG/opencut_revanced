# Working on Concat for phones

Paste this whole file into your coding agent (Claude Code or similar) as the first message, or read it yourself before you start.

---

You are helping build **Concat's Android editor**. Concat is a video editor written in Rust with a Slint UI. This fork turns it into a phone-first, Instagram-Reels clipping editor that looks and works like **VN Video Editor**. Read this before touching anything.

## What we are building

- **Android only, for now.** Desktop still has to compile, but nobody ships or tests it. Releases are APK only.
- **The target is VN.** When unsure how something should look or behave, do what VN does ([VN help](https://vlognow.me/help/getting-started/editing-101/)). The owner compares every change against VN screenshots.
- **Instagram first.**
  - New projects are 9:16, 4K, 30 fps.
  - Export is H.264 through the phone's hardware encoder (MediaCodec).
  - The preview can overlay Instagram's Reels safe zones, in fractions of a 1080×1920 frame: top bar 14%, profile and caption the bottom 13%, side margins 6%, and a button rail 227 px in from the right, starting 1150 px down.
- **Basics first.** Get smooth editing, text, overlays, music, subtitles and export working well before anything else. Looks, filters, FX, transitions, animations, templates, cutout and text-to-speech are deliberately **hidden from the phone UI**. Don't bring them back until the owner asks.

## Where things are

| Path | What |
|---|---|
| `src/crates/concat/ui/phone/phone-editor.slint` | The phone editor. Contains the top bar, preview, transport + scrub bar, resize handle, timeline, tool row, quick-action bar ("tooltip"), bottom sheets, and the Reels full-screen preview (`ReelChrome`). |
| `src/crates/concat/ui/phone/phone-home.slint` | The "Your projects" home screen and its **+** button, which makes a new project and opens the gallery. |
| `src/crates/concat/ui/timeline/lanes.slint` | The timeline lanes. Phone mode (`phone: true`) draws VN rows, sticky row icons, the Cover box, the **+** after the footage, a centred playhead with a jog knob, and white trim tabs. |
| `src/crates/concat/ui/workspace/preview-pane.slint` | The monitor: safe zones, overlap marks, pinch-to-scale, and `reel` mode. |
| `src/crates/concat/ui/editor.slint` | The `Editor` global: every property and callback the UI shares with Rust. |
| `src/crates/concat/src/studio.rs` | All editor state and logic (big file). Phone logic is keyed on `self.compact` (the window is narrower than 860 px). |
| `src/crates/concat/src/lib.rs` | Wires Slint callbacks to `Studio`. Actions go through `Editor.shortcut("…")` or `Editor.menu-selected("…")`, which reaches `Studio::clip_action`. |
| `src/crates/concat-project/src/commands.rs` | The edit commands (engine). Every change to a project is a `Command`, applied through `Studio::apply`. |
| `src/crates/concat-android/` | The Android shell. `src/lib.rs` handles the JNI file picker, haptics, and registering the JavaVM with FFmpeg. `java/ConcatFiles.java` is the picker and file copy. |
| `.github/workflows/android.yml`, `release.yml` | CI builds the APK. `release.yml` publishes a release when a `vX.Y.Z` tag is pushed. |
| `scripts/phone.sh` | Installs the current commit's CI build on a phone over adb. |

## Rules the phone timeline follows

- **Fixed rows.** Each lane has a role, top to bottom: music, text, overlay, footage, then the footage's sound row, which has a mute switch.
  - `Studio::lane_roles`, `role_rank`, `lane_for`, `place_by_role` and `place_media` decide where clips go.
  - Clips never move between rows on a phone.
  - Text and overlays always land on a lane *above* the footage in the compositing stack, so they are never hidden behind the video.
- **Row length.** Rows run from the start of the edit to its end, with blank space on both sides.
- **Clip length.** A video or audio clip can't be trimmed past the end of its file. This is enforced in the engine (`TrimClip`) and during the drag (`tail_limit`). On a phone, titles and stills stop at the end of the footage.
- **Quick-action bar ("tooltip").** It opens only when a clip is tapped (`Editor.tip-for`), and closes on any other touch or when a tool is used.
- **Extend handles.** "Extend to clip beginning/ending" appears only after a quick tap (under 0.3 s) that starts and ends on the same ‹ › tab.
- **Speed.** The phone preview composites at most 960 px on its long side (`Studio::preview_scale`). Keep it light, because smoothness is the number-one complaint.

## How we work (important)

1. **Never build locally.** A local build takes 20+ minutes and the owner stops it. CI does all the compiling.
2. **Read before you edit.** Trace the whole flow: Slint → `Editor` global → `lib.rs` → `Studio` → `Command`. Then make the smallest correct change, in the place every caller goes through.
3. **Match the house style.** It's Rust 2024, Slint 1.17, and doc comments that explain *why*. Keep new strings in `I18n.t("…")` (Slint) or `t("…")` / `tf("…")` (Rust).
4. **Before every push:**
   ```
   python3 scripts/locales.py          # regenerate the English string list
   python3 scripts/locales.py --check  # must exit 0
   nix develop --command bash -c 'cd src && cargo fmt --all'
   ```
   CI fails on unformatted code.
5. **Every logic change gets one small test** next to the code (`#[test]`), for example the trim clamp, the caption word grouping, or the highlight scoring.
6. **Test on a phone without releasing:**
   - Push to `main`, then run `scripts/phone.sh --logs`.
   - The script waits for the CI build of the current commit, installs it with `adb install -r` (projects are kept), opens it, and follows the log.
   - Connect the phone by USB debugging, or by Wireless debugging with `adb pair` and then `adb connect`.
7. **To release (only when the owner asks):**
   - Bump `version` in `src/Cargo.toml` and the `concat*` entries in `src/Cargo.lock`.
   - Commit, push `main`, then push the tag: `git tag vX.Y.Z && git push origin vX.Y.Z`.
   - Watch the "Release" run. When it succeeds, delete the previous release (keep its tag).
   - **If the publish step fails with a GitHub error page after the APK built:** download the artifact, `gh release upload vX.Y.Z <apk>`, then `gh release edit vX.Y.Z --draft=false --latest`. Don't rebuild.
8. **If CI fails:** read the first error with `gh run view <id> --log-failed`. Fix it, commit, and push again. For a failed tag, move the tag onto the fix and push it again.
9. **Signing.** APKs are signed with the repository key (secrets `ANDROID_KEYSTORE` / `ANDROID_KEYSTORE_PASSWORD`), so every build updates the last one. Don't change or regenerate that key.
10. **Commits** use a short summary line, then bullets saying what changed and why.
11. **Reporting.** When you report back, say plainly what was **not** tested on a device. Nothing counts as working until it has run on a phone.

## Known open work

- **Removing hidden features from the build.** Effects, cutout (ONNX) and TTS (sherpa) are hidden from the UI but still compiled. Taking them out of the build would speed up CI, but they are wired into the preview and export code.
- **CI speed.** The "Build the APK" step takes about 12 minutes. Setting `CARGO_PROFILE_RELEASE_LTO=false` and `CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16` in `android.yml` would roughly halve it. This has been suggested but not approved yet.
- **Cover box.** It only jumps to the start. Choosing a real cover frame isn't built.
- **Old projects.** Projects made before v1.0.9 may have clips in the wrong rows.
- **Export via MediaCodec** (v1.0.9) hasn't been confirmed on a device.

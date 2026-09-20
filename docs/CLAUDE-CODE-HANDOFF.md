# Handoff — give this to the next Claude Code session on Concat

> Read this top-to-bottom. Then verify the "current state" at the bottom before
> touching anything. Two things you MUST do differently from how the last
> session burned itself dead: (1) the release CI checked tag-vs-version and the
> v1.0.18 run failed until the version bump landed — always bump version +
> commit BEFORE tagging; (2) vision is single-image only, so never batch.

---

## 0. What "Concat" is and how we work

**Repo:** `/home/archbishop/Dev/Concat` — a Rust + Slint Android video editor that
is a **1:1 clone of VN Video Editor** (package `com.frontrow`), phone layout under
`src/crates/concat/ui/phone/`, desktop under `ui/workspace/`, single window under
`ui/app.slint`. The goal: every page, screen and interaction matches VN's, judged
by screenshot analysis, not by guessing.

**Workflow (do not break this):** never build locally — push to GitHub and let the
**Android Build / Release / CI** workflows build the APK; you tag `v1.0.x` and the
**Release workflow** checks `tag == version in src/Cargo.toml` else it fails. The
APK lands as a GitHub release asset. Then `git push origin main --tags`. That's the
whole deploy loop: **edit → commit → bump version in src/Cargo.toml → tag v1.0.x →
push main + tags → CI builds APK → user installs.** The user installs the APK
themselves on a Xiaomi Redmi Note 11 Pro+ (adb `4a9edf0fc5b3`); apps are
`com.frontrow` (VN) and `app.concat.editor` (Concat).

**Decision rule:** present the screenshot analysis + a crisp gap list and **get the
user's approval before writing code**. Ask which to do first; default to chat reply,
not a page. Keep edits surgical and explainable. Never invent "gaps" — verify by
reading the real code first (the last session cost two full turns shipping corrections
because two of its three "missing" pages were already at parity).

---

## 1. The vision problem — READ THIS FIRST, IT IS THE HARDEST PART

The screenshots are real WhatsApp images under `screenshots/`. The **single hardest
thing this session**: actually *seeing* them. Raw vision calls failed or returned
empty for most images, and multi-image batches returned nothing. Do not re-derive a
vision pipeline from scratch — the machine's config changed (key no longer in the
usual files), so reuse the **proven driver**:

- `/tmp/opencode/see.py` — vision helper written this session. It reads
  `OPENROUTER_API_KEY` from live `/proc/*/environ` (NUL-safe regex), uses model
  `inclusionai/ling-3.0-flash-vl:free`, and **takes exactly ONE image** — multi-image
  calls return `None` and silently waste budget. Do NOT send arrays. Reuse as-is.

The populated analysis lives in `/tmp/opencode/serial_vn.md` (VN ground truth, real
verbatim content on sections 1, 2, 3, 4, 18, 24) and `/tmp/opencode/serial_cut.md`
(ours, sections 1, 2, 5). If the free vision tier ratelimits ("free-models-per-day"),
tell the user plainly and re-run later — never invent what a screenshot shows.

### What the screenshots literally contain (so you don't re-walk this)

The VN set under `screenshots/VN/` is the **single-recording flow** — from a fresh
tap through export to the export panel (flows sections ~1-4, plus 18 = export screen,
24 = text editor). It does NOT contain a home-page or gallery-picker screenshot.
Sections that returned "None" in serial_vn.md are the ones the (single-image) model
couldn't parse yet — re-run vision on those files one at a time if you need them.

The Concut set `screenshots/Concut_latest/` (7 images) is OUR phone app's screens:
home ("Your projects" / error banner / hero / recents), empty bin state, library
sheet grid.

**What is genuinely confirmed & shipped this session (v1.0.18):**
- The phone home "+" (`phone-home.slint`) no longer hands off to the system
  picker; it opens the **in-app media library page** instead — `phone-editor.slint`
  watches a new `Editor.library-token` and opens its Media Library sheet on the
  fresh project, the way VN lands a new edit on its library pagecars. The library
  sheet's own Import button still opens the device picker.
- The workspace Cargo version was realigned to the tag (that's what made the earlier
  releases green in CI).

**Honest parity audit (read `docs/screenshots-analysis.md` in the repo — it is the
authoritative gap list, corrected this session):** crop ratios (Free/1:1/2:3/…), export
sizes, text tools, and settings were all found **already at parity** — three "gaps"
from earlier analysis were false positives killed by actually reading the code. The
**remaining real gaps** are in that doc.

---

## 2. Remaining tasks (do these next, in order)

1. **Text editor grips (VN #24):** the selected text clip does not yet stamp a
   white bilinear bounding box + 8 grips on the preview, movable/resizable in
   place, the way VN does (VN shows a filled rounded-rect with a move + resize
   grip on the text layer). This was approved as "text first". The crop
   machinery already exists (preview-pane.slint ~688-748 grip TouchArea + 8
   handles) — reuse it for the selected text clip, bound to
   `text-offset-x/-y/text-width/-height`.
2. **Export panel visual polish** — making the export sheet look exactly like VN's
   export screen (the biggest visible difference the analysis found).
3. Re-scan the "None" serial sections once vision is available again (one image
   per call). Especially sections 18 (export) and 24 (text editor) — those had
   real content but may need a re-read for detail.
4. Keep the bool "include media import + blur backdrop" behavior coherent: the
   library sheet already dims/blurs behind it; make sure the phone editor's blur
   conveys selection like VN (thumbnails stay sharp, the timeline area dims).

---

## 3. Where the code lives (precise anchors)

- `src/crates/concat/ui/phone/phone-editor.slint` — the phone editor: topline
  (lines ~281-350): `back-token`, `add-token`, timing controls; tool row
  (~462) switch by `ClipKind`; Library sheet is `if root.sheet == "library"`.
- `src/crates/concat/ui/phone/phone-home.slint` — home: hero, recents rows,
  "+" FAB (fires `Editor.shortcut("new-from-gallery")`).
- `src/crates/concat/ui/phone/phone-editor.slint` (the "+"-watcher): at
  ~line 319 (`property <bool> library-was-opened` + `changed Editor.library-token`).
- `src/crates/concat/ui/editor.slint` — global `Editor`: token stores (~line 64:
  `add-role/add-token`, ~72: `library-token`), media model, `selected-text-*`.
- `src/crates/concat/src/lib.rs` — `new-from-gallery` shortcut arm routing
  (it calls `quick_project()` then lands on the library sheet via
  `set_library_token` instead of `pick_media_async`). Other arms: "import",
  "export", "text", "crop".
- `src/crates/concat/ui/workspace/preview-pane.slint` — grip machinery for
  crop (~688-748); reuse for the text box grips.
- `src/crates/concat/ui/workspace/media-pane.slint` — the MediaPane grid; the
  `bare` (phone) branch powers the library sheet's grid; watch `bare`, "none"
  pages, filter tabs.

---

## 4. Release checksums you should not re-learn

- `version = "1.0.x"` must match in BOTH `src/Cargo.toml`:6 AND the tag name
  (`v1.0.x`). Push version change BEFORE tagging.
- Release workflow lives in `.github/workflows/release.yml`; Build in
  `android.yml`; CI in `ci.yml`. All run on push; Release + Build also on tags.
- `git tag d` cleanup not needed — just bump + re-tag the same `v1.0.x` name and
  force-push both main and the tag; CI starts a fresh run.
- Android device: `adb devices` → `4a9edf0fc5b3` for install if the user wants
  an on-device screenshot; the `app.concat.editor` package.

---

## 5. Current state (verified at session end)

- `src/Cargo.toml`: `version = "1.0.18"`, tag `v1.0.18` pushed, Release/Build/CI
  workflows re-running (fresh, in_progress) — that run may still be finishing;
  when it's green the APK for v1.0.18 is the release asset.
- Working tree is clean (all edits committed). Docs of this session:
  `docs/screenshots-analysis.md` (corrected gap list) is committed to the repo.
- The user is out of free tokens this session ("big pickle credit burned out") and
  WILL return with fresh credits — that's why this handoff matters. They asked
  specifically: tell the next session how hard the images were to understand, where
  they are, everything done this session, and what remains. **Answer with this file.**

---

## 6. Talpline for the user

This session the user speaks in sleeps: they go to AI-action-of-sleeping while you
ship — do your final commit, tag, push, confirm CI, then give a short status and
punch out. Do NOT run OS-level shutdown; end the session and let them restart.

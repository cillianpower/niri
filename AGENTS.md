# AGENTS.md — niri fork

## Current sync and local deployment — 2026-09-25

`sync-focus-ring-anim` merged the latest `upstream/main` at `1f03391e` in
`5309f438`. This adds one upstream documentation commit after the earlier
`e3c52255` sync; no source conflict or compatibility change was needed.
`cargo test --workspace --locked` passed (207 niri and 19 config tests, plus
wiki, IPC, and doc tests), as did clippy with `-D warnings` and a release build.

A nested winit run used a throwaway copy of the desktop dotfiles config,
including the live focus-ring, shadow, corner, and DMS layout settings. It
validated, launched two Ghostty windows, and IPC focus changes showed the
selected ring moving between them in screenshots. The nested output was one
winit display; physical multi-monitor behavior remains unverified.

The tested release binary (`v26.04-172-g5309f438`) was installed to
`~/.local/bin/niri`, with the previous user binary backed up beside it. The
desktop dotfiles now opt into the 300 ms fade and manage the niri service
override. `/usr/bin/niri` remains the stock Fedora fallback. The compositor
service was restarted, and IPC confirmed the new build and config load.
The source checkout is `/home/user/Projects/niri`.

## Latest upstream sync — 2026-09-25

`sync-focus-ring-anim` merged `upstream/main` at `e3c52255` (24 new commits) without conflicts.
The previous sync is recorded below. This branch remains a fork work branch; upstream's
`CONTRIBUTING.md` asks PR authors to rebase and keep feature commits focused. The new PR
template explicitly disallows LLM-written PR code and descriptions, so this branch must
not be presented as a ready-to-submit upstream PR without human authorship and review.

The focus transitions now start from layout refresh, before rendering. Focus-ring selection
and shadow activation have separate transition state: this preserves the selected ring on
inactive monitors and lets the shadow use its inactive color. Both use the same opt-in
animation config. A deselected ring remains rendered and keeps its prior active/inactive
color until its fade-out finishes. The transition helper has a unit test for initialization,
reversal, completion, and `off`. `cargo test --workspace --locked` passed (207 niri tests,
19 config tests, wiki parsing, IPC, and a doc test), including the multi-monitor
selection regression test. Clippy, build, and
nightly format checks passed. The default and throwaway nested configs validated.
A nested winit compositor ran with two Ghostty windows; IPC focus switching and
screenshot captures showed both rings changing alpha smoothly over a one-second
linear fade. The nested config was then restored to 300 ms EaseOutQuad. This
verifies rendering on one winit output. Multi-monitor behavior needs hands-on testing.

## Latest upstream sync — 2026-09-17

On `sync-focus-ring-anim`, existing uncommitted shadow-fade/scheduling work was preserved
in `841a9844`, then `upstream/main` at `b735805d` was merged without conflicts in
`62f8e0ac` (53 upstream commits). No compatibility fixes were needed.
`cargo check --locked`, `cargo build --locked`, `cargo test --locked` (199 passed),
and `cargo clippy --all-targets --locked` passed. Existing local config validation
also passed. Built binary: `target/debug/niri`; nothing installed or restarted.
Visual focus-ring/shadow behavior remains unverified after this sync (headless session).
Older branch, installed-binary, and test-count references below describe prior work.

Fork: `cillianpower/niri`, branch `focus-ring-anim` from tag `v26.04` (matches Fedora 44 `/usr/bin/niri`). Upstream: `niri-wm/niri`. Niri source checked out at `/home/user/Development/niri`.

> **Materials system** is config-only and does not belong in this fork. Design docs and pipeline have moved to the dotfiles repo at `~/Development/dotfiles/docs/niri-materials/` (`README.md`, `materials-design.md`, `architecture.md`, `niri-reference.md`). Implementation is an external orchestration script that emits `~/.config/niri/materials.kdl` + `gsettings`/terminal fan-out — no niri source changes. See that location as the source of truth; do not add materials logic here unless explicitly asked to implement a fork extension (tint/per-output/per-window blur).

## Scope — single active body of work in this repo

| # | Body of work | Status | Docs |
| --- | --- | --- | --- |
| **Focus ring alpha fade + shadow cross-fade** — animate focus ring alpha and shadow color on focus change | **Implemented & dogfooded** on this branch. Off by default (opt-in). | This file + `FOCUS_RING_AND_SHADOW_ANIM_PLAN.md`, `HANDOVER.md` |

This repo is only for work that extends niri itself.

---

## Focus ring alpha fade + shadow cross-fade (dogfooded)

### Status

Implemented, tested, working on branch `focus-ring-anim`. Builds to `/usr/bin/niri` (release + line tables, ~154MB); original Fedora build backed up at `/usr/bin/niri.backup`. Restart via `niri msg action quit` or `systemctl --user restart niri.service`. See `HANDOVER.md:20-29`.

### What it does

Focus ring previously snapped. Now fades alpha `0 ↔ max-opacity` when selection changes; shadow cross-fades between `color` and `inactive_color` when activation changes. The two transitions use the same `animations.focus-ring` config and start together when a selected window gains focus on the active monitor. The feature is **off by default**.

Opt-in config:

```kdl
animations {
    focus-ring {
        duration-ms 300
        curve "ease-out-quad" // or linear, ease-out-cubic, ease-out-expo, cubic-bezier, spring
    }
}
```
`focus-ring { off }` restores instant. See `FOCUS_RING_AND_SHADOW_ANIM_PLAN.md:56-66` for default-off `Default` impl note.

### Files changed

1. **`niri-config/src/animations.rs:14-45`** — new `FocusRingAnim(pub Animation)` newtype over `Animation`, `Default`, `knuffel::Decode`, field `Animations.focus_ring` + `AnimationsPart` + `merge_clone!`. Pattern follows `ScreenshotUiOpenAnim` etc. Shadow reuses same config (no separate key).
2. **`src/layout/tile.rs`** — `FocusTransition` tracks an initial target and optional animation. `Tile::update_focus()` runs during `Layout::refresh()` via scrolling/floating spaces, before render. Separate ring and shadow transitions preserve distinct selection and activation semantics. Interruption starts from the current progress; `Tile::advance_animations()` clears completed animations; `Tile::are_transitions_ongoing()` keeps frames scheduled.
3. **`resources/default-config.kdl:93-99`** — commented-out example.
4. **`niri-config/src/lib.rs`** — insta snapshot updated.
5. **`niri-config` `Color::mix`** + `Shadow::update_render_elements(focus_progress: f64)` change (`FOCUS_RING_AND_SHADOW_ANIM_PLAN.md:393-410`). Non-focus callers (`mapped.rs`, `workspace.rs`) pass `1.`.

Architecture diagram `AGENTS.md:109-140` (pre-shadow) still applies, with `focus_progress` driving both ring and shadow.

### Edge cases handled

`off` = early-out (no `Animation`, steady-state alpha); first refresh records state without animating (no startup flash); interrupted transitions start from current progress; fullscreen/maximize hidden via `expanded_progress`; config merge via `merge_clone!`. Ring selection remains visible on inactive monitors. Shadow animation is skipped when shadows are disabled.

### Visual verification

The prior render-time transition creation has been replaced with refresh-time creation.
Check smoothness and multi-monitor selection in the nested compositor; static tests do not
prove animation timing on a real display.

### Testing (nested, no session pollution)

Build without installing, run as nested winit compositor inside real session (auto-selected when `WAYLAND_DISPLAY`/`DISPLAY` set and not `--session`):

```bash
cargo build            # or --release
./target/debug/niri --config /tmp/test-niri.kdl  # throwaway config with animations { focus-ring { ... } }
./target/debug/niri validate --config /tmp/test-niri.kdl
```

See `FOCUS_RING_AND_SHADOW_ANIM_PLAN.md:120-271` for full workflow, throwaway config (`/tmp/niri-focus-ring-test.kdl`), variants (spring, slow 1000ms rapid Alt+Tab), and winit limitations. Terminal on this machine is `ghostty` (not `alacritty`).

### Upstream PR strategy

Keep any upstream submission focused and rebased onto current `upstream/main`. The fork
currently has ring and shadow animation coupled to one config, while the historical PR
plan below proposed ring only; settle the submission scope before preparing commits.
The wiki now documents the option with a `Since: next release` tag.

---

## Working conventions for agents

- **Build/test:** `cargo test` (195 pass expected), `cargo clippy --all-targets`, `cargo build` (never `cargo install` over system niri). Validate KDL with `./target/debug/niri validate --config <file>`.
- **Config parsing:** new `Animation` newtypes follow `knuffel::Decode` newtype pattern + `merge_clone!` triple wiring.
- **Do not touch** `~/.config/niri/config.kdl` or live session unless asked; use `/tmp/*.kdl` + nested winit.
- **Docs sync:** when changing this fork, update `FOCUS_RING_AND_SHADOW_ANIM_PLAN.md` + this file; materials docs live in `~/Development/dotfiles/docs/niri-materials/` — edit there, not here.

## Agartha desktop glass fixes — 2026-10-04

This separate combined build checkout uses the pinned overlay maintained in
`~/Projects/dotfiles/hosts/desktop/niri/glass-overlay.patch`; the original niri
and DMS forks are unchanged. Layer glass now requires a nonempty committed
client blur region, so DMS closing/input surfaces cannot become geometry-shaped
glass. KWin mode now honours existing adaptive dim/boost controls. Render-loop
diagnostics use trace. Installed release passed 143 nested checks including GPU
saturation/highlight and empty-layer-region regressions, and workspace tests
(207 niri, 21 config, support/doc tests). Full report:
`~/Projects/dotfiles/docs/niri-materials/tests/2026-10-04-glass-lifecycle.md`.

The follow-up corner shader uses the clipping radius for KWin refraction and
smooths the nearest-edge transition inside the bevel. GTK palette/backdrop
fixes and the local 20 px clip-radius override remain in dotfiles. Verification
report: `~/Projects/dotfiles/docs/niri-materials/tests/2026-10-04-corners-gtk.md`.

The 2026-10-05 follow-up keeps the true 12 px clipping contour and eases
KWin optical normals toward 24 px interior curvature over 12 px depth.
Only interior refraction normals change; clip/highlight distance stays exact.
Verification: `~/Projects/dotfiles/docs/niri-materials/tests/2026-10-05-interior-curvature.md`.

DMS layer glass now derives optical corner radii from committed rounded
blur-region scanlines when layer rules leave the radius unset. Separate optical
uniforms leave coverage and explicit clip rules intact. Rectangular/disconnected
regions keep square optics. Checks and deployment are recorded in
`~/Projects/dotfiles/docs/niri-materials/tests/2026-10-05-dms-contours.md`.

## Migration preflight — moves pending

Moves remain pending. Read the [desktop migration audit](/home/user/Projects/DESKTOP-MIGRATION-AUDIT.md) before relocation; this section does not authorize moves or runtime changes.

- This is a separate `git clone --shared` checkout, not a linked worktree. `.git/objects/info/alternates` names `/home/user/Projects/niri/.git/objects`, and `origin` names `/home/user/Projects/niri`. Moving either checkout requires coordinated repair and uninterrupted object access.
- The audit found detached HEAD `24b1cd50`, modified glass sources/docs and untracked `src/render_helpers/liquid_glass.rs`. Preserve the complete dirty checkout; a clean clone plus the old overlay is not a substitute.
- Dotfiles' `hosts/desktop/niri/prepare-glass-source.sh` defaults to this path and records the shared-clone/overlay provenance. Keep those references and experiment evidence with the family.
- This experimental glass source is distinct from the current glass-free live fork. Historical deployment notes above do not authorize replacing the installed compositor or restarting the desktop.

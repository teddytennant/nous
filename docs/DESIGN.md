# Design

Nous is a decentralized everything-app. The interface should feel like a well-set
page, not a dashboard. Editorial energy — European typographic discipline, the
restraint of Japanese print, the quiet confidence of Apple's best work — over
crypto-native maximalism.

The previous identity ("Infinite Minimalism," gold on near-black) read like
every other dApp. This pass replaces it with something quieter and more
specific: ink, ivory, and a single oxblood mark.

The canonical token values live in [`design/tokens.json`](./design/tokens.json).
Surface-specific files (`apps/web/src/app/globals.css`,
`apps/ios/Nous/Sources/Theme/NousTheme.swift`,
`apps/android/.../ui/theme/Theme.kt`, `crates/nous-design/`) are derivations of
that source of truth.

---

## Principles

### 1. Typography is the instrument

Hierarchy comes from size, weight, and tracking — never boxes, shadows, or
colored chips. A page reads top to bottom because the type tells you where to
look, not because a card outline does.

> A section heading is `display 2.25rem, weight 400, tracking -0.02em` in
> Source Serif 4. Body is `1rem, weight 400, tracking 0` in Geist. The shift
> from serif to sans is the hierarchy. No background fill, no rule, no badge.

### 2. One accent, used like a stamp

Oxblood (`#B23A3A`) is the only accent in the system. It marks intent —
primary actions, the active nav item, focus rings, the dot beside a live
peer count. It is never used as a wash, a gradient, or a glow. Treat it the
way a printer treats spot color: precious, deliberate, sparse.

> A "Send" button is ivory text on the page surface with an oxblood
> underline-on-hover and an oxblood 1px ring on focus. The button itself has
> no fill.

### 3. Hairlines, not borders

Dividers are 1px rules at `--rule` (`#1F1D1A` on dark, `#DCD7CC` on light).
No 2px chunky outlines. No drop shadows under cards. Panels are bounded by
type, whitespace, or hairlines — in that order of preference.

> A card is just typography on `--ink-2` with a single 1px rule underneath.
> No surrounding border, no rounded corners above 4px, no elevation.

### 4. Generous whitespace, asymmetric grids

Editorial breathing room over packed UIs. A 12-column grid where the body
sits in columns 2-9 and metadata sits in column 11 reads like a page;
a stretch-to-edges flex layout reads like a settings panel.

> The dashboard isn't a 3×3 stat-grid. It's a primary column for the active
> conversation/feed and a narrow rail for status — peer count, sync state,
> identity — with a hairline between them.

### 5. Mono carries data

Anything machine-readable — DIDs, public keys, hashes, peer counts, balances,
block heights — is set in Geist Mono. This is the deepest hierarchy cue: if
it's monospaced, it's a fact, not a sentence.

> `did:nous:z6Mki…J9` and `0.04231 NOUS` and `127 peers` are all mono. The
> word "peers" beside `127` stays in sans.

### 6. Motion clarifies, never decorates

Motion exists to communicate a state change: a dialog opening, a value
updating, focus moving. If you can delete an animation without losing
information, delete it. State change is `160ms`, page change is `240ms`,
everything else is `0`. Easing is `cubic-bezier(0.2, 0, 0, 1)` — a single
deceleration curve, used everywhere. `prefers-reduced-motion` honoured fully.

> A peer joining the network does not pulse, breathe, or shimmer. The number
> updates. That's the animation.

### 7. Iconography earns its place

Primary navigation uses custom 1px-stroke marks drawn for this product.
Secondary chrome may use a restrained line-icon set, but never a decorative
or multi-stroke library glyph. Icons are monochrome (`--ivory-dim` at rest,
`--oxblood` when active) and never colored to mean a category.

> A "messages" mark is a single hairline rectangle with a notch — drawn for
> us. We do not pull a chat-bubble from Heroicons.

### 8. Dark default, light first-class

Dark mode is the default because the majority of long-form Nous use happens
at night, in terminals, and at low ambient light. Light mode is not an
afterthought — it has its own intentional palette (`--paper`, `--ink-text`)
tuned for daylight reading on glass. The accent does not change between
modes.

> A user opening the iPad app on a sunlit balcony gets `paper` background,
> `ink-text` body, the same `oxblood` actions. The shift is calm, not jarring.

---

## What we don't do

A short, deliberate list of what is out of bounds. If you find yourself
reaching for one of these, stop and re-read the principle it violates.

- **Gradients.** No background gradients. No text gradients ("hero shimmer"
  is dead). No oxblood-to-magenta. The page is `--ink`. The accent is a
  single hex value.
- **Drop shadows.** Panels do not float. If a panel needs to feel "raised,"
  raise the surface (`--ink-2`) or change the type weight. Never use
  `box-shadow` for elevation.
- **Glow effects.** No gold halo on focus, no neon ring on hover, no
  pulsing CTA. Focus is a 1px oxblood ring at `--ring`. State is type.
- **Frosted glass on content.** Backdrop blur is permitted only on chrome
  that overlays content (e.g. a navigation sheet during transition). Never
  on cards, never on long-form content, never on the page surface itself.
- **Material elevation.** The Compose theme uses Material3 plumbing because
  Compose insists, but the values are hairline-flat. No tonal elevation
  ramp, no shadow under bottom-sheets, no FAB.
- **FABs.** Primary actions live in the document flow or in the page
  header. Floating action buttons do not exist in this product.
- **Library icons in hero shots.** Marketing pages and onboarding never
  show generic Heroicons / Lucide / Material icons in featured spots.
  Custom marks only.
- **Multi-color palettes.** Eight tokens span the entire interface
  (`ink, ink-2, ivory, ivory-dim, stone, rule, oxblood`, plus `sage`/`clay`
  for state). There is never a teal, a violet, or a "secondary brand."
- **Motion for delight.** Bouncy springs, breathing pulses, parallax,
  confetti, idle-loop animations on hero networks. The motion budget is
  spent on legibility, not whimsy.

---

## Tokens at a glance

For canonical values see [`design/tokens.json`](./design/tokens.json).

```
Color (dark, default)
  ink         #0E0E0C   primary surface
  ink-2       #161613   raised surface
  ivory       #EFEAE0   primary text
  ivory-dim   #C9C3B6   secondary text
  stone       #6F6A60   tertiary / metadata
  rule        #1F1D1A   1px hairlines
  oxblood     #B23A3A   single accent
  oxblood-dim #7A2A2A   pressed / visited
  sage        #8FA48A   positive state
  clay        #C2785A   warning state

Type
  display   Source Serif 4, 400, -0.02em   (serif)
  heading   Geist, 500, -0.01em
  body      Geist, 400, 0
  meta      Geist, 400, +0.04em, uppercase
  mono      Geist Mono, 400

  Scale (rem)  0.75 / 0.875 / 1 / 1.125 / 1.375 / 1.75 / 2.25 / 3 / 4.5

Space (4px base)
  4 8 12 16 24 32 48 64 96 128

Radius
  sharp 0   soft 2   panel 4   pill 999

Motion
  state 160ms   page 240ms   easing cubic-bezier(0.2, 0, 0, 1)
```

---

## Phase notes

This document is part of the Phase 1 foundation pass. Phase 1 owns:

- This document, `docs/design/tokens.json`.
- The web `:root` / `.dark` token block (`apps/web/src/app/globals.css`).
- The iOS `NousTheme` enum (`apps/ios/Nous/Sources/Theme/NousTheme.swift`).
- The Android `Theme.kt` / `Color.kt` / `Type.kt`.
- The shared Rust `crates/nous-design/` token crate.
- The new brand mark (`apps/web/public/icon.svg`) and wordmark.

Phase 2 surface agents own the screens, components, and animation cleanup
that consume these tokens.

### Raster favicons

Raster favicons (`icon-192.png`, `icon-512.png`, `apple-touch-icon.png`,
`favicon.ico`) were regenerated from the new `icon.svg` using ImageMagick
(`magick`) in this Phase 1 pass. Neither `rsvg-convert` nor `inkscape` is
installed on the build host; if a future regeneration pass requires those
tools (e.g. for true sub-pixel hinting on the 16×16 favicon), install
`librsvg` first and re-run the conversion step.

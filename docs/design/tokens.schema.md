# tokens.json — schema

Canonical token shape for `docs/design/tokens.json`. Hand-edited; all surface
files (web `globals.css`, iOS `NousTheme.swift`, Android `Color.kt`/`Type.kt`/
`Theme.kt`, the Rust `nous-design` crate) derive from these values. The schema
is intentionally minimal: a token that can't be expressed here doesn't exist
in the design language.

## Top-level object

```jsonc
{
  "$comment": string,            // free-text provenance / hand-edit note
  "color":   <ColorBlock>,
  "type":    <TypeBlock>,
  "scale":   <ScaleBlock>,
  "space":   <SpaceBlock>,
  "radius":  <RadiusBlock>,
  "motion":  <MotionBlock>
}
```

No additional top-level keys. If a future surface needs something the schema
can't express (e.g. a new state token), extend the schema here first, then add
the value in `tokens.json`, then derive on each surface.

## ColorBlock

```jsonc
{
  "dark":  { <name>: "#rrggbb", ... },
  "light": { <name>: "#rrggbb", ... }
}
```

- Keys are lowercase-kebab-case (`oxblood-dim`, `ivory-dim`, `ink-text`).
- Values are six-digit lowercase hex; no alpha. Translucency is expressed with
  semantic tokens (`oxblood-dim`, `stone`) rather than RGBA.
- The `dark` block is the default surface. `light` is a first-class variant —
  it omits `ink` / `ink-2` (which are dark-only surface tokens) and gains
  `paper` / `ink-text` / `hairline` (light-only). `oxblood` is identical
  across modes by design; `sage` and `clay` shift slightly for daylight
  contrast.

## TypeBlock

```jsonc
{
  "<role>": {
    "family":     string,         // e.g. "Source Serif 4", "Geist", "Geist Mono"
    "weight":     number,         // 100..900, OpenType convention
    "tracking":   string,         // CSS em string: "-0.02em", "0", "0.04em"
    "lineHeight": number,         // multiplier of font size
    "transform":  string?         // optional, e.g. "uppercase" for meta
  }
}
```

Five canonical roles: `display`, `heading`, `body`, `meta`, `mono`. Adding a
sixth requires a principle update in `DESIGN.md` — hierarchy is a contract.

## ScaleBlock

```jsonc
{ "<step>": "<rem>", ... }
```

Keys are stringified integers `0`..`8`; values are `rem` strings. The scale
is fixed at `0.75 / 0.875 / 1 / 1.125 / 1.375 / 1.75 / 2.25 / 3 / 4.5`. Body
sits at step `2` (`1rem`). Surface code translates to platform units (sp on
Android, pt on iOS, px implicit on web).

## SpaceBlock

```jsonc
{ "<step>": "<px>", ... }
```

Keys are stringified integers from the 4px-base ramp (`1, 2, 3, 4, 6, 8, 12,
16, 24, 32`). Values are px strings. Steps `5`, `7`, `9`, `10`, `11`, `13–15`
intentionally do not exist — the gaps are the design.

## RadiusBlock

```jsonc
{
  "sharp": 0,        // square — default for content
  "soft":  2,        // small affordances (chips, inputs)
  "panel": 4,        // maximum for content panels
  "pill":  999       // controls only (toggles), never panels
}
```

`8`, `12`, `16` are deliberately absent. If a panel needs to feel "softer than
4px," the answer is more whitespace, not a larger radius.

## MotionBlock

```jsonc
{
  "state":   "<ms>",   // 160ms — state changes (focus, hover settle, toggle)
  "page":    "<ms>",   // 240ms — route transitions
  "instant": "<ms>",   // 0ms — most things; the explicit zero is a contract
  "easing":  "cubic-bezier(0.2, 0, 0, 1)"
}
```

A single deceleration curve, used everywhere. Decorative easings (overshoot,
elastic, bouncy springs) are out of bounds — see "What we don't do" in
`DESIGN.md`.

## Conventions

- Hand-edit only. There is no generator. Diffs are reviewable.
- Keep keys sorted alphabetically within each block, except where a documented
  ordering carries meaning (palette tokens grouped surface → text → state →
  accent).
- Hex values are full six digits, lowercase, no shorthand.
- When a token is removed, surface derivations must be updated in the same
  changeset — don't orphan a CSS variable.

## Derivation map

| Surface       | File                                                                | Derives                                  |
|---------------|---------------------------------------------------------------------|------------------------------------------|
| Web (CSS)     | `apps/web/src/app/globals.css`                                      | Custom props on `:root` and `.dark`      |
| iOS (SwiftUI) | `apps/ios/Nous/Sources/Theme/NousTheme.swift`                       | `NousTheme` enum: `Color`, `Font`, …     |
| Android       | `apps/android/.../ui/theme/{Color,Type,Theme}.kt`                   | Material3 `ColorScheme` + `Typography`   |
| Rust / TUI    | `crates/nous-design/src/{palette,typography,spacing}.rs`            | RGB triples + optional `ratatui::Color`  |

A change in `tokens.json` is the trigger for a four-surface follow-up. Phase 2
agents own those derivations on a per-surface basis; this schema is the
contract between them.

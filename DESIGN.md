# GODsend PSP design system

The "midnight console" look: this app shares a design family with GODsend 360
and Keepsake Atelier. Near-black ink, one laser accent spectrum, brass as the
single warm note, hairline glass surfaces, monospaced data, staggered
entrances, and a hard reduced-motion kill switch.

Everything below is implemented in `app/src/styles.css`. Tokens live on
`:root`; raw hex in components is banned.

## Tokens

```css
/* surfaces */
--ink-0: #04060c;  --ink-1: #070a12;  --ink-2: #0b0f1a;
--surface: rgba(16, 20, 34, 0.72);
--surface-2: rgba(22, 27, 45, 0.82);
--surface-3: rgba(30, 37, 58, 0.9);

/* lines */
--line: rgba(255, 255, 255, 0.065);
--line-strong: rgba(255, 255, 255, 0.13);
--line-laser: rgba(109, 139, 255, 0.32);

/* type */
--text: #eef2fb;  --text-dim: #a5b0c7;  --text-faint: #7b869f;

/* spectrum: indigo -> cyan identifies this app */
--laser: #6d8bff;  --laser-2: #56e8f5;  --violet: #b79bff;  --brass: #f2c078;
--good: #4ae3a5;  --warn: #ffc46b;  --bad: #ff8484;
--grad-laser: linear-gradient(112deg, #56e8f5, #6d8bff 52%, #8678ff);

/* shape & motion */
--radius-xl: 22px; --radius-lg: 16px; --radius: 12px; --radius-sm: 9px;
--ease: cubic-bezier(0.22, 0.8, 0.24, 1);
--ease-out: cubic-bezier(0.16, 1, 0.3, 1);

/* glow */
--glow-laser: 0 10px 34px -12px rgba(109, 139, 255, 0.55);
```

## Fonts

- Chakra Petch (400-700) for UI copy, via Google Fonts import.
- JetBrains Mono (400/500) for every identifier, path, serial, size and
  timestamp: if a number changes on screen it is mono.

## Shell layout

Five regions on a CSS grid:

```
topbar      62px (84px on macOS, padding-top 22px, drag region)
rail        236px nav, 9 sections, two-letter mono glyphs, collapses to
            icon-only under 1080px
workspace   one scrolling page, content max-width 980px
inspector   fixed right column 300px: device card + job queue, hidden
            under 1240px
statusbar   34px: device dot, ffmpeg state, eject, version
```

On macOS the window uses an overlay title bar. `app/src/main.tsx` sets
`data-os="mac"` inside the real Tauri app and the stylesheet pads the topbar
22px from the top inside an 84px row, so the brand clears the traffic lights.
Do not use `padding-left`; it shifts the brand sideways instead. Browser
previews have no traffic lights and cannot show the inset.

## Ambient backdrop

Fixed radial gradients, three blurred drifting orbs (cyan, indigo, brass,
`orb-drift 26s`), and a 56px hairline grid. The whole element sits at
`z-index: -1`; do not raise it. Do not put a `mask-image` over the full
viewport: full-viewport masked fixed layers composite over app content in
both WebKit and Chromium and erase text (this was a real bug here).

## Components

- `.btn` ghost / `.btn.primary` (laser gradient, dark text, glow, diagonal
  sheen sweep on hover) / `.btn.danger` / `.btn.small`.
- `.chip` with a `.dot`: state is never color-only, the chip always carries
  a word. `.chip.live .dot` breathes.
- `.seg` segmented control for filters and mode toggles.
- `.bar` 8px progress track; `.bar.live .fill` runs a 1.8s sheen loop.
- `.switch` toggle with a gradient knob.
- `.card` / `.panel` translucent surfaces with hairline border and inset
  highlight.
- `.tile` game cover grid cell: ICON0 art or a monogram, title, mono
  subtitle, Open/Delete actions.
- `.store-card` homebrew catalog card: name, tag, description, repo,
  note (brass), installed chip, install button.
- `.empty` dashed hairline box for empty states.
- `.toast` bottom-center status pill.

## Motion

Entrances: `.page` fades and rises 10px (340ms), direct children stagger
10-130ms, rail items stagger 20-220ms left. Durations `--fast 0.16s`,
`--base 0.26s`, `--slow 0.5s`, easings `--ease` / `--ease-out`. Only
transform and opacity animate. Hard kill switch:

```css
@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after {
    animation-duration: 0.001ms !important;
    transition-duration: 0.001ms !important;
  }
  .backdrop .orb { animation: none !important; }
}
```

## Accessibility

- `:focus-visible` rings in `--laser-2`, offset 2px.
- `aria-current="page"` on the rail, `role="status"` on the topbar title and
  toasts, skip link to `#workspace`, Alt+1..9 section jumps.
- State chips pair a dot with a word; WCAG AA contrast on ink surfaces.

# PairPlay Motion — visual thesis

## Direction: the motion edition

PairPlay looks like a monochrome Sunday broadsheet that has suddenly become a
scoreboard. The newspaper language makes a group activity feel public,
immediate, and easy to scan across a room; oversized scores and rule lines do
the work that glossy game chrome normally does. Motion appears as registration
offsets, cropped arcs, and stamped status words—not as a generic neon gradient.

The interface is explicitly single-mode. Warm newsprint is calmer and more
legible in a shared room than a theme toggle, and preserves the product's
physical-paper premise.

## Tokens

- `--paper: #f3efe3` — warm uncoated newsprint background.
- `--ink: #171714` — softened printer's black; 15.8:1 on paper.
- `--muted: #5b5a52` — secondary copy; 6.1:1 on paper.
- `--sheet: #fffdf5` — raised reading surfaces.
- `--rule: #8a887e` — structural rules and inactive outlines.
- `--signal: #b63b22` — one-color stop-press red for primary actions and live
  state, with white text at 5.1:1.
- `--success: #29633d`, `--warning: #7b5310`, `--danger: #9f281f` — semantic
  inks always paired with a word or icon.

## Type

No runtime font service is used. Headlines use Georgia's high-contrast,
editorial serif; interface copy uses the locally available/system sans stack
`Arial, Helvetica, sans-serif`. This pairing costs zero font bytes and makes
headlines feel printed while controls stay blunt and readable. The six-step
scale is 14 / 16 / 20 / 28 / 44 / 72px with fluid clamps for the top two.
Body text is at least 16px, 1.5 leading, and limited to 68 characters.

## Space and composition

An 8px base rhythm uses 8, 16, 24, 32, 48, and 64px intervals. A thin masthead
rule, narrow utility column, and wider lead column create the broadsheet grid.
Independent choices (games and players) may be boxed; sequential instructions
stay in open columns grouped by proximity. Touch targets are at least 48px and
fixed controls account for safe-area insets. At 390px, the utility column moves
below the lead, metadata abbreviates, and the score rail becomes a two-column
grid. Nothing horizontally scrolls.

## Interaction grammar

- A filled red rectangle is the one current primary action. Secondary actions
  are paper buttons with a 2px ink border.
- Status appears as a small uppercase slug (`LIVE`, `CALIBRATING`, `OFFLINE`)
  before its plain-language explanation. Color is never the only signal.
- Room codes use spaced tabular figures and can be copied. QR and manual code
  are peers, so camera access is never assumed.
- Phone motion is visualized as a black registration cross whose red ghost
  moves with the sensor. On unsupported hardware, the same instrument becomes
  four large direction controls plus a shake button.
- Focus uses a 3px signal outline with 3px offset; pressed states translate 2px
  like a letterpress block.

## Motion policy

Transitions last 180–240ms and change only opacity or transform. New scoreboard
items rise from their list position; the motion ghost follows device input with
no decorative inertia; the stop-press callout folds down from the masthead.
There are no looping ornamental animations or flashes. Under
`prefers-reduced-motion: reduce`, transitions are removed and motion changes
become instant opacity/state updates. Game cues remain text, shape, and sound-
optional, so reduced motion does not reduce playability.

## Original asset plan and prompt sheet

The hero is one generated editorial cut-paper illustration: four anonymous
hands lift unbranded phones around an empty tabletop, with torn halftone motion
arcs and room for the masthead. It explains “old phones become motion
controllers” without depicting an unavailable screen or a specific brand.
All interface marks, calibration crosses, and game symbols are authored in CSS
or SVG so they remain precise and lightweight.

Prompt sheet:

- Subject: four varied but anonymous hands holding plain, unbranded smartphones
  above a shared tabletop; no faces.
- World/materials: 1930s newspaper photo montage, torn paper edges, coarse
  halftone ink, scissors-and-glue registration offsets.
- Light/lens: flat overhead editorial composition, hard paper shadows, generous
  negative newsprint space.
- Palette words: warm ivory paper, carbon black, a single muted stop-press red.
- Negative list: no readable text, no logos, no watermarks, no copyrighted
  characters, no UI screenshots, no extra fingers, no distorted phones, no
  gradients, no glossy 3D.

Asset provenance: `hero-broadsheet.png` and its optimized derivatives are
generated for PairPlay Motion with the Param Factory image deployment on
2026-08-28. The exact prompt and generation metadata live beside the source in
`assets/src/hero-broadsheet.json`. Generated imagery is original and is
disclosed in the product footer.

`pairplay-social-1200x630.webp` and `apple-touch-icon.png` are cropped,
optimized derivatives of that same original hero source. They were made locally
on 2026-09-06 for social previews and installed-app recognition; no additional
third-party art or font is used.

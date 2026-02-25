# Layout Best Practices

## Goals

- Make each slide readable at a glance.
- Preserve visual consistency across the deck.
- Avoid overflow in fixed 1280x720 frames.

## Core composition rules

- Keep one main idea per slide.
- Prefer two-level hierarchy: title + body.
- Keep body content constrained to one of these patterns:
  - single column narrative
  - two-column comparison (`split` + `col`)
  - narrative + visual (chart/image/video)

## Constrained component surface

Use these MDX components as the default layout API:

- `<Stack gap="md|lg|...">`: vertical composition
- `<Row gap="md|lg|..." align="start|center|end|stretch">`: horizontal composition
- `<Grid cols={1|2|3} gap="md|lg|...">`: bounded grid layouts
- `<Card title="..." subtitle="..." tone="default|accent|success|warning|danger">`: consistent panel
- `<Metric label="..." value="..." hint="...">`: KPI-style callout
- `<Caption>...</Caption>`: small informational/meta text

Token-driven conformity (in `slides.css`):

- `--slide-layout-gap`
- `--slide-card-bg`, `--slide-card-border`, `--slide-card-radius`, `--slide-card-padding`
- `--slide-meta-font`, `--slide-meta-size`, `--slide-meta-color`

Prefer these primitives over arbitrary utility class mixes for better visual consistency.

## Density heuristics

Use these as practical limits, not hard law:

- Words per slide target: 40-110
- Words per slide warning: >140
- Bullet points per slide target: 3-6
- Bullet points warning: >8
- Longest paragraph warning: >55 words

When limits are exceeded, split content into another slide.

## Typography and spacing

- Keep top-level titles short and high-signal.
- Use `.prose-compact` for text-heavy slides.
- Keep clear hierarchy: title > section header > body copy.
- Do not style section headers (`h2`/`h3`) at nearly the same size as paragraphs.
- Useful ratio targets for readability:
  - title/body font-size ratio >= 1.45
  - section-header/body ratio >= 1.18
- Leave white space around charts and media.
- Avoid stacking many dense blocks with the same visual weight.

## Two-column pattern

When using `.split`:

- Keep both columns balanced in visual height.
- Keep a visible separation cue (divider, spacing contrast, or background cue).
- Do not place two dense prose blocks side by side.
- Pair text with structured lists, metrics, or visuals where possible.

## Visual/media usage

- Store media inside the project folder.
- Use relative references: `./images/...`, `./media/...`.
- Use images for static context; use video only when motion is essential.
- For large visuals, dedicate slide real estate instead of squeezing text.
- Use explicit alt text that describes why the visual is on that slide.

## Chart/component usage

- Keep chart labels concise.
- Keep legends and captions minimal.
- Place charts in containers that can grow (`flex-1 min-h-0`) when inside column layouts.

## Narrative flow

Recommended slide progression:

1. Context/problem
2. Current state and pain
3. Proposed approach
4. Implementation plan
5. Impact and risk
6. Decision/next steps

## QA checklist before hand-off

- Every slide has a clear message.
- No overflow or clipped content.
- Local assets resolve correctly.
- Slide count and order match requested narrative.
- Preview and build both succeed.
- `review-slides` report is reviewed for density, typography, and image warnings.
- PNG export succeeds when requested.

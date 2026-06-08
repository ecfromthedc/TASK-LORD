# TASK LORD — Colorway & Component Loop ("BEDROCK & DIAMOND")

Paste this as a single self-paced `/loop` (no interval — it iterates design →
render → critique → refine, and stops itself when the bar is met). It converges
the board to the research-grounded palette and component system in
`GOAL-COLOR.md` while preserving everything in `GOAL-DESIGN.md`.

```
/loop Enhance the colorway, design specs, and components of the TASK LORD board at ~/Projects/active/tasklord/board/index.html until it fully satisfies ~/Projects/active/tasklord/GOAL-COLOR.md (the BEDROCK & DIAMOND palette) while still passing every point in ~/Projects/active/tasklord/GOAL-DESIGN.md. The feeling to hit: cool deep bedrock you focus into, ONE warm amber-gold strike that marks the single primary action (free will, the shovel going in), and a diamond-cyan + old-gold reward when a task reaches done (unearthing diamonds). Each iteration: (1) re-read GOAL-COLOR.md and the hard rules in GOAL-DESIGN.md; (2) advance ONE area meaningfully — migrate the violet-cast variables to the Bedrock & Diamond palette, the cool graphite-blue ground and subtle cool blooms, the steel-cyan focus state, the single warm strike primary-action affordance with its <150ms underline, the done-column diamond facet + gold seal and its one-shot completion glint, the state-spine hue mapping (slate/steel-cyan/red/diamond), eyebrow priority hue, column header level-markers, and the right-aligned status gutter; (3) ensure `tasklord serve` is running, then reload and screenshot the LIVE board via Chrome DevTools at ~1440px and ~420px; (4) critique the rendered pixels against the 14-point acceptance bar (1-8 from GOAL-DESIGN, 9-14 from GOAL-COLOR), INCLUDING a grayscale check that hierarchy survives with hue removed; (5) fix the single weakest point. Hard constraints every pass: keep the Swiss grid (1px hairline gutters, modular columns, tracked mono labels, ~4-6px max radius, dividers not boxes); no emoji; no attribute-pills; no decorative glow; only the strike accent is fully saturated, everything else muted; no pure black ground or pure white ink (avoid halation); exactly one warm strike element active per card; motion is functional only and sub-150ms. Keep cook/continue, dismiss, filters, search, and the live data wiring fully functional throughout. Do NOT declare done from the code — only when all 14 criteria pass on the rendered screenshots, with the dispatch action feeling like sinking a shovel into earth and done feeling like an unearthed diamond. When it passes, stop the loop and send a one-line summary of what changed and why it now meets the bar.
```

## Notes

- Self-paced: design → render → critique → refine, stops on its own at the bar.
- Judges from screenshots, not code — the only honest way to hit a visual bar.
- Layers on top of the existing structural redesign; does not undo `GOAL-DESIGN.md`.
- Must keep every existing behavior working (cook, dismiss, filters, search, live data).
- Cancel anytime; progress commits incrementally to git.
- Palette rationale and citations live in `GOAL-COLOR.md` (color psychology table).
```

# TASK LORD — Redesign Loop

Paste this as a single self-paced `/loop` (no interval — it iterates and stops
itself when the bar is met):

```
/loop Redesign the TASK LORD board at ~/Projects/active/tasklord/board/index.html until it fully satisfies ~/Projects/active/tasklord/GOAL-DESIGN.md — the most effective task-to-agent management surface in the world: sleek, efficient, regal. Each iteration: (1) re-read GOAL-DESIGN.md and the hard rules; (2) advance ONE area meaningfully — the obsidian/metallic palette, the typographic hierarchy and type scale, the leading state-spine, the small-caps middot meta line that replaces all attribute-pills, the unmistakable-but-quiet primary command affordance, the restrained hover dismiss control, column headers, grid/density, and responsive narrow layout; (3) make sure `tasklord serve` is running, then reload and screenshot the LIVE board via Chrome DevTools at ~1440px and ~420px; (4) critique the rendered pixels against the 8-point acceptance bar; (5) fix the single weakest point. Hard constraints every pass: no emoji, no attribute-pills or circles-around-everything, no decorative glow, minimal rounding, zero cognitive load per card, every element must carry information. Keep the cook/continue, dismiss, filters, and DeepSeek data wiring fully functional throughout. Do NOT declare done from the code — only when all 8 criteria pass on the rendered screenshots. When it passes, stop the loop and send a one-line summary of what changed and why it now meets the bar.
```

## Notes

- Self-paced: it loops design → render → critique → refine, then stops on its own.
- It judges from screenshots, not code — the only honest way to hit a visual bar.
- It must keep every existing behavior working (cook, dismiss, filters, live data).
- Cancel anytime; progress is committed incrementally to git.

# TASK LORD — Design North-Star

The goal: the board should feel like **the most effective task-to-agent
management surface in the world** — the tool a serious operator reaches for to
see every thread of work and dispatch an agent at it in one move. Sleek,
efficient, regal. Premium by restraint, not decoration.

This document is the contract. Every design decision is judged against it.

## The feeling

Obsidian calm. Editorial confidence. A command surface, not a dashboard.
Think the precision of Linear, the speed-feel of Superhuman, the quiet authority
of a private terminal — but purpose-built for dispatching agents at work.

When someone opens it, the reaction should be "this is serious software,"
followed immediately by understanding the whole board without effort.

## Hard rules (non-negotiable)

1. **No emoji. Anywhere.** Not in labels, not in buttons, not in toasts.
2. **No pill-soup. No circles around everything.** Kill the rounded-chip-for-
   every-attribute pattern. Metadata is typographic, not bubbled.
3. **No decorative glow / neon for its own sake.** Light is used to direct the
   eye to one thing, never to dazzle. (The prior vaporwave-neon treatment reads
   cheesy at this bar — retire it.)
4. **Zero cognitive load per card.** A card must be fully understood in under
   one second. If the eye has to decode what a shape means, it failed.
5. **Every visual element carries information.** If it's decorative, delete it.
6. **Buttons are unmistakable and quiet.** The primary action is obvious without
   shouting; secondary actions recede. No ambiguous icon-only mystery-meat.

## Design language

**Typography is the interface.** Hierarchy comes from size, weight, and three
ink levels — not from borders, fills, or bubbles. A tight, deliberate type
scale. A refined grotesk for UI; mono for identifiers, paths, and counts.

**Color is signal, never decoration.** A near-black canvas (obsidian/charcoal)
and a single restrained metallic accent (champagne/platinum register — regal,
not loud). State and priority are conveyed by the *calmest possible device*:

- **State** → a thin vertical spine on the card's leading edge, keyed by a muted
  state hue. Not a top bar, not a pill. One quiet stripe that the eye reads as
  position-in-pipeline.
- **Priority** → a typographic marker (e.g. `P1`/`P2` in mono, or a single
  leading rule weight), not a glowing dot.
- **Area / type** → a single quiet "eyebrow" meta line in small caps, middot-
  separated (`SOFTWARE · BUILD · P1`), low-ink. No boxes around each token.

**Affordances over buttons.** The card is the surface. The primary verb
("Continue in a fresh session", "Open") is a clear, low-chrome command — text +
a hairline, revealed or quietly present at the card's foot — phrased as the
consequence, not a generic label. Dismiss is a small, restrained control that
appears on hover, never competing for attention.

**Density with breath.** Aligned to a grid. Confident negative space. Dense
enough to see the whole operation, calm enough to never feel busy. Hairline
dividers, not boxes. Small radii (≈4–6px), never balloon-rounded.

**Motion is functional.** Sub-150ms, easing that feels expensive. Movement
confirms an action or guides attention — never ambient animation.

## Agent-management framing

This is a tool for dispatching *agents* at work, not a sticky-note wall. The
language and the primary action should reflect that: each card is a live thread
you can hand to an agent. The continue/cook action is the heartbeat — make it
the most considered interaction on the screen.

Worth exploring (judge each on the bar, keep only what earns its place):
- A keyboard-first model (j/k to move, enter to dispatch, x to dismiss) and a
  command palette feel.
- Column headers as quiet labels with a count and a single hairline accent.
- Right-aligned, low-ink metadata (idle time, git dirty, health) that reads as
  a status gutter, not tags.
- A focused "now" lane or a way to see what an agent should pick up next.

## Acceptance bar — the "world's best" test

The redesign is done only when ALL are true:
1. A first-time viewer understands any card in <1 second.
2. There is not one purely decorative element on the screen.
3. No emoji; no attribute-pills; no gratuitous glow; minimal rounding.
4. The primary action on a card is unmistakable; secondary actions recede.
5. Typography alone establishes the hierarchy (it still reads with color off).
6. It looks like premium, intentional, expensive software — "regal."
7. Information density is high but the screen feels calm.
8. Nothing on screen creates a moment of "wait, what does that mean?"

## How to verify (every iteration)

Render it for real and look. `tasklord serve` serves `board/index.html` fresh on
each request — edit the file, reload, screenshot via Chrome DevTools at desktop
(≈1440px) and narrow (≈420px) widths. Critique the screenshot against the 8-point
bar above. Fix the single weakest point. Repeat. Do not declare done from the
code — declare it from the rendered pixels meeting every criterion.

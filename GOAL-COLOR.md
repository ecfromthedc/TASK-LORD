# TASK LORD — Color & Component North-Star ("BEDROCK & DIAMOND")

This document layers a **research-grounded colorway, design spec, and component
system** on top of the structural contract in `GOAL-DESIGN.md`. Everything in
`GOAL-DESIGN.md` still holds (Swiss grid, no emoji, no pill-soup, no decorative
glow, typographic hierarchy, functional motion, <1s card legibility). This file
decides the **palette and the components that carry it.**

When the two conflict, structure wins (`GOAL-DESIGN.md`); when they agree, this
file specifies the exact hues and component behavior.

---

## The metaphor (read this first — it governs every color choice)

You are **sticking a shovel into bedrock and unearthing diamonds.** That is what
finishing real work feels like, and the board must feel like it.

- **The ground is bedrock** — cool, deep, heavy graphite-blue. This is the earth
  you dig through. It reads as *focus and weight*, not decoration.
- **The work is the dig** — a calm steel-cyan marks what's actively being worked.
  Blue/cyan is the most documented focus hue: it lowers cortisol and sustains
  concentration. Active work should feel *clear-headed*, not frantic.
- **The action is the strike** — ONE warm amber-gold accent, and only one, marks
  the primary command on a card (dispatch / continue / dig in). A single warm
  point on a cool field is the highest-salience, lowest-cognitive-load way to say
  "this is the one thing to do." It is the shovel going in. It reads as *will,
  agency, getting it done.*
- **The reward is the diamond** — when a task reaches done, it resolves to a
  brilliant **diamond-cyan facet sealed in old-gold.** This is the payoff: the
  unearthed diamond. It must feel *earned and satisfying*, never loud.

Focus (cool bedrock) → Will (the warm strike) → Reward (the diamond). Every
color on screen serves one of those three beats. If a color serves none, delete it.

---

## Color psychology — why these hues (the evidence)

| Hue role | Color | Documented effect | Used for |
|---|---|---|---|
| **Focus** | steel-cyan / blue | Calm, mental clarity, sustained concentration; cool tones lower cortisol (~18%) | Active / in-progress state |
| **Balance / satisfaction** | teal-green | Reduces eye strain over long sessions; reads as contentment & completion | Done facet, calm gutters |
| **Will / decisive energy** | amber-gold | Warm = stimulation & drive; a lone warm accent on a cool field maximizes salience for one target | The single primary action |
| **Reward / value** | diamond-cyan + old-gold | Brilliant cool facet = the "strike"; gold = earned worth | Done seal, completion moment |
| **Urgency / attention** | ink-red | Red heightens attention-to-detail & signals urgency (Elliot et al. 2007) — used sparingly | Blocked / urgent only |

**Dark-mode discipline (non-negotiable, from the research):**
- **One thing glows.** Full saturation is reserved for the primary strike accent.
  Everything else is muted. If every element glows, nothing does.
- **Near-black ground, near-white ink.** No pure `#000` (halation), no pure `#fff`
  text (visual vibration/blur). Cool near-black bedrock, cool aged near-white ink.
- **Contrast is perceived, not just calculated.** Hierarchy comes from ink level
  and spacing first, hue second.

---

## The palette (decided — this is the target the loop converges to)

Replace the violet-cast Midnight Press variables with the **Bedrock & Diamond**
system. Keep the *structure* of the variable scheme; change the values.

```css
:root{
  /* GROUND — cool graphite-blue bedrock (the earth you dig through) */
  --bg:#0A0E14;            /* near-black, cool blue cast — focus, weight */
  --bg-2:#0E131C;
  --surface:#121A25;       /* dug face — card ground */
  --surface-hi:#18222F;    /* hover — freshly turned earth */

  /* INK — cool aged near-white, three levels, no pure white */
  --ink:#EAF0F6;
  --ink-mid:rgba(234,240,246,.62);
  --ink-lo:rgba(234,240,246,.40);
  --ink-faint:rgba(234,240,246,.26);

  /* HAIRLINES — Swiss dividers, never boxes */
  --line:rgba(234,240,246,.10);
  --line-hi:rgba(234,240,246,.22);

  /* THE THREE BEATS */
  --focus:#3E9BC4;         /* steel-cyan — active work / in_progress (concentration) */
  --focus-dim:#2E6E8C;
  --strike:#F0A33C;        /* amber-gold — THE primary action, the shovel-strike (will) */
  --strike-dim:#B97E2E;
  --diamond:#9FE8E0;       /* diamond-cyan facet — the reward glint */
  --diamond-hi:#CFF4F0;
  --gold:#C9A85F;          /* old-gold — earned seal on done */
  --red:#C8301C;           /* ink-red — blocked / urgent only */

  /* STATE SPINE — the mining pipeline, keyed by depth */
  --st-backlog:#5A6B7A;    /* cool slate — unmined ore, dormant */
  --st-prog:#3E9BC4;       /* steel-cyan — actively digging */
  --st-blocked:#C8301C;    /* red — struck rock */
  --st-done:#9FE8E0;       /* diamond-cyan — struck the vein */
  --urgent:#F0A33C;        /* warm — pull-forward urgency (reuses strike, not red) */
}
```

**Background bloom:** at most two extremely subtle cool radial blooms (steel-cyan
top, faint teal bottom) at very low opacity — they suggest depth/ore, never
dazzle. No warm bloom; warmth is rationed to the strike accent alone.

---

## Component spec — how the palette becomes the interface

Swiss grid throughout: 1px hairline gutters, modular columns, mono tracked
labels, tight type scale, ~4–6px max radius, hairline dividers not boxes.

1. **State spine (leading edge, 2px).** The pipeline read at a glance: slate →
   steel-cyan → red → diamond-cyan. The single most information-dense pixel-column
   on each card. Muted except done, which may carry the faint diamond facet.

2. **The strike (primary action) — the most considered element on screen.**
   - Low-chrome at rest (text + hairline, `--ink-mid`), but the verb is phrased as
     the consequence ("Dig in — fresh session", "Continue", "Open").
   - On hover/focus it **becomes the one warm thing on the card**: text resolves to
     `--strike`, a hairline underline in `--strike` draws in from the left (<150ms,
     expensive easing). This is the shovel entering the ground — make it feel
     tactile, deliberate, earned. Exactly one strike-colored element per card.

3. **Done = the diamond moment.** When a card lands in done (or renders in the
   done column): the spine is diamond-cyan, the title carries a small old-gold
   seal mark (typographic, not a badge), and on the *transition* a single
   sub-150ms facet glint sweeps once across the leading edge — the unearthing.
   Never ambient; fires once, on completion only.

4. **Eyebrow meta line.** Small-caps mono, middot-separated, low-ink
   (`SOFTWARE · BUILD · P1`). Priority is the only token that may take hue: `P1`
   in `--strike` (pull-forward), urgent in `--red`. No boxes, no pills.

5. **Column headers.** Quiet mono label + count, single 2px top accent in the
   column's state hue. Reads as a mineshaft level marker, not a tab.

6. **Status gutter (right-aligned, low-ink mono).** Idle time, git-dirty, health
   — a calm instrument readout, never tags. Tabular-nums.

7. **Motion is functional only.** The strike underline, the done facet glint,
   card enter/settle. Sub-150ms, expensive easing. Zero ambient animation.

---

## Acceptance bar — the "shovel & diamond" test (ALL must pass on rendered pixels)

Inherits all 8 points from `GOAL-DESIGN.md`, plus:

9.  **The ground reads as cool, deep, focused bedrock** — heavy and serious, not
    playful; not a single warm pixel in the field except the strike.
10. **There is exactly one warm strike accent active per card**, and it is
    unmistakably *the* action. Nothing else competes for that warmth.
11. **Active work reads as calm focus** (steel-cyan), never frantic; the palette
    still establishes state with hue removed (test grayscale).
12. **Reaching done feels like a reward** — the diamond-cyan + gold facet lands as
    earned satisfaction, fires once, and never becomes ambient decoration.
13. **Saturation discipline holds**: only the strike is fully saturated; ground,
    ink, and all other states are muted; no halation, no neon vibration, no pure
    black/white.
14. **The whole thing feels like big, important work** — the reaction is "this is
    where serious things get finished," and the act of dispatching a card feels
    like sinking a shovel into the earth.

## How to verify (every iteration)

`tasklord serve` serves `board/index.html` fresh. Edit → reload → screenshot via
Chrome DevTools at ~1440px and ~420px. Critique the rendered pixels against
points 1–14. Fix the single weakest point. Declare done only from the pixels,
never from the code.

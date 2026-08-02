# Mister Bombastik — simplified gameplay patch

This patch targets the current structure of:

`firancly/keep-grinding-and-nobody-dies`

## What changes

### The Button

Before, the answer depended on button color, label, battery count, CAR, FRK,
strip color, and timer digits.

Now the screen gives only:

- the physical button number
- `TAP` or `HOLD`

Rules:

- `TAP`: press and release quickly
- `HOLD`: hold until `READY — RELEASE NOW`, then release

### Color Memory

The old Simon rules depended on serial vowels, strike count, a translation
table, and a growing sequence.

Now:

- exactly three colors flash
- repeat the same three colors directly
- RED = Button 1
- BLUE = Button 2
- GREEN = Button 3
- YELLOW = Button 4

### Colored wires

The physical order does not change:

- BLUE = old Wire 1 = GPIO25
- WHITE = old Wire 2 = GPIO13
- BLACK = old Wire 3 = GPIO16
- YELLOW = old Wire 4 = GPIO17

The Alien Wires manual now uses color names instead of numbers.

## Apply

Extract this folder into the repository root, then run:

```bash
python apply_simplified_games.py
pnpm tauri dev
```

The script makes one-time `.before-simplify` backups beside every file it
changes.

## Files changed

- `src-tauri/src/engine/button_module.rs`
- `src-tauri/src/engine/simon.rs`
- `src-tauri/src/engine/mod.rs`
- `src-tauri/src/view.rs`
- `src-tauri/src/tablet.html`
- `src/expert/Manual.tsx`
- `src/types.ts`
- comments in `keep_grinding_ful_game.ino`

The numeric Memory module is unchanged. Here, “memory color game” is treated as
the Simon/Color Memory module.

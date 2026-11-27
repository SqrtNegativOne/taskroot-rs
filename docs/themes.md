# Colour schemes

Choose **Settings → Appearance → Colour scheme**:

- **Slate** (default): dark blue-grey surfaces, cool blue accents, cool grey text.
- **Ember**: warm charcoal and gold palette.

Palettes are bundled offline CSS stylesheets overriding shared custom properties
(no external stylesheet loader or plugin system). The saved `colour_scheme`
setting is global; `src/lib/themes/Theme.svelte` applies it as `data-theme` on
`<html>` (defaulting to Slate during bootstrap) and updates all open windows
via `STORE_UPDATED` without a restart.

## Files and tokens

`src/lib/themes/index.css` loads these files through `src/app.css`:

| File | Responsibility |
|---|---|
| `slate.css` | Default dark blue-grey/cool-blue base palette (also supplies `:root` loading fallback) |
| `ember.css` | Warm base palette |
| `tokens.css` | Shared semantic aliases and `color-mix()` derived tints, fills, glass and shadows |

Palette files declare colours only; layout, spacing and fonts stay in component
styles. Components consume semantic tokens rather than literal colours.

| Tokens | Role |
|---|---|
| `--bg`, `--bg-highlight`, `--bg-surface`, `--bg-surface-hover`, `--bg-active` | Background and interaction layers |
| `--border`, `--border-soft`, `--border-strong` | Dividers and outlines |
| `--fg`, `--fg-muted`, `--fg-dim`, `--fg-dimmer`, `--fg-dimmest` | Text hierarchy (faintest levels are decorative) |
| `--accent` | Selected states and primary actions |
| `--tag-red`, `--tag-yellow`, `--tag-green`, `--tag-purple`, `--tag-gold` | Semantic highlights |
| `--danger` | Destructive controls |
| `--priority-0` … `--priority-4` | Task priority colours across lists, circles and calendars |
| `--event-style` | Calendar event fill strategy: `tint` (default) or `solid` |

Today badges, timeline headers, current-time lines and task-start indicators use
the shared red `--today`, `--today-bg`, `--on-today` and `--today-glow` tokens
in `tokens.css`, not `--accent`, so they stay red across palettes. Theme-specific
exceptions can override any semantic token inside `:root[data-theme='…']`.

## Add a palette

1. Copy a palette to `src/lib/themes/<id>.css` using `:root[data-theme='<id>']`
   and define every base token above (only Slate defines the unqualified `:root`
   fallback).
2. Import it from `src/lib/themes/index.css` before `tokens.css`.
3. Add the variant to `ColourScheme` in `src-tauri/src/settings/colour_scheme.rs`
   and its option to `#[setting(options = …)]` in `src-tauri/src/settings/mod.rs`.
4. Run backend checks to validate and regenerate bindings, run frontend checks,
   and extend `tests/e2e/themes.spec.ts`.
5. Update the list at the top of this doc and inspect text, selected states and
   floating windows manually (Playwright checks representative contrast and CSS
   switching, not full visual design).

## Server-owned calendar colours

Calendar collection colours and event `COLOR` overrides are server data, never
modified by palette changes. At render time, `src/lib/eventColor.ts` (recomputed
on theme changes by `src/lib/themes/eventTheme.svelte.ts`) parses `#RRGGBB` /
`#RRGGBBAA` (dropping alpha, falling back to a default colour if malformed) and
derives fill, text and accent bar:

- `--event-style: tint` (default): faint tinted fill with theme text roles.
- `--event-style: solid`: solid calendar colour fill with contrast-picked black
  or white text.
- `--on-event`: fallback text colour for events without a stored colour.

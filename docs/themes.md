# Colour schemes

Choose **Settings → Appearance → Colour scheme**:

- **Slate** (default): dark blue-grey surfaces, cool blue accents and cool grey text.
- **Ember**: the original warm charcoal/gold palette.

The choice is the typed `colour_scheme` field in `settings.json`, independent of
CalDAV accounts. Saving uses the existing `update_setting` command and
`STORE_UPDATED` event; each open window refreshes its settings. The shared layout
mounts `src/lib/themes/Theme.svelte`, which applies the choice as `data-theme` on
`<html>`. Newly opened windows read the same saved choice. No restart is needed
when switching the bundled palettes from Settings.

If a saved `settings.json` still has `"colour_scheme": "midnight"`, change that
field by hand to `"slate"` before starting the updated app to retain its other
settings. There is no migration; unsupported values trigger the normal whole-file
default/reset behaviour.

## The Obsidian model, kept small

Obsidian themes are CSS stylesheets, commonly distributed with a `manifest.json`
and `theme.css`. They override shared CSS custom properties and sometimes
component selectors. CSS snippets provide additional overrides; plugins are not
required for basic colour changes.

Taskroot uses the same **shared variables + CSS palette** idea, not Obsidian's
installer, manifest format or plugin system. Its palettes are bundled offline
with the app. There is currently no external stylesheet picker, theme marketplace
or user-data-directory CSS loader.

## Where to edit colours

`src/lib/themes/index.css` loads these files through `src/app.css`:

| File | Responsibility |
|---|---|
| `slate.css` | Default dark blue-grey/cool-blue base palette |
| `ember.css` | Warm base palette |
| `tokens.css` | Shared semantic aliases and derived tints, fills, glass and shadows |

Palette files contain colour declarations only. Layout, spacing and fonts stay
in app/component styles. Components consume semantic tokens instead of literal
palette colours, so a palette change also reaches priority markers, stopwatch
feedback, dialogs and floating windows.

Useful base tokens:

| Tokens | Role |
|---|---|
| `--bg`, `--bg-highlight`, `--bg-surface`, `--bg-surface-hover`, `--bg-active` | Background and interaction layers |
| `--border`, `--border-soft`, `--border-strong` | Dividers and outlines |
| `--fg`, `--fg-muted`, `--fg-dim`, `--fg-dimmer`, `--fg-dimmest` | Text hierarchy; the faintest levels are decorative |
| `--accent` | Selected states and primary actions |
| `--tag-red`, `--tag-yellow`, `--tag-green`, `--tag-purple`, `--tag-gold` | Semantic highlights |
| `--danger` | Destructive controls |
| `--priority-0` … `--priority-4` | Task priority colours, shared by lists, circles and calendars |

Today's date badges, timeline headers, current-time line/labels and task-start
indicators use the shared red `--today`, `--today-bg`, `--on-today` and
`--today-glow` tokens in `tokens.css`, not the palette accent. Their colours stay
red when switching themes.

`tokens.css` derives translucent accent states with `color-mix()`, so changing
`--accent` also changes its glow, line and hover tints. Priority fills and floating
backgrounds are derived similarly. Theme-specific exceptions can override a
semantic token in the palette's more-specific `:root[data-theme='…']` selector.

For example, to adjust Slate's blue, edit `--accent` in `slate.css`:

```css
--accent: #8ab4f8;
```

Source CSS edits are picked up during Vite development; a packaged app must be
rebuilt to include edited palette files. Settings only switches bundled palettes.

## Add a palette

1. Copy a palette to `src/lib/themes/<id>.css`, change its selector to
   `:root[data-theme='<id>']`, and define all the base tokens above. Only Slate
   supplies the unqualified `:root` fallback for the pre-settings loading frame.
2. Import the new file from `src/lib/themes/index.css` before `tokens.css`.
3. Add its variant to `ColourScheme` in
   `src-tauri/src/settings/colour_scheme.rs` and its ID/label to the field's
   `#[setting(options = …)]` in `src-tauri/src/settings/mod.rs`.
4. Run the backend checks to validate persistence and regenerate bindings; never
   edit `src/lib/bindings/` by hand. Run frontend checks and extend the browser
   palette fixtures in `tests/e2e/themes.spec.ts` for the new scheme.
5. Update the choices listed here and manually inspect text, selected states and
   floating windows. The browser tests check representative contrast and CSS
   switching, not the complete visual design.

Calendar collection colours and event `COLOR` overrides remain server-owned data,
not theme colours. Switching palettes does not modify them. `--on-event` controls
text over those arbitrary backgrounds; it preserves the existing white-text
behaviour rather than guessing a new colour for the server resource.

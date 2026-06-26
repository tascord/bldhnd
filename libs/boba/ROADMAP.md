# Boba MVP Roadmap

## Where we are

Boba is an event-driven async Rust TUI library built on top of `ratatui`, `crossterm`, and `tokio`. It already has:

- An `App` / `View` runtime loop with raw-mode + alternate screen + mouse capture
- An `EventTarget<T>` pub/sub system (with priorities, cancellation, bubbling, and mpsc streams)
- A centralized `Animator` with `BobaValue<T>` for animated properties
- A `Theme` system with presets (default, light, ocean, solarized, high contrast)
- A broad widget catalog (~30 components): inputs, buttons, lists, tabs, modals, spinners, progress, etc.
- Some helpers (`BobStyle`, `BobBlock`, `gradient_text`, border presets)

What works today: the `gallery` example runs and shows a functional dashboard of widgets.

## What we are aiming for

The goal is to reach an MVP that can replicate the "niceness" and layout expressiveness of the `lipgloss` library. The canonical target is the `charmbracelet/lipgloss` `examples/layout/main.go` demo.

That example is **not** a widget demo. It is a layout demo. It does not say "here is a button component"; it says:

- "Here is a styled string with margin, padding, and a custom border around it"
- "Here is a tab bar built from styled strings joined horizontally"
- "Here is a floating dialog box absolutely positioned at (x, y)"
- "Here is a status bar joined from colored segments"
- "Here is a gradient applied per-character to a string"
- "Here is a 2D color blended grid"

The primitives are **style-as-layout** and **string-compositing**, not **widgets**.

## What is missing

### 1. Style engine: `BobaStyle` must become a layout primitive

`BobStyle` is just a newtype over `ratatui::style::Style`. It can set colors, bold, italic. It cannot:
- Add margin or padding to content
- Add a width/height constraint
- Add inline dimensions (e.g. "this text is exactly 30 cols wide, center aligned")
- Return something you can join with other things

We need `BobaStyle` (renamed from `BobStyle`) to carry layout parameters and produce a renderable `Surface`.

### 2. Custom borders

`BobBlock` is a newtype over `ratatui::widgets::Block`. It can do Plain, Rounded, Double, Thick. It cannot:
- let the user specify arbitrary border runes (e.g. tabs with `┘` bottom-left corners)
- apply a border to any arbitrary "styled string"

We need a `Border` struct with top/bottom/left/right + 4 corner chars, and a way to apply it to any `Surface`.

### 3. Layout primitives

`boba` currently has `ratatui::Layout` for splitting `Rect`s. The `lipgloss` example uses:
- `JoinHorizontal(align, ...)` — stitch styled blocks left-to-right
- `JoinVertical(align, ...)` — stitch styled blocks top-to-bottom
- `Place(width, height, h_align, v_align, content, opts)` — center something in a box
- `Width(s)` / `Height(s)` — measure rendered strings

We need these as first-class functions that work on `Surface`s.

### 4. Compositing with offsets

The example ends with a `Compositor` / `Layer` system that lets a widget say "render at X=58, Y=44". `boba`'s `LayerStack` draws full-screen layers in order but does not support arbitrary offset blitting. We need each `Layer` to carry an `(x, y)` offset and blit onto the frame buffer.

### 5. Color utilities

The example needs:
- `hex_color("#EDFF82")` helper
- `Blend1D(n, from, to)` — linear color interpolation
- `Blend2D` / `ColorGrid` — 2D gradient for the color grid demo

### 6. Background detection

`lipgloss.HasDarkBackground` detects terminal light/dark mode. We'll add the function with a mocked default (assumes dark) so the adaptive palette logic works, and leave real OSC querying for later.

## Execution order

1. **Rename** `Bob*` -> `Boba*` everywhere (consistency).
2. **Add color utilities** (`hex`, `blend_1d`, `blend_2d`). Background detection mock.
3. **Build the core `Surface` and `BobaStyle` v2**: a `Surface` is a `Vec<Vec<Cell>>` with width/height/span. `BobaStyle` applies padding, margin, border, width/height, alignment to text and returns a `Surface`.
4. **Build `Border` and join/place primitives**: `join_horizontal`, `join_vertical`, `place_with`, `width`/`height` on surfaces.
5. **Update `Layer`/`Compositor`**: add `x`/`y` offsets to `Layer` so `Compositor` can blit at absolute positions into the frame buffer.
6. **Port the example**: re-implement the lipgloss layout demo as a `boba` example app, using the new primitives.
7. **Verify**: `cargo check` and `cargo run --example layout` must render without errors.

## After MVP

- Real background detection via OSC 10/11 queries
- String-level ANSI measurement utilities
- Additional `lipgloss` features omitted from MVP (lists with bullet separators, tree rendering, etc.)

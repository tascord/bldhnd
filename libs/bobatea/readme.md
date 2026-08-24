<img width="200" height="200" align="left" style="float: left; margin: 0 10px 0 0;" alt="Icon" src="https://raw.githubusercontent.com/tascord/boba/refs/heads/main/icon.png"> 

# Boba(tea)
### Terminal layout + styling you won't hate.

[![GitHub top language](https://img.shields.io/github/languages/top/tascord/boba?color=0072CE&style=for-the-badge)](#)
[![Crates.io Version](https://img.shields.io/crates/v/boba?style=for-the-badge)](https://crates.io/crates/bobatea)
[![docs.rs](https://img.shields.io/docsrs/boba?style=for-the-badge)](https://docs.rs/bobatea)

<br><br>

## Overview

`bobatea` is a TUI layout and styling library for Rust, inspired by [Lip Gloss](https://github.com/charmbracelet/lipgloss). It provides:

- **Surface-based rendering**: Compose styled text and shapes into an in-memory framebuffer
- **Layout primitives**: Padding, margins, borders, alignment — all with a fluent builder API
- **Component system**: Interactive components with keyboard/mouse event handling
- **Compositing**: Layer multiple surfaces with precise positioning

## Installation

```bash
cargo add bobatea
```

## Quick Start

```rust
use bobatea::{
    App, View, theme::Theme,
    components::style::BobaStyle,
    surface::{Cell, join_vertical, Position},
};

struct MyView;

impl View for MyView {
    fn render(&self, ctx: &mut ratatui::Frame<'_>, theme: &Theme) {
        let text = BobaStyle::new()
            .fg(hex_color("#FF5F87"))
            .bg(hex_color("#1A1A2E"))
            .render("Hello, Boba!");
        
        let surface = join_vertical(Position::Center, &[text]);
        // render surface to frame...
    }
}
```

## Core Concepts

### Surface

A `Surface` is an in-memory grid of styled cells — like a terminal framebuffer. Create one with `Surface::new()`, render text into it with `BobaStyle::render()`, and compose multiple surfaces with `join_horizontal()` / `join_vertical()`.

### BobaStyle

The `BobaStyle` struct provides a Lip Gloss-like fluent API:

```rust
let styled = BobaStyle::new()
    .fg(Color::Cyan)
    .bg(Color::Black)
    .padding(1, 2, 1, 2)   // top, right, bottom, left
    .margin(0, 1, 0, 1)
    .border(Border::rounded())
    .align(Alignment::Center, Position::Center)
    .render("Styled text");
```

### Components

Components implement the `Component` trait with optional event handling:

```rust
use bobatea::components::{Button, button::ButtonVariant};

let btn = Button::new("Click me")
    .variant(ButtonVariant::Primary)
    .on_click(|| println!("Clicked!"));
```

Available components:
- **Button** — clickable buttons with variants
- **Input** / **TextArea** — text input fields
- **Tabs** — tabbed navigation
- **List** / **Tree** — hierarchical data display
- **Table** — tabular data
- **Modal** / **Dialog** — overlay dialogs
- **Paginator** — page navigation
- **FilePicker** — file/folder selection
- **Viewport** — scrollable view wrapper

### Compositor

The `Compositor` layers multiple surfaces with precise x/y positioning:

```rust
use bobatea::components::layer::{Compositor, CompositorLayer};

let comp = Compositor::new(vec![
    CompositorLayer::new(background).x(0).y(0),
    CompositorLayer::new(foreground).x(10).y(10),
]);
comp.render_to_buf(area, buf);
```

## Examples

Run the layout demo (a Lip Gloss showcase port):

```bash
cargo run --example layout
```

Other examples:
- `hello_layout` — basic style and surface usage
- `hello_tabs` — tabbed navigation
- `hello_table` — table rendering

## Features

- **Fluent API** — chainable style builders
- **Flexible layout** — padding, margins, borders, alignment
- **Wide character support** — emoji and CJK handled correctly
- **Mouse + keyboard events** — per-component event handling
- **Theme support** — light/dark palette switching
- **Gradient text** — character-by-character color gradients

## License

MIT
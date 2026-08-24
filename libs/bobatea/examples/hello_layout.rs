//! Minimal example showing how to use `BobaStyle` + `Surface` layout.
//!
//! Run with: `cargo run --example hello_layout`

use {
    bobatea::{
        App, AppEvent, View,
        components::{
            border::Border,
            style::{BobaStyle, hex_color},
        },
        events::{EventTarget, SubscriptionPriority},
        surface::{Cell, Position, join_horizontal, join_vertical, place},
        theme::Theme,
    },
    crossterm::event::KeyCode,
    ratatui::{
        Frame,
        style::{Color, Style},
    },
};

struct HelloView;

impl View for HelloView {
    fn mount(&self, app: &EventTarget<AppEvent>) {
        let app_for_quit = app.clone();
        app.on_key(SubscriptionPriority::High, move |ev, key| {
            if key.code == KeyCode::Char('q') {
                ev.cancel();
                app_for_quit.emit(AppEvent::Quit);
            }
        })
        .forget();
    }

    fn render(&self, ctx: &mut Frame<'_>, _theme: &Theme) {
        let area = ctx.area();
        let buf = ctx.buffer_mut();

        // Clear background
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                buf[(x, y)].reset();
                buf[(x, y)].set_bg(Color::Black);
            }
        }

        // Build some styled text surfaces
        let hello = BobaStyle::new().fg(hex_color("#FF5F87")).bg(Color::Black).bold().render("Hello");

        let world = BobaStyle::new().fg(hex_color("#A550DF")).bg(Color::Black).italic().render("World");

        // Join them horizontally with a divider
        let divider = BobaStyle::new().fg(Color::DarkGray).render("│");
        let joined = join_horizontal(Position::Center, &[hello, divider, world]);

        // Put a rounded border around it
        let boxed = BobaStyle::new()
            .border(Border::rounded())
            .border_fg(hex_color("#874BFD"))
            .padding(1, 2, 1, 2)
            .render_surface(&joined);

        // Add a small label below
        let label = BobaStyle::new().fg(Color::DarkGray).margin_top(1).render("press 'q' to quit");

        let doc = join_vertical(Position::Center, &[boxed, label]);

        // Center the whole thing on screen
        let centered = place(
            area.width as usize,
            area.height as usize,
            Position::Center,
            Position::Center,
            &doc,
            &Cell::blank(Style::default().bg(Color::Black)),
        );

        // Render into the frame buffer
        centered.render_to_area(area, buf);
    }
}

fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async {
        App::new(HelloView).run().await.unwrap();
    });
}

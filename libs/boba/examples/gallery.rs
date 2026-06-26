//! Demo app showcasing the boba TUI component library.
//!
//! Run with: `cargo run --example gallery`

use {
    boba::{
        App, AppEvent, View,
        components::{
            asciiimg::{AsciiImage, Pixel},
            bigtext::BigText,
            button::Button,
            input::Input,
            list::List,
            modal::{DialogButtons, Modal},
            powerline::{CommandPalette, Powerline, Segment},
            progress::Progress,
            spinner::Spinner,
            style::{BobaStyle, gradient_text, hsl},
            tabs::Tabs,
            toast::Toaster,
        },
        events::{EventTarget, SubscriptionPriority},
        theme::Theme,
    },
    crossterm::event::KeyCode,
    futures_signals::signal::Mutable,
    ratatui::{
        Frame,
        layout::{Constraint, Layout, Rect},
        prelude::Buffer,
        style::Color,
        text::{Line, Span},
        widgets::{Block, Borders, Paragraph, Widget},
    },
};

fn clear_bg(area: Rect, buf: &mut Buffer, bg: Color) {
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            buf[(x, y)].reset();
            buf[(x, y)].set_bg(bg);
        }
    }
}

struct DemoView {
    focus_idx: Mutable<usize>,
    counter: Mutable<u32>,
    toaster: Toaster,
    status: Mutable<String>,
    show_modal: Mutable<bool>,
    palette: CommandPalette,
    input: Input,
    list: List,
    button: Button,
    tabs: Tabs,
    modal: Modal,
    theme_idx: Mutable<usize>,
    pulse: Mutable<f64>,
    pulsing: Mutable<bool>,
}

impl DemoView {
    fn new() -> Self {
        let list = List::new([
            "Reactive labels",
            "Gradient fills",
            "Screen effects",
            "Layer compositor",
            "Animations",
            "Big text",
            "ASCII images",
            "Command palette",
            "Modal dialogs",
        ]);
        list.focus();

        Self {
            focus_idx: Mutable::new(1),
            counter: Mutable::new(0),
            toaster: Toaster::new(4),
            status: Mutable::new("Tab: cycle  |  Arrows: nav  |  Space: act  |  t: theme  |  q: quit".to_string()),
            show_modal: Mutable::new(false),
            palette: CommandPalette::new(["Quit", "Open Modal", "Refresh", "Toggle Tab", "Show Help"]),
            input: Input::new("Username").placeholder("type here"),
            list,
            button: Button::new("Click me!"),
            tabs: Tabs::new(["Widgets", "Effects", "Forms", "Tables"]),
            modal: Modal::new("Confirm", "Are you sure you want to continue?")
                .with_buttons(DialogButtons::YesNo)
                .size(40, 10),
            theme_idx: Mutable::new(0),
            pulse: Mutable::new(0.0),
            pulsing: Mutable::new(false),
        }
    }

    fn blur_all(&self) {
        self.input.blur();
        self.list.blur();
        self.button.blur();
    }

    fn set_focus(&self, idx: usize) {
        self.blur_all();
        self.focus_idx.set(idx);
        match idx {
            0 => self.input.focus(),
            1 => self.list.focus(),
            2 => self.button.focus(),
            _ => {}
        }
    }
}

impl View for DemoView {
    fn mount(&self, app: &EventTarget<AppEvent>) {
        let app_for_callback = app.clone();
        let counter = self.counter.clone();
        let toaster = self.toaster.clone();
        let show_modal = self.show_modal.clone();
        let palette = self.palette.clone();
        let input = self.input.clone();
        let list = self.list.clone();
        let button = self.button.clone();
        let _tabs = self.tabs.clone();
        let modal = self.modal.clone();
        let theme_idx = self.theme_idx.clone();
        let pulse = self.pulse.clone();
        let pulsing = self.pulsing.clone();

        app.on_key(SubscriptionPriority::High, move |ev, key| {
            if palette.is_visible() {
                let _ = palette.on_key(key.code);
                ev.cancel();
                return;
            }

            if show_modal.get() {
                if modal.on_key(key.code).is_some() {
                    show_modal.set(false);
                }
                ev.cancel();
                return;
            }

            // ── global shortcuts ──
            match key.code {
                KeyCode::Char('q') => {
                    ev.cancel();
                    app_for_callback.emit(AppEvent::Quit);
                    return;
                }
                KeyCode::Char('m') => {
                    show_modal.set(true);
                    modal.show();
                    ev.cancel();
                    return;
                }
                KeyCode::Char('t') => {
                    let next = (theme_idx.get() + 1) % 5;
                    theme_idx.set(next);
                    app_for_callback.emit(AppEvent::SetTheme(next));
                    ev.cancel();
                    return;
                }
                KeyCode::Char('a') => {
                    pulse.set(0.0);
                    pulsing.set(true);
                    ev.cancel();
                    return;
                }
                KeyCode::Char('p') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                    palette.show();
                    ev.cancel();
                    return;
                }
                _ => {}
            }

            // ── focus cycling ──
            match key.code {
                KeyCode::Tab => {
                    let next = list.is_focused() as u8 * 0 + input.is_focused() as u8 * 1 + button.is_focused() as u8 * 2;
                    let next = (next + 1) % 3;
                    input.blur();
                    list.blur();
                    button.blur();
                    match next {
                        0 => input.focus(),
                        1 => list.focus(),
                        2 => button.focus(),
                        _ => {}
                    }
                    ev.cancel();
                    return;
                }
                KeyCode::BackTab => {
                    // simplified—just cycle forward for now
                }
                _ => {}
            }

            // ── dispatch to focused widget ──
            let f0 = input.is_focused();
            let f1 = list.is_focused();
            let f2 = button.is_focused();

            if f0 {
                input.on_key(key.code);
                ev.cancel();
            } else if f1 {
                list.on_key(key.code);
                ev.cancel();
            } else if f2 {
                button.on_key(key.code);
                if key.code == KeyCode::Enter || key.code == KeyCode::Char(' ') {
                    let c = counter.get() + 1;
                    counter.set(c);
                    toaster.info(format!("Click count: {}", c));
                }
                ev.cancel();
            }
        })
        .forget();
    }

    fn render(&self, ctx: &mut Frame<'_>, theme: &Theme) {
        let area = ctx.area();
        clear_bg(area, ctx.buffer_mut(), theme.global_bg);

        // ── Root layout: header / body / footer ──
        let root = Layout::vertical([Constraint::Length(1), Constraint::Fill(1), Constraint::Length(3)]).split(area);

        let header = root[0];
        let body = root[1];
        let footer = root[2];

        // ── Header ──
        let powerline = Powerline::new(vec![
            Segment::text(" BOBA ").fg(Color::White).bg(Color::Blue),
            Segment::arrow().fg(Color::Blue).bg(Color::Cyan),
            Segment::text(" gallery ").fg(Color::Black).bg(Color::Cyan),
            Segment::arrow().fg(Color::Cyan).bg(Color::Black),
        ]);
        powerline.render_to_buf(header, ctx.buffer_mut(), theme);

        // ── Body ──
        let chunks = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).spacing(1).split(body);
        let left = chunks[0];
        let right = chunks[1];

        // Left panel — stacked with 1-row gaps (space-y-1)
        let left_layout = Layout::vertical([
            Constraint::Length(7), // big text
            Constraint::Length(1), // tabs
            Constraint::Length(3), // input
            Constraint::Length(1), // spacer
            Constraint::Length(3), // button
            Constraint::Length(1), // spacer
            Constraint::Length(3), // progress
            Constraint::Length(1), // spinner
        ])
        .spacing(1)
        .split(left);

        // Right panel
        let right_layout = Layout::vertical([
            Constraint::Length(1), // label
            Constraint::Fill(1),   // list
            Constraint::Length(8), // ascii image
        ])
        .spacing(1)
        .split(right);

        // ── Left renders ──
        BigText::new("BOBA").color(hsl(180.0, 0.8, 0.6)).render_to_buf(left_layout[0], ctx.buffer_mut(), theme);

        self.tabs.render_to_buf(left_layout[1], ctx.buffer_mut(), theme);

        self.input.render_to_buf(left_layout[2], ctx.buffer_mut(), theme);

        self.button.render_to_buf(left_layout[4], ctx.buffer_mut(), theme);

        let progress = Progress::new().label("Loading...");
        progress.set(0.65);
        progress.render_to_buf(left_layout[6], ctx.buffer_mut(), theme);

        Spinner::dots().with_label("working").render_to_buf(left_layout[7], ctx.buffer_mut(), theme);

        // ── Right renders ──
        let label_colors = gradient_text("Features", theme.palette.accent.to_rgb(), theme.palette.primary.to_rgb());
        let label_spans: Vec<Span> = label_colors
            .into_iter()
            .map(|(ch, color)| {
                Span::styled(
                    ch.to_string(),
                    ratatui::style::Style::default().fg(color).add_modifier(ratatui::style::Modifier::BOLD),
                )
            })
            .collect();
        Paragraph::new(Line::from(label_spans)).render(right_layout[0], ctx.buffer_mut());

        self.list.render_to_buf(right_layout[1], ctx.buffer_mut(), theme);

        let mut img = AsciiImage::new(8, 4).scale(2, 1);
        for y in 0..4 {
            for x in 0..8 {
                let hue = ((x + y * 8) as f64 * 45.0) % 360.0;
                img.pixel_raw(x, y, Pixel::new('█', hsl(hue, 0.8, 0.5)));
            }
        }
        img.render_to_buf(right_layout[2], ctx.buffer_mut(), theme);

        // ── Overlays ──
        self.toaster.render_to_buf(area, ctx.buffer_mut(), theme);

        // ── Footer ──
        let status_text = self.status.get_cloned();
        let muted = ratatui::style::Style::default().fg(theme.palette.fg_muted.to_rgb());
        let accent = ratatui::style::Style::default().fg(theme.palette.accent.to_rgb());
        let info = ratatui::style::Style::default().fg(theme.palette.info.to_rgb());
        let footer_block =
            Block::default().borders(Borders::TOP).border_style(ratatui::style::Style::default().fg(theme.border_subtle));
        let text = Line::from(vec![
            Span::styled(" status: ", muted),
            Span::styled(status_text, accent),
            Span::styled(" | ", muted),
            Span::styled("Tab", info),
            Span::styled(" focus ", muted),
            Span::styled("Arrows", info),
            Span::styled(" nav ", muted),
            Span::styled("Space", info),
            Span::styled(" click ", muted),
            Span::styled("t", info),
            Span::styled(" theme ", muted),
            Span::styled("q", info),
            Span::styled(" quit ", muted),
        ]);
        Paragraph::new(text).block(footer_block).render(footer, ctx.buffer_mut());

        // ── Modal ──
        if self.show_modal.get() {
            self.modal.show();
        }
        self.modal.render_to_buf(area, ctx.buffer_mut(), theme);

        // ── Command palette ──
        self.palette.render_to_buf(area, ctx.buffer_mut(), theme);
    }
}

fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async {
        App::new(DemoView::new()).run().await.unwrap();
    });
}

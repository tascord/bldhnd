use {
    crate::{
        data::SearchResult,
        events::{SubscriptionHandle, SubscriptionPriority},
        ui::{
            components::{
                Focusable,
                scroll::{ScrollItem, Scroller},
            },
            views::{ModelEvent, home::BANNER_FONT, model},
        },
    },
    crossterm::event::KeyCode,
    futures_signals::signal::Mutable,
    ratatui::{
        layout::Constraint,
        prelude::*,
        widgets::{Block, Paragraph, WidgetRef},
    },
    std::sync::Arc,
};

pub struct ResultsView {
    banner: Vec<String>,
    scroller: Arc<Scroller>,
    _subs: SubscriptionHandle<ModelEvent>,
}

pub struct SearchItem(SearchResult, Mutable<bool>);
impl SearchItem {
    pub fn new(s: SearchResult) -> Self { Self(s, Mutable::new(false)) }
}

fn res_line(area: Rect, buf: &mut Buffer, ofs: u16, s: Style, l: (&str, &str)) {
    let l = (l.0.trim(), l.1.trim());

    let w = Layout::new(Direction::Horizontal, [
        Constraint::Length(l.0.len() as u16),
        Constraint::Fill(1),
        Constraint::Length(l.1.len() as u16),
    ])
    .split(area)[1]
        .width;

    Line::from_iter([Span::styled(l.0, s), Span::raw(" ".repeat(w as usize)), Span::styled(l.1, s)])
        .render(Rect { x: area.x, y: area.top() + ofs, width: area.width, height: area.height }, buf);
}

impl WidgetRef for SearchItem {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        let focused = self.1.get();
        let s = match focused {
            true => Style::new().on_white().black(),
            false => Style::new(),
        };

        Block::new().style(s).render(area, buf);
        res_line(area, buf, 0, s, (&self.0.name, &self.0.rel_fmt()));
        res_line(area, buf, 1, s, (&self.0.ty_fmt(), &self.0.gb_fmt()));
    }
}

impl Focusable for SearchItem {
    fn focus(&self) { self.1.set(true); }

    fn blur(&self) { self.1.set(false); }
}

impl ScrollItem for SearchItem {
    fn height(&self) -> u16 { 2 }

    fn width(&self) -> u16 { 0 }
}

#[allow(clippy::new_without_default)]
impl ResultsView {
    pub fn new(items: Vec<SearchResult>, esc: impl Fn() + Send + Sync + 'static) -> Self {
        let flet = figlet_rs::FIGlet::from_content(BANNER_FONT).unwrap();
        let text = flet.convert("results").unwrap().to_string();

        let sub = model().target.on(SubscriptionPriority::Low, move |ev| match **ev {
            ModelEvent::KeyPress(key_event) if key_event.code == KeyCode::Esc => {
                esc();
            }
            _ => {}
        });

        let items = items.into_iter().map(SearchItem::new);
        let this = Self {
            banner: text.lines().map(|l| l.to_string()).collect::<Vec<_>>(),
            scroller: Scroller::new().items(items).into(),
            _subs: sub,
        };

        this.scroller.focus();
        this
    }
}

impl WidgetRef for ResultsView {
    fn render_ref(&self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let text = Text::from_iter(self.banner.iter().map(|l| Line::from(Span::raw(l.clone()))));

        let layout =
            Layout::vertical([Constraint::Length(text.height() as u16), Constraint::Length(3), Constraint::Fill(1)])
                .split(area);

        Paragraph::new(text).render(layout[0], buf);

        Paragraph::new(Text::from_iter([
            Line::raw(""),
            Line::styled(
                std::iter::repeat_n(' ', layout[1].width as usize).collect::<String>(),
                Style::new().add_modifier(Modifier::CROSSED_OUT),
            ),
            Line::raw(""),
        ]))
        .render(layout[1], buf);

        self.scroller.render_ref(layout[2], buf);
    }
}
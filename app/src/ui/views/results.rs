use std::{fmt::Display, sync::{Arc, atomic::{AtomicBool, Ordering::SeqCst}}};

use crossterm::event::KeyCode;

use crate::{data::SearchResult, events::{SubscriptionHandle, SubscriptionPriority}, ui::{components::{Focusable, scroll::{ScrollItem, Scroller}}, views::{ModelEvent, model}}};

use {
    crate::ui::views::home::BANNER_FONT,
    ratatui::{
        layout::Constraint,
        prelude::*,
        widgets::{Paragraph, WidgetRef},
    },
};

pub struct ResultsView {
    banner: Vec<String>,
    scroller: Arc<Scroller>,
    _subs: SubscriptionHandle<ModelEvent>,
}
pub fn around<'a, T: Display>(s: impl IntoIterator<Item = T>, a: Rect) -> Line<'a> {
    let parts: Vec<String> = s.into_iter().map(|v| v.to_string()).collect();
    if parts.is_empty() {
        return Line::from("");
    }

    let total_len: usize = parts.iter().map(|p| p.len()).sum();
    let width = a.width as usize;

    if width <= total_len || parts.len() == 1 {
        return Line::from_iter(parts.into_iter().map(Span::raw));
    }

    let gaps = parts.len().saturating_sub(1);
    let extra = width.saturating_sub(total_len);
    let base = extra / gaps;
    let mut rem = extra % gaps;

    let mut spans = Vec::with_capacity(parts.len() + gaps);
    for (i, part) in parts.into_iter().enumerate() {
        spans.push(Span::raw(part));
        if i < gaps {
            let n = if rem > 0 { rem -= 1; base + 1 } else { base };
            spans.push(Span::raw(" ".repeat(n)));
        }
    }

    Line::from_iter(spans)
}

pub struct SearchItem(SearchResult, Arc<AtomicBool>);
impl SearchItem {
    pub fn new(s: SearchResult) -> Self {
        Self(s, AtomicBool::new(false).into())
    }
}

impl WidgetRef for SearchItem {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        let l1 = [self.0.name.clone(), self.0.release.to_string()];
        let l2 = [self.0.ty.clone(), self.0.size_gb.to_string()];

        Paragraph::new(Text::from_iter([
            around(l1, area),
            around(l2, area),
        ].into_iter().flatten())).style(match self.1.load(SeqCst) {
            true => Style::new().on_white().black(),
            false => Style::new(),
        }).render(area, buf);
    }
}

impl Focusable for SearchItem {
    fn focus(&self) {
        self.1.store(true, SeqCst);
    }

    fn blur(&self) {
        self.1.store(false, SeqCst);
    }
}

impl ScrollItem for SearchItem {
    fn height(&self) -> u16 {
        2
    }

    fn width(&self) -> u16 {
        0
    }
}

#[allow(clippy::new_without_default)]
impl ResultsView {
    pub fn new(items: Vec<SearchResult>, esc: impl Fn() + Send + Sync + 'static) -> Self {
        let flet = figlet_rs::FIGlet::from_content(BANNER_FONT).unwrap();
        let text = flet.convert("results").unwrap().to_string();

        let sub = model().target.on(SubscriptionPriority::Low, move |ev| {
            match **ev {
                ModelEvent::KeyPress(key_event) if key_event.code == KeyCode::Esc => {
                    esc();
                },
                _ => {}
            }
        });

        let items = items.into_iter().map(SearchItem::new);
        let this = Self { banner: text.lines().map(|l| l.to_string()).collect::<Vec<_>>(), scroller: Scroller::new(items).into(), _subs: sub };

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

        // Title
        Paragraph::new(text).render(layout[0], buf);

        // Rule
        Paragraph::new(Text::from_iter([
            Line::raw(""),
            Line::styled(std::iter::repeat_n(' ', layout[1].width as usize).collect::<String>(), Style::new().add_modifier(Modifier::CROSSED_OUT)),
            Line::raw(""),
        ]))
        .render(layout[1], buf);


        // Library
        self.scroller.render_ref(layout[2], buf);
    }
}

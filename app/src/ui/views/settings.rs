use std::sync::Arc;

use crate::{config, events::{SubscriptionHandle, SubscriptionPriority}, ui::components::{Focusable, InputEvent, input::Input, scroll::Scroller}};

use {
    crate::ui::views::home::BANNER_FONT,
    ratatui::{
        layout::Constraint,
        prelude::*,
        widgets::{Paragraph, WidgetRef},
    },
};

pub struct SettingsView {
    banner: Vec<String>,
    scroller: Arc<Scroller>,
    _subs: Vec<SubscriptionHandle<InputEvent<String>>>,
}

#[allow(clippy::new_without_default)]
impl SettingsView {
    pub fn new() -> Self {
        let flet = figlet_rs::FIGlet::from_content(BANNER_FONT).unwrap();
        let text = flet.convert("settings").unwrap().to_string();

        let c = config();
        let c = c.read().unwrap();

        let (subs, inputs): (Vec<SubscriptionHandle<InputEvent<String>>>, Vec<Input>) = vec![{
                let i = Input::new("MusicBrainz API Key", c.key_mb.clone());
                (i.on(SubscriptionPriority::Low, |ev| {
                    if let InputEvent::Submit(ev) = (**ev).clone() {
                        let c = config();
                        let mut c = c.write().unwrap();

                        c.key_mb = ev;
                        c.commit();
                    }
                }), i)
            },

            {
                let i = Input::new("TVDB API Key", c.key_mb.clone());
                (i.on(SubscriptionPriority::Low, |ev| {
                    if let InputEvent::Submit(ev) = (**ev).clone() {
                        let c = config();
                        let mut c = c.write().unwrap();

                        c.key_tv = ev;
                        c.commit();
                    }
                }), i)
            },

            {
                let i = Input::new("TMDB API Key", c.key_mb.clone());
                (i.on(SubscriptionPriority::Low, |ev| {
                    if let InputEvent::Submit(ev) = (**ev).clone() {
                        let c = config();
                        let mut c = c.write().unwrap();

                        c.key_tm = ev;
                        c.commit();
                    }
                }), i)
            }
        ].into_iter().unzip();


        let this = Self { banner: text.lines().map(|l| l.to_string()).collect::<Vec<_>>(), _subs: subs, scroller: Scroller::new(inputs).into() };

        this.scroller.focus();
        this
    }
}

impl WidgetRef for SettingsView {
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

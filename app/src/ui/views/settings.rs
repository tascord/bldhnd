use {
    crate::{
        config,
        events::{SubscriptionHandle, SubscriptionPriority},
        ui::{
            components::{
                Focusable, InputEvent,
                button::Button,
                scroll::{ScrollText, Scroller},
            },
            views::home::BANNER_FONT,
        },
    },
    ratatui::{
        layout::Constraint,
        prelude::*,
        widgets::{Paragraph, WidgetRef},
    },
    std::sync::Arc,
    tracing::warn,
};

pub struct SettingsView {
    banner: Vec<String>,
    scroller: Arc<Scroller>,
    _subs: (Vec<SubscriptionHandle<InputEvent<()>>>, ()),
}

#[allow(clippy::new_without_default)]
impl SettingsView {
    pub fn new() -> Self {
        let flet = figlet_rs::FIGlet::from_content(BANNER_FONT).unwrap();
        let text = flet.convert("settings").unwrap().to_string();

        let c = config();
        let _c = c.read().unwrap();

        let mut subs = (Vec::new(), ());
        let scroller = Scroller::new();

        let c = config().read().unwrap().clone();
        scroller.item_ref(ScrollText::new(format!("Volumes ({}): ", c.volumes.len())));

        let b = Button::new("Add New");
        subs.0.push(b.on(SubscriptionPriority::Low, |ev| {
            if let InputEvent::Submit(_) = (**ev).clone() {
                warn!("zzz");
            }
        }));

        scroller.item_ref(b);

        let this =
            Self { banner: text.lines().map(|l| l.to_string()).collect::<Vec<_>>(), _subs: subs, scroller: scroller.into() };

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
            Line::styled(
                std::iter::repeat_n(' ', layout[1].width as usize).collect::<String>(),
                Style::new().add_modifier(Modifier::CROSSED_OUT),
            ),
            Line::raw(""),
        ]))
        .render(layout[1], buf);

        // Library
        self.scroller.render_ref(layout[2], buf);
    }
}

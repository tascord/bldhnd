use {
    crate::{
        config,
        events::{SubscriptionHandle, SubscriptionPriority},
        ui::{
            components::{
                Focusable, InputEvent,
                button::Button,
                modal::{self, ModalRequest},
                scroll::{register_scroller_blur, register_scroller_focus, ScrollText, Scroller},
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

        let mut subs = (Vec::new(), ());
        let scroller = Scroller::new();

        Self::refresh_volumes(&scroller);

        let add_btn = Button::new("Add New");
        subs.0.push(add_btn.on(SubscriptionPriority::Low, |ev| {
            if let InputEvent::Submit(_) = (**ev).clone() {
                modal::modal().push(ModalRequest::VolumeAdd);
            }
        }));

        scroller.item_ref(add_btn);

        let this =
            Self { banner: text.lines().map(|l| l.to_string()).collect::<Vec<_>>(), _subs: subs, scroller: Arc::new(scroller) };

        this.scroller.focus();
        register_scroller_focus(Box::new({
            let scroller = this.scroller.clone();
            move || scroller.focus()
        }));
        register_scroller_blur(Box::new({
            let scroller = this.scroller.clone();
            move || scroller.blur()
        }));
        this
    }

    fn refresh_volumes(scroller: &Scroller) {
        let c = config().read().unwrap().clone();
        scroller.item_ref(ScrollText::new(format!("Volumes ({}): ", c.volumes.len())));

        for (i, v) in c.volumes.iter().enumerate() {
            let label = format!("[{}] {} ({}) - {:?}", i + 1, v.name, v.path, v.max_size_gb);
            scroller.item_ref(ScrollText::new(label));
        }
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
use {
    crate::{
        data::{KnowledgeBase, mb, tm, tv},
        events::{SubscriptionHandle, SubscriptionPriority},
        ui::{
            components::{Focusable, InputEvent, input::Input, radio::Radio},
            views::{hcenter, home::BANNER_FONT, results::ResultsView, vstack},
        },
    },
    ratatui::{
        layout::Constraint,
        prelude::*,
        widgets::{Paragraph, WidgetRef},
    },
    std::sync::{Arc, RwLock},
    to_and_fro::ToAndFro,
    tokio::{spawn, task::spawn_local},
    tracing::warn,
};

#[derive(ToAndFro)]
pub enum SearchType {
    Music,
    Movie,
    Series,
}

pub struct SearchView {
    banner: Vec<String>,
    input: Arc<Input>,
    radio: Arc<Radio>,
    search_ty: Arc<RwLock<SearchType>>,

    results: Arc<RwLock<Option<ResultsView>>>,

    _radio: Option<SubscriptionHandle<InputEvent<usize>>>,
    _input: Option<SubscriptionHandle<InputEvent<String>>>,
}

#[allow(clippy::new_without_default)]
impl SearchView {
    pub fn new() -> Self {
        let flet = figlet_rs::FIGlet::from_content(BANNER_FONT).unwrap();
        let text = flet.convert("search").unwrap().to_string();

        let mut this = Self {
            banner: text.lines().map(|l| l.to_string()).collect::<Vec<_>>(),
            search_ty: Arc::new(RwLock::new(SearchType::Music)),

            input: Input::new("", "").into(),
            radio: Radio::new(SearchType::list()).into(),

            results: Default::default(),

            _radio: None,
            _input: None,
        };

        this._radio = Some(this.radio.on(SubscriptionPriority::Low, {
            let st = this.search_ty.clone();
            move |ev| {
                if let InputEvent::Submit(ev) = (**ev).clone() {
                    *st.write().unwrap() = SearchType::list()[ev]
                }
            }
        }));

        this._input = Some(this.input.on(SubscriptionPriority::High, {
            let rs = this.results.clone();
            let st = this.search_ty.clone();
            let inp = this.input.clone();
            let rad = this.radio.clone();

            move |ev| {
                let rs = rs.clone();
                let inp = inp.clone();
                let rad = rad.clone();

                if let InputEvent::Submit(q) = (**ev).clone() {
                    let kb = match *st.read().unwrap() {
                        SearchType::Music => mb() as Arc<dyn KnowledgeBase>,
                        SearchType::Movie => tm() as Arc<dyn KnowledgeBase>,
                        SearchType::Series => tv() as Arc<dyn KnowledgeBase>,
                    };

                    inp.blur();
                    rad.blur();
                    inp.load(true);

                    spawn(async move {
                        match kb.search(&q).await {
                            Ok(r) => {
                                *rs.write().unwrap() = Some(ResultsView::new(r, {
                                    let rs = rs.clone();
                                    move || {
                                        *rs.write().unwrap() = None;
                                        inp.focus();
                                        rad.focus();
                                        inp.load(false);
                                    }
                                }));
                            }
                            Err(_) => {
                                warn!("Failed to fetch data");
                                inp.focus();
                                rad.focus();
                                inp.load(false);
                            }
                        }
                    });
                }
            }
        }));

        this.input.focus();
        this.radio.focus();

        this
    }
}

impl WidgetRef for SearchView {
    fn render_ref(&self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        if let Some(res) = self.results.read().unwrap().as_ref() {
            res.render_ref(area, buf);
            return;
        }

        let text = Text::from_iter(self.banner.iter().map(|l| Line::from(Span::raw(l.clone()))));

        let w = text.width();
        let h = text.height();

        let inner = area.centered(Constraint::Length((w as u16).max(area.width.min(32))), Constraint::Fill(1));
        let layout = vstack(&[h as u16, 3, 5], inner);

        // Figlet Banner
        Paragraph::new(text).alignment(HorizontalAlignment::Center).render(layout[0], buf);

        // Search Bar
        self.input.render_ref(layout[1], buf);

        // Radio
        self.radio.render_ref(hcenter(w as u16, layout[2]), buf);
    }
}

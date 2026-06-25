use {
    crate::{
        data::{SearchResult, data},
        events::{SubscriptionHandle, SubscriptionPriority},
        ui::{
            components::{Focusable, InputEvent, input::Input, radio::Radio},
            views::{hcenter, home::BANNER_FONT, results::ResultsView, vstack},
        },
    },
    futures_signals::signal::Mutable,
    ratatui::{
        layout::Constraint,
        prelude::*,
        widgets::{Paragraph, WidgetRef},
    },
    std::sync::Arc,
    to_and_fro::ToAndFro,
    tokio::spawn,
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
    search_ty: Mutable<SearchType>,

    results: Mutable<Option<ResultsView>>,

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
            search_ty: Mutable::new(SearchType::Music),

            input: Input::new("", "").into(),
            radio: Radio::new(SearchType::list()).into(),

            results: Mutable::new(None),

            _radio: None,
            _input: None,
        };

        this._radio = Some(this.radio.on(SubscriptionPriority::Low, {
            let st = this.search_ty.clone();
            move |ev| {
                if let InputEvent::Submit(ev) = (**ev).clone() {
                    st.set(SearchType::list()[ev])
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
                let st = st.clone();

                if let InputEvent::Submit(q) = (**ev).clone() {
                    inp.blur();
                    rad.blur();
                    inp.load(true);

                    spawn(async move {
                        let search_type = st.get();
                        match search_type {
                            SearchType::Music => match data().music((q, 0)).await {
                                Ok(v) => {
                                    rs.set(
                                        Some(ResultsView::new(v.into_iter().map(SearchResult::from).collect(), {
                                            let rs = rs.clone();
                                            move || {
                                                rs.set(None);
                                                inp.focus();
                                                rad.focus();
                                                inp.load(false);
                                            }
                                        }))
                                    )
                                }
                                Err(e) => {
                                    warn!("Failed to search: {e:?}");
                                    inp.focus();
                                    rad.focus();
                                    inp.load(false);
                                }
                            },
                            SearchType::Movie | SearchType::Series => match data().media((q, 0)).await {
                                Ok(v) => {
                                    rs.set(
                                        Some(ResultsView::new(v.into_iter().map(SearchResult::from).collect(), {
                                            let rs = rs.clone();
                                            move || {
                                                rs.set(None);
                                                inp.focus();
                                                rad.focus();
                                                inp.load(false);
                                            }
                                        }))
                                    )
                                }
                                Err(e) => {
                                    warn!("Failed to search: {e:?}");
                                    inp.focus();
                                    rad.focus();
                                    inp.load(false);
                                }
                            },
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
        if let Some(res) = self.results.lock_ref().as_ref() {
            res.render_ref(area, buf);
            return;
        }

        let text = Text::from_iter(self.banner.iter().map(|l| Line::from(Span::raw(l.clone()))));

        let w = text.width();
        let h = text.height();

        let inner = area.centered(Constraint::Length((w as u16).max(area.width.min(32))), Constraint::Fill(1));
        let layout = vstack(&[h as u16, 3, 5], inner);

        Paragraph::new(text).alignment(HorizontalAlignment::Center).render(layout[0], buf);

        self.input.render_ref(layout[1], buf);

        self.radio.render_ref(hcenter(w as u16, layout[2]), buf);
    }
}
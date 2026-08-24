use {
    bobatea::{AppEvent, View, theme::Theme},
    bldhnd::ui::views::BldhndView,
    ratatui::{Terminal, backend::TestBackend},
};

fn key(code: crossterm::event::KeyCode) -> AppEvent {
    AppEvent::KeyEvent(crossterm::event::KeyEvent::from(code))
}

fn click(col: u16, row: u16) -> AppEvent {
    AppEvent::MouseEvent(crossterm::event::MouseEvent {
        kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
        column: col,
        row,
        modifiers: crossterm::event::KeyModifiers::empty(),
    })
}

fn dump(view: &BldhndView, label: &str, from: u16) {
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    terminal.draw(|f| view.render(f, &Theme::default())).unwrap();
    println!("=== {label} ===");
    let buf = terminal.backend().buffer();
    for y in from..buf.area.height {
        let mut line = String::new();
        for x in 0..buf.area.width {
            line.push_str(buf[(x, y)].symbol());
        }
        let t = line.trim_end();
        if !t.is_empty() {
            println!("{:2} |{}", y, t);
        }
    }
    println!();
}

#[tokio::main]
async fn main() {
    let view = BldhndView::new();
    let ev = bobatea::events::EventTarget::<AppEvent>::new("app");
    view.mount(&ev);

    // Mouse click on "Search" tab label ("│ Home " is cols 0-6, "│ Search " 7-15)
    ev.emit(click(10, 1));
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    dump(&view, "CLICK 'SEARCH' TAB", 0);

    // Click past the labels (empty bar area) should do nothing
    ev.emit(click(80, 1));
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    dump(&view, "CLICK EMPTY BAR (should stay on Search)", 1);

    // Tab cycles focus input <-> list
    ev.emit(key(crossterm::event::KeyCode::Enter)); // focus input
    ev.emit(key(crossterm::event::KeyCode::Char('m')));
    ev.emit(key(crossterm::event::KeyCode::Tab)); // -> list
    ev.emit(key(crossterm::event::KeyCode::Tab)); // -> input
    ev.emit(key(crossterm::event::KeyCode::Char('!'))); // should type into input
    dump(&view, "TAB FOCUS CYCLING", 3);

    // Settings editing
    ev.emit(key(crossterm::event::KeyCode::Esc));
    ev.emit(key(crossterm::event::KeyCode::Char('4'))); // settings tab
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    ev.emit(key(crossterm::event::KeyCode::Down)); // select soulseek user
    ev.emit(key(crossterm::event::KeyCode::Enter)); // start edit
    ev.emit(key(crossterm::event::KeyCode::End));
    for ch in ['t', 'e', 's', 't'] {
        ev.emit(key(crossterm::event::KeyCode::Char(ch)));
    }
    dump(&view, "SETTINGS EDITING", 3);
    ev.emit(key(crossterm::event::KeyCode::Enter)); // save
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    dump(&view, "SETTINGS SAVED", 3);
}

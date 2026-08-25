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

    // Tab cycles focus input -> list -> results -> input
    ev.emit(key(crossterm::event::KeyCode::Enter)); // focus input
    ev.emit(key(crossterm::event::KeyCode::Char('m')));
    ev.emit(key(crossterm::event::KeyCode::Tab)); // -> list
    ev.emit(key(crossterm::event::KeyCode::Tab)); // -> results
    ev.emit(key(crossterm::event::KeyCode::Tab)); // -> input
    ev.emit(key(crossterm::event::KeyCode::Char('!'))); // should type into input
    dump(&view, "TAB FOCUS CYCLING", 3);

    // ── Settings: General section (download dir edit) ─────────────────────
    ev.emit(key(crossterm::event::KeyCode::Esc));
    ev.emit(key(crossterm::event::KeyCode::Char('4'))); // settings tab
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    dump(&view, "SETTINGS GENERAL", 3);
    ev.emit(key(crossterm::event::KeyCode::Right)); // enter General section
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    // Selection starts on the download dir row.
    ev.emit(key(crossterm::event::KeyCode::Enter)); // start edit
    for ch in ['/', 't', 'm', 'p', '/', 'd', 'l'] {
        ev.emit(key(crossterm::event::KeyCode::Char(ch)));
    }
    dump(&view, "SETTINGS EDITING", 3);
    ev.emit(key(crossterm::event::KeyCode::Enter)); // save
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    dump(&view, "SETTINGS SAVED", 3);

    // ── Volumes section ────────────────────────────────────────────────────
    ev.emit(key(crossterm::event::KeyCode::Left)); // back to section menu
    ev.emit(key(crossterm::event::KeyCode::Down)); // -> Volumes
    ev.emit(key(crossterm::event::KeyCode::Right)); // enter Volumes
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    // Add a volume: 'a', type name, Tab, type path, Enter
    ev.emit(key(crossterm::event::KeyCode::Char('a')));
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    for ch in ['m', 'u', 's', 'i', 'c'] {
        ev.emit(key(crossterm::event::KeyCode::Char(ch)));
    }
    ev.emit(key(crossterm::event::KeyCode::Tab));
    for ch in ['/', 't', 'm', 'p'] {
        ev.emit(key(crossterm::event::KeyCode::Char(ch)));
    }
    dump(&view, "ADD VOLUME FORM", 3);
    ev.emit(key(crossterm::event::KeyCode::Enter)); // commit volume
    tokio::time::sleep(std::time::Duration::from_millis(500)).await; // let save finish
    dump(&view, "VOLUME ADDED", 3);

    // Rename volume 0: selection is on name row; Enter opens rename.
    ev.emit(key(crossterm::event::KeyCode::Enter));
    ev.emit(key(crossterm::event::KeyCode::End));
    ev.emit(key(crossterm::event::KeyCode::Char('-')));
    ev.emit(key(crossterm::event::KeyCode::Enter));
    dump(&view, "VOLUME RENAMED?", 3);

    // Max size: Down x2 lands on max row; Enter edits, empty clears cap.
    for _ in 0..2 {
        ev.emit(key(crossterm::event::KeyCode::Down));
    }
    ev.emit(key(crossterm::event::KeyCode::Enter));
    for ch in ['1', '2'] {
        ev.emit(key(crossterm::event::KeyCode::Char(ch)));
    }
    ev.emit(key(crossterm::event::KeyCode::Enter));
    dump(&view, "VOLUME MAX SET?", 3);

    // ── Torrent indexers section ───────────────────────────────────────────
    ev.emit(key(crossterm::event::KeyCode::Left)); // back to menu
    ev.emit(key(crossterm::event::KeyCode::Down)); // -> Soulseek
    ev.emit(key(crossterm::event::KeyCode::Down)); // -> Torrent
    ev.emit(key(crossterm::event::KeyCode::Right)); // enter Torrent
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    ev.emit(key(crossterm::event::KeyCode::Char('a'))); // add indexer form
    for ch in ['p', 'r', 'o', 'w', 'l'] {
        ev.emit(key(crossterm::event::KeyCode::Char(ch)));
    }
    ev.emit(key(crossterm::event::KeyCode::Tab));
    for ch in ['h', 't', 't', 'p', ':', '/', '/', 'l', 'o', 'c', 'a', 'l', ':', '9', '6', '9', '6'] {
        ev.emit(key(crossterm::event::KeyCode::Char(ch)));
    }
    ev.emit(key(crossterm::event::KeyCode::Tab));
    for ch in ['k', 'e', 'y', '1', '2', '3'] {
        ev.emit(key(crossterm::event::KeyCode::Char(ch)));
    }
    dump(&view, "ADD INDEXER FORM", 3);
    ev.emit(key(crossterm::event::KeyCode::Enter));
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    dump(&view, "INDEXER ADDED (+ qbt block)", 3);

    // qBittorrent url row is right after indexer rows (index 3)
    for _ in 0..3 {
        ev.emit(key(crossterm::event::KeyCode::Down));
    }
    ev.emit(key(crossterm::event::KeyCode::Enter));
    for ch in ['8', '0', '8', '1'] {
        ev.emit(key(crossterm::event::KeyCode::Char(ch)));
    }
    ev.emit(key(crossterm::event::KeyCode::Enter));
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    dump(&view, "QBT URL SET", 3);

    // Delete the indexer: Up x3 back to its name row, then 'd'
    for _ in 0..3 {
        ev.emit(key(crossterm::event::KeyCode::Up));
    }
    ev.emit(key(crossterm::event::KeyCode::Char('d')));
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    dump(&view, "AFTER DELETE", 3);

    // Search tab with query + rules
    ev.emit(key(crossterm::event::KeyCode::Char('2')));
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    ev.emit(key(crossterm::event::KeyCode::Enter));
    for ch in ['m', 'e', 't', 'a', 'l'] {
        ev.emit(key(crossterm::event::KeyCode::Char(ch)));
    }
    dump(&view, "SEARCH TAB", 0);

    // Downloads tab (empty queue on fresh db). Esc leaves the query input first.
    ev.emit(key(crossterm::event::KeyCode::Esc));
    ev.emit(key(crossterm::event::KeyCode::Char('6')));
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    dump(&view, "DOWNLOADS TAB", 3);
}

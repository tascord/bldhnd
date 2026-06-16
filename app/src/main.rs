use bldhnd::ui::views::{Model, model};


fn main() -> anyhow::Result<()> {
    ratatui::run(Model::run)
}
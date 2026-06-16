use bldhnd::ui::views::Model;

fn main() -> anyhow::Result<()> { ratatui::run(Model::run) }

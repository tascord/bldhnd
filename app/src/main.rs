use bldhnd::ui::views::Model;

#[tokio::main]
async fn main() -> anyhow::Result<()> { ratatui::run(Model::run) }

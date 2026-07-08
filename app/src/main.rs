use bldhnd::ui::views::BldhndView;

#[tokio::main]
async fn main() -> anyhow::Result<()> { bobatea::App::new(BldhndView::new()).run().await }

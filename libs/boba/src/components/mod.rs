use {crate::Ctx, async_trait::async_trait};

pub mod stopwatch;

#[async_trait]
pub trait Component {
    async fn width(&self) -> Option<usize> { None }
    async fn height(&self) -> Option<usize> { None }

    async fn render(&mut self, ctx: Ctx);
}

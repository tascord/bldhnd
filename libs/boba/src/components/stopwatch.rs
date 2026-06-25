use std::time::Instant;

use async_trait::async_trait;

use crate::{Ctx, components::Component};

pub struct Stopwatch(Instant);

#[async_trait]
impl Component for Stopwatch {
    async fn render(&mut self, ctx: Ctx) {
        
    }
}
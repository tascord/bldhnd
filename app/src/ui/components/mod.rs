pub mod button;
pub mod input;
pub mod modal;
pub mod radio;
pub mod rainbow;
pub mod scroll;
pub mod sonner;

#[derive(Debug, Clone)]
pub enum InputEvent<V> {
    Submit(V),
    Blur,
    Focus,
}

pub trait Focusable {
    fn focus(&self);
    fn blur(&self);
}

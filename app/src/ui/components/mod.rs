pub mod input;
pub mod radio;
pub mod rainbow;

#[derive(Debug)]
pub enum InputEvent<V> {
    Submit(V),
    Blur,
    Focus,
}

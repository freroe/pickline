#[derive(Clone, PartialEq)]
pub enum RedrawAction {
    SingleLine(usize),
    LinePair(usize, usize),
    Bar,
    All
}

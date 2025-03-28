use crate::picker::select_action::SelectAction;

#[derive(PartialEq, Clone)]
pub enum Mode {
    Normal,
    Filter,
    Hint(SelectAction),
    DisplaySelection,
}
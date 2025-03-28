use crate::picker::modes::Mode;
use crate::picker::select_action::SelectAction;

pub(crate) enum Command {
    EnterMode(Mode),
    MoveUp,
    MoveDown,
    PreviousPage,
    NextPage,
    ToggleSelection(SelectAction),
    ShowSelection,
    Filter(String),
    AddHintChar(char, SelectAction),
    RemoveHintChar,
    Exit,
}

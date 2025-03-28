use crate::picker::modes::Mode;

pub(crate) enum Command {
    EnterMode(Mode),
    MoveUp,
    MoveDown,
    PreviousPage,
    NextPage,
    ToggleSelection,
    ShowSelection,
    SelectAndExit,
    Filter(String),
    AddHintChar(char),
    RemoveHintChar,
    Exit,
}

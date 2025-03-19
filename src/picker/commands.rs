use crate::picker::modes::Mode;

pub(crate) enum Command {
    EnterMode(Mode),
    MoveUp,
    MoveDown,
    PreviousPage,
    NextPage,
    Select,
    Filter(String),
    Hint(String),
    Exit,
}

use super::{AccountError, Digest};

#[derive(Debug)]
pub enum TransactionError {
    LoadAccountFailed(AccountError),
    CompileNoteScriptFailed,
    NoteIncompatibleWithAccount(Digest),
}

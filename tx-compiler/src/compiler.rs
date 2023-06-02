use super::{
    AccountCode, AccountId, Assembler, AssemblyContext, AssemblyContextType, CodeBlock,
    CompiledTransaction, Digest, HashMap, ModuleAst, NoteScript, ProgramAst, TransactionError,
};

#[derive(Default)]
pub struct TransactionComplier {
    assembler: Assembler,
    account_procedures: HashMap<AccountId, Vec<Digest>>,
}

impl TransactionComplier {
    /// Compiles the provided module into [AccountCode] and associates the resulting procedures
    /// with the specified account ID.
    pub fn load_account(
        &mut self,
        account_id: AccountId,
        account_code: ModuleAst,
    ) -> Result<AccountCode, TransactionError> {
        let account_code = AccountCode::new(account_code, account_id, &mut self.assembler)
            .map_err(TransactionError::LoadAccountFailed)?;
        self.account_procedures.insert(account_id, account_code.procedures().to_vec());
        Ok(account_code)
    }

    /// Compiles the provided program into the [NoteScript] and checks (to the extent possible)
    /// if a note could be executed against an account with the specified interface.
    pub fn compile_note_script(
        &mut self,
        note_script_ast: ProgramAst,
        target_account_proc: &Option<Vec<Digest>>,
    ) -> Result<NoteScript, TransactionError> {
        let mut assembly_context = AssemblyContext::new(AssemblyContextType::Program);
        let note_script =
            NoteScript::new(note_script_ast, &mut self.assembler, &mut assembly_context).unwrap();
        let note_root_code_block = note_script.code_block();

        // verify the note script is compatible with the target account interface
        if let Some(target_account_proc) = target_account_proc {
            verify_note_account_compatibility(note_root_code_block, &target_account_proc)?;
        }

        Ok(note_script)
    }

    /// Compiles a transaction which executes the provided notes against the specified account.
    ///
    /// The account is assumed to have been previously loaded into this compiler.
    pub fn compile_transaction(
        &mut self,
        // account_id: AccountId,
        // notes: Vec<Note>,
        // tx_script: Option<ProgramAst>,
    ) -> Result<CompiledTransaction, TransactionError> {
        todo!()
    }
}

/// Verifies that the provided note is compatible with the target account interface.
/// This is achieved by checking that at least one execution branch in the note script is compatible
/// with the target account interface.
///
/// # Errors
/// Returns an error if the note script is not compatible with the target account interface.
fn verify_note_account_compatibility(
    note_root: &CodeBlock,
    target_account_procs: &[Digest],
) -> Result<(), TransactionError> {
    // initialize an empty stack of execution branches
    let mut branches = vec![vec![]];

    // collect all call branches
    collect_call_branches(note_root, &mut branches);

    // if none of the branches are compatible with the target account, return an error
    if !branches
        .iter()
        .any(|call_targets| call_targets.iter().all(|target| target_account_procs.contains(target)))
    {
        return Err(TransactionError::NoteIncompatibleWithAccount(note_root.hash()));
    }

    Ok(())
}

/// Generates a list of calls invoked in each execution branch of the provided code block.
fn collect_call_branches(code_block: &CodeBlock, branches: &mut Vec<Vec<Digest>>) {
    match code_block {
        CodeBlock::Join(block) => {
            collect_call_branches(block.first(), branches);
            collect_call_branches(block.second(), branches);
        }
        CodeBlock::Split(block) => {
            let current_len = branches.last().expect("at least one execution branch").len();
            collect_call_branches(block.on_false(), branches);

            // If the previous branch had additional calls we need to create a new branch
            if branches.last().expect("at least one execution branch").len() > current_len {
                branches.push(
                    branches.last().expect("at least one execution branch")[..current_len].to_vec(),
                );
            }

            collect_call_branches(block.on_true(), branches);
        }
        CodeBlock::Loop(block) => {
            collect_call_branches(block.body(), branches);
        }
        CodeBlock::Call(block) => {
            branches
                .last_mut()
                .expect("at least one execution branch")
                .push(block.fn_hash());
        }
        CodeBlock::Span(_) => {}
        CodeBlock::Proxy(_) => {}
    }
}

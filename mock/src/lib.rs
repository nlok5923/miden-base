use miden_lib::{assembler::assembler, memory};
use miden_objects::{
    accounts::Account, notes::Note, notes::NoteVault, transaction::PreparedTransaction,
    BlockHeader, ChainMmr, Felt, StarkField,
};
use std::{fs::File, io::Read, path::PathBuf};
use vm_processor::{
    AdviceProvider, DefaultHost, ExecutionError, ExecutionOptions, Process, Program, StackInputs,
    Word,
};

pub mod builders;
pub mod constants;
pub mod mock;
pub mod procedures;
pub mod utils;

// TEST BRACE
// ================================================================================================

/// Loads the specified file and append `code` into its end.
fn load_file_with_code(imports: &str, code: &str, assembly_file: PathBuf) -> String {
    let mut module = String::new();
    File::open(assembly_file).unwrap().read_to_string(&mut module).unwrap();
    let complete_code = format!("{imports}{module}{code}");

    // This hack is going around issue #686 on miden-vm
    complete_code.replace("export", "proc")
}

/// Inject `code` along side the specified file and run it
pub fn run_tx<A>(
    program: Program,
    stack_inputs: StackInputs,
    mut host: DefaultHost<A>,
) -> Result<Process<DefaultHost<A>>, ExecutionError>
where
    A: AdviceProvider,
{
    // mock account method for testing from root context
    host.advice_provider_mut()
        .insert_into_map(Word::default(), vec![Felt::new(255)])
        .unwrap();

    let mut process =
        Process::new(program.kernel().clone(), stack_inputs, host, ExecutionOptions::default());
    process.execute(&program)?;
    Ok(process)
}

/// Inject `code` along side the specified file and run it
pub fn run_within_tx_kernel<A>(
    imports: &str,
    code: &str,
    stack_inputs: StackInputs,
    mut host: DefaultHost<A>,
    file_path: Option<PathBuf>,
) -> Result<Process<DefaultHost<A>>, ExecutionError>
where
    A: AdviceProvider,
{
    // mock account method for testing from root context
    host.advice_provider_mut()
        .insert_into_map(Word::default(), vec![Felt::new(255)])
        .unwrap();

    let assembler = assembler();

    let code = match file_path {
        Some(file_path) => load_file_with_code(imports, code, file_path),
        None => format!("{imports}{code}"),
    };

    let program = assembler.compile(code).unwrap();

    let mut process =
        Process::new(program.kernel().clone(), stack_inputs, host, ExecutionOptions::default());
    process.execute(&program)?;
    Ok(process)
}

// TEST HELPERS
// ================================================================================================
pub fn consumed_note_data_ptr(note_idx: u32) -> memory::MemoryAddress {
    memory::CONSUMED_NOTE_SECTION_OFFSET + (1 + note_idx) * 1024
}

#[allow(clippy::too_many_arguments)]
pub fn prepare_transaction(
    account: Account,
    account_seed: Option<Word>,
    block_header: BlockHeader,
    chain: ChainMmr,
    notes: Vec<Note>,
    code: &str,
    imports: &str,
    file_path: Option<PathBuf>,
) -> PreparedTransaction {
    let assembler = assembler();

    let code = match file_path {
        Some(file_path) => load_file_with_code(imports, code, file_path),
        None => format!("{imports}{code}"),
    };

    let program = assembler.compile(code).unwrap();

    PreparedTransaction::new(account, account_seed, block_header, chain, notes, None, program)
        .unwrap()
}

use super::{
    Account, AccountCode, AccountId, AccountStorage, Asset, BlockHeader, ChainMmr, Digest,
    ExecutedTransaction, Felt, FieldElement, FungibleAsset, MerkleStore, NodeIndex, Note,
    NoteOrigin, StorageItem, TransactionInputs, Word, NOTE_LEAF_DEPTH, NOTE_TREE_DEPTH,
};
use assembly::{Assembler, AssemblyContext, AssemblyContextType, ModuleAst, ProgramAst};
use crypto::merkle::SimpleSmt;
use miden_objects::notes::NoteScript;
use test_utils::rand;

// MOCK DATA
// ================================================================================================
pub const ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN: u64 = 0b0010011011u64 << 54;
pub const ACCOUNT_ID_SENDER: u64 = 0b0110111011u64 << 54;

pub const ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN: u64 = 0b1010011100 << 54;

pub const NONCE: Felt = Felt::ZERO;

pub const STORAGE_INDEX_0: u8 = 20;
pub const STORAGE_VALUE_0: [Felt; 4] = [Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)];
pub const STORAGE_INDEX_1: u8 = 30;
pub const STORAGE_VALUE_1: [Felt; 4] = [Felt::new(5), Felt::new(6), Felt::new(7), Felt::new(8)];
pub const STORAGE_ITEM_0: StorageItem = (STORAGE_INDEX_0, STORAGE_VALUE_0);
pub const STORAGE_ITEM_1: StorageItem = (STORAGE_INDEX_1, STORAGE_VALUE_1);

pub const CHILD_ROOT_PARENT_LEAF_INDEX: u8 = 10;
pub const CHILD_SMT_DEPTH: u8 = 64;
pub const CHILD_STORAGE_INDEX_0: u64 = 40;
pub const CHILD_STORAGE_VALUE_0: [Felt; 4] =
    [Felt::new(11), Felt::new(12), Felt::new(13), Felt::new(14)];

pub fn mock_block_header(
    block_num: Felt,
    chain_root: Option<Digest>,
    note_root: Option<Digest>,
) -> BlockHeader {
    let prev_hash: Digest = rand::rand_array().into();
    let chain_root: Digest = chain_root.unwrap_or(rand::rand_array().into());
    let state_root: Digest = rand::rand_array().into();
    let note_root: Digest = note_root.unwrap_or(rand::rand_array().into());
    let batch_root: Digest = rand::rand_array().into();
    let proof_hash: Digest = rand::rand_array().into();

    BlockHeader::new(
        prev_hash, block_num, chain_root, state_root, note_root, batch_root, proof_hash,
    )
}

pub fn mock_chain_data(consumed_notes: &mut [Note]) -> ChainMmr {
    let mut note_trees = Vec::new();

    // TODO: Consider how to better represent note authentication data.
    // we use the index for both the block number and the leaf index in the note tree
    for (index, note) in consumed_notes.iter().enumerate() {
        let tree_index = 2 * index;
        let smt_entries = vec![
            (tree_index as u64, note.hash().into()),
            ((tree_index + 1) as u64, note.metadata().into()),
        ];
        let smt = SimpleSmt::with_leaves(NOTE_LEAF_DEPTH, smt_entries).unwrap();
        note_trees.push(smt);
    }

    // create a dummy chain of block headers
    let block_chain = vec![
        mock_block_header(Felt::ZERO, None, Some(note_trees[0].root().into())),
        mock_block_header(Felt::ONE, None, Some(note_trees[1].root().into())),
        mock_block_header(Felt::new(2), None, None),
        mock_block_header(Felt::new(3), None, None),
    ];

    // convert block hashes into words
    let block_hashes: Vec<Word> = block_chain.iter().map(|h| Word::from(h.hash())).collect();

    // instantiate and populate MMR
    let mut chain_mmr = ChainMmr::default();
    for hash in block_hashes.iter() {
        chain_mmr.mmr_mut().add(*hash)
    }

    // set origin for consumed notes using chain and block data
    for (index, note) in consumed_notes.iter_mut().enumerate() {
        let block_header = &block_chain[index];
        let auth_index = NodeIndex::new(NOTE_TREE_DEPTH, index as u64).unwrap();
        note.set_origin(
            NoteOrigin::new(
                block_header.block_num(),
                block_header.sub_hash(),
                block_header.note_root(),
                index as u64,
                note_trees[index].get_path(auth_index).unwrap(),
            )
            .unwrap(),
        );
    }

    chain_mmr
}

fn mock_account(nonce: Option<Felt>, assembler: &mut Assembler) -> Account {
    // Create account id
    let account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN).unwrap();

    // Create an account merkle store
    let mut account_merkle_store = MerkleStore::new();
    let child_smt =
        SimpleSmt::with_leaves(CHILD_SMT_DEPTH, [(CHILD_STORAGE_INDEX_0, CHILD_STORAGE_VALUE_0)])
            .unwrap();
    account_merkle_store.extend(child_smt.inner_nodes());

    // create account storage
    let account_storage = AccountStorage::new(
        vec![STORAGE_ITEM_0, STORAGE_ITEM_1, (CHILD_ROOT_PARENT_LEAF_INDEX, child_smt.root())],
        account_merkle_store,
    )
    .unwrap();

    let account_code = "\
    export.account_procedure_1
        push.1.2
        add
    end

    export.account_procedure_2
        push.2.1
        sub
    end
    ";
    let account_module_ast = ModuleAst::parse(account_code).unwrap();
    let account_code = AccountCode::new(account_module_ast, account_id, assembler).unwrap();

    // Create an account with storage items
    let account =
        Account::new(account_id, account_storage, account_code, nonce.unwrap_or(Felt::ZERO))
            .unwrap();

    account
}

pub fn mock_inputs() -> TransactionInputs {
    // Create assembler and assembler context
    let mut assembler = Assembler::default();
    let mut assembler_context = AssemblyContext::new(AssemblyContextType::Program);

    // Create an account with storage items
    let account = mock_account(None, &mut assembler);

    // Consumed notes
    let mut consumed_notes = mock_consumed_notes(&mut assembler, &mut assembler_context);

    // Chain data
    let chain_mmr: ChainMmr = mock_chain_data(&mut consumed_notes);

    // Block header
    let block_header: BlockHeader = mock_block_header(
        Felt::new(4),
        Some(chain_mmr.mmr().accumulator().hash_peaks().into()),
        None,
    );

    // Transaction inputs
    TransactionInputs::new(account, block_header, chain_mmr, consumed_notes, None)
}

pub fn mock_executed_tx() -> ExecutedTransaction {
    // Create assembler and assembler context
    let mut assembler = Assembler::default();
    let mut assembler_context = AssemblyContext::new(AssemblyContextType::Program);

    // Initial Account
    let initial_account = mock_account(Some(Felt::ZERO), &mut assembler);

    // Finial Account (nonce incremented by 1)
    let final_account = mock_account(Some(Felt::ONE), &mut assembler);

    // Consumed notes
    let mut consumed_notes = mock_consumed_notes(&mut assembler, &mut assembler_context);

    // Created notes
    let created_notes = mock_created_notes(&mut assembler, &mut assembler_context);

    // Chain data
    let chain_mmr: ChainMmr = mock_chain_data(&mut consumed_notes);

    // Block header
    let block_header: BlockHeader = mock_block_header(
        Felt::new(4),
        Some(chain_mmr.mmr().accumulator().hash_peaks().into()),
        None,
    );

    // Executed Transaction

    ExecutedTransaction::new(
        initial_account,
        final_account,
        consumed_notes,
        created_notes,
        None,
        block_header,
        chain_mmr,
    )
}

fn mock_consumed_notes(
    assembler: &mut Assembler,
    assembler_context: &mut AssemblyContext,
) -> Vec<Note> {
    // Note Assets
    let faucet_id_1 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let faucet_id_2 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN + 10).unwrap();
    let faucet_id_3 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN + 20).unwrap();
    let fungible_asset_1: Asset = FungibleAsset::new(faucet_id_1, 100).unwrap().into();
    let fungible_asset_2: Asset = FungibleAsset::new(faucet_id_2, 200).unwrap().into();
    let fungible_asset_3: Asset = FungibleAsset::new(faucet_id_3, 300).unwrap().into();

    // Sender account
    let sender = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();

    // create note script
    let note_program_ast = ProgramAst::parse("begin push.1 end").unwrap();
    let note_script = NoteScript::new(note_program_ast, assembler, assembler_context).unwrap();

    // Consumed Notes
    const SERIAL_NUM_1: Word = [Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)];
    let note_1 = Note::new(
        note_script.clone(),
        &[Felt::new(1)],
        &[fungible_asset_1, fungible_asset_2, fungible_asset_3],
        SERIAL_NUM_1,
        sender,
        Felt::ZERO,
        None,
    )
    .unwrap();

    const SERIAL_NUM_2: Word = [Felt::new(5), Felt::new(6), Felt::new(7), Felt::new(8)];
    let note_2 = Note::new(
        note_script,
        &[Felt::new(2)],
        &[fungible_asset_1, fungible_asset_2, fungible_asset_3],
        SERIAL_NUM_2,
        sender,
        Felt::ZERO,
        None,
    )
    .unwrap();

    vec![note_1, note_2]
}

fn mock_created_notes(
    assembler: &mut Assembler,
    assembler_context: &mut AssemblyContext,
) -> Vec<Note> {
    // Note assets
    let faucet_id_1 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let faucet_id_2 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN + 10).unwrap();
    let faucet_id_3 = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN + 20).unwrap();
    let fungible_asset_1: Asset = FungibleAsset::new(faucet_id_1, 100).unwrap().into();
    let fungible_asset_2: Asset = FungibleAsset::new(faucet_id_2, 100).unwrap().into();
    let fungible_asset_3: Asset = FungibleAsset::new(faucet_id_3, 100).unwrap().into();

    // sender account
    let sender = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();

    // create note script
    let note_program_ast = ProgramAst::parse("begin push.1 end").unwrap();
    let note_script = NoteScript::new(note_program_ast, assembler, assembler_context).unwrap();

    // Created Notes
    const SERIAL_NUM_1: Word = [Felt::new(9), Felt::new(10), Felt::new(11), Felt::new(12)];
    let note_1 = Note::new(
        note_script.clone(),
        &[Felt::new(1)],
        &[fungible_asset_1, fungible_asset_2],
        SERIAL_NUM_1,
        sender,
        Felt::ZERO,
        None,
    )
    .unwrap();

    const SERIAL_NUM_2: Word = [Felt::new(13), Felt::new(14), Felt::new(15), Felt::new(16)];
    let note_2 = Note::new(
        note_script.clone(),
        &[Felt::new(2)],
        &[fungible_asset_1, fungible_asset_2, fungible_asset_3],
        SERIAL_NUM_2,
        sender,
        Felt::ZERO,
        None,
    )
    .unwrap();

    const SERIAL_NUM_3: Word = [Felt::new(17), Felt::new(18), Felt::new(19), Felt::new(20)];
    let note_3 = Note::new(
        note_script,
        &[Felt::new(2)],
        &[fungible_asset_1, fungible_asset_2, fungible_asset_3],
        SERIAL_NUM_3,
        sender,
        Felt::ZERO,
        None,
    )
    .unwrap();

    vec![note_1, note_2, note_3]
}

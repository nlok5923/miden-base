use super::{ContextId, Felt, MemAdviceProvider, ProcessState, StackInputs, Word, ONE, ZERO};
use crate::memory::{ACCT_CODE_ROOT_PTR, ACCT_NEW_CODE_ROOT_PTR};
use miden_objects::accounts::{
    AccountId, AccountType, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN, ACCOUNT_ID_INSUFFICIENT_ONES,
    ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN, ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN,
    ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
};
use mock::{
    constants::{
        storage_item_0, storage_item_1, CHILD_ROOT_PARENT_LEAF_INDEX, CHILD_SMT_DEPTH,
        CHILD_STORAGE_INDEX_0, CHILD_STORAGE_VALUE_0,
    },
    mock::{
        account::MockAccountType, notes::AssetPreservationStatus, transaction::mock_executed_tx,
        transaction::mock_inputs,
    },
    prepare_transaction,
    procedures::{created_notes_data_procedure, prepare_word},
    run_tx, run_within_tx_kernel,
};

// TESTS
// ================================================================================================

#[test]
pub fn test_set_code_is_not_immediate() {
    let (account, block_header, chain, notes, auxiliary_data) =
        mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);

    let code = "
        use.miden::sat::internal::prologue
        use.miden::sat::account
        begin
            exec.prologue::prepare_transaction
            push.1.2.3.4
            exec.account::set_code
        end
        ";

    let transaction = prepare_transaction(
        account,
        None,
        block_header,
        chain,
        notes,
        None,
        auxiliary_data,
        code,
        "",
        None,
    );

    let process = run_tx(
        transaction.tx_program().clone(),
        transaction.stack_inputs(),
        MemAdviceProvider::from(transaction.advice_provider_inputs()),
    )
    .unwrap();

    // assert the code root is not changed
    assert_eq!(
        process.get_mem_value(ContextId::root(), ACCT_CODE_ROOT_PTR).unwrap(),
        transaction.account().code().root().as_elements()
    );

    // assert the new code root is cached
    assert_eq!(
        process.get_mem_value(ContextId::root(), ACCT_NEW_CODE_ROOT_PTR).unwrap(),
        [ONE, Felt::new(2), Felt::new(3), Felt::new(4)]
    );
}

#[test]
pub fn test_set_code_succeeds() {
    let executed_transaction = mock_executed_tx(AssetPreservationStatus::Preserved);

    let created_notes_data_procedure =
        created_notes_data_procedure(executed_transaction.created_notes());

    let code = format!(
        "
        use.miden::sat::account
        use.miden::sat::internal::prologue
        use.miden::sat::internal::epilogue

        {created_notes_data_procedure}
        begin
            exec.prologue::prepare_transaction

            push.0.1.2.3
            exec.account::set_code

            exec.create_mock_notes

            push.1
            exec.account::incr_nonce

            exec.epilogue::finalize_transaction
        end
        "
    );

    let process = run_within_tx_kernel(
        "",
        &code,
        executed_transaction.stack_inputs(),
        MemAdviceProvider::from(executed_transaction.advice_provider_inputs()),
        None,
    )
    .unwrap();

    // assert the code root is changed after the epilogue
    assert_eq!(
        process.get_mem_value(ContextId::root(), ACCT_CODE_ROOT_PTR).unwrap(),
        [ZERO, ONE, Felt::new(2), Felt::new(3)]
    );
}

#[test]
pub fn test_account_type() {
    let procedures = vec![
        ("is_fungible_faucet", AccountType::FungibleFaucet),
        ("is_non_fungible_faucet", AccountType::NonFungibleFaucet),
        ("is_updatable_account", AccountType::RegularAccountUpdatableCode),
        ("is_immutable_account", AccountType::RegularAccountImmutableCode),
    ];

    let test_cases = vec![
        ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN,
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
        ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
        ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN,
    ];

    for (procedure, expected_type) in procedures {
        for account_id in test_cases.iter() {
            let account_id = AccountId::try_from(*account_id).unwrap();

            let code = format!(
                "
                use.miden::sat::internal::layout
                use.miden::sat::internal::account

                begin
                    exec.account::{}
                end
                ",
                procedure
            );

            let process = run_within_tx_kernel(
                "",
                &code,
                StackInputs::new(vec![account_id.into()]),
                MemAdviceProvider::default(),
                None,
            )
            .unwrap();

            let expected_result = if account_id.account_type() == expected_type {
                ONE
            } else {
                ZERO
            };
            assert_eq!(process.stack.get(0), expected_result);
        }
    }
}

#[test]
fn test_validate_id_fails_on_insuficcient_ones() {
    let code = format!(
        "
        use.miden::sat::internal::account

        begin
            push.{ACCOUNT_ID_INSUFFICIENT_ONES}
            exec.account::validate_id
        end
        "
    );

    let result =
        run_within_tx_kernel("", &code, StackInputs::default(), MemAdviceProvider::default(), None);

    assert!(result.is_err());
}

#[test]
fn test_get_item() {
    for storage_item in [storage_item_0(), storage_item_1()] {
        let (account, block_header, chain, notes, auxiliary_data) =
            mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);

        let code = format!(
            "
        use.miden::sat::account
        use.miden::sat::internal::prologue


        begin
            # prepare the transaction
            exec.prologue::prepare_transaction

            # push the account storage item index
            push.{item_index}

            # get the item
            exec.account::get_item

            # assert the item value is correct
            push.{item_value} assert_eqw
        end
        ",
            item_index = storage_item.0,
            item_value = prepare_word(&storage_item.1 .1)
        );

        let transaction = prepare_transaction(
            account,
            None,
            block_header,
            chain,
            notes,
            None,
            auxiliary_data,
            &code,
            "",
            None,
        );

        let _process = run_tx(
            transaction.tx_program().clone(),
            StackInputs::from(transaction.stack_inputs()),
            MemAdviceProvider::from(transaction.advice_provider_inputs()),
        )
        .unwrap();
    }
}

#[test]
fn test_get_child_tree_item() {
    let (account, block_header, chain, notes, auxiliary_data) =
        mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);

    let code = format!(
        "
        use.miden::sat::account
        use.miden::sat::internal::prologue

        begin
            # prepare the transaction
            exec.prologue::prepare_transaction

            # push the acount storage index the child root is stored at
            push.{CHILD_ROOT_PARENT_LEAF_INDEX}

            # get the child root
            exec.account::get_item

            # get a value from the child tree
            push.{CHILD_STORAGE_INDEX_0}

            # get the item
            push.{CHILD_SMT_DEPTH} mtree_get

            # assert the child value is correct
            push.{child_value} assert_eqw
        end
        ",
        child_value = prepare_word(&CHILD_STORAGE_VALUE_0)
    );

    let transaction = prepare_transaction(
        account,
        None,
        block_header,
        chain,
        notes,
        None,
        auxiliary_data,
        code.as_str(),
        "",
        None,
    );

    let _process = run_tx(
        transaction.tx_program().clone(),
        StackInputs::from(transaction.stack_inputs()),
        MemAdviceProvider::from(transaction.advice_provider_inputs()),
    )
    .unwrap();
}

#[test]
fn test_set_item() {
    let (account, block_header, chain, notes, auxiliary_data) =
        mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);

    // copy the initial account slots (SMT)
    let mut account_smt = account.storage().slots().clone();
    let init_root = account_smt.root();

    // insert a new leaf value
    const NEW_ITEM_INDEX: u64 = 12;
    const NEW_ITEM_VALUE: Word = [Felt::new(91), Felt::new(92), Felt::new(93), Felt::new(94)];
    account_smt.update_leaf(NEW_ITEM_INDEX, NEW_ITEM_VALUE).unwrap();
    assert_ne!(account_smt.root(), init_root);

    let code = format!(
        "
    use.miden::sat::account
    use.miden::sat::internal::layout
    use.miden::sat::internal::prologue

    begin
        # prepare the transaction
        exec.prologue::prepare_transaction

        # push the new storage item onto the stack
        push.{new_value}

        # push the account storage item index
        push.{NEW_ITEM_INDEX}

        # get the item
        exec.account::set_item

        # assert empty old value
        padw assert_eqw

        # get the new storage root
        exec.layout::get_acct_storage_root

        # assert the item value is correct
        push.{new_root} assert_eqw
    end
    ",
        new_value = prepare_word(&NEW_ITEM_VALUE),
        new_root = prepare_word(&account_smt.root()),
    );

    let transaction = prepare_transaction(
        account,
        None,
        block_header,
        chain,
        notes,
        None,
        auxiliary_data,
        &code,
        "",
        None,
    );

    let _process = run_tx(
        transaction.tx_program().clone(),
        StackInputs::from(transaction.stack_inputs()),
        MemAdviceProvider::from(transaction.advice_provider_inputs()),
    )
    .unwrap();
}

#[test]
fn test_is_faucet_procedure() {
    let test_cases = vec![
        ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN,
        ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_OFF_CHAIN,
        ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
        ACCOUNT_ID_NON_FUNGIBLE_FAUCET_OFF_CHAIN,
    ];

    for account_id in test_cases.iter() {
        let account_id = AccountId::try_from(*account_id).unwrap();

        // assembly codes that checks if an account is a fauct
        let code = format!(
            "
        use.miden::sat::internal::account

        begin
            # push the account id on to the stack
            push.{account_id}

            # execute is_faucet procedure
            exec.account::is_faucet

            # assert it matches expected result
            eq.{expected} assert
        end
    ",
            account_id = account_id,
            expected = if account_id.is_faucet() { 1 } else { 0 },
        );

        let _process = run_within_tx_kernel(
            "",
            &code,
            StackInputs::default(),
            MemAdviceProvider::default(),
            None,
        )
        .unwrap();
    }
}

#[test]
fn test_authenticate_procedure() {
    let (account, _, _, _, _) =
        mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);

    let test_cases = vec![
        (account.code().procedure_tree().get_leaf(0).unwrap(), true),
        (account.code().procedure_tree().get_leaf(1).unwrap(), true),
        ([ONE, ZERO, ONE, ZERO], false),
    ];

    for (root, valid) in test_cases.into_iter() {
        let (account, block_header, chain, notes, auxiliary_data) =
            mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);

        let code = format!(
            "\
            use.miden::sat::internal::account
            use.miden::sat::internal::prologue

            begin
                # prepare the transaction
                exec.prologue::prepare_transaction

                # push test procedure root onto stack
                push.{root}

                # authenticate procedure
                exec.account::authenticate_procedure
            end
        ",
            root = prepare_word(&root)
        );

        let transaction = prepare_transaction(
            account,
            None,
            block_header,
            chain,
            notes,
            None,
            auxiliary_data,
            &code,
            "",
            None,
        );

        let process = run_tx(
            transaction.tx_program().clone(),
            StackInputs::from(transaction.stack_inputs()),
            MemAdviceProvider::from(transaction.advice_provider_inputs()),
        );

        match valid {
            true => assert!(process.is_ok()),
            false => assert!(process.is_err()),
        }
    }
}

#[test]
fn test_get_vault_commitment() {
    let (account, block_header, chain, notes, auxiliary_data) =
        mock_inputs(MockAccountType::StandardExisting, AssetPreservationStatus::Preserved);

    let code = format!(
        "
    use.miden::sat::account
    use.miden::sat::internal::prologue

    begin
        # prepare the transaction
        exec.prologue::prepare_transaction

        # push the new storage item onto the stack
        exec.account::get_vault_commitment
        push.{expected_vault_commitment}
        assert_eqw
    end
    ",
        expected_vault_commitment = prepare_word(&account.vault().commitment()),
    );

    let transaction = prepare_transaction(
        account,
        None,
        block_header,
        chain,
        notes,
        None,
        auxiliary_data,
        &code,
        "",
        None,
    );

    let _process = run_tx(
        transaction.tx_program().clone(),
        StackInputs::from(transaction.stack_inputs()),
        MemAdviceProvider::from(transaction.advice_provider_inputs()),
    )
    .unwrap();
}

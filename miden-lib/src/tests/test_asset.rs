use super::{DefaultHost, Hasher, MemAdviceProvider, Word};
use mock::{
    constants::{
        non_fungible_asset, ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
        ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN, FUNGIBLE_ASSET_AMOUNT, NON_FUNGIBLE_ASSET_DATA,
    },
    mock::{account::MockAccountType, notes::AssetPreservationStatus, transaction::mock_inputs},
    prepare_transaction,
    procedures::prepare_word,
    run_tx,
};

#[test]
fn test_create_fungible_asset_succeeds() {
    let (account, block_header, chain, notes) = mock_inputs(
        MockAccountType::FungibleFaucet(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN),
        AssetPreservationStatus::Preserved,
    );

    let code = format!(
        "
        use.miden::sat::internal::prologue
        use.miden::sat::asset

        begin
            # prepare the transaction
            exec.prologue::prepare_transaction

            # push asset amount onto stack
            push.{FUNGIBLE_ASSET_AMOUNT}

            # create fungible asset
            exec.asset::create_fungible_asset

            # assert the asset is correctly formed
            push.{FUNGIBLE_ASSET_AMOUNT}.0.0.{ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN}
            assert_eqw
        end
        "
    );

    let transaction =
        prepare_transaction(account, None, block_header, chain, notes, &code, "", None);

    let _process = run_tx(
        transaction.tx_program().clone(),
        transaction.stack_inputs(),
        DefaultHost::new(MemAdviceProvider::from(transaction.advice_provider_inputs())),
    );
}

#[test]
fn test_create_non_fungible_asset_succeeds() {
    let (account, block_header, chain, notes) =
        mock_inputs(MockAccountType::NonFungibleFaucet, AssetPreservationStatus::Preserved);
    let non_fungible_asset = non_fungible_asset(ACCOUNT_ID_NON_FUNGIBLE_FAUCET_ON_CHAIN);

    let code = format!(
        "
        use.miden::sat::internal::prologue
        use.miden::sat::asset

        begin
            # prepare the transaction
            exec.prologue::prepare_transaction

            # push non-fungible asset data hash onto the stack
            push.{non_fungible_asset_data_hash}
            exec.asset::create_non_fungible_asset

            # assert the non-fungible asset is correctly formed
            push.{expected_non_fungible_asset}
            assert_eqw
        end
        ",
        non_fungible_asset_data_hash = prepare_word(&*Hasher::hash(&NON_FUNGIBLE_ASSET_DATA)),
        expected_non_fungible_asset = prepare_word(&Word::from(non_fungible_asset))
    );

    let transaction =
        prepare_transaction(account, None, block_header, chain, notes, &code, "", None);

    let _process = run_tx(
        transaction.tx_program().clone(),
        transaction.stack_inputs(),
        DefaultHost::new(MemAdviceProvider::from(transaction.advice_provider_inputs())),
    )
    .unwrap();
}

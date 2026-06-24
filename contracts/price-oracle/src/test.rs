#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    Address, Bytes, Env, String, Symbol, Vec,
};

use crate::{Asset, PriceData, PriceEntry, PriceOracleContract, PriceOracleContractClient};

fn create_contract(e: &Env) -> PriceOracleContractClient<'_> {
    e.mock_all_auths();
    let contract_id = e.register(PriceOracleContract, ());
    PriceOracleContractClient::new(e, &contract_id)
}

fn clear_auth(e: &Env) {
    use soroban_sdk::xdr::SorobanAuthorizationEntry;
    e.set_auths(&[] as &[SorobanAuthorizationEntry]);
}

fn init_admin(client: &PriceOracleContractClient<'_>, admin: &Address) {
    client.initialize(
        admin,
        &2u32,
        &10u32,
        &18u32,
        &String::from_str(&client.env, "Stellar Price Oracle Aggregator"),
    );
}

fn setup_basic(e: &Env) -> (PriceOracleContractClient<'_>, Address, Address, Address) {
    let admin = Address::generate(e);
    let client = create_contract(e);
    init_admin(&client, &admin);

    let source1 = Address::generate(e);
    let source2 = Address::generate(e);
    let asset1 = Address::generate(e);

    client.add_source(&source1, &String::from_str(e, "Chainlink"));
    client.add_source(&source2, &String::from_str(e, "Band"));
    client.register_asset(&asset1);

    (client, admin, source1, asset1)
}

fn ledger_default(e: &Env, seq: u32, timestamp: u64) {
    e.ledger().set(LedgerInfo {
        timestamp,
        protocol_version: 26,
        sequence_number: seq,
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 10,
        min_persistent_entry_ttl: 10,
        max_entry_ttl: 4096,
    });
}

#[test]
fn test_initialize() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_contract(&e);

    client.initialize(
        &admin,
        &2u32,
        &10u32,
        &18u32,
        &String::from_str(&e, "Stellar Price Oracle Aggregator"),
    );

    assert_eq!(client.get_admin_address(), admin);
    assert_eq!(client.get_min_sources_required(), 2u32);
    assert_eq!(client.get_max_history_length(), 10u32);
    assert_eq!(client.get_decimals(), 18u32);
    assert_eq!(
        client.get_description(),
        String::from_str(&e, "Stellar Price Oracle Aggregator")
    );
}

#[test]
fn test_initialize_defaults() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_contract(&e);

    client.initialize(&admin, &0u32, &0u32, &6u32, &String::from_str(&e, "Test"));

    assert_eq!(client.get_min_sources_required(), 1u32);
    assert_eq!(client.get_max_history_length(), 100u32);
    assert_eq!(client.get_decimals(), 6u32);
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn test_initialize_twice() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_contract(&e);

    client.initialize(&admin, &2u32, &10u32, &18u32, &String::from_str(&e, "Test"));
    client.initialize(&admin, &2u32, &10u32, &18u32, &String::from_str(&e, "Test"));
}

#[test]
fn test_set_admin() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let new_admin = Address::generate(&e);
    client.set_admin(&new_admin);
    assert_eq!(client.get_admin_address(), new_admin);
}

#[test]
fn test_set_admin_unauthorized() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let new_admin = Address::generate(&e);
    clear_auth(&e);
    assert!(client.try_set_admin(&new_admin).is_err());
}

#[test]
fn test_admin_functions() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    client.set_min_sources_required(&3u32);
    assert_eq!(client.get_min_sources_required(), 3u32);

    client.set_max_history_length(&50u32);
    assert_eq!(client.get_max_history_length(), 50u32);

    client.set_decimals(&8u32);
    assert_eq!(client.get_decimals(), 8u32);

    client.set_description(&String::from_str(&e, "Updated Description"));
    assert_eq!(
        client.get_description(),
        String::from_str(&e, "Updated Description")
    );
}

#[test]
fn test_register_asset() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let asset = Address::generate(&e);
    client.register_asset(&asset);
    assert!(client.is_asset_registered(&asset));
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn test_register_asset_twice() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let asset = Address::generate(&e);
    client.register_asset(&asset);
    client.register_asset(&asset);
}

#[test]
fn test_unregister_asset() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let asset = Address::generate(&e);
    client.register_asset(&asset);
    assert!(client.is_asset_registered(&asset));

    client.unregister_asset(&asset);
    assert!(!client.is_asset_registered(&asset));
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn test_unregister_unregistered_asset() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let asset = Address::generate(&e);
    client.unregister_asset(&asset);
}

#[test]
fn test_add_remove_source() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let source = Address::generate(&e);
    client.add_source(&source, &String::from_str(&e, "Chainlink"));
    assert!(client.is_source(&source));

    client.remove_source(&source);
    assert!(!client.is_source(&source));
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_add_source_twice() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let source = Address::generate(&e);
    client.add_source(&source, &String::from_str(&e, "Chainlink"));
    client.add_source(&source, &String::from_str(&e, "Chainlink"));
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn test_remove_nonexistent_source() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let source = Address::generate(&e);
    client.remove_source(&source);
}

#[test]
fn test_get_oracle_sources() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let source1 = Address::generate(&e);
    let source2 = Address::generate(&e);

    client.add_source(&source1, &String::from_str(&e, "Chainlink"));
    client.add_source(&source2, &String::from_str(&e, "Band"));

    let sources = client.get_oracle_sources();
    assert_eq!(sources.sources.len(), 2);
    assert_eq!(
        sources.metadata.get(source1.clone()).unwrap(),
        String::from_str(&e, "Chainlink")
    );
    assert_eq!(
        sources.metadata.get(source2.clone()).unwrap(),
        String::from_str(&e, "Band")
    );
}

#[test]
fn test_submit_price_and_get_price() {
    let e = Env::default();
    ledger_default(&e, 100, 1234567890);

    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let source1 = Address::generate(&e);
    let source2 = Address::generate(&e);

    client.add_source(&source1, &String::from_str(&e, "Chainlink"));
    client.add_source(&source2, &String::from_str(&e, "Band"));
    client.set_min_sources_required(&2u32);

    let asset = Address::generate(&e);
    client.register_asset(&asset);

    client.submit_price(&source1, &asset, &100i128, &1234567890);

    let price = client.get_price(&asset);
    assert_eq!(price.price, 0i128);
    assert_eq!(price.num_sources, 0u32);

    client.submit_price(&source2, &asset, &110i128, &1234567890);

    let price = client.get_price(&asset);
    assert_eq!(price.price, 105i128);
    assert_eq!(price.num_sources, 2u32);
    assert_eq!(price.timestamp, 1234567890u64);
    assert_eq!(price.decimals, 18u32);
}

#[test]
fn test_submit_price_median_odd() {
    let e = Env::default();
    ledger_default(&e, 100, 1234567890);

    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let source1 = Address::generate(&e);
    let source2 = Address::generate(&e);
    let source3 = Address::generate(&e);

    client.add_source(&source1, &String::from_str(&e, "Chainlink"));
    client.add_source(&source2, &String::from_str(&e, "Band"));
    client.add_source(&source3, &String::from_str(&e, "Redstone"));
    client.set_min_sources_required(&3u32);

    let asset = Address::generate(&e);
    client.register_asset(&asset);

    client.submit_price(&source1, &asset, &100i128, &1234567890);
    client.submit_price(&source2, &asset, &200i128, &1234567890);
    client.submit_price(&source3, &asset, &300i128, &1234567890);

    let price = client.get_price(&asset);
    assert_eq!(price.price, 200i128);
    assert_eq!(price.num_sources, 3u32);
}

#[test]
fn test_submit_price_median_even() {
    let e = Env::default();
    ledger_default(&e, 100, 1234567890);

    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let source1 = Address::generate(&e);
    let source2 = Address::generate(&e);
    let source3 = Address::generate(&e);
    let source4 = Address::generate(&e);

    client.add_source(&source1, &String::from_str(&e, "A"));
    client.add_source(&source2, &String::from_str(&e, "B"));
    client.add_source(&source3, &String::from_str(&e, "C"));
    client.add_source(&source4, &String::from_str(&e, "D"));
    client.set_min_sources_required(&4u32);

    let asset = Address::generate(&e);
    client.register_asset(&asset);

    client.submit_price(&source1, &asset, &100i128, &1234567890);
    client.submit_price(&source2, &asset, &200i128, &1234567890);
    client.submit_price(&source3, &asset, &300i128, &1234567890);
    client.submit_price(&source4, &asset, &400i128, &1234567890);

    let price = client.get_price(&asset);
    assert_eq!(price.price, 250i128);
    assert_eq!(price.num_sources, 4u32);
}

#[test]
#[should_panic(expected = "Error(Contract, #0)")]
fn test_submit_price_unauthorized_source() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let fake_source = Address::generate(&e);
    let asset = Address::generate(&e);

    client.register_asset(&asset);

    client.submit_price(&fake_source, &asset, &100i128, &1234567890);
}

#[test]
#[should_panic(expected = "Error(Contract, #7)")]
fn test_submit_price_invalid_zero() {
    let e = Env::default();
    let (client, _admin, source1, asset1) = setup_basic(&e);

    client.submit_price(&source1, &asset1, &0i128, &1234567890);
}

#[test]
#[should_panic(expected = "Error(Contract, #7)")]
fn test_submit_price_invalid_negative() {
    let e = Env::default();
    let (client, _admin, source1, asset1) = setup_basic(&e);

    client.submit_price(&source1, &asset1, &(-100i128), &1234567890);
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn test_submit_price_unregistered_asset() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let source1 = Address::generate(&e);
    client.add_source(&source1, &String::from_str(&e, "Chainlink"));

    let unregistered_asset = Address::generate(&e);
    client.submit_price(&source1, &unregistered_asset, &100i128, &1234567890);
}

#[test]
fn test_get_source_price() {
    let e = Env::default();
    let (client, _admin, source1, asset1) = setup_basic(&e);

    client.submit_price(&source1, &asset1, &100i128, &1234567890);

    let entry = client.get_source_price(&asset1, &source1);
    assert_eq!(entry.price, 100i128);
    assert_eq!(entry.timestamp, 1234567890u64);
    assert_eq!(entry.source, source1);
    assert_eq!(entry.decimals, 18u32);
}

#[test]
#[should_panic(expected = "Error(Contract, #0)")]
fn test_get_source_price_nonexistent_source() {
    let e = Env::default();
    let (client, _admin, _source1, asset1) = setup_basic(&e);

    let fake_source = Address::generate(&e);
    client.get_source_price(&asset1, &fake_source);
}

#[test]
fn test_get_all_prices() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let source1 = Address::generate(&e);
    let source2 = Address::generate(&e);

    client.add_source(&source1, &String::from_str(&e, "Chainlink"));
    client.add_source(&source2, &String::from_str(&e, "Band"));
    client.set_min_sources_required(&2u32);

    let asset = Address::generate(&e);
    client.register_asset(&asset);

    client.submit_price(&source1, &asset, &100i128, &1234567890);
    client.submit_price(&source2, &asset, &200i128, &1234567890);

    let all_prices = client.get_all_prices(&asset);
    assert_eq!(all_prices.len(), 2);

    let price0: PriceEntry = all_prices.get_unchecked(0);
    let price1: PriceEntry = all_prices.get_unchecked(1);
    assert_eq!(price0.price, 100i128);
    assert_eq!(price1.price, 200i128);
}

#[test]
fn test_get_latest_ledger() {
    let e = Env::default();
    ledger_default(&e, 42, 1234567890);

    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);
    assert_eq!(client.get_latest_ledger(), 42u32);
}

#[test]
fn test_get_price_no_data() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let asset = Address::generate(&e);
    client.register_asset(&asset);

    let price = client.get_price(&asset);
    assert_eq!(price.price, 0i128);
    assert_eq!(price.num_sources, 0u32);
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn test_get_price_unregistered_asset() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let asset = Address::generate(&e);
    client.get_price(&asset);
}

#[test]
fn test_historical_prices() {
    let e = Env::default();
    ledger_default(&e, 100, 1234567890);

    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let source1 = Address::generate(&e);
    let source2 = Address::generate(&e);

    client.add_source(&source1, &String::from_str(&e, "Chainlink"));
    client.add_source(&source2, &String::from_str(&e, "Band"));
    client.set_min_sources_required(&2u32);

    let asset = Address::generate(&e);
    client.register_asset(&asset);

    client.submit_price(&source1, &asset, &100i128, &1234567890);
    client.submit_price(&source2, &asset, &110i128, &1234567890);

    assert!(client.has_historical_price(&asset, &100u32));

    let history = client.get_historical_price(&asset, &100u32);
    assert_eq!(history.price, 105i128);
    assert_eq!(history.ledger, 100u32);
    assert_eq!(history.num_sources, 2u32);

    let history_range = client.get_historical_prices(&asset, &100u32, &100u32);
    assert_eq!(history_range.len(), 1);

    let empty_range = client.get_historical_prices(&asset, &101u32, &110u32);
    assert_eq!(empty_range.len(), 0);
}

#[test]
fn test_historical_prices_multiple() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let source1 = Address::generate(&e);
    let source2 = Address::generate(&e);
    let source3 = Address::generate(&e);

    client.add_source(&source1, &String::from_str(&e, "Chainlink"));
    client.add_source(&source2, &String::from_str(&e, "Band"));
    client.add_source(&source3, &String::from_str(&e, "Redstone"));
    client.set_min_sources_required(&3u32);

    let asset = Address::generate(&e);
    client.register_asset(&asset);

    ledger_default(&e, 100, 1234567890);
    client.submit_price(&source1, &asset, &100i128, &1234567890);
    client.submit_price(&source2, &asset, &200i128, &1234567890);
    client.submit_price(&source3, &asset, &300i128, &1234567890);

    ledger_default(&e, 101, 1234567891);
    client.submit_price(&source1, &asset, &110i128, &1234567891);
    client.submit_price(&source2, &asset, &210i128, &1234567891);
    client.submit_price(&source3, &asset, &310i128, &1234567891);

    ledger_default(&e, 102, 1234567892);
    client.submit_price(&source1, &asset, &120i128, &1234567892);
    client.submit_price(&source2, &asset, &220i128, &1234567892);
    client.submit_price(&source3, &asset, &320i128, &1234567892);

    let history_range = client.get_historical_prices(&asset, &100u32, &102u32);
    assert_eq!(history_range.len(), 3);
    assert_eq!(history_range.get_unchecked(0).price, 200i128);
    assert_eq!(history_range.get_unchecked(1).price, 210i128);
    assert_eq!(history_range.get_unchecked(2).price, 220i128);
}

#[test]
fn test_has_historical_price_false() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let asset = Address::generate(&e);
    client.register_asset(&asset);

    assert!(!client.has_historical_price(&asset, &999u32));
}

#[test]
fn test_has_historical_price_unregistered_asset() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let asset = Address::generate(&e);
    assert!(!client.has_historical_price(&asset, &100u32));
}

#[test]
fn test_upgrade() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let wasm = include_bytes!("../../../target/wasm32v1-none/release/price_oracle.wasm");
    let new_wasm_hash = e
        .deployer()
        .upload_contract_wasm(Bytes::from_slice(&e, wasm));
    client.upgrade(&new_wasm_hash);
}

#[test]
fn test_upgrade_unauthorized() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let wasm = include_bytes!("../../../target/wasm32v1-none/release/price_oracle.wasm");
    let new_wasm_hash = e
        .deployer()
        .upload_contract_wasm(Bytes::from_slice(&e, wasm));
    clear_auth(&e);
    assert!(client.try_upgrade(&new_wasm_hash).is_err());
}

#[test]
fn test_unauthorized_add_source() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let source = Address::generate(&e);
    clear_auth(&e);
    assert!(client
        .try_add_source(&source, &String::from_str(&e, "Bad Source"))
        .is_err());
}

#[test]
fn test_unauthorized_remove_source() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let source = Address::generate(&e);
    client.add_source(&source, &String::from_str(&e, "Test"));

    clear_auth(&e);
    assert!(client.try_remove_source(&source).is_err());
}

#[test]
fn test_unauthorized_set_min_sources() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    clear_auth(&e);
    assert!(client.try_set_min_sources_required(&5u32).is_err());
}

#[test]
fn test_unauthorized_set_max_history() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    clear_auth(&e);
    assert!(client.try_set_max_history_length(&50u32).is_err());
}

#[test]
fn test_unauthorized_set_decimals() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    clear_auth(&e);
    assert!(client.try_set_decimals(&8u32).is_err());
}

#[test]
fn test_unauthorized_set_description() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    clear_auth(&e);
    assert!(client
        .try_set_description(&String::from_str(&e, "Hacked"))
        .is_err());
}

#[test]
fn test_multiple_assets() {
    let e = Env::default();
    ledger_default(&e, 100, 1234567890);

    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let source1 = Address::generate(&e);
    let source2 = Address::generate(&e);

    client.add_source(&source1, &String::from_str(&e, "Chainlink"));
    client.add_source(&source2, &String::from_str(&e, "Band"));
    client.set_min_sources_required(&2u32);

    let xlm = Address::generate(&e);
    let eth = Address::generate(&e);
    let btc = Address::generate(&e);

    client.register_asset(&xlm);
    client.register_asset(&eth);
    client.register_asset(&btc);

    client.submit_price(&source1, &xlm, &100i128, &1234567890);
    client.submit_price(&source2, &xlm, &102i128, &1234567890);

    client.submit_price(&source1, &eth, &180000i128, &1234567890);
    client.submit_price(&source2, &eth, &181000i128, &1234567890);

    client.submit_price(&source1, &btc, &30000000i128, &1234567890);
    client.submit_price(&source2, &btc, &31000000i128, &1234567890);

    let xlm_price = client.get_price(&xlm);
    assert_eq!(xlm_price.price, 101i128);

    let eth_price = client.get_price(&eth);
    assert_eq!(eth_price.price, 180500i128);

    let btc_price = client.get_price(&btc);
    assert_eq!(btc_price.price, 30500000i128);
}

#[test]
fn test_submit_price_updates_timestamp() {
    let e = Env::default();
    ledger_default(&e, 100, 1234567890);

    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let source1 = Address::generate(&e);
    let source2 = Address::generate(&e);

    client.add_source(&source1, &String::from_str(&e, "Chainlink"));
    client.add_source(&source2, &String::from_str(&e, "Band"));
    client.set_min_sources_required(&2u32);

    let asset = Address::generate(&e);
    client.register_asset(&asset);

    client.submit_price(&source1, &asset, &100i128, &1000u64);
    client.submit_price(&source2, &asset, &110i128, &2000u64);

    let price = client.get_price(&asset);
    assert_eq!(price.timestamp, 2000u64);

    client.submit_price(&source2, &asset, &120i128, &3000u64);

    let price = client.get_price(&asset);
    assert_eq!(price.timestamp, 3000u64);
}

#[test]
fn test_single_source_no_aggregation() {
    let e = Env::default();
    ledger_default(&e, 100, 1234567890);

    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let source1 = Address::generate(&e);

    client.add_source(&source1, &String::from_str(&e, "Chainlink"));
    client.set_min_sources_required(&1u32);

    let asset = Address::generate(&e);
    client.register_asset(&asset);

    client.submit_price(&source1, &asset, &100i128, &1234567890);

    let price = client.get_price(&asset);
    assert_eq!(price.price, 100i128);
    assert_eq!(price.num_sources, 1u32);
}

#[test]
fn test_price_source_not_affected_by_other_assets() {
    let e = Env::default();
    ledger_default(&e, 100, 1234567890);

    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let source1 = Address::generate(&e);
    let source2 = Address::generate(&e);

    client.add_source(&source1, &String::from_str(&e, "Chainlink"));
    client.add_source(&source2, &String::from_str(&e, "Band"));
    client.set_min_sources_required(&2u32);

    let asset_a = Address::generate(&e);
    let asset_b = Address::generate(&e);
    client.register_asset(&asset_a);
    client.register_asset(&asset_b);

    client.submit_price(&source1, &asset_a, &100i128, &1234567890);
    client.submit_price(&source2, &asset_a, &110i128, &1234567890);

    let price_a = client.get_price(&asset_a);
    assert_eq!(price_a.price, 105i128);

    let price_b = client.get_price(&asset_b);
    assert_eq!(price_b.price, 0i128);
}

// ---- SEP-40 Oracle Interface Tests ----

#[test]
fn test_sep40_base() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let result = client.base();
    assert_eq!(result, Asset::Other(Symbol::new(&e, "USD")));
}

#[test]
fn test_sep40_assets() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let asset1 = Address::generate(&e);
    let asset2 = Address::generate(&e);
    client.register_asset(&asset1);
    client.register_asset(&asset2);

    let assets = client.assets();
    assert_eq!(assets.len(), 2);
    assert_eq!(assets.get_unchecked(0), Asset::Stellar(asset1));
    assert_eq!(assets.get_unchecked(1), Asset::Stellar(asset2));
}

#[test]
fn test_sep40_resolution() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    assert_eq!(client.resolution(), 0u32);

    client.set_resolution(&300u32);
    assert_eq!(client.resolution(), 300u32);
}

#[test]
fn test_sep40_lastprice() {
    let e = Env::default();
    ledger_default(&e, 100, 1234567890);

    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let source1 = Address::generate(&e);
    let source2 = Address::generate(&e);
    client.add_source(&source1, &String::from_str(&e, "Chainlink"));
    client.add_source(&source2, &String::from_str(&e, "Band"));
    client.set_min_sources_required(&2u32);

    let asset = Address::generate(&e);
    client.register_asset(&asset);

    client.submit_price(&source1, &asset, &100i128, &1234567890);
    client.submit_price(&source2, &asset, &110i128, &1234567890);

    let result = client.lastprice(&Asset::Stellar(asset));
    assert!(result.is_some());
    let data: PriceData = result.unwrap();
    assert_eq!(data.price, 105i128);
    assert_eq!(data.timestamp, 1234567890u64);
}

#[test]
fn test_sep40_lastprice_unregistered() {
    let e = Env::default();
    ledger_default(&e, 100, 1234567890);

    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let unregistered = Address::generate(&e);
    let result = client.lastprice(&Asset::Stellar(unregistered));
    assert!(result.is_none());
}

#[test]
fn test_sep40_lastprice_other() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let result = client.lastprice(&Asset::Other(Symbol::new(&e, "EUR")));
    assert!(result.is_none());
}

#[test]
fn test_sep40_lastprice_stale() {
    let e = Env::default();
    ledger_default(&e, 100, 1234567890);

    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let source1 = Address::generate(&e);
    client.add_source(&source1, &String::from_str(&e, "Chainlink"));
    client.set_min_sources_required(&1u32);
    client.set_resolution(&10u32);

    let asset = Address::generate(&e);
    client.register_asset(&asset);

    client.submit_price(&source1, &asset, &100i128, &1234567890);

    // Advance ledger past resolution window
    ledger_default(&e, 200, 1234567910);
    let result = client.lastprice(&Asset::Stellar(asset));
    assert!(result.is_none());
}

#[test]
fn test_sep40_price() {
    let e = Env::default();
    ledger_default(&e, 100, 1234567890);

    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let source1 = Address::generate(&e);
    let source2 = Address::generate(&e);
    client.add_source(&source1, &String::from_str(&e, "Chainlink"));
    client.add_source(&source2, &String::from_str(&e, "Band"));
    client.set_min_sources_required(&2u32);

    let asset = Address::generate(&e);
    client.register_asset(&asset);

    client.submit_price(&source1, &asset, &100i128, &1234567890);
    client.submit_price(&source2, &asset, &110i128, &1234567890);

    let result = client.price(&Asset::Stellar(asset), &1234567890u64);
    assert!(result.is_some());
    let data: PriceData = result.unwrap();
    assert_eq!(data.price, 105i128);
}

#[test]
fn test_sep40_price_wrong_timestamp() {
    let e = Env::default();
    // Keep ledger low so history back-scan stays under footprint limit (100)
    ledger_default(&e, 50, 1000);

    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let source1 = Address::generate(&e);
    client.add_source(&source1, &String::from_str(&e, "Chainlink"));
    client.set_min_sources_required(&1u32);

    let asset = Address::generate(&e);
    client.register_asset(&asset);

    client.submit_price(&source1, &asset, &100i128, &1000);

    // Query with timestamp before data exists → should find no match
    let result = client.price(&Asset::Stellar(asset), &999u64);
    assert!(result.is_none());
}

#[test]
fn test_sep40_price_other() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let result = client.price(&Asset::Other(Symbol::new(&e, "BTC")), &1234567890u64);
    assert!(result.is_none());
}

#[test]
fn test_sep40_prices() {
    let e = Env::default();
    ledger_default(&e, 100, 1234567890);

    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let source1 = Address::generate(&e);
    let source2 = Address::generate(&e);
    let source3 = Address::generate(&e);

    client.add_source(&source1, &String::from_str(&e, "Chainlink"));
    client.add_source(&source2, &String::from_str(&e, "Band"));
    client.add_source(&source3, &String::from_str(&e, "Redstone"));
    client.set_min_sources_required(&3u32);

    let asset = Address::generate(&e);
    client.register_asset(&asset);

    client.submit_price(&source1, &asset, &100i128, &1234567890);
    client.submit_price(&source2, &asset, &200i128, &1234567890);
    client.submit_price(&source3, &asset, &300i128, &1234567890);

    let result = client.prices(&Asset::Stellar(asset), &5u32);
    assert!(result.is_some());
    let prices: Vec<PriceData> = result.unwrap();
    assert!(prices.len() >= 1);
    assert_eq!(prices.get_unchecked(0).price, 200i128);
}

#[test]
fn test_sep40_prices_empty() {
    let e = Env::default();
    ledger_default(&e, 100, 1234567890);

    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let asset = Address::generate(&e);
    client.register_asset(&asset);

    let result = client.prices(&Asset::Stellar(asset), &5u32);
    assert!(result.is_some());
    let prices: Vec<PriceData> = result.unwrap();
    // Falls back to aggregate entry with price 0 when no history exists
    assert_eq!(prices.len(), 1);
    assert_eq!(prices.get_unchecked(0).price, 0i128);
}

#[test]
fn test_sep40_prices_unregistered_asset() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let unregistered = Address::generate(&e);
    let result = client.prices(&Asset::Stellar(unregistered), &5u32);
    assert!(result.is_none());
}

// ---- Task 1: Duplicate Submission Detection Tests ----

#[test]
fn test_duplicate_submission_same_ledger_updates_price() {
    let e = Env::default();
    ledger_default(&e, 100, 1234567890);
    let (client, _admin, source1, asset1) = setup_basic(&e);

    // First submission
    client.submit_price(&source1, &asset1, &100i128, &1234567890);
    let entry = client.get_source_price(&asset1, &source1);
    assert_eq!(entry.price, 100i128);

    // Second submission same ledger — should update
    client.submit_price(&source1, &asset1, &200i128, &1234567890);
    let entry = client.get_source_price(&asset1, &source1);
    assert_eq!(entry.price, 200i128);
}

#[test]
fn test_duplicate_submission_different_ledger_no_dedup_event() {
    let e = Env::default();
    let (client, _admin, source1, asset1) = setup_basic(&e);

    ledger_default(&e, 100, 1000);
    client.submit_price(&source1, &asset1, &100i128, &1000);

    // Different ledger — not a duplicate
    ledger_default(&e, 101, 2000);
    client.submit_price(&source1, &asset1, &200i128, &2000);
    let entry = client.get_source_price(&asset1, &source1);
    assert_eq!(entry.price, 200i128);
}

#[test]
fn test_duplicate_submission_dedup_reflected_in_aggregate() {
    let e = Env::default();
    ledger_default(&e, 100, 1234567890);

    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let source1 = Address::generate(&e);
    let source2 = Address::generate(&e);
    client.add_source(&source1, &String::from_str(&e, "A"));
    client.add_source(&source2, &String::from_str(&e, "B"));
    client.set_min_sources_required(&2u32);

    let asset = Address::generate(&e);
    client.register_asset(&asset);

    client.submit_price(&source1, &asset, &100i128, &1234567890);
    client.submit_price(&source2, &asset, &200i128, &1234567890);

    let price = client.get_price(&asset);
    assert_eq!(price.price, 150i128);

    // Duplicate from source1 with updated price — aggregate should change
    client.submit_price(&source1, &asset, &300i128, &1234567890);
    let price = client.get_price(&asset);
    assert_eq!(price.price, 250i128); // median of [200, 300]
}

#[test]
fn test_duplicate_submission_multiple_sources_same_ledger() {
    let e = Env::default();
    ledger_default(&e, 50, 500);
    let (client, _admin, source1, asset1) = setup_basic(&e);

    // source1 submits twice in same ledger
    client.submit_price(&source1, &asset1, &100i128, &500);
    client.submit_price(&source1, &asset1, &150i128, &500);
    client.submit_price(&source1, &asset1, &175i128, &500);

    let entry = client.get_source_price(&asset1, &source1);
    assert_eq!(entry.price, 175i128);
}

// ---- Task 2: Empty Asset Registry Edge Case Tests ----

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn test_get_price_empty_registry() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let asset = Address::generate(&e);
    client.get_price(&asset);
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn test_submit_price_empty_registry() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let source = Address::generate(&e);
    client.add_source(&source, &String::from_str(&e, "S"));

    let asset = Address::generate(&e);
    client.submit_price(&source, &asset, &100i128, &1000);
}

#[test]
fn test_is_asset_registered_empty_registry() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let asset1 = Address::generate(&e);
    let asset2 = Address::generate(&e);
    assert!(!client.is_asset_registered(&asset1));
    assert!(!client.is_asset_registered(&asset2));
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn test_get_all_prices_empty_registry() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let asset = Address::generate(&e);
    client.get_all_prices(&asset);
}

#[test]
fn test_sep40_assets_empty_registry() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let assets = client.assets();
    assert_eq!(assets.len(), 0);
}

#[test]
fn test_sep40_lastprice_empty_registry() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let asset = Address::generate(&e);
    let result = client.lastprice(&Asset::Stellar(asset));
    assert!(result.is_none());
}

// ---- Task 3: i128 Boundary Value Tests ----

#[test]
fn test_i128_max_single_source() {
    let e = Env::default();
    ledger_default(&e, 100, 1000);

    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let source = Address::generate(&e);
    client.add_source(&source, &String::from_str(&e, "S"));
    client.set_min_sources_required(&1u32);

    let asset = Address::generate(&e);
    client.register_asset(&asset);

    client.submit_price(&source, &asset, &i128::MAX, &1000);
    let price = client.get_price(&asset);
    assert_eq!(price.price, i128::MAX);
}

#[test]
fn test_i128_max_even_sources_no_overflow() {
    // median of (MAX-1, MAX) = (MAX-1) + (MAX - (MAX-1)) / 2 = MAX-1 + 1/2 = MAX-1
    // overflow-safe: a + (b-a)/2  where a=MAX-1, b=MAX  => MAX-1 + 1/2 = MAX-1
    let e = Env::default();
    ledger_default(&e, 100, 1000);

    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let source1 = Address::generate(&e);
    let source2 = Address::generate(&e);
    client.add_source(&source1, &String::from_str(&e, "A"));
    client.add_source(&source2, &String::from_str(&e, "B"));
    client.set_min_sources_required(&2u32);

    let asset = Address::generate(&e);
    client.register_asset(&asset);

    client.submit_price(&source1, &asset, &(i128::MAX - 1), &1000);
    client.submit_price(&source2, &asset, &i128::MAX, &1000);

    let price = client.get_price(&asset);
    // overflow-safe: (MAX-1) + (MAX - (MAX-1)) / 2 = MAX-1 + 0 = MAX-1
    assert_eq!(price.price, i128::MAX - 1);
}

#[test]
fn test_i128_large_values_median_odd() {
    let e = Env::default();
    ledger_default(&e, 100, 1000);

    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let source1 = Address::generate(&e);
    let source2 = Address::generate(&e);
    let source3 = Address::generate(&e);
    client.add_source(&source1, &String::from_str(&e, "A"));
    client.add_source(&source2, &String::from_str(&e, "B"));
    client.add_source(&source3, &String::from_str(&e, "C"));
    client.set_min_sources_required(&3u32);

    let asset = Address::generate(&e);
    client.register_asset(&asset);

    let large = i128::MAX / 2;
    client.submit_price(&source1, &asset, &1i128, &1000);
    client.submit_price(&source2, &asset, &large, &1000);
    client.submit_price(&source3, &asset, &i128::MAX, &1000);

    let price = client.get_price(&asset);
    assert_eq!(price.price, large);
}

#[test]
fn test_i128_mixed_extreme_and_normal_even() {
    // median of (1, 1000, MAX/2, MAX) — even count
    // sorted: [1, 1000, MAX/2, MAX]
    // median = overflow_safe_mid(1000, MAX/2)
    let e = Env::default();
    ledger_default(&e, 100, 1000);

    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let source1 = Address::generate(&e);
    let source2 = Address::generate(&e);
    let source3 = Address::generate(&e);
    let source4 = Address::generate(&e);
    client.add_source(&source1, &String::from_str(&e, "A"));
    client.add_source(&source2, &String::from_str(&e, "B"));
    client.add_source(&source3, &String::from_str(&e, "C"));
    client.add_source(&source4, &String::from_str(&e, "D"));
    client.set_min_sources_required(&4u32);

    let asset = Address::generate(&e);
    client.register_asset(&asset);

    let half_max = i128::MAX / 2;
    client.submit_price(&source1, &asset, &1i128, &1000);
    client.submit_price(&source2, &asset, &1000i128, &1000);
    client.submit_price(&source3, &asset, &half_max, &1000);
    client.submit_price(&source4, &asset, &i128::MAX, &1000);

    let price = client.get_price(&asset);
    // median of [1, 1000, half_max, MAX]: overflow-safe mid(1000, half_max)
    let expected = 1000i128 + (half_max - 1000i128) / 2;
    assert_eq!(price.price, expected);
}

#[test]
fn test_i128_boundary_single_value_one() {
    let e = Env::default();
    ledger_default(&e, 100, 1000);

    let admin = Address::generate(&e);
    let client = create_contract(&e);
    init_admin(&client, &admin);

    let source = Address::generate(&e);
    client.add_source(&source, &String::from_str(&e, "S"));
    client.set_min_sources_required(&1u32);

    let asset = Address::generate(&e);
    client.register_asset(&asset);

    client.submit_price(&source, &asset, &1i128, &1000);
    let price = client.get_price(&asset);
    assert_eq!(price.price, 1i128);
}

// ---- Task 4: Upgrade State Preservation Test ----

#[test]
fn test_upgrade_preserves_state() {
    let e = Env::default();
    ledger_default(&e, 100, 1234567890);

    let admin = Address::generate(&e);
    let client = create_contract(&e);
    client.initialize(
        &admin,
        &2u32,
        &50u32,
        &8u32,
        &String::from_str(&e, "Oracle"),
    );

    let source1 = Address::generate(&e);
    let source2 = Address::generate(&e);
    client.add_source(&source1, &String::from_str(&e, "Chainlink"));
    client.add_source(&source2, &String::from_str(&e, "Band"));

    let asset = Address::generate(&e);
    client.register_asset(&asset);

    client.submit_price(&source1, &asset, &100i128, &1234567890);
    client.submit_price(&source2, &asset, &200i128, &1234567890);

    // Perform upgrade
    let wasm = include_bytes!("../../../target/wasm32v1-none/release/price_oracle.wasm");
    let new_wasm_hash = e
        .deployer()
        .upload_contract_wasm(soroban_sdk::Bytes::from_slice(&e, wasm));
    client.upgrade(&new_wasm_hash);

    // Verify admin preserved
    assert_eq!(client.get_admin_address(), admin);

    // Verify sources preserved
    assert!(client.is_source(&source1));
    assert!(client.is_source(&source2));
    let sources = client.get_oracle_sources();
    assert_eq!(sources.sources.len(), 2);

    // Verify asset preserved
    assert!(client.is_asset_registered(&asset));

    // Verify config preserved
    assert_eq!(client.get_min_sources_required(), 2u32);
    assert_eq!(client.get_max_history_length(), 50u32);
    assert_eq!(client.get_decimals(), 8u32);
    assert_eq!(client.get_description(), String::from_str(&e, "Oracle"));

    // Verify price history preserved
    let price = client.get_price(&asset);
    assert_eq!(price.price, 150i128);
    assert_eq!(price.num_sources, 2u32);

    assert!(client.has_historical_price(&asset, &100u32));
    let history = client.get_historical_price(&asset, &100u32);
    assert_eq!(history.price, 150i128);
}
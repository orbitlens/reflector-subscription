#![cfg(test)]

use super::*;
use soroban_sdk::{
    symbol_short, testutils::{Address as _, Events, Ledger, LedgerInfo}, token::StellarAssetClient, vec, Bytes, Env, IntoVal, String
};
use types::{
    asset::Asset, config_data::ConfigData, create_subscription::CreateSubscription,
    ticker_asset::TickerAsset,
};

fn init_contract_with_admin<'a>() -> (Env, SubscriptionContractClient<'a>, ConfigData) {
    let env = Env::default();

    let admin = Address::generate(&env);

    let contract_id = env.register_contract(None, SubscriptionContract);
    let client: SubscriptionContractClient<'a> =
        SubscriptionContractClient::new(&env, &contract_id);

    let token = env.register_stellar_asset_contract(admin.clone());

    let init_data = ConfigData {
        admin: admin.clone(),
        token,
        fee: 100,
    };

    env.mock_all_auths();

    //set admin
    client.config(&init_data);

    (env, client, init_data)
}

#[test]
fn test() {
    let (env, client, config) = init_contract_with_admin();

    let owner = Address::generate(&env);

    let token_client = StellarAssetClient::new(&env, &config.token);
    token_client.mint(&owner, &1000);

    let subscription = CreateSubscription {
        owner: owner.clone(),
        asset1: TickerAsset {
            asset: Asset::Other(symbol_short!("BTC")),
            source: String::from_str(&env, "source1"),
        },
        asset2: TickerAsset {
            asset: Asset::Other(symbol_short!("ETH")),
            source: String::from_str(&env, "source2"),
        },
        threshold: 10,
        heartbeat: 5,
        webhook: Bytes::from_array(&env, &[0; 2048]),
    };

    // create subscription
    let (subscription_id, data) = client.create_subscription(&subscription, &200);
    assert!(subscription_id == 1);

    let mut event = (
        client.address.clone(),
        (symbol_short!("SUBS"), symbol_short!("created")).into_val(&env),
        (1u64, data).into_val(&env),
    );
    assert_eq!(
        vec![&env, env.events().all().last().unwrap()],
        vec![&env, event]
    );

    let trigger_hash: BytesN<32> = BytesN::from_array(&env, &[0; 32]);
    // heartbeat subscription
    client.trigger(&1u64, &trigger_hash);
    event = (
        client.address.clone(),
        (symbol_short!("SUBS"), symbol_short!("trigger")).into_val(&env),
        (1u64, trigger_hash).into_val(&env),
    );

    let trigger_event = vec![&env, env.events().all().last().unwrap()];
    assert_eq!(trigger_event, vec![&env, event]);

    // deposit subscription
    client.deposit(&owner, &1, &100);
    event = (
        client.address.clone(),
        (symbol_short!("SUBS"), symbol_short!("deposit")).into_val(&env),
        (1u64, 100u64).into_val(&env),
    );
    assert_eq!(
        vec![&env, env.events().all().last().unwrap()],
        vec![&env, event]
    );

    let mut subs = client.get_subscription(&subscription_id);
    assert_eq!(subs.balance, 200);

    let ledger_info = env.ledger().get();
    env.ledger().set(LedgerInfo {
        timestamp: 86400 * 2,
        ..ledger_info
    });

    // charge subscription
    client.charge(&vec![&env, 1u64]);

    // check balance and status
    subs = client.get_subscription(&subscription_id);
    assert_eq!(subs.balance, 0);
    assert_eq!(subs.status, SubscriptionStatus::Suspended);
    assert_eq!(subs.last_charge, 86400 * 2 * 1000);

    // deposit subscription to renew
    client.deposit(&owner, &1, &200);
    subs = client.get_subscription(&subscription_id);
    assert_eq!(subs.balance, 100); // 100 is activation fee
    assert_eq!(subs.status, SubscriptionStatus::Active);

    // cancel subscription
    client.cancel(&1u64);
    subs = client.get_subscription(&subscription_id);
    assert_eq!(subs.status, SubscriptionStatus::Cancelled);
}

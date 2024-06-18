#![cfg(test)]

use super::*;
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events, Ledger, LedgerInfo},
    token::StellarAssetClient,
    vec, Env, IntoVal, String
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
        webhook: String::from_str(&env, "webhook"),
    };

    // create subscription
    let subscription_id = client.create_subscription(&subscription, &200);
    assert!(subscription_id == 1);
    
    let mut event = (
        client.address.clone(),
        (symbol_short!("SUBS"), symbol_short!("created")).into_val(&env),
        1u64.into_val(&env),
    );
    assert_eq!(vec![&env, env.events().all().last().unwrap()], vec![&env, event]);

    // heartbeat subscription
    client.trigger(&vec![&env, 1], &true);
    event = (
        client.address.clone(),
        (symbol_short!("SUBS"), symbol_short!("heartbeat")).into_val(&env),
        vec![&env, 1u64].into_val(&env),
    );
    assert_eq!(vec![&env, env.events().all().last().unwrap()], vec![&env, event]);

    // trigger subscription
    client.trigger(&vec![&env, 1], &false);
    event = (
        client.address.clone(),
        (symbol_short!("SUBS"), symbol_short!("triggered")).into_val(&env),
        vec![&env, 1u64].into_val(&env),
    );
    assert_eq!(vec![&env, env.events().all().last().unwrap()], vec![&env, event]);

    // deposit subscription
    client.deposit(&owner, &1, &100);
    event = (
        client.address.clone(),
        (symbol_short!("SUBS"), symbol_short!("deposit")).into_val(&env),
        (1u64, 100u64).into_val(&env),
    );
    assert_eq!(vec![&env, env.events().all().last().unwrap()], vec![&env, event]);

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
    assert_eq!(subs.is_active, false);
    assert_eq!(subs.last_charge, 86400 * 2);
}

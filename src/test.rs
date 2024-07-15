#![cfg(test)]

use super::*;
use soroban_sdk::{
    symbol_short, testutils::{storage::Persistent, Address as _, Ledger, LedgerInfo}, token::StellarAssetClient, vec, Bytes, Env, String
};
use types::{
    asset::Asset, contract_config::ContractConfig, subscription_init_params::SubscriptionInitParams,
    ticker_asset::TickerAsset,
};

fn init_contract_with_admin<'a>() -> (Env, SubscriptionContractClient<'a>, ContractConfig) {
    let env = Env::default();

    let admin = Address::generate(&env);

    let contract_id = env.register_contract(None, SubscriptionContract);
    let client: SubscriptionContractClient<'a> =
        SubscriptionContractClient::new(&env, &contract_id);

    let token = env.register_stellar_asset_contract(admin.clone());

    let init_data = ContractConfig {
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
    token_client.mint(&owner, &1200);

    let subscription = SubscriptionInitParams {
        owner: owner.clone(),
        base: TickerAsset {
            asset: Asset::Other(symbol_short!("BTC")),
            source: String::from_str(&env, "source1"),
        },
        quote: TickerAsset {
            asset: Asset::Other(symbol_short!("ETH")),
            source: String::from_str(&env, "source2"),
        },
        threshold: 10,
        heartbeat: 5,
        webhook: Bytes::from_array(&env, &[0; 2048]),
    };

    // create subscription
    let (subscription_id, _) = client.create_subscription(&subscription, &200);
    assert!(subscription_id == 1);

    env.as_contract(&client.address, || {
        let ttl = env.storage().persistent().get_ttl(&subscription_id);
        assert_eq!(ttl, ttl);
    });

    let trigger_hash: BytesN<32> = BytesN::from_array(&env, &[0; 32]);
    // heartbeat subscription
    client.trigger(&1u64, &trigger_hash);

    // deposit subscription
    client.deposit(&owner, &1, &100);

    env.as_contract(&client.address, || {
        let ttl = env.storage().persistent().get_ttl(&subscription_id);
        assert_eq!(ttl, ttl);
    });

    let mut subs = client.get_subscription(&subscription_id);
    assert_eq!(subs.balance, 100);

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
    assert_eq!(subs.updated, 86400 * 2 * 1000);

    // deposit subscription to renew
    client.deposit(&owner, &1, &200);
    subs = client.get_subscription(&subscription_id);
    assert_eq!(subs.balance, 100); // 100 is activation fee
    assert_eq!(subs.status, SubscriptionStatus::Active);

    // cancel subscription
    client.cancel(&1u64);
    env.as_contract(&client.address, || {
        let subs = env.get_subscription(subscription_id);
        assert_eq!(subs, None);
    });  
}

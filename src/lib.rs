#![no_std]

mod extensions;
mod types;

use extensions::env_extensions::EnvExtensions;
use soroban_sdk::{
    contract, contractimpl, panic_with_error, symbol_short, token::TokenClient, Address, Env, Symbol, Vec};
use types::{config_data::ConfigData, create_subscription::CreateSubscription, error::Error, subscription::Subscription};

const SUBS: Symbol = symbol_short!("SUBS");

#[contract]
pub struct SubscriptionContract;

#[contractimpl]
impl SubscriptionContract {
    pub fn init(e: Env, config: ConfigData) {
        config.admin.require_auth();
        if e.is_initialized() {
            e.panic_with_error(Error::AlreadyInitialized);
        }

        e.set_admin(&config.admin);
        e.set_base_fee(config.fee);
        e.set_token(&config.token);
        e.set_last_subscription_id(0);
    }

    pub fn create_subscription(
        e: Env,
        new_subscription: CreateSubscription,
        amount: u64,
    ) -> u64 {
        if !e.is_initialized() {
            e.panic_with_error(Error::NotInitialized);
        }
        if amount < e.get_base_fee() {
            e.panic_with_error(Error::InvalidAmount);
        }
        new_subscription.owner.require_auth();
        let token = TokenClient::new(&e, &e.get_token());
        token.transfer(
            &new_subscription.owner,
            &e.current_contract_address(),
            &(amount as i128),
        );
        //todo: check if the subscription is valid and the amount is enough
        let subscription_id = e.get_last_subscription_id() + 1;
        let subscription = Subscription {
            owner: new_subscription.owner,
            asset1: new_subscription.asset1,
            asset2: new_subscription.asset2,
            threshold: new_subscription.threshold,
            heartbeat: new_subscription.heartbeat,
            webhook: new_subscription.webhook,
            balance: amount,
            last_notification: 0
        };
        e.set_subscription(subscription_id, &subscription);
        e.set_last_subscription_id(subscription_id);
        e.events()
            .publish((SUBS, symbol_short!("created")), subscription_id);
        return subscription_id;
    }

    pub fn trigger(e: Env, subscription_ids: Vec<u64>, is_heartbeat: bool) {
        if !e.is_initialized() {
            e.panic_with_error(Error::NotInitialized);
        }
        let now = e.ledger().timestamp();
        for subscription_id in subscription_ids.iter() {
            if let Some(mut subscription) = e.get_subscription(subscription_id) {
                subscription.last_notification = now;
                e.set_subscription(subscription_id, &subscription);
            }
        }
        let event = if is_heartbeat {
            symbol_short!("heartbeat")
        } else {
            symbol_short!("triggered")
        };
        e.events().publish((SUBS, event), subscription_ids)
    }

    pub fn deposit(e: Env, from: Address, subscription_id: u64, amount: u64) {
        if !e.is_initialized() {
            e.panic_with_error(Error::NotInitialized);
        }
        from.require_auth();
        if amount == 0 {
            e.panic_with_error(Error::InvalidAmount);
        }
        let mut subscription = e.get_subscription(subscription_id).unwrap_or_else(|| panic_with_error!(e, Error::SubscriptionNotFound));
        let token = TokenClient::new(&e, &e.get_token());
        token.transfer(
            &from,
            &e.current_contract_address(),
            &(amount as i128),
        );
        subscription.balance += amount;
        e.set_subscription(subscription_id, &subscription);
        e.events().publish((SUBS, symbol_short!("deposit")), (subscription_id, amount));
    }

    pub fn get_subscription(e: Env, subscription_id: u64) -> Subscription {
        if !e.is_initialized() {
            e.panic_with_error(Error::NotInitialized);
        }
        e.get_subscription(subscription_id).unwrap_or_else(|| panic_with_error!(e, Error::SubscriptionNotFound))
    }
}

mod test;

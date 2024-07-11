#![no_std]

mod extensions;
mod types;

use extensions::env_extensions::EnvExtensions;
use soroban_sdk::{
    contract, contractimpl, panic_with_error, symbol_short, token::TokenClient, Address, BytesN,
    Env, Symbol, Vec,
};
use types::{
    config_data::ConfigData, create_subscription::CreateSubscription, error::Error,
    subscription::Subscription, subscription_status::SubscriptionStatus,
};

const SUBS: Symbol = symbol_short!("SUBS");

// 1 day in milliseconds
const DAY: u64 = 86400 * 1000;

const MAX_WEBHOOK_SIZE: u32 = 2048;

// Minimum fee factor for the activation
const MIN_FEE_FACTOR: u64 = 1;

// Minimum heartbeat in minutes
const MIN_HEARTBEAT: u32 = 5;

#[contract]
pub struct SubscriptionContract;

#[contractimpl]
impl SubscriptionContract {
    // Admin only

    // Initializes the contract. Can be invoked only once.
    //
    // # Arguments
    //
    // * `config` - Contract configuration
    //
    // # Panics
    //
    // Panics if the contract is already initialized
    pub fn config(e: Env, config: ConfigData) {
        config.admin.require_auth();
        if e.is_initialized() {
            e.panic_with_error(Error::AlreadyInitialized);
        }

        e.set_admin(&config.admin);
        e.set_fee(config.fee);
        e.set_token(&config.token);
        e.set_last_subscription_id(0);
    }

    // Sets the base fee for the contract. Can be invoked only by the admin account.
    //
    // # Arguments
    //
    // * `fee` - New base fee
    //
    // # Panics
    //
    // Panics if the caller doesn't match admin address
    pub fn set_fee(e: Env, fee: u64) {
        e.panic_if_not_admin();
        e.set_fee(fee);
    }

    // Triggers the subscription. Can be invoked only by the admin account.
    //
    // # Arguments
    //
    // * `timestamp` - Timestamp of the trigger
    // * `trigger_hash` - Hash of the trigger data
    //
    // # Panics
    //
    // Panics if the caller doesn't match admin address
    pub fn trigger(e: Env, timestamp: u64, trigger_hash: BytesN<32>) {
        e.panic_if_not_admin();
        e.events()
            .publish((SUBS, symbol_short!("trigger")), (timestamp, trigger_hash));
    }

    // Updates the contract source code. Can be invoked only by the admin account.
    //
    // # Arguments
    //
    // * `admin` - Admin account address
    // * `wasm_hash` - WASM hash of the contract source code
    //
    // # Panics
    //
    // Panics if the caller doesn't match admin address
    pub fn update_contract(e: Env, wasm_hash: BytesN<32>) {
        e.panic_if_not_admin();
        e.deployer().update_current_contract_wasm(wasm_hash)
    }

    // Withdraws funds from the contract, and updates balance of subscriptions. Can be invoked only by the admin account.
    //
    // # Arguments
    //
    // * `subscription_ids` - Subscription ID
    //
    // # Panics
    //
    // Panics if the caller doesn't match admin address
    pub fn charge(e: Env, subscription_ids: Vec<u64>) {
        e.panic_if_not_admin();
        let mut total_charge: u64 = 0;
        let now = now(&e);
        let fee = e.get_fee();
        let mut deactivated_subscriptions = Vec::new(&e);
        for subscription_id in subscription_ids.iter() {
            if let Some(mut subscription) = e.get_subscription(subscription_id) {
                let days = (now - subscription.last_charge) / DAY;
                if days == 0 {
                    continue;
                }
                let mut charge = days * fee;
                if subscription.balance < charge {
                    charge = subscription.balance;
                }
                subscription.balance -= charge;
                subscription.last_charge = now;
                if subscription.balance < fee {
                    // Deactivate the subscription if the balance is less than the fee
                    subscription.status = SubscriptionStatus::Suspended;
                    deactivated_subscriptions.push_back(subscription_id);
                }
                e.set_subscription(subscription_id, &subscription);

                total_charge += charge;
            }
        }
        // If there is nothing to charge, return
        if total_charge == 0 {
            return;
        }
        //Publish the events
        e.events()
            .publish((SUBS, symbol_short!("charged")), (now, subscription_ids));

        if !deactivated_subscriptions.is_empty() {
            e.events().publish(
                (SUBS, symbol_short!("suspended")),
                deactivated_subscriptions,
            );
        }

        //Burn the tokens
        get_token_client(&e).burn(&e.current_contract_address(), &(total_charge as i128));
    }

    // Public

    // Creates a new subscription.
    //
    // # Arguments
    //
    // * `new_subscription` - Subscription data
    // * `amount` - Initial deposit amount
    //
    // # Returns
    //
    // Subscription ID
    //
    // # Panics
    //
    // Panics if the contract is not initialized
    // Panics if the amount is less than the base fee
    // Panics if the caller doesn't match the owner address
    // Panics if the token transfer fails
    // Panics if the subscription is invalid
    pub fn create_subscription(
        e: Env,
        new_subscription: CreateSubscription,
        amount: u64,
    ) -> (u64, Subscription) {
        panin_if_not_initialized(&e);
        // Check the authorization
        new_subscription.owner.require_auth();

        // Check the amount
        let activation_fee = get_activation_fee(&e);
        if amount < activation_fee {
            e.panic_with_error(Error::InvalidAmount);
        }

        if MIN_HEARTBEAT > new_subscription.heartbeat {
            e.panic_with_error(Error::InvalidHeartbeat);
        }

        if new_subscription.threshold == 0 {
            e.panic_with_error(Error::InvalidThreshold);
        }

        if new_subscription.webhook.len() > MAX_WEBHOOK_SIZE {
            e.panic_with_error(Error::WebhookTooLong);
        }

        // Transfer and burn the tokens
        transfer_tokens_to_current_contract(&e, &new_subscription.owner, amount, activation_fee);

        //todo: check if the subscription is valid and the amount is enough
        let subscription_id = e.get_last_subscription_id() + 1;
        let subscription = Subscription {
            owner: new_subscription.owner,
            asset1: new_subscription.asset1,
            asset2: new_subscription.asset2,
            threshold: new_subscription.threshold,
            heartbeat: new_subscription.heartbeat,
            webhook: new_subscription.webhook,
            balance: amount - activation_fee,
            status: SubscriptionStatus::Active,
            last_charge: now(&e), // normalize to milliseconds
        };
        e.set_subscription(subscription_id, &subscription);
        e.set_last_subscription_id(subscription_id);
        let data = (subscription_id, subscription);
        e.events()
            .publish((SUBS, symbol_short!("created")), data.clone());
        return data;
    }

    // Deposits funds to the subscription.
    //
    // # Arguments
    //
    // * `from` - Sender address
    // * `subscription_id` - Subscription ID
    // * `amount` - Amount to deposit
    //
    // # Panics
    //
    // Panics if the contract is not initialized
    // Panics if the amount is zero
    // Panics if the subscription does not exist
    // Panics if the token transfer fails
    pub fn deposit(e: Env, from: Address, subscription_id: u64, amount: u64) {
        panin_if_not_initialized(&e);
        from.require_auth();
        if amount == 0 {
            e.panic_with_error(Error::InvalidAmount);
        }
        let mut subscription = e
            .get_subscription(subscription_id)
            .unwrap_or_else(|| panic_with_error!(e, Error::SubscriptionNotFound));
        let activation_fee = get_activation_fee(&e);
        let mut burn_amount = 0;

        match subscription.status {
            SubscriptionStatus::Suspended => {
                // Check if the subscription is suspended
                if amount < activation_fee {
                    e.panic_with_error(Error::InvalidAmount);
                }
                // Set the activation fee as the burn amount
                burn_amount = activation_fee;
                subscription.status = SubscriptionStatus::Active;
            }
            SubscriptionStatus::Cancelled => {
                e.panic_with_error(Error::InvalidSubscriptionStatusError);
            }
            _ => {}
        }

        // Transfer and burn the tokens
        transfer_tokens_to_current_contract(&e, &from, amount, burn_amount);

        subscription.balance += amount - burn_amount;
        e.set_subscription(subscription_id, &subscription);
        e.events()
            .publish((SUBS, symbol_short!("deposit")), (subscription_id, amount));
    }

    // Withdraws funds from the subscription and deactivates it.
    //
    // # Arguments
    //
    // * `subscription_id` - Subscription ID
    // # Panics if the contract is not initialized
    // # Panics if the subscription does not exist
    // # Panics if the caller doesn't match the owner address
    // # Panics if the subscription is not active
    // # Panics if the token transfer fails
    pub fn cancel(e: Env, subscription_id: u64) {
        panin_if_not_initialized(&e);
        let mut subscription = e
            .get_subscription(subscription_id)
            .unwrap_or_else(|| panic_with_error!(e, Error::SubscriptionNotFound));
        subscription.owner.require_auth();
        match subscription.status {
            SubscriptionStatus::Active => {}
            _ => {
                e.panic_with_error(Error::InvalidSubscriptionStatusError);
            }
        }
        // Transfer the remaining balance to the owner
        transfer_tokens(
            &e,
            &e.current_contract_address(),
            &subscription.owner,
            subscription.balance,
        );
        subscription.status = SubscriptionStatus::Cancelled;
        subscription.balance = 0;
        e.set_subscription(subscription_id, &subscription);
        e.events()
            .publish((SUBS, symbol_short!("cancelled")), subscription_id);
    }

    // Gets the subscription by ID.
    //
    // # Arguments
    //
    // * `subscription_id` - Subscription ID
    //
    // # Returns
    //
    // Subscription data
    //
    // # Panics
    //
    // Panics if the contract is not initialized
    pub fn get_subscription(e: Env, subscription_id: u64) -> Subscription {
        panin_if_not_initialized(&e);
        e.get_subscription(subscription_id)
            .unwrap_or_else(|| panic_with_error!(e, Error::SubscriptionNotFound))
    }

    // Returns admin address of the contract.
    //
    // # Returns
    //
    // Contract admin account address
    pub fn admin(e: Env) -> Option<Address> {
        e.get_admin()
    }

    // Returns current protocol version of the contract.
    //
    // # Returns
    //
    // Contract protocol version
    pub fn version(_e: Env) -> u32 {
        env!("CARGO_PKG_VERSION")
            .split(".")
            .next()
            .unwrap()
            .parse::<u32>()
            .unwrap()
    }

    // Returns the base fee of the contract.
    //
    // # Returns
    //
    // Base fee
    pub fn fee(e: Env) -> u64 {
        panin_if_not_initialized(&e);
        e.get_fee()
    }

    // Returns the token address of the contract.
    //
    // # Returns
    //
    // Token address
    pub fn token(e: Env) -> Address {
        panin_if_not_initialized(&e);
        e.get_token()
    }
}

fn panin_if_not_initialized(e: &Env) {
    if !e.is_initialized() {
        panic_with_error!(e, Error::NotInitialized);
    }
}

fn get_token_client(e: &Env) -> TokenClient {
    TokenClient::new(e, &e.get_token())
}

fn get_activation_fee(e: &Env) -> u64 {
    e.get_fee() * MIN_FEE_FACTOR
}

fn transfer_tokens_to_current_contract(e: &Env, from: &Address, amount: u64, burn_amount: u64) {
    transfer_tokens(e, from, &e.current_contract_address(), amount);
    if burn_amount > 0 {
        let token_client = get_token_client(e);
        token_client.burn(&e.current_contract_address(), &(burn_amount as i128));
    }
}

fn transfer_tokens(e: &Env, from: &Address, to: &Address, amount: u64) {
    let token_client = get_token_client(e);
    token_client.transfer(from, to, &(amount as i128));
}

fn now(e: &Env) -> u64 {
    e.ledger().timestamp() * 1000 // normalize to milliseconds
}

mod test;

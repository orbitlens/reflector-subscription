use soroban_sdk::{contracttype, Address, String};

use super::ticker_asset::TickerAsset;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]

// The configuration parameters for the contract.
pub struct Subscription {
    // The owner address.
    pub owner: Address,
    // The asset 1.
    pub asset1: TickerAsset,
    // The asset 2.
    pub asset2: TickerAsset,
    // The threshold in percentage.
    pub threshold: u32,
    // The heartbeat in minutes.
    pub heartbeat: u32,
    // The last heartbeat.
    pub last_notification: u64,
    // The webhook.
    pub webhook: String,
    // Balance
    pub balance: u64,
    // The subscription status.
    pub is_active: bool,
    // The last change timestamp.
    pub last_charge: u64
}

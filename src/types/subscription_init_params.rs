use soroban_sdk::{contracttype, Address, Bytes};

use super::ticker_asset::TickerAsset;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]

// The configuration parameters for the contract.
pub struct SubscriptionInitParams {
    // The owner address.
    pub owner: Address,
    // Base ticker asset.
    pub base: TickerAsset,
    // Quote ticker asset.
    pub quote: TickerAsset,
    // The threshold in percentage.
    pub threshold: u32,
    // The heartbeat in minutes.
    pub heartbeat: u32,
    // The webhook.
    pub webhook: Bytes,
}

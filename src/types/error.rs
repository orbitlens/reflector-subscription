use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
// The error codes for the contract.
pub enum Error {
    // The contract is already initialized.
    AlreadyInitialized = 0,
    // The caller is not authorized to perform the operation.
    Unauthorized = 1,
    // The subscription does not exist.
    SubscriptionNotFound = 2,
    // The contract is not initialized.
    NotInitialized = 3,
    // The amount is invalid.
    InvalidAmount = 4,
    // The heartbeat is invalid.
    InvalidHeartbeat = 5,
    // The threshold is invalid.
    InvalidThreshold = 6,
    // The webhook is too long.
    WebhookTooLong = 7,
}

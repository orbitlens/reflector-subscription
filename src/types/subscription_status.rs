use soroban_sdk::contracttype;


#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq, Copy)]
pub enum SubscriptionStatus {
    Active = 0,
    Suspended = 1,
    Cancelled = 2
}
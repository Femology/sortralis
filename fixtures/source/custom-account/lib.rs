use soroban_sdk::{contract, contractimpl, auth::{CustomAccountInterface, Context}, BytesN, Hash, Error as SdkError, Env, Vec};
#[contract]
pub struct Account;
#[contractimpl]
impl CustomAccountInterface for Account {
    type Signature = BytesN<64>;
    type Error = SdkError;
    fn __check_auth(_env: Env, _payload: Hash<32>, _signature: Self::Signature, _contexts: Vec<Context>) -> Result<(), Self::Error> {
        panic!("Source-only fixture; never execute")
    }
}

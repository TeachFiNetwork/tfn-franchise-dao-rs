multiversx_sc::imports!();

#[multiversx_sc::proxy]
pub trait LaunchpadProxy {
    // main dao sc address
    #[view(getMainDAO)]
    #[storage_mapper("main_dao")]
    fn main_dao(&self) -> SingleValueMapper<ManagedAddress>;
}

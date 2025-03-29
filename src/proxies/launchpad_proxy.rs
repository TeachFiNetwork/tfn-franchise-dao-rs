multiversx_sc::imports!();

#[multiversx_sc::proxy]
pub trait LaunchpadProxy {
    // main dao sc address
    #[view(getMainDAO)]
    #[storage_mapper("main_dao")]
    fn main_dao(&self) -> SingleValueMapper<ManagedAddress>;

    // platform address
    #[view(getPlatform)]
    #[storage_mapper("platform")]
    fn platform(&self) -> SingleValueMapper<ManagedAddress>;

    // template employee sc address
    #[view(getTemplateEmployee)]
    #[storage_mapper("template_employee")]
    fn template_employee(&self) -> SingleValueMapper<ManagedAddress>;

    // template student sc address
    #[view(getTemplateStudent)]
    #[storage_mapper("template_student")]
    fn template_student(&self) -> SingleValueMapper<ManagedAddress>;
}

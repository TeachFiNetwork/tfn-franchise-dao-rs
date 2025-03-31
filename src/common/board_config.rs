multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[type_abi]
#[derive(NestedEncode, NestedDecode, TopEncode, TopDecode, Clone)]
pub enum BoardAction<M: ManagedTypeApi> {
    Nothing,

    AddBoardMember(ManagedAddress<M>),
    RemoveBoardMember(ManagedAddress<M>),
    ChangeBoardQuorum(usize),

    ChangeQuorum(BigUint<M>),
    ChangeVotingPeriod(u64),

    AddVotingToken(TokenIdentifier<M>, BigUint<M>),
    RemoveVotingToken(TokenIdentifier<M>),
}

#[multiversx_sc::module]
pub trait BoardConfigModule {
    // board members
    #[view(getBoardMembers)]
    #[storage_mapper("board_members")]
    fn board_members(&self) -> UnorderedSetMapper<ManagedAddress>;

    #[view(isBoardMember)]
    fn is_board_member(&self, address: &ManagedAddress) -> bool {
        self.board_members().contains(address)
    }

    // actions
    #[view(getActions)]
    #[storage_mapper("action_data")]
    fn action_mapper(&self) -> VecMapper<BoardAction<Self::Api>>;

    // signers
    #[view(getActionSignerIds)]
    #[storage_mapper("action_signer_ids")]
    fn action_signers(&self, action_id: usize) -> UnorderedSetMapper<ManagedAddress>;

    // board quorum
    #[view(getBoardQuorum)]
    #[storage_mapper("board_quorum")]
    fn board_quorum(&self) -> SingleValueMapper<usize>;

    // views
    #[view(quorumReached)]
    fn quorum_reached(&self, action_id: usize) -> bool {
        self.get_action_valid_signer_count(action_id) >= self.board_quorum().get()
    }

    #[view]
    fn signed(&self, user: ManagedAddress, action_id: usize) -> bool {
        self.action_signers(action_id).contains(&user)
    }

    #[view(getActionValidSignerCount)]
    fn get_action_valid_signer_count(&self, action_id: usize) -> usize {
        let signer_ids = self.action_signers(action_id);
        let board = self.board_members();
        signer_ids
            .iter()
            .filter(|signer| {
                board.contains(signer)
            })
            .count()
    }
}

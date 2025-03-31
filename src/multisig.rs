use crate::common::{board_config::*, errors::*};

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait MultisigModule:
crate::common::board_config::BoardConfigModule
+ crate::common::config::ConfigModule
+ crate::common::school_config::SchoolConfigModule
{
    #[endpoint]
    fn sign(&self, action_id: usize) {
        require!(
            !self.action_mapper().item_is_empty_unchecked(action_id),
            "action does not exist"
        );

        let caller = self.blockchain().get_caller();
        require!(self.board_members().contains(&caller), ERROR_ONLY_BOARD_MEMBERS);

        self.action_signers(action_id).insert(caller);
    }

    #[endpoint]
    fn unsign(&self, action_id: usize) {
        require!(
            !self.action_mapper().item_is_empty_unchecked(action_id),
            "action does not exist"
        );

        let caller = self.blockchain().get_caller();
        require!(self.board_members().contains(&caller), ERROR_ONLY_BOARD_MEMBERS);

        self.action_signers(action_id).swap_remove(&caller);
    }

    #[endpoint(discardAction)]
    fn discard_action(&self, action_id: usize) {
        let caller = self.blockchain().get_caller();
        require!(self.board_members().contains(&caller), ERROR_ONLY_BOARD_MEMBERS);
        require!(
            self.get_action_valid_signer_count(action_id) == 0,
            "cannot discard action with valid signatures"
        );

        self.action_mapper().clear_entry_unchecked(action_id);
        self.action_signers(action_id).clear();
    }

    fn propose_action(&self, action: BoardAction<Self::Api>) -> usize {
        let caller = self.blockchain().get_caller();
        require!(self.board_members().contains(&caller), ERROR_ONLY_BOARD_MEMBERS);

        let action_id = self.action_mapper().push(&action);
        self.action_signers(action_id).insert(caller);

        action_id
    }

    #[endpoint(proposeAddBoardMember)]
    fn propose_add_board_member(&self, board_member_address: ManagedAddress) -> usize {
        require!(!self.board_members().contains(&board_member_address), ERROR_ALREADY_BOARD_MEMBER);

        self.propose_action(BoardAction::AddBoardMember(board_member_address))
    }

    #[endpoint(proposeRemoveUser)]
    fn propose_remove_user(&self, user_address: ManagedAddress) -> usize {
        require!(self.board_members().contains(&user_address), ERROR_NOT_BOARD_MEMBER);

        self.propose_action(BoardAction::RemoveBoardMember(user_address))
    }

    #[endpoint(proposeChangeBoardQuorum)]
    fn propose_change_board_quorum(&self, new_quorum: usize) -> usize {
        require!(new_quorum > 0, ERROR_ZERO_VALUE);

        self.propose_action(BoardAction::ChangeBoardQuorum(new_quorum))
    }

    #[endpoint(proposeChangeQuorum)]
    fn propose_change_quorum(&self, new_quorum: BigUint) -> usize {
        require!(new_quorum > 0, ERROR_ZERO_VALUE);

        self.propose_action(BoardAction::ChangeQuorum(new_quorum))
    }

    #[endpoint(proposeChangeVotingPeriod)]
    fn propose_change_vorint_period(&self, new_period: u64) -> usize {
        require!(new_period > 0, ERROR_ZERO_VALUE);

        self.propose_action(BoardAction::ChangeVotingPeriod(new_period))
    }

    #[endpoint(proposeAddVotingToken)]
    fn propose_add_voting_token(
        &self,
        token: TokenIdentifier,
        weight: BigUint,
    ) -> usize {
        require!(!self.voting_tokens().contains_key(&token), ERROR_TOKEN_ALREADY_EXISTS);
        require!(weight > 0, ERROR_ZERO_VALUE);

        self.propose_action(BoardAction::AddVotingToken(token, weight))
    }

    #[endpoint(proposeRemoveVotingToken)]
    fn propose_remove_voting_token(&self, token: TokenIdentifier) -> usize {
        require!(self.voting_tokens().contains_key(&token), ERROR_TOKEN_NOT_FOUND);

        self.propose_action(BoardAction::RemoveVotingToken(token))
    }

    #[endpoint(proposeChangeTaxAmount)]
    fn propose_change_tax_amount(&self, new_tax_amount: BigUint) -> usize {
        require!(new_tax_amount > 0, ERROR_ZERO_VALUE);

        self.propose_action(BoardAction::ChangeTaxAmount(new_tax_amount))
    }

    #[endpoint(performAction)]
    fn perform_action_endpoint(&self, action_id: usize) {
        let caller = self.blockchain().get_caller();
        require!(self.board_members().contains(&caller), ERROR_ONLY_BOARD_MEMBERS);
        require!(
            self.quorum_reached(action_id),
            "quorum has not been reached"
        );

        self.perform_action(action_id)
    }

    fn perform_action(&self, action_id: usize) {
        let action = self.action_mapper().get(action_id);
        self.action_mapper().clear_entry_unchecked(action_id);
        self.action_signers(action_id).clear();
        match action {
            BoardAction::Nothing=>return,
            BoardAction::AddBoardMember(board_member_address) => {
                self.board_members().insert(board_member_address);
            },
            BoardAction::RemoveBoardMember(board_member_address) => {
                self.board_members().swap_remove(&board_member_address);
            },
            BoardAction::ChangeBoardQuorum(new_quorum) => {
                self.board_quorum().set(new_quorum);
            },
            BoardAction::ChangeQuorum(new_quorum) => {
                self.quorum().set(new_quorum);
            },
            BoardAction::ChangeVotingPeriod(new_voting_period) => {
                self.voting_period().set(new_voting_period);
            },
            BoardAction::AddVotingToken(token, weight) => {
                self.voting_tokens().insert(token, weight);
            },
            BoardAction::RemoveVotingToken(token) => {
                self.voting_tokens().remove(&token);
                if self.voting_tokens().is_empty() {
                    self.set_state_inactive();
                }
            },
            BoardAction::ChangeTaxAmount(new_tax_amount) => {
                self.tax_amount().set(new_tax_amount);
            },
        };
    }
}

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::common::errors::*;

#[type_abi]
#[derive(ManagedVecItem, TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Eq, Copy, Clone, Debug)]
pub enum State {
    Inactive,
    Active,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, ManagedVecItem)]
pub struct Action<M: ManagedTypeApi> {
    pub gas_limit: u64,
    pub dest_address: ManagedAddress<M>,
    pub payment_token: EgldOrEsdtTokenIdentifier<M>,
    pub payment_amount: BigUint<M>,
    pub endpoint_name: ManagedBuffer<M>,
    pub arguments: ManagedVec<M, ManagedBuffer<M>>,
}

#[type_abi]
#[derive(TopEncode, TopDecode)]
pub struct ProposalCreationArgs<M: ManagedTypeApi> {
    pub description: ManagedBuffer<M>,
    pub action: Action<M>,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Debug, Clone)]
pub enum VoteType {
    Upvote = 1,
    DownVote = 2,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Debug, ManagedVecItem)]
pub enum ProposalStatus {
    Pending, //Starts from 0
    Active,
    Defeated,
    Succeeded,
    Executed,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, ManagedVecItem)]
pub struct Proposal<M: ManagedTypeApi> {
    pub id: u64,
    pub creation_block: u64,
    pub proposer: ManagedAddress<M>,
    pub title: ManagedBuffer<M>,
    pub status: ProposalStatus,

    pub was_executed: bool,
    pub action: Action<M>,

    pub num_upvotes: BigUint<M>,
    pub num_downvotes: BigUint<M>,
}

#[multiversx_sc::module]
pub trait ConfigModule {
    // owner
    #[view(getOwner)]
    #[storage_mapper("owner")]
    fn owner(&self) -> SingleValueMapper<ManagedAddress>;

    // state
    #[endpoint(setStateActive)]
    fn set_state_active(&self) {
        self.only_owner();
        require!(self.quorum().get() > 0, ERROR_QUORUM_NOT_SET);
        require!(self.voting_period().get() > 0, ERROR_VOTING_PERIOD_NOT_SET);
        require!(self.min_proposal_amount().get() > 0, ERROR_PROPOSAL_AMOUNT_NOT_SET);

        self.state().set(State::Active);
    }

    #[endpoint(setStateInactive)]
    fn set_state_inactive(&self) {
        self.only_owner();
        self.state().set(State::Inactive);
    }

    #[view(getState)]
    #[storage_mapper("state")]
    fn state(&self) -> SingleValueMapper<State>;

    // main dao sc address
    #[view(getMainDAO)]
    #[storage_mapper("main_dao")]
    fn main_dao(&self) -> SingleValueMapper<ManagedAddress>;

    // template employee sc address
    #[view(getTemplateEmployee)]
    #[storage_mapper("template_employee")]
    fn template_employee(&self) -> SingleValueMapper<ManagedAddress>;

    // template student sc address
    #[view(getTemplateStudent)]
    #[storage_mapper("template_student")]
    fn template_student(&self) -> SingleValueMapper<ManagedAddress>;

    // governance token
    #[view(getGovernanceToken)]
    #[storage_mapper("governance_token")]
    fn governance_token(&self) -> SingleValueMapper<TokenIdentifier>;

    // min proposal amount
    #[endpoint(setMinProposalAmount)]
    fn set_min_proposal_amount(&self, amount: &BigUint) {
        self.only_owner();
        self.min_proposal_amount().set(amount);
    }

    #[view(getMinProposalAmount)]
    #[storage_mapper("min_proposal_amount")]
    fn min_proposal_amount(&self) -> SingleValueMapper<BigUint>;

    // voting period (blocks)
    #[endpoint(setVotingPeriod)]
    fn set_voting_period(&self, period: u64) {
        self.only_owner();
        self.voting_period().set(period);
    }

    #[view(getVotingPeriod)]
    #[storage_mapper("voting_period")]
    fn voting_period(&self) -> SingleValueMapper<u64>;

    // quorum
    #[endpoint(setQuorum)]
    fn set_quorum(&self, quorum: &BigUint) {
        self.only_owner();
        self.quorum().set(quorum);
    }

    #[view(getQuorum)]
    #[storage_mapper("quorum")]
    fn quorum(&self) -> SingleValueMapper<BigUint>;

    // last proposal id
    #[view(getLastProposalId)]
    #[storage_mapper("last_proposal_id")]
    fn last_proposal_id(&self) -> SingleValueMapper<u64>;

    // proposal
    #[view(getProposal)]
    #[storage_mapper("proposals")]
    fn proposals(&self, id: u64) -> SingleValueMapper<Proposal<Self::Api>>;

    // voters amounts
    #[view(getVoterAmount)]
    #[storage_mapper("voters_amounts")]
    fn voters_amounts(&self, voter: &ManagedAddress, proposal_id: u64) -> SingleValueMapper<BigUint>;

    // proposal voters
    #[view(getProposalVoters)]
    #[storage_mapper("proposal_voters")]
    fn proposal_voters(&self, id: u64) -> UnorderedSetMapper<ManagedAddress>;

    // voter proposals
    #[view(getVoterProposals)]
    #[storage_mapper("voter_proposals")]
    fn voter_proposals(&self, voter: &ManagedAddress) -> UnorderedSetMapper<u64>;

    // get number of proposals with the specified status
    #[view(getProposalsCount)]
    fn get_proposals_count(&self, status: OptionalValue<ProposalStatus>) -> u64 {
        let all = status.is_none();
        let filter_status = match status {
            OptionalValue::Some(value) => value,
            OptionalValue::None => ProposalStatus::Pending
        };
        let mut count = 0;
        for idx in 0..self.last_proposal_id().get() {
            if self.proposals(idx).is_empty() {
                continue;
            }

            let proposal = self.proposals(idx).get();
            let proposal_status = self.get_proposal_status(&proposal);
            if all || proposal_status == filter_status {
                count += 1;
            }
        }

        count
    }

    // view paginated proposals of certain type
    #[view(getProposals)]
    fn get_proposals(&self, idx_from: u64, idx_to: u64, status: OptionalValue<ProposalStatus>) -> ManagedVec<Proposal<Self::Api>> {
        let mut proposals: ManagedVec<Proposal<Self::Api>> = ManagedVec::new();
        let all = status.is_none();
        let filter_status = match status {
            OptionalValue::Some(value) => value,
            OptionalValue::None => ProposalStatus::Pending
        };
        let mut real_idx: u64 = 0;
        for idx in 0..self.last_proposal_id().get() {
            if self.proposals(idx).is_empty() {
                continue;
            }

            let mut proposal = self.proposals(idx).get();
            let proposal_status = self.get_proposal_status(&proposal);
            if !all && proposal_status != filter_status {
                continue
            }

            if real_idx >= idx_from && real_idx <= idx_to {
                proposal.status = proposal_status;
                proposals.push(proposal);
            }
            real_idx += 1;
        }

        proposals
    }

    // proposal status
    #[view(getProposalStatus)]
    fn get_proposal_status_view(&self, proposal_id: u64) -> ProposalStatus {
        require!(!self.proposals(proposal_id).is_empty(), ERROR_PROPOSAL_NOT_FOUND);

        let proposal = self.proposals(proposal_id).get();
        self.get_proposal_status(&proposal)
    }

    fn get_proposal_status(&self, proposal: &Proposal<Self::Api>) -> ProposalStatus {
        if proposal.was_executed {
            return ProposalStatus::Executed;
        }

        let current_block = self.blockchain().get_block_nonce();
        let proposal_block = proposal.creation_block;
        let voting_period = self.voting_period().get();

        let voting_start = proposal_block;
        let voting_end = voting_start + voting_period;

        if current_block < voting_start {
            return ProposalStatus::Pending;
        }
        if current_block >= voting_start && current_block < voting_end {
            return ProposalStatus::Active;
        }

        let total_upvotes = &proposal.num_upvotes;
        let total_downvotes = &proposal.num_downvotes;
        let quorum = self.quorum().get();

        if total_upvotes > total_downvotes && total_upvotes - total_downvotes >= quorum {
            ProposalStatus::Succeeded
        } else {
            ProposalStatus::Defeated
        }
    }

    // helpers

    fn only_owner(&self) {
        let caller = self.blockchain().get_caller();
        require!(
            caller == self.owner().get() || caller == self.blockchain().get_owner_address(),
            ERROR_ONLY_OWNER
        );
    }
}

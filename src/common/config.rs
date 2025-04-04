multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::common::errors::*;
use super::board_config;

#[type_abi]
#[derive(ManagedVecItem, TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Eq, Copy, Clone, Debug)]
pub enum State {
    Inactive,
    Active,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Clone, Debug, ManagedVecItem)]
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
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Debug, ManagedVecItem)]
pub enum ProposalTypeEnum {
    Nothing,

    NewTransfer,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Clone, Debug)]
pub enum ProposalType<M: ManagedTypeApi> {
    Nothing,

    NewTransfer(TransferProposal<M>),
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Debug)]
pub struct Proposal<M: ManagedTypeApi> {
    pub id: u64,
    pub proposal_data: ProposalType<M>,
    pub proposal_type: ProposalTypeEnum,
    pub creation_timestamp: u64,
    pub proposer: ManagedAddress<M>,
    pub title: ManagedBuffer<M>,
    pub description: ManagedBuffer<M>,
    pub status: ProposalStatus,
    pub was_executed: bool,
    pub num_upvotes: BigUint<M>,
    pub num_downvotes: BigUint<M>,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Clone, Debug)]
pub struct TransferProposal<M: ManagedTypeApi> {
    pub actions: ManagedVec<M, Action<M>>,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, ManagedVecItem)]
pub struct ContractInfo<M: ManagedTypeApi> {
    pub state: State,
    pub governance_token: TokenIdentifier<M>,
    pub voting_tokens: ManagedVec<M, TokenIdentifier<M>>,
    pub voting_token_weights: ManagedVec<M, BigUint<M>>,
    pub voting_period: u64,
    pub quorum: BigUint<M>,
    pub board_quorum: usize,
    pub board_members: ManagedVec<M, ManagedAddress<M>>,
    pub last_proposal_id: u64,
    pub proposals_count: u64,
}

#[multiversx_sc::module]
pub trait ConfigModule:
board_config::BoardConfigModule
{
    // state
    #[endpoint(setStateActive)]
    fn set_state_active(&self) {
        self.only_board_members();
        require!(self.quorum().get() > 0, ERROR_QUORUM_NOT_SET);
        require!(self.voting_period().get() > 0, ERROR_VOTING_PERIOD_NOT_SET);
        require!(!self.voting_tokens().is_empty(), ERROR_NO_VOTING_TOKENS);

        self.state().set(State::Active);
    }

    #[endpoint(setStateInactive)]
    fn set_state_inactive(&self) {
        self.only_board_members();
        self.state().set(State::Inactive);
    }

    #[view(getState)]
    #[storage_mapper("state")]
    fn state(&self) -> SingleValueMapper<State>;

    // contracts
    #[view(getMainDAOAddress)]
    #[storage_mapper("main_dao_sc")]
    fn main_dao(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getPlatformAddress)]
    #[storage_mapper("platform_sc")]
    fn platform_sc(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getDigitalIdentityAddress)]
    #[storage_mapper("digital_identity_sc")]
    fn digital_identity_sc(&self) -> SingleValueMapper<ManagedAddress>;

    // governance token
    #[view(getGovernanceToken)]
    #[storage_mapper("governance_token")]
    fn governance_token(&self) -> SingleValueMapper<TokenIdentifier>;

    // digital identity
    #[view(getIdentityId)]
    #[storage_mapper("identity_id")]
    fn identity_id(&self) -> SingleValueMapper<u64>;

    #[only_owner]
    #[endpoint(setIdentityId)]
    fn set_identity_id(&self, id: u64) {
        self.identity_id().set_if_empty(id);
    }

    // voting tokens
    #[view(getVotingTokens)]
    #[storage_mapper("voting_tokens")]
    fn voting_tokens(&self) -> MapMapper<TokenIdentifier, BigUint>;

    // voting period (blocks)
    #[endpoint(setVotingPeriod)]
    fn set_voting_period(&self, period: u64) {
        self.only_board_members();
        self.voting_period().set(period);
    }

    #[view(getVotingPeriod)]
    #[storage_mapper("voting_period")]
    fn voting_period(&self) -> SingleValueMapper<u64>;

    // quorum
    #[endpoint(setQuorum)]
    fn set_quorum(&self, quorum: &BigUint) {
        self.only_board_members();
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
    fn voters_amounts(&self, voter: &ManagedAddress, proposal_id: u64) -> SingleValueMapper<ManagedVec<EsdtTokenPayment>>;

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
    fn get_proposals(&self, idx_from: u64, idx_to: u64, proposal_type: ProposalTypeEnum) -> MultiValueEncoded<Proposal<Self::Api>> {
        let mut proposals = MultiValueEncoded::new();
        let mut real_idx: u64 = 0;
        for idx in 0..self.last_proposal_id().get() {
            if self.proposals(idx).is_empty() {
                continue;
            }

            let mut proposal = self.proposals(idx).get();
            if proposal.proposal_type != proposal_type {
                continue
            }

            if real_idx >= idx_from && real_idx <= idx_to {
                proposal.status = self.get_proposal_status(&proposal);
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

        let current_timestamp = self.blockchain().get_block_timestamp();
        let proposal_timestamp = proposal.creation_timestamp;
        let voting_period = self.voting_period().get();

        let voting_start = proposal_timestamp;
        let voting_end = voting_start + voting_period;

        if current_timestamp < voting_start {
            return ProposalStatus::Pending;
        }
        if current_timestamp >= voting_start && current_timestamp < voting_end {
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

    #[view(getContractInfo)]
    fn get_contract_info(&self) -> ContractInfo<Self::Api> {
        let state = self.state().get();
        let governance_token = self.governance_token().get();
        let mut voting_tokens = ManagedVec::new();
        let mut voting_token_weights = ManagedVec::new();
        for (token, weight) in self.voting_tokens().iter() {
            voting_tokens.push(token);
            voting_token_weights.push(weight);
        }
        let voting_period = self.voting_period().get();
        let quorum = self.quorum().get();
        let board_quorum = self.board_quorum().get();
        let mut board_members = ManagedVec::new();
        for member in self.board_members().into_iter() {
            board_members.push(member);
        }
        let last_proposal_id = self.last_proposal_id().get();
        let proposals_count = self.get_proposals_count(OptionalValue::None);

        ContractInfo {
            state,
            governance_token,
            voting_tokens,
            voting_token_weights,
            voting_period,
            quorum,
            board_quorum,
            board_members,
            last_proposal_id,
            proposals_count,
        }
    }

    // helpers
    fn only_board_members(&self) {
        let caller = self.blockchain().get_caller();
        require!(self.board_members().contains(&caller), ERROR_ONLY_BOARD_MEMBERS);
    }
}

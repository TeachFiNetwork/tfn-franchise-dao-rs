#![no_std]

multiversx_sc::imports!();

pub mod common;

use common::{config::*, errors::*};

#[multiversx_sc::contract]
pub trait TFNFranchiseDAOContract<ContractReader>:
    common::config::ConfigModule
{
    #[init]
    fn init(&self) {
        self.set_state_inactive();
    }

    #[upgrade]
    fn upgrade(&self) {
        self.set_state_inactive();
    }

    #[payable("*")]
    #[endpoint]
    fn propose(&self, args: ProposalCreationArgs<Self::Api>) -> u64 {
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);

        let payment = self.call_value().single_esdt();
        require!(payment.token_identifier == self.governance_token().get(), ERROR_INVALID_PAYMENT);
        require!(payment.amount >= self.min_proposal_amount().get(), ERROR_NOT_ENOUGH_FUNDS_TO_PROPOSE);

        let proposal = Proposal {
            id: self.last_proposal_id().get(),
            creation_block: self.blockchain().get_block_nonce(),
            proposer: self.blockchain().get_caller(),
            description: args.description,
            was_executed: false,
            actions: args.actions,
            num_upvotes: payment.amount.clone(),
            num_downvotes: BigUint::zero(),
        };
        self.proposals(proposal.id).set(&proposal);
        self.last_proposal_id().set(proposal.id + 1);

        let caller = self.blockchain().get_caller();
        self.proposal_voters(proposal.id).insert(caller.clone());
        self.voter_proposals(&caller).insert(proposal.id);
        self.voters_amounts(&caller, proposal.id).update(|value| *value += payment.amount);

        proposal.id
    }

    #[payable("*")]
    #[endpoint]
    fn upvote(&self, proposal_id: u64) {
        self.vote(proposal_id, VoteType::Upvote)
    }

    #[payable("*")]
    #[endpoint]
    fn downvote(&self, proposal_id: u64) {
        self.vote(proposal_id, VoteType::DownVote)
    }

    fn vote(&self, proposal_id: u64, vote_type: VoteType) {
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);
        require!(!self.proposals(proposal_id).is_empty(), ERROR_PROPOSAL_NOT_FOUND);

        let mut proposal = self.proposals(proposal_id).get();
        let pstat = self.get_proposal_status(&proposal);
        require!(pstat == ProposalStatus::Active, ERROR_PROPOSAL_NOT_ACTIVE);

        let payment = self.call_value().single_esdt();
        require!(payment.token_identifier == self.governance_token().get(), ERROR_INVALID_PAYMENT);
        require!(payment.amount > 0, ERROR_ZERO_PAYMENT);

        match vote_type {
            VoteType::Upvote => proposal.num_upvotes += &payment.amount,
            VoteType::DownVote => proposal.num_downvotes += &payment.amount,
        }
        self.proposals(proposal_id).set(&proposal);

        let caller = self.blockchain().get_caller();
        self.proposal_voters(proposal.id).insert(caller.clone());
        self.voter_proposals(&caller).insert(proposal.id);
        self.voters_amounts(&caller, proposal.id).update(|value| *value += payment.amount);
    }

    #[endpoint]
    fn redeem(&self, proposal_id: u64) {
        let proposal = self.proposals(proposal_id).get();
        let pstat = self.get_proposal_status(&proposal);
        require!(
            pstat == ProposalStatus::Succeeded || pstat == ProposalStatus::Defeated || pstat == ProposalStatus::Executed,
            ERROR_VOTING_PERIOD_NOT_ENDED,
        );

        let caller = self.blockchain().get_caller();
        let amount = self.voters_amounts(&caller, proposal_id).take();
        self.voter_proposals(&caller).swap_remove(&proposal_id);
        self.proposal_voters(proposal_id).swap_remove(&caller);
        self.send().direct_esdt(
            &caller,
            &self.governance_token().get(),
            0,
            &amount,
        );
    }

    #[endpoint]
    fn execute(&self, proposal_id: u64) {
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);
        require!(!self.proposals(proposal_id).is_empty(), ERROR_PROPOSAL_NOT_FOUND);

        let mut proposal = self.proposals(proposal_id).get();
        let pstat = self.get_proposal_status(&proposal);
        require!(pstat == ProposalStatus::Succeeded, ERROR_PROPOSAL_NOT_SUCCEEDED);

        self.execute_proposal(&proposal);
        proposal.was_executed = true;
        self.proposals(proposal_id).set(&proposal);
    }

    fn execute_proposal(&self, proposal: &Proposal<Self::Api>) {
        for action in proposal.actions.iter() {
            self.execute_action(&action).unwrap()
        }
    }

    fn execute_action(&self, action: &Action<Self::Api>) -> Result<(), &'static [u8]> {
        self.send()
            .contract_call::<()>(action.dest_address.clone(), action.endpoint_name.clone())
            .with_raw_arguments(ManagedArgBuffer::from(action.arguments.clone()))
            .with_gas_limit(action.gas_limit)
            .transfer_execute();
        Result::Ok(())
    }
}

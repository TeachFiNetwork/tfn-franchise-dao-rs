#![no_std]

multiversx_sc::imports!();

pub mod common;
pub mod school;
pub mod proxies;
pub mod multisig;

use common::{config::*, consts::*, errors::*};
use crate::proxies::launchpad_proxy::{self};

#[multiversx_sc::contract]
pub trait TFNFranchiseDAOContract<ContractReader>:
common::config::ConfigModule
+common::school_config::SchoolConfigModule
+common::board_config::BoardConfigModule
+school::SchoolModule
+multisig::MultisigModule
{
    #[init]
    fn init(
        &self,
        owner: &ManagedAddress,
        token: &TokenIdentifier,
    ) {
        self.governance_token().set(token);
        let caller = self.blockchain().get_caller();
        if !self.blockchain().is_smart_contract(&caller) {
            return
        }

        let main_dao: ManagedAddress = self.launchpad_contract_proxy()
            .contract(caller.clone())
            .main_dao()
            .execute_on_dest_context();
        let template_employee: ManagedAddress = self.launchpad_contract_proxy()
            .contract(caller.clone())
            .template_employee()
            .execute_on_dest_context();
        let template_student: ManagedAddress = self.launchpad_contract_proxy()
            .contract(caller.clone())
            .template_student()
            .execute_on_dest_context();
        let platform: ManagedAddress = self.launchpad_contract_proxy()
            .contract(caller)
            .platform()
            .execute_on_dest_context();
        self.main_dao().set(main_dao);
        self.template_employee().set(template_employee);
        self.template_student().set(template_student);
        self.platform().set(platform);

        self.board_members().insert(owner.clone());
        self.voting_tokens().insert(token.clone(), BigUint::from(ONE));
    }

    #[upgrade]
    fn upgrade(&self) {
    }

    #[payable("*")]
    #[endpoint(addFunds)]
    fn add_funds(&self) {}

    #[endpoint(proposeNewTransfer)]
    fn propose_new_transfer(
        &self,
        title: ManagedBuffer,
        description: ManagedBuffer,
        transfer_proposal: TransferProposal<Self::Api>,
    ) -> u64 {
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);

        let caller = self.blockchain().get_caller();
        require!(self.board_members().contains(&caller), ERROR_ONLY_BOARD_MEMBERS);

        let proposal = Proposal {
            id: self.last_proposal_id().get(),
            proposal_data: ProposalType::NewTransfer(transfer_proposal),
            proposal_type: ProposalTypeEnum::NewTransfer,
            creation_timestamp: self.blockchain().get_block_timestamp(),
            proposer: caller,
            title,
            description,
            status: ProposalStatus::Pending,
            was_executed: false,
            num_upvotes: BigUint::zero(),
            num_downvotes: BigUint::zero(),
        };
        self.proposals(proposal.id).set(&proposal);
        self.last_proposal_id().set(proposal.id + 1);

        proposal.id
    }

    #[payable("*")]
    #[endpoint(upvote)]
    fn upvote(&self, proposal_id: u64) {
        self.vote(proposal_id, VoteType::Upvote)
    }

    #[payable("*")]
    #[endpoint(downvote)]
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
        require!(self.voting_tokens().contains_key(&payment.token_identifier), ERROR_INVALID_PAYMENT);
        require!(payment.amount > 0, ERROR_ZERO_PAYMENT);

        let vote_weight = payment.amount.clone() * self.voting_tokens().get(&payment.token_identifier).unwrap() / ONE;
        match vote_type {
            VoteType::Upvote => proposal.num_upvotes += vote_weight.sqrt(),
            VoteType::DownVote => proposal.num_downvotes += vote_weight.sqrt(),
        }
        self.proposals(proposal_id).set(&proposal);

        let caller = self.blockchain().get_caller();
        self.proposal_voters(proposal.id).insert(caller.clone());
        self.voter_proposals(&caller).insert(proposal.id);
        
        // update the amount of tokens voted by the caller
        let mut new_vec: ManagedVec<EsdtTokenPayment> = ManagedVec::new();
        let old_vec = self.voters_amounts(&caller, proposal.id).get();
        let mut found = false;
        for old_payment in old_vec.iter() {
            if old_payment.token_identifier == payment.token_identifier && old_payment.token_nonce == payment.token_nonce {
                new_vec.push(EsdtTokenPayment::new(
                    payment.token_identifier.clone(),
                    payment.token_nonce,
                    &old_payment.amount + &payment.amount,
                ));
                found = true;
            } else {
                new_vec.push(old_payment.clone());
            }
        }
        if !found {
            new_vec.push(payment.clone());
        }
        self.voters_amounts(&caller, proposal.id).set(&new_vec);
    }

    #[endpoint(redeem)]
    fn redeem(&self, proposal_id: u64) {
        let proposal = self.proposals(proposal_id).get();
        let pstat = self.get_proposal_status(&proposal);
        require!(
            pstat == ProposalStatus::Succeeded || pstat == ProposalStatus::Defeated || pstat == ProposalStatus::Executed,
            ERROR_VOTING_PERIOD_NOT_ENDED,
        );

        let caller = self.blockchain().get_caller();
        let payments = self.voters_amounts(&caller, proposal_id).take();
        self.voter_proposals(&caller).swap_remove(&proposal_id);
        self.proposal_voters(proposal_id).swap_remove(&caller);
        require!(!payments.is_empty(), ERROR_NOTHING_TO_REDEEM);

        self.send().direct_multi(&caller, &payments);
    }

    #[endpoint(execute)]
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
        match proposal.proposal_data.clone() {
            ProposalType::Nothing => return,

            ProposalType::NewTransfer(transfer_proposal) => {
                for action in transfer_proposal.actions.iter() {
                    self.execute_action(&action).unwrap();
                }
            },
        };
    }

    fn execute_action(&self, action: &Action<Self::Api>) -> Result<(), &'static [u8]> {
        let payment =
            EgldOrEsdtTokenPayment::new(action.payment_token.clone(), 0, action.payment_amount.clone());
        if action.payment_amount > 0 {
            self.send()
                .contract_call::<()>(action.dest_address.clone(), action.endpoint_name.clone())
                .with_egld_or_single_esdt_transfer(payment)
                .with_raw_arguments(ManagedArgBuffer::from(action.arguments.clone()))
                .with_gas_limit(action.gas_limit)
                .transfer_execute();
        } else {
            self.send()
                .contract_call::<()>(action.dest_address.clone(), action.endpoint_name.clone())
                .with_raw_arguments(ManagedArgBuffer::from(action.arguments.clone()))
                .with_gas_limit(action.gas_limit)
                .transfer_execute();
        }

        Result::Ok(())
    }

    // proxies
    #[proxy]
    fn launchpad_contract_proxy(&self) -> launchpad_proxy::Proxy<Self::Api>;
}

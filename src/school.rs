multiversx_sc::imports!();

use crate::common::errors::*;
use crate::common::consts::*;
use crate::common::school_config::{self, *};
use crate::common::config::{self, State};
use crate::common::board_config;

use tfn_dao::common::board_config::ProxyTrait as _;
use tfn_platform::ProxyTrait as _;
use tfn_digital_identity::{ProxyTrait as _, common::config::{ProxyTrait as _, Identity, Value}};

#[multiversx_sc::module]
pub trait SchoolModule:
school_config::SchoolConfigModule
+board_config::BoardConfigModule
+config::ConfigModule
{
    // classes endpoints
    #[endpoint(createClass)]
    fn create_class(&self, year: usize, name: ManagedBuffer<Self::Api>) -> u64 {
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);
        self.only_board_members();

        let class_id = self.last_class_id().get();
        let class = Class {
            id: class_id,
            year,
            name,
            schedule: ManagedVec::new(),
        };
        self.classes(class_id).set(class);
        self.last_class_id().set(class_id + 1);

        class_id
    }

    #[endpoint(editClass)]
    fn edit_class(&self, class_id: u64, year: usize, name: ManagedBuffer<Self::Api>) {
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);
        self.only_board_members();
        require!(!self.classes(class_id).is_empty(), ERROR_CLASS_NOT_FOUND);

        let mut class = self.classes(class_id).get();
        class.year = year;
        class.name = name;
        self.classes(class_id).set(class);
    }

    #[endpoint(deleteClass)]
    fn delete_class(&self, class_id: u64) {
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);
        self.only_board_members();
        require!(!self.classes(class_id).is_empty(), ERROR_CLASS_NOT_FOUND);
        require!(self.get_class_students(class_id).is_empty(), ERROR_CLASS_NOT_EMPTY);

        self.classes(class_id).clear();
    }

    #[endpoint(setClassSchedule)]
    fn set_class_schedule(&self, class_id: u64, schedule: ManagedVec<Self::Api, SubjectSlot<Self::Api>>) {
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);
        self.only_board_members();
        require!(!self.classes(class_id).is_empty(), ERROR_CLASS_NOT_FOUND);

        let mut class = self.classes(class_id).get();
        class.schedule = schedule;
        self.classes(class_id).set(class);
    }

    // students endpoints
    #[endpoint(registerStudentIdentity)]
    fn register_student_identity(
        &self,
        student_id: u64,
    ) -> u64 {
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);
        self.only_board_members();

        let mut keys: ManagedVec<ManagedBuffer> = ManagedVec::new();
        keys.push(CLASS_KEY.into());
        keys.push(MARK_KEY.into());
        keys.push(ABSENCE_KEY.into());
        keys.push(TAX_VALIDITY_KEY.into());
        self.digital_identity_contract_proxy()
            .contract(self.digital_identity_sc().get())
            .request_link(
                self.identity_id().get(),
                student_id,
                STUDENT_RELATION,
                OptionalValue::Some(keys),
            )
            .execute_on_dest_context()
    }

    #[endpoint(enrollStudent)]
    fn enroll_student(&self, student_identity_id: u64, class_id: u64) -> u64 {
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);
        self.only_board_members();

        let is_parent: bool = self.digital_identity_contract_proxy()
            .contract(self.digital_identity_sc().get())
            .is_parent_of(self.identity_id().get(), student_identity_id)
            .execute_on_dest_context();
        require!(is_parent, ERROR_IDENTITY_NOT_REGISTERED);

        let mut keys_values: MultiValueEncoded<(ManagedBuffer, ManagedBuffer)> = MultiValueEncoded::new();
        keys_values.push((CLASS_KEY.into(), BigUint::from(class_id).to_bytes_be_buffer()));
        keys_values.push((TAX_VALIDITY_KEY.into(), BigUint::zero().to_bytes_be_buffer()));
        self.digital_identity_contract_proxy()
            .contract(self.digital_identity_sc().get())
            .add_identity_keys_values(student_identity_id, keys_values)
            .execute_on_dest_context::<()>();
        let student_identity: Identity<Self::Api> = self.digital_identity_contract_proxy()
            .contract(self.digital_identity_sc().get())
            .identities(student_identity_id)
            .execute_on_dest_context();

        let student_id = self.last_student_id().get();
        self.students(student_id).set(student_identity_id);
        self.last_student_id().set(student_id + 1);

        self.platform_contract_proxy()
            .contract(self.platform_sc().get())
            .whitelist_address(student_identity.address)
            .execute_on_dest_context::<()>();

        student_id
    }

    #[endpoint(expellStudent)]
    fn expell_student(&self, student_id: u64) {
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);
        self.only_board_members();
        require!(!self.students(student_id).is_empty(), ERROR_STUDENT_NOT_FOUND);

        let student_identity: Identity<Self::Api> = self.digital_identity_contract_proxy()
            .contract(self.digital_identity_sc().get())
            .identities(self.students(student_id).take())
            .execute_on_dest_context();

        self.platform_contract_proxy()
            .contract(self.platform_sc().get())
            .remove_address(student_identity.address)
            .execute_on_dest_context::<()>();

        self.unregister_identity(student_identity.id);
    }

    // employees endpoints
    #[endpoint(registerEmployeeIdentity)]
    fn register_employee_identity(
        &self,
        employee_id: u64,
        is_teacher: bool,
    ) -> u64 {
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);
        self.only_board_members();

        let mut keys: ManagedVec<ManagedBuffer> = ManagedVec::new();
        keys.push(JOB_KEY.into());
        keys.push(SALARY_KEY.into());
        let relation = if is_teacher {
            TEACHER_RELATION
        } else {
            EMPLOYEE_RELATION
        };
        self.digital_identity_contract_proxy()
            .contract(self.digital_identity_sc().get())
            .request_link(
                self.identity_id().get(),
                employee_id,
                relation,
                OptionalValue::Some(keys),
            )
            .execute_on_dest_context()
    }

    #[endpoint(hireEmployee)]
    fn hire_employee(&self, employee_identity_id: u64, job: ManagedBuffer, salary: BigUint) -> u64 {
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);
        self.only_board_members();

        let is_parent: bool = self.digital_identity_contract_proxy()
            .contract(self.digital_identity_sc().get())
            .is_parent_of(self.identity_id().get(), employee_identity_id)
            .execute_on_dest_context();
            require!(is_parent, ERROR_IDENTITY_NOT_REGISTERED);

        let mut keys_values: MultiValueEncoded<(ManagedBuffer, ManagedBuffer)> = MultiValueEncoded::new();
            keys_values.push((JOB_KEY.into(), job));
            keys_values.push((SALARY_KEY.into(), salary.to_bytes_be_buffer()));
        self.digital_identity_contract_proxy()
            .contract(self.digital_identity_sc().get())
            .add_identity_keys_values(employee_identity_id, keys_values)
            .execute_on_dest_context::<()>();
        let employee_identity: Identity<Self::Api> = self.digital_identity_contract_proxy()
            .contract(self.digital_identity_sc().get())
            .identities(employee_identity_id)
            .execute_on_dest_context();

        let employee_id = self.last_employee_id().get();
        self.employees(employee_id).set(employee_identity_id);
        self.last_employee_id().set(employee_id + 1);

        self.platform_contract_proxy()
            .contract(self.platform_sc().get())
            .whitelist_address(employee_identity.address)
            .execute_on_dest_context::<()>();

        employee_id
    }

    #[endpoint(fireEmployee)]
    fn fire_employee(&self, employee_id: u64) {
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);
        self.only_board_members();
        require!(!self.employees(employee_id).is_empty(), ERROR_EMPLOYEE_NOT_FOUND);

        let employee_identity: Identity<Self::Api> = self.digital_identity_contract_proxy()
            .contract(self.digital_identity_sc().get())
            .identities(self.employees(employee_id).take())
            .execute_on_dest_context();

        self.platform_contract_proxy()
            .contract(self.platform_sc().get())
            .remove_address(employee_identity.address)
            .execute_on_dest_context::<()>();

        self.unregister_identity(employee_identity.id);
    }

    #[endpoint(changeSalary)]
    fn change_salary(&self, employee_id: u64, new_salary: BigUint) {
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);
        self.only_board_members();
        require!(!self.employees(employee_id).is_empty(), ERROR_EMPLOYEE_NOT_FOUND);

        let employee_identity: Identity<Self::Api> = self.digital_identity_contract_proxy()
            .contract(self.digital_identity_sc().get())
            .identities(self.employees(employee_id).get())
            .execute_on_dest_context();

        let opt_salary_value: Option<Value<Self::Api>> = self.digital_identity_contract_proxy()
            .contract(self.digital_identity_sc().get())
            .get_last_value_of_key(employee_identity.id, ManagedBuffer::from(SALARY_KEY))
            .execute_on_dest_context();

        match opt_salary_value {
            Some(value) => {
                self.digital_identity_contract_proxy()
                    .contract(self.digital_identity_sc().get())
                    .edit_identity_key_value(employee_identity.id, ManagedBuffer::from(SALARY_KEY), value.id, new_salary.to_bytes_be_buffer())
                    .execute_on_dest_context::<()>();
            }
            None => {
                self.digital_identity_contract_proxy()
                    .contract(self.digital_identity_sc().get())
                    .add_identity_key_value(employee_identity.id, ManagedBuffer::from(SALARY_KEY), new_salary.to_bytes_be_buffer())
                    .execute_on_dest_context::<()>();
            }
        };
    }

    #[endpoint(giveMark)]
    fn give_mark(
        &self,
        student_id: u64,
        subject: ManagedBuffer<Self::Api>,
        score: usize,
    ) {
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);
        require!(!self.students(student_id).is_empty(), ERROR_STUDENT_NOT_FOUND);

        let teacher_identity = match self.get_identity_by_address(self.blockchain().get_caller()) {
            Some(identity) => {
                identity
            }
            None => { sc_panic!(ERROR_EMPLOYEE_NOT_FOUND) }
        };
        let teacher_id = match self.get_employee_id_by_identity_id(teacher_identity.id) {
            Some(employee_id) => {
                employee_id
            }
            None => { sc_panic!(ERROR_EMPLOYEE_NOT_FOUND) }
        };

        let student_identity_id = self.students(student_id).get();
        let student_identity: Identity<Self::Api> = self.digital_identity_contract_proxy()
            .contract(self.digital_identity_sc().get())
            .identities(student_identity_id)
            .execute_on_dest_context();
        let opt_class_value: Option<Value<Self::Api>> = self.digital_identity_contract_proxy()
            .contract(self.digital_identity_sc().get())
            .get_last_value_of_key(student_identity_id, ManagedBuffer::from(CLASS_KEY))
            .execute_on_dest_context();
        let class_id = match opt_class_value {
            Some(value) => {
                BigUint::from(value.value).to_u64().unwrap()
            }
            None => { sc_panic!(ERROR_CLASS_NOT_FOUND) }
        };
        require!(self.is_teacher_of_class(teacher_identity.id, class_id, Some(subject.clone())), ERROR_NOT_TEACHER_OF_CLASS_FOR_SUBJECT);

        let mark = Mark{
            teacher_id,
            subject,
            score,
            timestamp: self.blockchain().get_block_timestamp(),
        };
        self.digital_identity_contract_proxy()
            .contract(self.digital_identity_sc().get())
            .add_identity_key_value(student_identity.id, ManagedBuffer::from(MARK_KEY), mark.to_bytes())
            .execute_on_dest_context::<()>();
    }

    #[endpoint(setAbsence)]
    fn set_absence(
        &self,
        student_id: u64,
        day_of_week: usize,
        start_time: usize,
        end_time: usize,
        subject: ManagedBuffer<Self::Api>,
    ) {
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);
        require!(!self.students(student_id).is_empty(), ERROR_STUDENT_NOT_FOUND);

        let teacher_identity = match self.get_identity_by_address(self.blockchain().get_caller()) {
            Some(identity) => {
                identity
            }
            None => { sc_panic!(ERROR_EMPLOYEE_NOT_FOUND) }
        };
        let teacher_id = match self.get_employee_id_by_identity_id(teacher_identity.id) {
            Some(employee_id) => {
                employee_id
            }
            None => { sc_panic!(ERROR_EMPLOYEE_NOT_FOUND) }
        };

        let student_identity_id = self.students(student_id).get();
        let student_identity: Identity<Self::Api> = self.digital_identity_contract_proxy()
            .contract(self.digital_identity_sc().get())
            .identities(student_identity_id)
            .execute_on_dest_context();
        let opt_class_value: Option<Value<Self::Api>> = self.digital_identity_contract_proxy()
            .contract(self.digital_identity_sc().get())
            .get_last_value_of_key(student_identity_id, ManagedBuffer::from(CLASS_KEY))
            .execute_on_dest_context();
        let class_id = match opt_class_value {
            Some(value) => {
                BigUint::from(value.value).to_u64().unwrap()
            }
            None => { sc_panic!(ERROR_CLASS_NOT_FOUND) }
        };
        require!(self.is_teacher_of_class(teacher_identity.id, class_id, Some(subject.clone())), ERROR_NOT_TEACHER_OF_CLASS_FOR_SUBJECT);

        let absence = Absence{
            employee_id: teacher_id,
            day_of_week,
            start_time,
            end_time,
            subject,
            justified: false,
            reason: ManagedBuffer::new(),
            timestamp: self.blockchain().get_block_timestamp(),
        };
        self.digital_identity_contract_proxy()
            .contract(self.digital_identity_sc().get())
            .add_identity_key_value(student_identity.id, ManagedBuffer::from(ABSENCE_KEY), absence.to_bytes())
            .execute_on_dest_context::<()>();
    }

    // helpers
    fn unregister_identity(
        &self,
        id: u64,
    ) {
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);
        self.only_board_members();

        self.digital_identity_contract_proxy()
            .contract(self.digital_identity_sc().get())
            .remove_identity_link(self.identity_id().get(), id)
            .execute_on_dest_context::<()>();
    }

    fn is_dao_board_member(&self, address: &ManagedAddress) -> bool {
        self.dao_contract_proxy()
            .contract(self.main_dao().get())
            .is_board_member(address)
            .execute_on_dest_context()
    }

    // proxies
    #[proxy]
    fn dao_contract_proxy(&self) -> tfn_dao::Proxy<Self::Api>;

    #[proxy]
    fn platform_contract_proxy(&self) -> tfn_platform::Proxy<Self::Api>;
}

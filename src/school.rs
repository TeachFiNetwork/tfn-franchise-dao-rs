multiversx_sc::imports!();

use crate::common::errors::*;
use crate::common::school_config::{self, *};
use crate::common::config::{self, State};
use crate::common::board_config;

use tfn_dao::common::config::ProxyTrait as _;
use tfn_dao::common::board_config::ProxyTrait as _;
use tfn_employee::ProxyTrait as EmployeeProxy;
use tfn_employee::common::config::ProxyTrait as _;
use tfn_student::ProxyTrait as StudentProxy;
use tfn_student::common::config::ProxyTrait as _;
use tfn_platform::ProxyTrait as PlatformProxy;

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
    #[endpoint(enrollStudent)]
    fn enroll_student(&self, name: ManagedBuffer, class_id: u64) -> ManagedAddress {
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);
        self.only_board_members();

        let template_student = self.dao_contract_proxy()
            .contract(self.main_dao().get())
            .template_student()
            .execute_on_dest_context();
        let (new_address, ()) = self.student_contract_proxy()
            .init(name)
            .deploy_from_source(
                &template_student,
                CodeMetadata::UPGRADEABLE | CodeMetadata::READABLE | CodeMetadata::PAYABLE_BY_SC,
            );
        let student = Student {
            id: self.last_student_id().get(),
            sc: new_address.clone(),
            wallet: ManagedAddress::zero(),
            class_id,
            tax_validity: 0,
        };
        self.students(student.id).set(&student);
        self.last_student_id().set(student.id + 1);

        self.platform_contract_proxy()
            .contract(self.platform_sc().get())
            .whitelist_address(student.sc)
            .execute_on_dest_context::<()>();

        new_address
    }

    #[endpoint(expellStudent)]
    fn expell_student(&self, student_id: u64) {
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);
        self.only_board_members();
        require!(!self.students(student_id).is_empty(), ERROR_STUDENT_NOT_FOUND);

        let student = self.students(student_id).get();
        self.student_contract_proxy()
            .contract(student.sc.clone())
            .set_state_inactive()
            .execute_on_dest_context::<()>();
        self.students(student_id).clear();

        self.platform_contract_proxy()
            .contract(self.platform_sc().get())
            .remove_address(student.sc)
            .execute_on_dest_context::<()>();
        if student.wallet != ManagedAddress::zero() {
            self.platform_contract_proxy()
                .contract(self.platform_sc().get())
                .remove_address(student.wallet)
                .execute_on_dest_context::<()>();
        }
    }

    #[endpoint(reEnrollStudent)]
    fn re_enroll_student(&self, class_id: u64, sc: ManagedAddress) {
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);
        self.only_board_members();

        let wallet: ManagedAddress = self.student_contract_proxy()
            .contract(sc.clone())
            .wallet()
            .execute_on_dest_context();
        let student = Student {
            id: self.last_student_id().get(),
            sc,
            wallet: wallet.clone(),
            class_id,
            tax_validity: 0,
        };
        self.students(student.id).set(&student);
        self.last_student_id().set(student.id + 1);
        self.student_contract_proxy()
            .contract(student.sc.clone())
            .set_state_active()
            .execute_on_dest_context::<()>();
        self.platform_contract_proxy()
            .contract(self.platform_sc().get())
            .whitelist_address(student.sc)
            .execute_on_dest_context::<()>();
        if wallet != ManagedAddress::zero() {
            self.platform_contract_proxy()
                .contract(self.platform_sc().get())
                .whitelist_address(wallet)
                .execute_on_dest_context::<()>();
        }
    }

    #[endpoint(changeStudentWallet)]
    fn change_student_wallet(&self, student_id: u64, new_wallet: ManagedAddress) {
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);
        self.only_board_members();
        require!(!self.students(student_id).is_empty(), ERROR_STUDENT_NOT_FOUND);

        let mut student = self.students(student_id).get();
        student.wallet = new_wallet;
        self.students(student_id).set(&student);
    }

    // employees endpoints
    #[endpoint(hireEmployee)]
    fn hire_employee(&self, name: ManagedBuffer, is_teacher: bool) -> ManagedAddress {
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);
        self.only_board_members();

        let template_employee = self.dao_contract_proxy()
            .contract(self.main_dao().get())
            .template_employee()
            .execute_on_dest_context();
        let (new_address, ()) = self.employee_contract_proxy()
            .init(name)
            .deploy_from_source(
                &template_employee,
                CodeMetadata::UPGRADEABLE | CodeMetadata::READABLE | CodeMetadata::PAYABLE_BY_SC,
            );
        let employee = Employee {
            id: self.last_employee_id().get(),
            sc: new_address.clone(),
            wallet: ManagedAddress::zero(),
            is_teacher,
        };
        self.employees(employee.id).set(&employee);
        self.last_employee_id().set(employee.id + 1);

        new_address
    }

    #[endpoint(fireEmployee)]
    fn fire_employee(&self, employee_id: u64) {
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);
        self.only_board_members();
        require!(!self.employees(employee_id).is_empty(), ERROR_EMPLOYEE_NOT_FOUND);

        let employee = self.employees(employee_id).get();
        self.employee_contract_proxy()
            .contract(employee.sc)
            .set_state_inactive()
            .execute_on_dest_context::<()>();
        self.employees(employee_id).clear();
    }

    #[endpoint(reHireEmployee)]
    fn re_hire_employee(&self, sc: ManagedAddress, is_teacher: bool) {
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);
        self.only_board_members();

        let wallet = self.employee_contract_proxy()
            .contract(sc.clone())
            .wallet()
            .execute_on_dest_context();
        let employee = Employee {
            id: self.last_employee_id().get(),
            sc,
            wallet,
            is_teacher,
        };
        self.employees(employee.id).set(&employee);
        self.last_employee_id().set(employee.id + 1);
        self.employee_contract_proxy()
            .contract(employee.sc.clone())
            .set_state_active()
            .execute_on_dest_context::<()>();
    }

    #[endpoint(changeEmployeeWallet)]
    fn change_employee_wallet(&self, employee_id: u64, new_wallet: ManagedAddress) {
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);
        self.only_board_members();
        require!(!self.employees(employee_id).is_empty(), ERROR_EMPLOYEE_NOT_FOUND);

        let mut employee = self.employees(employee_id).get();
        employee.wallet = new_wallet;
        self.employees(employee_id).set(&employee);
    }

    // upgrade endpoints
    #[endpoint(upgradeStudent)]
    fn upgrade_student(
        &self,
        address: ManagedAddress,
        args: OptionalValue<ManagedArgBuffer<Self::Api>>
    ) {
        let student = self.get_student_by_wallet_or_address(address);
        require!(student.is_some(), ERROR_STUDENT_NOT_FOUND);

        let caller = self.blockchain().get_caller();
        if caller != student.clone().unwrap().wallet && !self.is_dao_board_member(&caller) {
            self.only_board_members();
        }

        let upgrade_args = match args {
            OptionalValue::Some(args) => args,
            OptionalValue::None => ManagedArgBuffer::new(),            
        };
        let template_student: ManagedAddress = self.dao_contract_proxy()
            .contract(self.main_dao().get())
            .template_student()
            .execute_on_dest_context();
        self.tx()
            .to(student.unwrap().sc)
            .gas(self.blockchain().get_gas_left())
            .raw_upgrade()
            .arguments_raw(upgrade_args)
            .from_source(template_student)
            .code_metadata(CodeMetadata::UPGRADEABLE | CodeMetadata::READABLE | CodeMetadata::PAYABLE_BY_SC)
            .upgrade_async_call_and_exit();
    }

    #[endpoint(upgradeEmployee)]
    fn upgrade_employee(
        &self,
        address: ManagedAddress,
        args: OptionalValue<ManagedArgBuffer<Self::Api>>
    ) {
        let employee = self.get_employee_by_wallet_or_address(address);
        require!(employee.is_some(), ERROR_EMPLOYEE_NOT_FOUND);

        let caller = self.blockchain().get_caller();
        if caller != employee.clone().unwrap().wallet && !self.is_dao_board_member(&caller) {
            self.only_board_members();
        }

        let upgrade_args = match args {
            OptionalValue::Some(args) => args,
            OptionalValue::None => ManagedArgBuffer::new(),            
        };
        let template_employee: ManagedAddress = self.dao_contract_proxy()
            .contract(self.main_dao().get())
            .template_employee()
            .execute_on_dest_context();
        self.tx()
            .to(employee.unwrap().sc)
            .gas(self.blockchain().get_gas_left())
            .raw_upgrade()
            .arguments_raw(upgrade_args)
            .from_source(template_employee)
            .code_metadata(CodeMetadata::UPGRADEABLE | CodeMetadata::READABLE | CodeMetadata::PAYABLE_BY_SC)
            .upgrade_async_call_and_exit();
    }

    // helpers
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
    fn employee_contract_proxy(&self) -> tfn_employee::Proxy<Self::Api>;

    #[proxy]
    fn student_contract_proxy(&self) -> tfn_student::Proxy<Self::Api>;

    #[proxy]
    fn platform_contract_proxy(&self) -> tfn_platform::Proxy<Self::Api>;
}

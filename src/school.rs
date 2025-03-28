use crate::common::errors::*;
use crate::common::school_config::{self, *};
use crate::common::config::{self, State};

multiversx_sc::imports!();

use tfn_employee::ProxyTrait as EmployeeProxy;
use tfn_employee::common::config::ProxyTrait as _;
use tfn_student::ProxyTrait as StudentProxy;
use tfn_student::common::config::ProxyTrait as _;

#[multiversx_sc::module]
pub trait SchoolModule:
school_config::SchoolConfigModule
+config::ConfigModule
{
    // classes endpoints
    #[endpoint(createClass)]
    fn create_class(&self, year: usize, name: ManagedBuffer<Self::Api>) -> u64{
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);
        self.only_owner();

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
        self.only_owner();
        require!(!self.classes(class_id).is_empty(), ERROR_CLASS_NOT_FOUND);

        let mut class = self.classes(class_id).get();
        class.year = year;
        class.name = name;
        self.classes(class_id).set(class);
    }

    #[endpoint(setClassSchedule)]
    fn set_class_schedule(&self, class_id: u64, schedule: ManagedVec<Self::Api, SubjectSlot<Self::Api>>) {
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);
        self.only_owner();
        require!(!self.classes(class_id).is_empty(), ERROR_CLASS_NOT_FOUND);

        let mut class = self.classes(class_id).get();
        class.schedule = schedule;
        self.classes(class_id).set(class);
    }

    // students endpoints
    #[endpoint(enrollStudent)]
    fn enroll_student(&self, name: ManagedBuffer, class_id: u64) {
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);
        self.only_owner();

        let (new_address, ()) = self.student_contract_proxy()
            .init(name)
            .deploy_from_source(
                &self.template_student().get(),
                CodeMetadata::UPGRADEABLE | CodeMetadata::READABLE | CodeMetadata::PAYABLE_BY_SC,
            );
        let student = Student {
            id: self.last_student_id().get(),
            sc: new_address,
            wallet: ManagedAddress::zero(),
            class_id,
            tax_validity: 0,
        };
        self.students(student.id).set(&student);
        self.last_student_id().set(student.id + 1);
    }

    #[endpoint(expellStudent)]
    fn expell_student(&self, student_id: u64) {
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);
        self.only_owner();
        require!(!self.students(student_id).is_empty(), ERROR_STUDENT_NOT_FOUND);

        let student = self.students(student_id).get();
        self.student_contract_proxy()
            .contract(student.sc)
            .set_state_inactive()
            .execute_on_dest_context::<()>();
        self.students(student_id).clear();
    }

    #[endpoint(reEnrollStudent)]
    fn re_enroll_student(&self, class_id: u64, sc: ManagedAddress) {
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);
        self.only_owner();

        let wallet = self.student_contract_proxy()
            .contract(sc.clone())
            .wallet()
            .execute_on_dest_context();
        let student = Student {
            id: self.last_student_id().get(),
            sc,
            wallet,
            class_id,
            tax_validity: 0,
        };
        self.students(student.id).set(&student);
        self.last_student_id().set(student.id + 1);
        self.student_contract_proxy()
            .contract(student.sc.clone())
            .set_state_active()
            .execute_on_dest_context::<()>();
    }

    #[endpoint(changeStudentWallet)]
    fn change_student_wallet(&self, student_id: u64, new_wallet: ManagedAddress) {
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);
        self.only_owner();
        require!(!self.students(student_id).is_empty(), ERROR_STUDENT_NOT_FOUND);

        let mut student = self.students(student_id).get();
        student.wallet = new_wallet;
        self.students(student_id).set(&student);
    }

    // employees endpoints
    #[endpoint(hireEmployee)]
    fn hire_employee(&self, name: ManagedBuffer, is_teacher: bool) {
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);
        self.only_owner();

        let (new_address, ()) = self.employee_contract_proxy()
            .init(name)
            .deploy_from_source(
                &self.template_employee().get(),
                CodeMetadata::UPGRADEABLE | CodeMetadata::READABLE | CodeMetadata::PAYABLE_BY_SC,
            );
        let employee = Employee {
            id: self.last_employee_id().get(),
            sc: new_address,
            wallet: ManagedAddress::zero(),
            is_teacher,
        };
        self.employees(employee.id).set(&employee);
        self.last_employee_id().set(employee.id + 1);
    }

    #[endpoint(fireEmployee)]
    fn fire_employee(&self, employee_id: u64) {
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);
        self.only_owner();
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
        self.only_owner();

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

    // proxies
    #[proxy]
    fn employee_contract_proxy(&self) -> tfn_employee::Proxy<Self::Api>;

    #[proxy]
    fn student_contract_proxy(&self) -> tfn_student::Proxy<Self::Api>;
}

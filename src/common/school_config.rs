multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use core::ops::Deref;

use crate::common::{consts::{CLASS_KEY, STUDENT_RELATION}, errors::*};
use super::board_config;
use tfn_digital_identity::common::config::{ProxyTrait as _, Identity};

#[type_abi]
#[derive(ManagedVecItem, TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Eq, Clone, Debug)]
pub struct Mark<M: ManagedTypeApi> {
    pub teacher_id: u64,
    pub subject: ManagedBuffer<M>,
    pub score: usize,
    pub timestamp: u64,
}

impl<M> Mark<M>
where
    M: ManagedTypeApi,
{
    pub fn to_bytes(&self) -> ManagedBuffer<M> {
        let mut items: ManagedVec<M, ManagedBuffer<M>> = ManagedVec::new();
        items.push(BigUint::from(self.teacher_id).to_bytes_be_buffer());
        items.push(self.subject.clone());
        items.push(BigUint::from(self.score).to_bytes_be_buffer());
        items.push(BigUint::from(self.timestamp).to_bytes_be_buffer());
        let mut buffer = ManagedBuffer::<M>::new();
        _ = items.top_encode(&mut buffer);
        
        buffer
    }

    pub fn from_bytes(buffer: ManagedBuffer<M>) -> Self {
        let result = ManagedVec::<M, ManagedBuffer<M>>::top_decode(buffer);
        let items: ManagedVec<M, ManagedBuffer<M>> = result.unwrap();
        let teacher_id: BigUint<M> = BigUint::from_bytes_be_buffer(items.get(0).deref());
        let subject = items.get(1).clone();
        let score: BigUint<M> = BigUint::from_bytes_be_buffer(items.get(2).deref());
        let timestamp: BigUint<M> = BigUint::from_bytes_be_buffer(items.get(3).deref());

        Mark {
            teacher_id: teacher_id.to_u64().unwrap(),
            subject: subject.deref().clone(),
            score: score.to_u64().unwrap() as usize,
            timestamp: timestamp.to_u64().unwrap(),
        }
    }
}

#[type_abi]
#[derive(ManagedVecItem, TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Eq, Clone, Debug)]
pub struct Absence<M: ManagedTypeApi> {
    pub employee_id: u64,
    pub day_of_week: usize,
    pub start_time: usize,
    pub end_time: usize,
    pub subject: ManagedBuffer<M>,

    pub justified: bool,
    pub reason: ManagedBuffer<M>,

    pub timestamp: u64,
}

impl<M> Absence<M>
where
    M: ManagedTypeApi,
{
    pub fn to_bytes(&self) -> ManagedBuffer<M> {
        let mut items: ManagedVec<M, ManagedBuffer<M>> = ManagedVec::new();
        items.push(BigUint::from(self.employee_id).to_bytes_be_buffer());
        items.push(BigUint::from(self.day_of_week).to_bytes_be_buffer());
        items.push(BigUint::from(self.start_time).to_bytes_be_buffer());
        items.push(BigUint::from(self.end_time).to_bytes_be_buffer());
        items.push(self.subject.clone());
        items.push(BigUint::from(self.justified as u8).to_bytes_be_buffer());
        items.push(self.reason.clone());
        items.push(BigUint::from(self.timestamp).to_bytes_be_buffer());
        let mut buffer = ManagedBuffer::<M>::new();
        _ = items.top_encode(&mut buffer);

        buffer
    }

    pub fn from_bytes(buffer: ManagedBuffer<M>) -> Self {
        let result = ManagedVec::<M, ManagedBuffer<M>>::top_decode(buffer);
        let items: ManagedVec<M, ManagedBuffer<M>> = result.unwrap();
        let employee_id: BigUint<M> = BigUint::from_bytes_be_buffer(items.get(0).deref());
        let day_of_week: BigUint<M> = BigUint::from_bytes_be_buffer(items.get(1).deref());
        let start_time: BigUint<M> = BigUint::from_bytes_be_buffer(items.get(2).deref());
        let end_time: BigUint<M> = BigUint::from_bytes_be_buffer(items.get(3).deref());
        let subject = items.get(4).clone();
        let justified: BigUint<M> = BigUint::from_bytes_be_buffer(items.get(5).deref());
        let reason = items.get(6).clone();
        let timestamp: BigUint<M> = BigUint::from_bytes_be_buffer(items.get(7).deref());

        Absence {
            employee_id: employee_id.to_u64().unwrap(),
            day_of_week: day_of_week.to_u64().unwrap() as usize,
            start_time: start_time.to_u64().unwrap() as usize,
            end_time: end_time.to_u64().unwrap() as usize,
            subject: subject.deref().clone(),
            justified: justified.to_u64().unwrap() == 1,
            reason: reason.deref().clone(),
            timestamp: timestamp.to_u64().unwrap(),
        }
    }
}

#[type_abi]
#[derive(NestedEncode, NestedDecode, TopEncode, TopDecode, ManagedVecItem, PartialEq, Eq, Clone, Debug)]
pub struct SubjectSlot<M: ManagedTypeApi> {
    pub id: u64,
    pub day_of_week: usize,
    pub start_time: usize,
    pub end_time: usize,
    pub subject: ManagedBuffer<M>,
    pub teacher_id: u64,
}

#[type_abi]
#[derive(NestedEncode, NestedDecode, TopEncode, TopDecode, ManagedVecItem, PartialEq, Eq, Clone, Debug)]
pub struct Class<M: ManagedTypeApi> {
    pub id: u64,
    pub year: usize,
    pub name: ManagedBuffer<M>,
    pub schedule: ManagedVec<M, SubjectSlot<M>>,
}

#[multiversx_sc::module]
pub trait SchoolConfigModule:
super::config::ConfigModule
+board_config::BoardConfigModule
{
    // classes
    #[view(getClass)]
    #[storage_mapper("classes")]
    fn classes(&self, id: u64) -> SingleValueMapper<Class<Self::Api>>;

    #[view(getLastClassId)]
    #[storage_mapper("last_class_id")]
    fn last_class_id(&self) -> SingleValueMapper<u64>;

    #[view(getClassesCount)]
    fn classes_count(&self) -> usize {
        let mut count = 0;
        for i in 0..self.last_class_id().get() {
            if !self.classes(i).is_empty() {
                count += 1;
            }
        }

        count
    }

    #[view(getClasses)]
    fn get_classes(&self) -> ManagedVec<Self::Api, Class<Self::Api>> {
        let mut classes = ManagedVec::new();
        for i in 0..self.last_class_id().get() {
            if !self.classes(i).is_empty() {
                classes.push(self.classes(i).get());
            }
        }

        classes
    }

    #[view(getClassStudents)]
    fn get_class_students(&self, class_id: u64) -> ManagedVec<Identity<Self::Api>> {
        require!(!self.classes(class_id).is_empty(), ERROR_CLASS_NOT_FOUND);

        let students: ManagedVec<Identity<Self::Api>> = self.digital_identity_contract_proxy()
            .contract(self.digital_identity_sc().get())
            .get_children_with_same_last_value(
                self.identity_id().get(),
                ManagedBuffer::from(CLASS_KEY),
                BigUint::from(class_id).to_bytes_be_buffer(),
                OptionalValue::Some(STUDENT_RELATION),
            )
            .execute_on_dest_context();

        students
    }

    #[view(getClassTeachers)]
    fn get_class_teachers(&self, class_id: u64) -> ManagedVec<Identity<Self::Api>> {
        let class = self.classes(class_id).get();
        let mut teachers_ids: ManagedVec<u64> = ManagedVec::new();
        for time_slot in class.schedule.iter() {
            let mut found = false;
            for teacher_id in teachers_ids.into_iter() {
                if teacher_id == time_slot.teacher_id {
                    found = true;
                    break;
                }
            }
            if !found {
                teachers_ids.push(time_slot.teacher_id);
            }
        }
        let teachers: ManagedVec<Identity<Self::Api>> = self.digital_identity_contract_proxy()
            .contract(self.digital_identity_sc().get())
            .get_multiple_identities(teachers_ids)
            .execute_on_dest_context();

        teachers
    }

    #[view(getFullClassInfo)]
    fn get_full_class_info(&self, class_id: u64) -> (Class<Self::Api>, ManagedVec<Identity<Self::Api>>, ManagedVec<Identity<Self::Api>>) {
        let students = self.get_class_students(class_id);
        let teachers = self.get_class_teachers(class_id);

        (self.classes(class_id).get(), students, teachers)
    }

    // employees
    #[view(getEmployee)]
    #[storage_mapper("employees")]
    fn employees(&self, id: u64) -> SingleValueMapper<u64>;

    #[view(getLastEmployeeId)]
    #[storage_mapper("last_employee_id")]
    fn last_employee_id(&self) -> SingleValueMapper<u64>;

    #[view(getEmployeesCount)]
    fn employees_count(&self) -> usize {
        let mut count = 0;
        for i in 0..self.last_employee_id().get() {
            if !self.employees(i).is_empty() {
                count += 1;
            }
        }
        count
    }

    #[view(getEmployees)]
    fn get_employees(&self) -> ManagedVec<Identity<Self::Api>> {
        let mut employees_ids = ManagedVec::new();
        for i in 0..self.last_employee_id().get() {
            if !self.employees(i).is_empty() {
                employees_ids.push(self.employees(i).get());
            }
        }

        let employees: ManagedVec<Identity<Self::Api>> = self.digital_identity_contract_proxy()
            .contract(self.digital_identity_sc().get())
            .get_multiple_identities(employees_ids)
            .execute_on_dest_context();
        employees
    }

    #[view(getIdentityByAddress)]
    fn get_identity_by_address(&self, address: ManagedAddress) -> Option<Identity<Self::Api>> {
        self.digital_identity_contract_proxy()
            .contract(self.digital_identity_sc().get())
            .get_identity_by_address(&address)
            .execute_on_dest_context()
    }

    #[view(getEmployeeIdByIdentityId)]
    fn get_employee_id_by_identity_id(&self, identity_id: u64) -> Option<u64> {
        for employee_id in 0..self.last_employee_id().get() {
            if self.employees(employee_id).is_empty() {
                continue;
            }

            if self.employees(employee_id).get() == identity_id {
                return Some(employee_id);
            }
        }

        None
    }

    #[view(isTeacherOfCass)]
    fn is_teacher_of_class(
        &self,
        teacher_id: u64,
        class_id: u64,
        opt_subject: Option<ManagedBuffer<Self::Api>>,
    ) -> bool {
        let class = self.classes(class_id).get();
        let (all, subject) = match opt_subject {
            Some(subject) => (false, subject),
            None => (true, ManagedBuffer::new()),
        };
        for time_slot in class.schedule.iter() {
            if time_slot.teacher_id == teacher_id && (all || time_slot.subject == subject) {
                return true;
            }
        }

        false
    }

    // students
    #[view(getStudent)]
    #[storage_mapper("students")]
    fn students(&self, id: u64) -> SingleValueMapper<u64>;

    #[view(getLastStudentId)]
    #[storage_mapper("last_student_id")]
    fn last_student_id(&self) -> SingleValueMapper<u64>;

    #[view(getStudentsCount)]
    fn students_count(&self) -> usize {
        let mut count = 0;
        for i in 0..self.last_student_id().get() {
            if !self.students(i).is_empty() {
                count += 1;
            }
        }
        count
    }

    #[view(getStudents)]
    fn get_students(&self) -> ManagedVec<Identity<Self::Api>> {
        let mut students_ids = ManagedVec::new();
        for i in 0..self.last_student_id().get() {
            if !self.students(i).is_empty() {
                students_ids.push(self.students(i).get());
            }
        }

        let students: ManagedVec<Identity<Self::Api>> = self.digital_identity_contract_proxy()
            .contract(self.digital_identity_sc().get())
            .get_multiple_identities(students_ids)
            .execute_on_dest_context();

        students
    }

    // tax amount
    #[view(getTaxAmount)]
    #[storage_mapper("tax_amount")]
    fn tax_amount(&self) -> SingleValueMapper<BigUint>;

    #[endpoint(setTaxAmount)]
    fn set_tax_amount(&self, new_tax_amount: BigUint) {
        self.only_board_members();

        self.tax_amount().set(new_tax_amount);
    }

    // proxies

    #[proxy]
    fn digital_identity_contract_proxy(&self) -> tfn_digital_identity::Proxy<Self::Api>;
}

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::common::errors::*;
use super::board_config;

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
pub struct Employee<M: ManagedTypeApi> {
    pub id: u64,
    pub sc: ManagedAddress<M>,
    pub wallet: ManagedAddress<M>,
    pub is_teacher: bool,
}

#[type_abi]
#[derive(NestedEncode, NestedDecode, TopEncode, TopDecode, ManagedVecItem, PartialEq, Eq, Clone, Debug)]
pub struct Student<M: ManagedTypeApi> {
    pub id: u64,
    pub sc: ManagedAddress<M>,
    pub wallet: ManagedAddress<M>,
    pub class_id: u64,
    pub tax_validity: u64,
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
    fn get_class_students(&self, class_id: u64) -> ManagedVec<Self::Api, Student<Self::Api>> {
        require!(!self.classes(class_id).is_empty(), ERROR_CLASS_NOT_FOUND);

        let mut students = ManagedVec::new();
        for i in 0..self.last_student_id().get() {
            if self.students(i).is_empty() {
                continue;
            }

            let student = self.students(i).get();
            if student.class_id == class_id {
                students.push(student);
            }
        }

        students
    }

    #[view(getClassTeachers)]
    fn get_class_teachers(&self, class_id: u64) -> ManagedVec<Self::Api, Employee<Self::Api>> {
        let class = self.classes(class_id).get();
        let mut teachers: ManagedVec<Self::Api, Employee<Self::Api>> = ManagedVec::new();
        for time_slot in class.schedule.iter() {
            let mut found = false;
            for t in teachers.into_iter() {
                if t.id == time_slot.teacher_id {
                    found = true;
                    break;
                }
            }
            if !found {
                teachers.push(self.employees(time_slot.teacher_id).get());
            }
        }

        teachers
    }

    #[view(getFullClassInfo)]
    fn get_full_class_info(&self, class_id: u64) -> (Class<Self::Api>, ManagedVec<Self::Api, Student<Self::Api>>, ManagedVec<Self::Api, Employee<Self::Api>>) {
        let students = self.get_class_students(class_id);
        let teachers = self.get_class_teachers(class_id);

        (self.classes(class_id).get(), students, teachers)
    }

    // employees
    #[view(getEmployee)]
    #[storage_mapper("employees")]
    fn employees(&self, id: u64) -> SingleValueMapper<Employee<Self::Api>>;

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
    fn get_employees(&self) -> ManagedVec<Self::Api, Employee<Self::Api>> {
        let mut employees = ManagedVec::new();
        for i in 0..self.last_employee_id().get() {
            if !self.employees(i).is_empty() {
                employees.push(self.employees(i).get());
            }
        }
        employees
    }

    #[view(getEmployeeByAddress)]
    fn get_employee_by_address(&self, address: ManagedAddress) -> Option<Employee<Self::Api>> {
        for i in 0..self.last_employee_id().get() {
            if self.employees(i).is_empty() {
                continue;
            }

            let employee = self.employees(i).get();
            if employee.sc == address {
                return Some(employee);
            }
        }

        None
    }

    #[view(getEmployeeByWallet)]
    fn get_employee_by_wallet(&self, address: ManagedAddress) -> Option<Employee<Self::Api>> {
        for i in 0..self.last_employee_id().get() {
            if self.employees(i).is_empty() {
                continue;
            }

            let employee = self.employees(i).get();
            if employee.wallet == address {
                return Some(employee);
            }
        }

        None
    }

    #[view(getEmployeeByWalletOrAddress)]
    fn get_employee_by_wallet_or_address(&self, address: ManagedAddress) -> Option<Employee<Self::Api>> {
        for i in 0..self.last_employee_id().get() {
            if self.employees(i).is_empty() {
                continue;
            }

            let employee = self.employees(i).get();
            if employee.wallet == address || employee.sc == address {
                return Some(employee);
            }
        }

        None
    }

    // students
    #[view(getStudent)]
    #[storage_mapper("students")]
    fn students(&self, id: u64) -> SingleValueMapper<Student<Self::Api>>;

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
    fn get_students(&self) -> ManagedVec<Self::Api, Student<Self::Api>> {
        let mut students = ManagedVec::new();
        for i in 0..self.last_student_id().get() {
            if !self.students(i).is_empty() {
                students.push(self.students(i).get());
            }
        }
        students
    }

    #[view(getStudentByAddress)]
    fn get_student_by_address(&self, address: ManagedAddress) -> Option<Student<Self::Api>> {
        for i in 0..self.last_student_id().get() {
            if self.students(i).is_empty() {
                continue;
            }

            let student = self.students(i).get();
            if student.sc == address {
                return Some(student);
            }
        }

        None
    }

    #[view(getStudentByWallet)]
    fn get_student_by_wallet(&self, address: ManagedAddress) -> Option<Student<Self::Api>> {
        for i in 0..self.last_student_id().get() {
            if self.students(i).is_empty() {
                continue;
            }

            let student = self.students(i).get();
            if student.wallet == address {
                return Some(student);
            }
        }

        None
    }

    #[view(getStudentByWalletOrAddress)]
    fn get_student_by_wallet_or_address(&self, address: ManagedAddress) -> Option<Student<Self::Api>> {
        for i in 0..self.last_student_id().get() {
            if self.students(i).is_empty() {
                continue;
            }

            let student = self.students(i).get();
            if student.wallet == address || student.sc == address {
                return Some(student);
            }
        }

        None
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
}

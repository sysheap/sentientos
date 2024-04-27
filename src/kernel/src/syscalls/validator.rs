use common::{
    net::UDPDescriptor,
    syscalls::userspace_argument::{UserspaceArgument, UserspaceArgumentValueExtractor},
};

use crate::processes::scheduler::get_current_process_expect;

pub trait FailibleSliceValidator<'a, T: 'a> {
    fn validate(self, len: usize) -> Result<&'a T, ()>;
}

pub trait FailibleMutableSliceValidator<'a, T: 'a> {
    fn validate(self, len: usize) -> Result<&'a mut T, ()>;
}

pub trait UserspaceArgumentValidator<T> {
    fn validate(self) -> T;
}

macro_rules! simple_type {
    ($type:ty) => {
        impl UserspaceArgumentValidator<$type> for UserspaceArgument<$type> {
            fn validate(self) -> $type {
                self.get()
            }
        }
    };
}

simple_type!(char);
simple_type!(u16);
simple_type!(usize);
simple_type!(isize);
simple_type!(u64);
simple_type!(UDPDescriptor);

impl<'a> FailibleSliceValidator<'a, u8> for UserspaceArgument<&'a u8> {
    fn validate(self, len: usize) -> Result<&'a u8, ()> {
        let current_process = get_current_process_expect();
        let current_process = current_process.borrow();
        let page_table = current_process.get_page_table();

        let addr = self.get() as *const u8;
        let last = addr.wrapping_add(len - 1);

        if page_table
            .translate_userspace_address_to_physical_address(last)
            .is_none()
        {
            return Err(());
        }

        page_table
            .translate_userspace_address_to_physical_address(addr)
            .map(|ptr| unsafe { &*ptr })
            .ok_or(())
    }
}
impl<'a> FailibleMutableSliceValidator<'a, u8> for UserspaceArgument<&'a mut u8> {
    fn validate(self, len: usize) -> Result<&'a mut u8, ()> {
        let current_process = get_current_process_expect();
        let current_process = current_process.borrow();
        let page_table = current_process.get_page_table();

        let addr = self.get() as *mut u8;
        let last = addr.wrapping_add(len - 1);

        if page_table
            .translate_userspace_address_to_physical_address(last)
            .is_none()
        {
            return Err(());
        }

        page_table
            .translate_userspace_address_to_physical_address(addr)
            .map(|ptr| unsafe { &mut *(ptr as *mut _) })
            .ok_or(())
    }
}

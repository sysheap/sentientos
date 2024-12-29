use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign};

use common::numbers::Number;

#[allow(clippy::upper_case_acronyms)]
pub struct MMIO<T> {
    addr: *mut T,
}

impl<T> MMIO<T> {
    pub const fn new(addr: usize) -> Self {
        Self {
            addr: addr as *mut T,
        }
    }

    pub const unsafe fn add(&self, count: usize) -> Self {
        unsafe {
            Self {
                addr: self.addr.add(count),
            }
        }
    }

    pub const unsafe fn new_type<U>(&self) -> MMIO<U> {
        unsafe { self.new_type_with_offset(0) }
    }

    pub const unsafe fn new_type_with_offset<U>(&self, offset: usize) -> MMIO<U> {
        unsafe {
            MMIO::<U> {
                addr: self.addr.byte_add(offset) as *mut U,
            }
        }
    }
}

impl<T: Copy> MMIO<T> {
    pub fn read(&self) -> T {
        unsafe { self.addr.read_volatile() }
    }

    pub fn write(&mut self, value: T) {
        unsafe {
            self.addr.write_volatile(value);
        }
    }
}

impl<T: Copy, const LENGTH: usize> MMIO<[T; LENGTH]> {
    pub fn read_index(&self, index: usize) -> T {
        self.get_index(index).read()
    }

    pub fn write_index(&mut self, index: usize, value: T) {
        self.get_index(index).write(value);
    }

    fn get_index(&self, index: usize) -> MMIO<T> {
        assert!(index < LENGTH, "Access out of bounds");
        unsafe { self.new_type_with_offset(index * core::mem::size_of::<T>()) }
    }
}

impl<T: Number + BitOr<T, Output = T>> BitOrAssign<T> for MMIO<T> {
    fn bitor_assign(&mut self, rhs: T) {
        self.write(self.read() | rhs)
    }
}

impl<T: Number + BitAnd<T, Output = T>> BitAndAssign<T> for MMIO<T> {
    fn bitand_assign(&mut self, rhs: T) {
        self.write(self.read() & rhs)
    }
}

unsafe impl<T> Send for MMIO<T> {}

impl<T> core::fmt::Pointer for MMIO<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:p}", self.addr)
    }
}

impl<T: core::fmt::Debug + Copy> core::fmt::Debug for MMIO<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self.read())
    }
}

#[macro_export]
macro_rules! mmio_struct {
    {
        $(#[$meta:meta])*
        struct $name:ident {
            $($field_name:ident : $field_type:ty),* $(,)?
        }
    } => {
            $(#[$meta])*
            #[derive(Clone, Copy, Debug)]
            #[allow(non_camel_case_types, dead_code)]
            pub struct $name {
                $(
                    $field_name: $field_type,
                )*
            }

            impl $crate::klibc::mmio::MMIO<$name> {
                $(
                    #[allow(dead_code)]
                    pub const fn $field_name(&self) -> $crate::klibc::mmio::MMIO<$field_type> {
                        unsafe {
                            self.new_type_with_offset(core::mem::offset_of!($name, $field_name))
                        }
                    }
                )*
            }
        };
}

#[cfg(test)]
mod tests {
    use core::{
        any::Any,
        cell::UnsafeCell,
        mem::offset_of,
        ptr::{addr_of, addr_of_mut},
    };

    use crate::io::uart::QEMU_UART;

    use super::*;

    mmio_struct! {
        #[repr(C)]
        struct mmio_b {
            b1: u16,
            b2: [u8; 3],
            b3: u64,
        }
    }

    mmio_struct! {
        #[repr(C)]
        struct mmio_a{
            a1: u64,
            a2: u8,
            a3: mmio_b,
            a4: u8
        }
    }

    fn get_test_data() -> mmio_a {
        mmio_a {
            a1: 18,
            a2: 43,
            a3: mmio_b {
                b1: 20,
                b2: [100, 102, 103],
                b3: 22,
            },
            a4: 199,
        }
    }

    macro_rules! check_offset {
        ($value:ident, $mmio: ident, $( $field_path:ident ).+) => {
            let addr1 = addr_of!($value.$($field_path).+ );
            let addr2 = $mmio.$( $field_path()).+.addr;
            assert_eq!(addr1, addr2);
        };
    }

    #[test_case]
    fn print_works() {
        let value = get_test_data();

        unsafe {
            QEMU_UART.disarm();
        }

        crate::println!("value at {:p}", &value);

        let mmio = MMIO::<mmio_a>::new(&value as *const _ as usize);

        crate::println!("{:?}", mmio);
    }

    #[test_case]
    fn offsets() {
        let value = get_test_data();

        let mmio = MMIO::<mmio_a>::new(&value as *const _ as usize);

        check_offset!(value, mmio, a1);
        check_offset!(value, mmio, a2);
        check_offset!(value, mmio, a3);

        check_offset!(value, mmio, a3.b1);
        check_offset!(value, mmio, a3.b2);
        check_offset!(value, mmio, a3.b3);

        check_offset!(value, mmio, a4);
    }

    #[test_case]
    fn struct_case() {
        let value = UnsafeCell::new(get_test_data());
        let ptr = value.get();

        let mmio = MMIO::<mmio_a>::new(ptr as usize);

        mmio.a1().write(0);
        mmio.a2().write(1);
        mmio.a3().b1().write(2);
        mmio.a3().b2().write_index(0, 3);
        mmio.a3().b2().write_index(1, 4);
        mmio.a3().b2().write_index(2, 5);
        mmio.a3().b3().write(6);
        mmio.a4().write(7);

        drop(mmio);

        let read_value = unsafe { value.get().read_unaligned() };
        unsafe {
            assert_eq!(core::ptr::addr_of!(read_value.a1).read_unaligned(), 0);
            assert_eq!(core::ptr::addr_of!(read_value.a2).read_unaligned(), 1);
            assert_eq!(core::ptr::addr_of!(read_value.a3.b1).read_unaligned(), 2);
            assert_eq!(core::ptr::addr_of!(read_value.a3.b2[0]).read_unaligned(), 3);
            assert_eq!(core::ptr::addr_of!(read_value.a3.b2[1]).read_unaligned(), 4);
            assert_eq!(core::ptr::addr_of!(read_value.a3.b2[2]).read_unaligned(), 5);
            assert_eq!(core::ptr::addr_of!(read_value.a3.b3).read_unaligned(), 6);
            assert_eq!(core::ptr::addr_of!(read_value.a4).read_unaligned(), 7);
        }
    }

    #[test_case]
    fn scalar() {
        let mut value = UnsafeCell::new(42);
        let ptr = value.get();

        let mut mmio = MMIO::<i32>::new(ptr as usize);

        assert_eq!(mmio.addr as *const i32, ptr);

        assert_eq!(mmio.read(), 42);

        mmio.write(128);

        drop(mmio);

        assert_eq!(*value.get_mut(), 128);
    }
}

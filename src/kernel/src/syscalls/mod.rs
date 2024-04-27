mod validator;

use core::ptr::{slice_from_raw_parts, slice_from_raw_parts_mut};

use alloc::string::String;
use common::{
    net::UDPDescriptor,
    syscalls::{
        kernel::KernelSyscalls, userspace_argument::UserspaceArgument, SysExecuteError,
        SysSocketError, SysWaitError,
    },
};

use crate::{
    debug,
    io::stdin_buf::STDIN_BUFFER,
    klibc::macros::unwrap_or_return,
    net::{udp::UdpHeader, ARP_CACHE, OPEN_UDP_SOCKETS},
    print,
    processes::scheduler::{self, get_current_process_expect, let_current_process_wait_for},
    syscalls::validator::UserspaceArgumentValidator,
};

use self::validator::{FailibleMutableSliceValidator, FailibleSliceValidator};

struct SyscallHandler;

impl KernelSyscalls for SyscallHandler {
    fn sys_write_char(c: UserspaceArgument<char>) {
        print!("{}", c.validate());
    }

    fn sys_read_input() -> Option<u8> {
        let mut stdin = STDIN_BUFFER.lock();
        stdin.pop()
    }

    fn sys_exit(status: UserspaceArgument<isize>) {
        debug!("Exit process with status: {}\n", status.validate());
        scheduler::kill_current_process();
    }

    fn sys_execute(
        name: UserspaceArgument<&u8>,
        length: UserspaceArgument<usize>,
    ) -> Result<u64, SysExecuteError> {
        let length = length.validate();

        if let Ok(physical_address) = name.validate(length) {
            let slice = unsafe { &*slice_from_raw_parts(physical_address, length) };
            let mut name = String::with_capacity(length);
            for c in slice {
                name.push(*c as char);
            }

            if let Some(pid) = scheduler::schedule_program(&name) {
                Ok(pid)
            } else {
                Err(SysExecuteError::InvalidProgram)
            }
        } else {
            Err(SysExecuteError::InvalidPtr)
        }
    }

    fn sys_wait(pid: UserspaceArgument<u64>) -> Result<(), SysWaitError> {
        if let_current_process_wait_for(pid.validate()) {
            Ok(())
        } else {
            Err(SysWaitError::InvalidPid)
        }
    }

    fn sys_mmap_pages(number_of_pages: UserspaceArgument<usize>) -> *mut u8 {
        let current_process = get_current_process_expect();
        let mut current_process = current_process.borrow_mut();
        current_process.mmap_pages(number_of_pages.validate())
    }

    fn sys_open_udp_socket(port: UserspaceArgument<u16>) -> Result<UDPDescriptor, SysSocketError> {
        let port = port.validate();
        let socket = match OPEN_UDP_SOCKETS.lock().try_get_socket(port) {
            None => return Err(SysSocketError::PortAlreadyUsed),
            Some(socket) => socket,
        };
        let current_process = get_current_process_expect();
        let mut current_process = current_process.borrow_mut();
        Ok(current_process.put_new_udp_socket(socket))
    }

    fn sys_write_back_udp_socket(
        descriptor: UserspaceArgument<UDPDescriptor>,
        buffer: UserspaceArgument<&u8>,
        length: UserspaceArgument<usize>,
    ) -> Result<usize, SysSocketError> {
        let length = length.validate();
        let pa = buffer.validate(length);

        let current_process = get_current_process_expect();
        let mut current_process = current_process.borrow_mut();

        let socket = unwrap_or_return!(
            current_process.get_shared_udp_socket(descriptor.validate()),
            Err(SysSocketError::InvalidDescriptor)
        )
        .lock();

        let recv_ip = unwrap_or_return!(socket.get_from(), Err(SysSocketError::NoReceiveIPYet));
        let recv_port = unwrap_or_return!(
            socket.get_received_port(),
            Err(SysSocketError::NoReceiveIPYet)
        );

        if let Ok(physical_address) = pa {
            let slice = unsafe { &*slice_from_raw_parts(physical_address, length) };
            // Get mac address of receiver
            // Since we already received a packet we should have it in the cache
            let destination_mac = *ARP_CACHE
                .lock()
                .get(&recv_ip)
                .expect("There must be a receiver mac already in the arp cache.");
            let constructed_packet = UdpHeader::create_udp_packet(
                recv_ip,
                recv_port,
                destination_mac,
                socket.get_port(),
                slice,
            );
            crate::net::send_packet(constructed_packet);
            Ok(length)
        } else {
            Err(SysSocketError::InvalidPtr)
        }
    }

    fn sys_read_udp_socket(
        descriptor: UserspaceArgument<UDPDescriptor>,
        buffer: UserspaceArgument<&mut u8>,
        length: UserspaceArgument<usize>,
    ) -> Result<usize, SysSocketError> {
        // Process packets
        crate::net::receive_and_process_packets();

        let length = length.validate();
        let pa = buffer.validate(length);
        let current_process = get_current_process_expect();
        let mut current_process = current_process.borrow_mut();

        let mut socket = unwrap_or_return!(
            current_process.get_shared_udp_socket(descriptor.validate()),
            Err(SysSocketError::InvalidDescriptor)
        )
        .lock();

        if let Ok(physical_address) = pa {
            let slice = unsafe { &mut *slice_from_raw_parts_mut(physical_address, length) };
            Ok(socket.get_data(slice))
        } else {
            Err(SysSocketError::InvalidPtr)
        }
    }
}

pub fn handle_syscall(nr: usize, arg1: usize, arg2: usize, arg3: usize) -> (usize, usize) {
    SyscallHandler::dispatch(nr, arg1, arg2, arg3)
}

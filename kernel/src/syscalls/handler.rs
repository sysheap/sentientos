use common::{
    errors::{SysExecuteError, SysSocketError},
    net::UDPDescriptor,
    pid::Tid,
    pointer::Pointer,
    syscalls::{SyscallStatus, kernel::KernelSyscalls, syscall_argument::SyscallArgument},
    unwrap_or_return,
};

use crate::{
    cpu::Cpu,
    debug,
    net::{self, arp, udp::UdpHeader},
    processes::{process::ProcessRef, thread::ThreadRef},
};

use super::validator::{UserspaceArgument, Validatable};

pub(super) struct SyscallHandler {
    current_process: ProcessRef,
    current_thread: ThreadRef,
    current_tid: Tid,
}

impl SyscallHandler {
    pub(super) fn new() -> Self {
        let current_thread = Cpu::with_scheduler(|s| s.get_current_thread().clone());
        let current_tid = current_thread.lock().get_tid();

        let current_process = current_thread.lock().process();
        Self {
            current_process,
            current_thread,
            current_tid,
        }
    }

    pub fn current_tid(&self) -> Tid {
        self.current_tid
    }

    pub fn current_process(&self) -> &ProcessRef {
        &self.current_process
    }

    pub fn current_thread(&self) -> &ThreadRef {
        &self.current_thread
    }

    pub fn sys_exit(&mut self, status: isize) {
        let exit_status = i32::try_from(status).expect("exit status fits in i32");

        Cpu::with_scheduler(|mut s| {
            s.kill_current_process(exit_status);
        });

        debug!("Exit process with status: {status}\n");
    }
}

impl KernelSyscalls for SyscallHandler {
    type ArgWrapper<T: SyscallArgument> = UserspaceArgument<T>;

    fn sys_execute<'a>(
        &mut self,
        name: UserspaceArgument<&str>,
        args: UserspaceArgument<&'a [&'a str]>,
    ) -> Result<Tid, SysExecuteError> {
        let name = name.validate(self)?;
        let args = args.validate(self)?;

        let tid = Cpu::with_scheduler(|mut s| s.start_program(name, &args))?;
        Ok(tid)
    }

    fn sys_open_udp_socket(
        &mut self,
        port: UserspaceArgument<u16>,
    ) -> Result<UDPDescriptor, SysSocketError> {
        use crate::processes::fd_table::FileDescriptor;
        let socket = match net::open_sockets()
            .lock()
            .try_get_socket(crate::net::sockets::Port::new(*port))
        {
            None => return Err(SysSocketError::PortAlreadyUsed),
            Some(socket) => socket,
        };
        let raw_fd = self
            .current_process
            .lock()
            .fd_table_mut()
            .allocate(FileDescriptor::UdpSocket(socket))
            .map_err(|_| SysSocketError::TooManyOpenFiles)?;
        Ok(UDPDescriptor::new(
            u64::try_from(raw_fd).expect("allocated fd is non-negative"),
        ))
    }

    fn sys_write_back_udp_socket(
        &mut self,
        descriptor: UserspaceArgument<UDPDescriptor>,
        buffer: UserspaceArgument<&[u8]>,
    ) -> Result<usize, SysSocketError> {
        let buffer = buffer.validate(self)?;

        descriptor.validate(self)?.with_lock(|socket| {
            let recv_ip = unwrap_or_return!(socket.get_from(), Err(SysSocketError::NoReceiveIPYet));
            let recv_port = unwrap_or_return!(
                socket.get_received_port(),
                Err(SysSocketError::NoReceiveIPYet)
            );

            // Get mac address of receiver
            // Since we already received a packet we should have it in the cache
            let destination_mac = arp::cache_lookup(&recv_ip)
                .expect("There must be a receiver mac already in the arp cache.");
            let constructed_packet = UdpHeader::create_udp_packet(
                recv_ip,
                recv_port.as_u16(),
                destination_mac,
                socket.get_port().as_u16(),
                buffer,
            );
            crate::net::send_packet(constructed_packet);
            Ok(buffer.len())
        })
    }

    fn sys_read_udp_socket(
        &mut self,
        descriptor: UserspaceArgument<UDPDescriptor>,
        buffer: UserspaceArgument<&mut [u8]>,
    ) -> Result<usize, SysSocketError> {
        // Process packets
        crate::net::receive_and_process_packets();

        let buffer = buffer.validate(self)?;

        descriptor
            .validate(self)?
            .with_lock(|mut socket| Ok(socket.get_data(buffer)))
    }

    #[doc = r" Validate a pointer such that it is a valid userspace pointer"]
    fn validate_and_translate_pointer<PTR: Pointer>(&self, ptr: PTR) -> Option<PTR> {
        self.current_process.with_lock(|p| {
            let pt = p.get_page_table();
            if !pt.is_valid_userspace_ptr(ptr, true) {
                return None;
            }
            let physical_address = unwrap_or_return!(
                pt.translate_userspace_address_to_physical_address(ptr),
                None
            );
            Some(physical_address)
        })
    }
}

pub fn handle_syscall(nr: usize, arg: usize, ret: usize) -> SyscallStatus {
    let mut handler = SyscallHandler::new();
    handler.dispatch(nr, arg, ret)
}

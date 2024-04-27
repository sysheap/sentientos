use core::net::Ipv4Addr;

use alloc::{
    collections::{btree_map::Entry, BTreeMap},
    sync::{Arc, Weak},
    vec::Vec,
};
use common::mutex::Mutex;

use crate::debug;

pub type SharedAssignedSocket = Arc<Mutex<AssignedSocket>>;
type WeakSharedAssignedSocket = Weak<Mutex<AssignedSocket>>;

type MutexSocketMap = Mutex<BTreeMap<u16, WeakSharedAssignedSocket>>;
type SharedSocketMap = Arc<MutexSocketMap>;
type WeakSharedSocketMap = Weak<MutexSocketMap>;

pub struct OpenSockets {
    sockets: SharedSocketMap,
}

impl OpenSockets {
    pub fn new() -> Self {
        Self {
            sockets: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    pub fn try_get_socket(&self, port: u16) -> Option<SharedAssignedSocket> {
        let mut sockets = self.sockets.lock();
        if sockets.contains_key(&port) {
            return None;
        }

        let weak_socket_map = Arc::downgrade(&self.sockets);
        let assigned_socket = AssignedSocket::new(port, weak_socket_map);

        let arc_socket = Arc::new(Mutex::new(assigned_socket));

        assert!(
            sockets.insert(port, Arc::downgrade(&arc_socket)).is_none(),
            "There must be no value before in the socket map."
        );

        Some(arc_socket)
    }

    pub fn put_data(&self, from: Ipv4Addr, from_port: u16, port: u16, data: &[u8]) {
        let mut sockets = self.sockets.lock();
        match sockets.entry(port) {
            Entry::Vacant(_) => debug!("Recived packet on {} but there is no listener.", port),
            Entry::Occupied(mut entry) => entry
                .get_mut()
                .upgrade()
                .expect("There must an assigned socket.")
                .lock()
                .put_data(from, from_port, data),
        }
    }
}

pub struct AssignedSocket {
    buffer: Vec<u8>,
    port: u16,
    received_from: Option<Ipv4Addr>,
    received_port: Option<u16>,
    open_sockets: WeakSharedSocketMap,
}

impl AssignedSocket {
    fn new(port: u16, open_sockets: WeakSharedSocketMap) -> Self {
        Self {
            buffer: Vec::new(),
            port,
            received_from: None,
            received_port: None,
            open_sockets,
        }
    }

    pub fn get_port(&self) -> u16 {
        self.port
    }

    fn put_data(&mut self, from: Ipv4Addr, from_port: u16, data: &[u8]) {
        self.received_from = Some(from);
        self.received_port = Some(from_port);
        self.buffer.extend_from_slice(data)
    }

    pub fn get_data(&mut self, out_buffer: &mut [u8]) -> usize {
        let len = usize::min(self.buffer.len(), out_buffer.len());
        let mut count = 0;

        for (idx, item) in self.buffer.drain(0..len).enumerate() {
            out_buffer[idx] = item;
            count += 1;
        }
        count
    }

    pub fn get_from(&self) -> Option<Ipv4Addr> {
        self.received_from
    }

    pub fn get_received_port(&self) -> Option<u16> {
        self.received_port
    }
}

impl Drop for AssignedSocket {
    fn drop(&mut self) {
        let sockets = self
            .open_sockets
            .upgrade()
            .expect("The original map must exist.");
        let mut sockets = sockets.lock();
        assert!(
            sockets.remove(&self.port).is_some(),
            "There must be a value to remove in the map."
        );
    }
}

#[cfg(test)]
mod tests {
    use core::net::Ipv4Addr;

    use super::OpenSockets;

    const PORT1: u16 = 1234;
    const FROM1: Ipv4Addr = Ipv4Addr::new(192, 168, 1, 1);

    const PORT2: u16 = 4444;
    const FROM2: Ipv4Addr = Ipv4Addr::new(192, 168, 1, 2);

    #[test_case]
    fn duplicate_ports() {
        let open_sockets = OpenSockets::new();

        let _assigned_socket = open_sockets
            .try_get_socket(PORT1)
            .expect("There must be a free port.");

        assert!(
            open_sockets.try_get_socket(PORT1).is_none(),
            "Ports must not handed out twice."
        );
    }

    #[test_case]
    fn data_delivery() {
        let open_sockets = OpenSockets::new();

        let assigned_port1 = open_sockets
            .try_get_socket(PORT1)
            .expect("Port must be free");

        let assigned_port2 = open_sockets
            .try_get_socket(PORT2)
            .expect("Port must be free");

        assert!(
            assigned_port1.lock().buffer.is_empty(),
            "Buffer must be empty intially"
        );
        assert!(
            assigned_port2.lock().buffer.is_empty(),
            "Buffer must be empty intially"
        );

        let port1_data = [1, 2, 3];
        let port2_data = [3, 2, 1];

        open_sockets.put_data(FROM1, PORT1, PORT1, &port1_data);

        assert!(
            &assigned_port1.lock().buffer == &port1_data,
            "Data must be delivered properly."
        );
        assert!(
            assigned_port2.lock().buffer.is_empty(),
            "Buffer must be still empty."
        );

        open_sockets.put_data(FROM2, PORT2, PORT2, &port2_data);

        let mut buf1 = [0; 10];
        let mut buf2 = [0; 10];

        assert_eq!(
            assigned_port1.lock().get_data(&mut buf1),
            3,
            "Data must be copied completely."
        );
        assert_eq!(
            assigned_port2.lock().get_data(&mut buf2),
            3,
            "Data must be copied completely."
        );

        assert_eq!(buf1[0..3], port1_data, "Data must be the same.");
        assert_eq!(buf2[0..3], port2_data, "Data must be the same.");

        assert!(
            assigned_port1.lock().buffer.is_empty(),
            "Buffer must be empty again"
        );
        assert!(
            assigned_port2.lock().buffer.is_empty(),
            "Buffer must be empty again"
        );
    }

    #[test_case]
    fn correct_number_of_data() {
        let open_sockets = OpenSockets::new();

        let socket = open_sockets
            .try_get_socket(PORT1)
            .expect("Socket must be free");

        socket
            .lock()
            .put_data(Ipv4Addr::UNSPECIFIED, PORT1, &[1, 2, 3, 4, 5]);

        let mut small_buffer = [0; 1];
        assert_eq!(
            socket.lock().get_data(&mut small_buffer),
            1,
            "Only one byte must be transfered"
        );

        assert_eq!(small_buffer[0], 1, "Correct byte must be transfered.");

        let mut big_buffer = [42; 32];

        assert_eq!(
            socket.lock().get_data(&mut big_buffer),
            4,
            "4 bytes must be transferred."
        );

        let mut correct_buffer = [42; 32];
        correct_buffer[0] = 2;
        correct_buffer[1] = 3;
        correct_buffer[2] = 4;
        correct_buffer[3] = 5;

        assert_eq!(
            big_buffer, correct_buffer,
            "Correct data must be transfered and rest ist unchanged."
        );
    }

    #[test_case]
    fn received_ip() {
        let open_sockets = OpenSockets::new();

        let assigned_socket = open_sockets
            .try_get_socket(PORT1)
            .expect("There must be a free socket.");

        assert!(
            assigned_socket.lock().get_from().is_none(),
            "From must be initially empty."
        );

        open_sockets.put_data(FROM1, PORT1, PORT1, &[1, 2, 3]);

        assert_eq!(
            assigned_socket.lock().get_from(),
            Some(FROM1),
            "There must be the last received ip address."
        );

        open_sockets.put_data(FROM2, PORT1, PORT1, &[1, 2, 3]);

        assert_eq!(
            assigned_socket.lock().get_from(),
            Some(FROM2),
            "There must be the last received ip address."
        );
    }

    #[test_case]
    fn drop_must_work_correctly() {
        let open_sockets = OpenSockets::new();

        let assigned_socket = open_sockets
            .try_get_socket(PORT1)
            .expect("There must be a free port.");

        assert!(
            open_sockets.sockets.lock().contains_key(&PORT1),
            "Open sockets must contain the port."
        );

        drop(assigned_socket);

        assert!(
            !open_sockets.sockets.lock().contains_key(&PORT1),
            "Open sockets must not contain port."
        );

        assert!(
            open_sockets.try_get_socket(PORT1).is_some(),
            "Port must be reusable after drop."
        );
    }
}

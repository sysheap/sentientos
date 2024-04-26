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

    pub fn put_data(&self, port: u16, data: &[u8]) {
        let mut sockets = self.sockets.lock();
        match sockets.entry(port) {
            Entry::Vacant(_) => debug!("Recived packet on {} but there is no listener.", port),
            Entry::Occupied(mut entry) => entry
                .get_mut()
                .upgrade()
                .expect("There must an assigned socket.")
                .lock()
                .put_data(data),
        }
    }
}

pub struct AssignedSocket {
    buffer: Vec<u8>,
    port: u16,
    open_sockets: WeakSharedSocketMap,
}

impl AssignedSocket {
    fn new(port: u16, open_sockets: WeakSharedSocketMap) -> Self {
        Self {
            buffer: Vec::new(),
            port,
            open_sockets,
        }
    }

    fn put_data(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data)
    }

    fn get_data(&mut self) -> Vec<u8> {
        core::mem::take(&mut self.buffer)
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
    use super::OpenSockets;

    const PORT1: u16 = 1234;
    const PORT2: u16 = 4444;

    #[test_case]
    fn duplicate_ports() {
        let mut open_sockets = OpenSockets::new();

        let assigned_socket = open_sockets
            .try_get_socket(PORT1)
            .expect("There must be a free port.");

        assert!(
            open_sockets.try_get_socket(PORT1).is_none(),
            "Ports must not handed out twice."
        );
    }

    #[test_case]
    fn data_delivery() {
        let mut open_sockets = OpenSockets::new();

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

        open_sockets.put_data(PORT1, &port1_data);

        assert!(
            &assigned_port1.lock().buffer == &port1_data,
            "Data must be delivered properly."
        );
        assert!(
            assigned_port2.lock().buffer.is_empty(),
            "Buffer must be still empty."
        );

        open_sockets.put_data(PORT2, &port2_data);

        assert!(
            &assigned_port1.lock().get_data() == &port1_data,
            "Data must be unchanged."
        );
        assert!(
            &assigned_port2.lock().get_data() == &port2_data,
            "Data must be delivered properly."
        );

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
    fn drop_must_work_correctly() {
        let mut open_sockets = OpenSockets::new();

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

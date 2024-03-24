use alloc::vec::Vec;
use common::mutex::Mutex;

use crate::drivers::virtio::net::NetworkDevice;

static NETWORK_DEVICE: Mutex<Option<NetworkDevice>> = Mutex::new(None);

pub fn assig_network_device(device: NetworkDevice) {
    *NETWORK_DEVICE.lock() = Some(device);
}

pub fn receive_packets() -> Vec<Vec<u8>> {
    NETWORK_DEVICE
        .lock()
        .as_mut()
        .expect("There must be a configured network device.")
        .receive_packets()
}

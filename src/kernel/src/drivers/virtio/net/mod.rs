pub struct NetworkDevice {
    address: usize,
}

impl NetworkDevice {
    pub fn initialize(address: usize) -> Result<Self, &'static str> {
        Ok(Self { address })
    }
}

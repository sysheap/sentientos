use crate::klibc::runtime_initialized::RuntimeInitializedData;
use alloc::vec::Vec;

use super::page_tables::XWRMode;

static RUNTIME_MAPPINGS: RuntimeInitializedData<Vec<RuntimeMapping>> =
    RuntimeInitializedData::new();

#[derive(Clone)]
pub struct RuntimeMapping {
    pub virtual_address_start: usize,
    pub size: usize,
    pub privileges: XWRMode,
    pub name: &'static str,
}

pub fn initialize_runtime_mappings(mappings: &[RuntimeMapping]) {
    RUNTIME_MAPPINGS.initialize(mappings.to_vec());
}

pub fn get_runtime_mappings() -> &'static [RuntimeMapping] {
    &RUNTIME_MAPPINGS
}

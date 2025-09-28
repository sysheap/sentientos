use core::borrow::Borrow;

use alloc::collections::BTreeMap;

pub trait SplitOffLowerThan<K, V> {
    fn split_off_lower_than<Q: ?Sized + Ord>(&mut self, key: &Q) -> BTreeMap<K, V>
    where
        K: Borrow<Q> + Ord;
}

impl<K, V> SplitOffLowerThan<K, V> for BTreeMap<K, V> {
    fn split_off_lower_than<Q: ?Sized + Ord>(&mut self, key: &Q) -> BTreeMap<K, V>
    where
        K: Borrow<Q> + Ord,
    {
        let upper = self.split_off(key);
        core::mem::replace(self, upper)
    }
}

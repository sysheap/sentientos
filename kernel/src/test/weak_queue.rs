#[cfg(test)]
mod tests {
    use alloc::sync::Arc;
    use common::weak_queue::WeakQueue;

    fn new() -> WeakQueue<u8> {
        WeakQueue::new()
    }

    #[test_case]
    fn empty() {
        let mut queue = new();
        assert!(queue.iter().next().is_none());
        assert!(queue.readonly_iter().next().is_none());
        assert!(queue.is_empty());
    }

    #[test_case]
    fn single_strong() {
        let mut queue = new();
        let strong = Arc::new(42);

        queue.add(Arc::downgrade(&strong));

        assert!(!queue.is_empty());
        assert_eq!(queue.iter().next().as_ref(), Some(&strong));
        assert_eq!(queue.readonly_iter().next().as_ref(), Some(&strong));

        assert!(!queue.is_empty());
        assert_eq!(queue.iter().next().as_ref(), Some(&strong));
        assert_eq!(queue.readonly_iter().next().as_ref(), Some(&strong));
    }

    #[test_case]
    fn single_weak() {
        let mut queue = new();
        let strong = Arc::new(42);

        queue.add(Arc::downgrade(&strong));

        assert!(!queue.is_empty());

        drop(strong);

        assert!(queue.iter().next().is_none());
        assert!(queue.is_empty());
        assert!(queue.readonly_iter().next().is_none());
        assert!(queue.is_empty());
    }

    #[test_case]
    fn mixed() {
        let mut queue = new();
        let strong1 = Arc::new(42);
        let strong2 = Arc::new(42);
        let strong3 = Arc::new(42);

        queue.add(Arc::downgrade(&strong1));
        queue.add(Arc::downgrade(&strong2));
        queue.add(Arc::downgrade(&strong3));

        drop(strong1);

        assert_eq!(queue.readonly_iter().next().as_ref(), Some(&strong2));
        assert_eq!(queue.readonly_iter().next().as_ref(), Some(&strong2));

        let mut iter = queue.iter();

        assert_eq!(iter.next().as_ref(), Some(&strong2));
        assert_eq!(iter.next().as_ref(), Some(&strong3));

        let mut iter = queue.iter();
        assert_eq!(iter.next().as_ref(), Some(&strong2));
        assert_eq!(iter.next().as_ref(), Some(&strong3));

        assert_eq!(queue.len(), 2);
    }
}

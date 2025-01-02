pub trait Constructable<T> {
    fn new(value: T) -> Self;
}

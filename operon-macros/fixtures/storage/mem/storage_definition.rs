#[derive(Default)]
pub struct MemCookingStorage {
    pub a: operon::__private::dashmap::DashMap<[usize; 1usize], A>,
    pub b: operon::__private::dashmap::DashMap<[usize; 2usize], B>,
    pub c: operon::__private::dashmap::DashMap<[usize; 2usize], C>,
    pub d: operon::__private::dashmap::DashMap<[usize; 3usize], D>,
    pub e: operon::__private::dashmap::DashMap<[usize; 2usize], E>,
    pub f: operon::__private::dashmap::DashMap<[usize; 1usize], F>,
}

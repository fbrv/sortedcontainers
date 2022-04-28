pub enum OrderType {
    Asc,
    Desc,
}
pub struct Svector<T> {
    data: Vec<Vec<T>>,
    maxes: Vec<T>,
    index: Vec<usize>,
    order_type: OrderType,
    len: usize,
}
impl<T: Ord + Clone> Default for Svector<T> {
    fn default() -> Self {
        Svector {
            data: vec![Vec::new()],
            maxes: Vec::new(),
            index: Vec::new(),
            order_type: OrderType::Asc,
            len: 0,
        }
    }
}
impl<T: Ord + Clone> Svector<T> {
    pub fn new(order_type: OrderType) -> Svector<T> {
        Svector{
            data: vec![Vec::new()],
            maxes: Vec::new(),
            index: Vec::new(),
            order_type,
            len: 0,
        }
    }
    pub const fn len(&self) -> usize {
        self.len
    }
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }
}
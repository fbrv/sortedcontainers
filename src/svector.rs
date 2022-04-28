use std::cmp::Ordering;
use std::ptr;

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
    expand_strategy: fn(usize, usize) -> bool,
    shrink_strategy: fn(usize, usize) -> bool,
}
impl<T: Ord + Clone> Default for Svector<T> {
    fn default() -> Self {
        Svector {
            data: vec![Vec::new()],
            maxes: Vec::new(),
            index: Vec::new(),
            order_type: OrderType::Asc,
            len: 0,
            expand_strategy: |len, pos| {
                len > 2_000
            },
            shrink_strategy:  |len, pos| {
                len < 500
            },
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
            expand_strategy: |len, pos| {
                len > 2_000
            },
            shrink_strategy:  |len, pos| {
                len < 500
            },
        }
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn insert(&mut self, value: T) {
        if self.maxes.is_empty() {
            self.data[0].push(value.clone());
            self.maxes.push(value);
            self.len += 1;
            return;
        }
        let mut pos: usize = 0;
        if self.maxes.len() > 1 {
            match self.bisect(&self.maxes, &value) {
                Ok(idx) => pos = idx,
                Err(idx) => pos = idx,
            }
        }
        if self.data.len() == pos {
            pos -= 1;
        }
        match self.bisect(&self.data[pos], &value) {
            Ok(idx) => self.data[pos][idx] = value,
            Err(idx) => {
                if idx == self.data[pos].len() {
                    self.maxes[pos] = value.clone();
                }
                self.data[pos].insert(idx, value);
                self.len += 1;
                self.update_index(pos, 1);
                if (self.expand_strategy)(self.data[pos].len(), pos) {
                    self.expand(pos);
                }
            }
        }
    }
    pub fn remove(&mut self, value: &T) -> Result<T, String> {
        let mut pos: usize = 0;
        if self.maxes.len() > 1 {
            match self.bisect(&self.maxes, value) {
                Ok(idx) => pos = idx,
                Err(idx) => pos = idx,
            }
        }
        if self.data.len() == pos {
            pos -= 1;
        }
        match self.bisect(&self.data[pos], value) {
            Ok(data_pos) => {
                let removed_val = self.data[pos].remove(data_pos);
                self.update_index(pos, -1);
                self.len -= 1;
                if self.len() == 0 {
                    self.maxes.clear();
                }
                if self.maxes.len() > 1 && (self.shrink_strategy)(self.data[pos].len(), pos) {
                    self.shrink(pos);
                }
                Ok(removed_val)
            }
            Err(_) => {
                Err(String::from("element not found!"))
            }
        }
    }
    fn positional_search(&self, position: &usize) -> Result<usize, usize> {
        let mut low: usize = 0;
        let mut high: usize = self.index.len();
        while low < high {
            let middle = (high + low) >> 1;
            match self.index[middle].cmp(position) {
                Ordering::Less => low = middle + 1,
                Ordering::Equal => return Ok(middle + 1),
                Ordering::Greater => high = middle
            }
        }
        Err(low)
    }
    fn tuple_from_index(&self, index: usize) -> (usize, usize) {
        let mut data_pos = 0;
        if index < self.data[0].len() {
            return (data_pos, index);
        }
        match self.positional_search(&index) {
            Ok(idx) => data_pos = idx - 1,
            Err(idx) => data_pos = idx - 1,
        }
        (data_pos, index - self.index[data_pos])
    }
    fn index_from_tuple(&self, pos: (usize, usize)) -> usize {
        self.index[pos.0] + pos.1
    }
    fn bisect(&self, values: &[T], value: &T) -> Result<usize, usize> {
        let mut low: usize = 0;
        let mut high: usize = values.len();
        while low < high {
            let middle = (high + low) >> 1;
            match values[middle].cmp(value) {
                Ordering::Less => {
                    match self.order_type {
                        OrderType::Asc => low = middle + 1,
                        OrderType::Desc => high = middle,
                    }
                },
                Ordering::Equal => return Ok(middle),
                Ordering::Greater => {
                    match self.order_type {
                        OrderType::Asc => high = middle,
                        OrderType::Desc => low = middle + 1
                    }
                }
            }
        }
        match self.order_type {
            OrderType::Asc => Err(low),
            OrderType::Desc => Err(high),
        }
    }
    ///if a subvector reach the size (capacity*2) + 1 the element is split
    /// into two vector of size (capacity*2) + 1 / 2 and are inserted in order
    /// into data vector of vector. maxes is updated according
    fn expand(&mut self, pos: usize) {
        let c_len = self.data[pos].len();
        let split_at = c_len / 2;
        let new_len = self.data[pos].len() - split_at;
        let mut new_vec = Vec::with_capacity(c_len);
        unsafe  {
            self.data[pos].set_len(split_at);
            new_vec.set_len(new_len);
            ptr::copy_nonoverlapping(self.data[pos].as_ptr().add(split_at), new_vec.as_mut_ptr(), new_vec.len());
        }
        self.maxes.insert(pos + 1, new_vec[new_vec.len() - 1].clone());
        self.data.insert(pos + 1, new_vec);
        self.maxes[pos] = self.data[pos][self.data[pos].len() - 1].clone();
        self.build_index();
    }
    fn shrink(&mut self, pos: usize) {
        let vec_to_expand: usize;
        if pos == 0 {
            vec_to_expand = 1;
        } else if pos == self.data.len() - 1 {
            vec_to_expand = self.data.len() - 2;
        } else if self.data[pos - 1].len() < self.data[pos + 1].len() {
            vec_to_expand = pos - 1;
        } else {
            vec_to_expand = pos + 1;
        }
        if vec_to_expand > pos {
            let mut values = self.data.remove(vec_to_expand);
            self.data[pos].append(&mut values);
            self.maxes[pos] = self.maxes.remove(vec_to_expand);
        } else {
            let mut values = self.data.remove(pos);
            self.data[vec_to_expand].append(&mut values);
            self.maxes[vec_to_expand] = self.maxes.remove(pos);
        }
        self.build_index();
    }
    fn build_index(&mut self) {
        if self.is_empty() || self.maxes.len() < 2 {
            return;
        }
        self.index.clear();
        self.index.push(0);
        for i in 0..self.data.len() {
            self.index.push(self.index[self.index.len() - 1] + self.data[i].len());
        }
    }
    fn update_index(&mut self, pos: usize, values_len: i32) {
        if self.maxes.len() > 1 {
            for i in (pos+1)..self.index.len() {
                self.index[i] = (self.index[i] as i32 + values_len) as usize;
            }
        }
    }
}
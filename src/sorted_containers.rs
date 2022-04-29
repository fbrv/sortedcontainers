use crate::errors::SortedContainersError;
use std::cmp::Ordering;
use std::ops::Index;
use std::ptr;

pub enum OrderType {
    Asc,
    Desc,
}
pub struct SortedContainers<T> {
    data: Vec<Vec<T>>,
    maxes: Vec<T>,
    index: Vec<usize>,
    order_type: OrderType,
    len: usize,
    expand_strategy: fn(usize, usize) -> bool,
    shrink_strategy: fn(usize, usize) -> bool,
}
impl<T: Ord + Clone> Default for SortedContainers<T> {
    fn default() -> Self {
        SortedContainers {
            data: vec![Vec::new()],
            maxes: Vec::new(),
            index: Vec::new(),
            order_type: OrderType::Asc,
            len: 0,
            expand_strategy: |len, _pos| len > 2_000,
            shrink_strategy: |len, _pos| len < 500,
        }
    }
}
impl<T: Ord + Clone> SortedContainers<T> {
    pub fn new(order_type: OrderType) -> SortedContainers<T> {
        SortedContainers {
            data: vec![Vec::new()],
            maxes: Vec::new(),
            index: Vec::new(),
            order_type,
            len: 0,
            expand_strategy: |len, _pos| len > 2_000,
            shrink_strategy: |len, _pos| len < 500,
        }
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn clear(&mut self) {
        self.data.clear();
        self.maxes.clear();
        self.index.clear();
        self.len = 0;
    }
    pub fn find(&self, value: &T) -> Result<usize, SortedContainersError<T>> {
        match self.search_element(value) {
            Ok(pos) => Ok(self.index_from_tuple(pos)),
            Err(_) => Err(SortedContainersError::ElementNotFound(value.clone())),
        }
    }
    pub fn insert(&mut self, value: T) -> Result<usize, SortedContainersError<T>> {
        if self.maxes.is_empty() {
            if self.data.len() == 0 {
                self.data.push(Vec::new());
            }
            self.data[0].push(value.clone());
            self.maxes.push(value);
            self.len += 1;
            return Ok(0);
        }
        match self.search_element(&value) {
            Ok(_) => Err(SortedContainersError::ElementAlreadyExist(value)),
            Err((pos, idx)) => {
                self.data[pos].insert(idx, value);
                self.len += 1;
                self.update_index(pos, 1);
                let final_pos = self.index_from_tuple((pos, idx));
                if (self.expand_strategy)(self.data[pos].len(), pos) {
                    self.expand(pos);
                }
                Ok(final_pos)
            }
        }
    }
    pub fn remove(&mut self, value: &T) -> Result<T, String> {
        match self.search_element(value) {
            Ok((pos, idx)) => {
                let removed_val = self.data[pos].remove(idx);
                self.update_index(pos, -1);
                self.len -= 1;
                if self.len() == 0 {
                    self.maxes.clear();
                    self.data.clear();
                    self.index.clear();
                    return Ok(removed_val);
                }
                if self.maxes.len() > 1 && (self.shrink_strategy)(self.data[pos].len(), pos) {
                    self.shrink(pos);
                }
                Ok(removed_val)
            }
            Err(_) => Err(String::from("element not found!")),
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
                Ordering::Greater => high = middle,
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
        if self.data.len() > 1 {
            return self.index[pos.0] + pos.1;
        }
        pos.1
    }
    fn search_element(&self, value: &T) -> Result<(usize, usize), (usize, usize)> {
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
            Ok(idx) => Ok((pos, idx)),
            Err(idx) => Err((pos, idx)),
        }
    }
    fn bisect(&self, values: &[T], value: &T) -> Result<usize, usize> {
        let mut low: usize = 0;
        let mut high: usize = values.len();
        while low < high {
            let middle = (high + low) >> 1;
            match values[middle].cmp(value) {
                Ordering::Less => match self.order_type {
                    OrderType::Asc => low = middle + 1,
                    OrderType::Desc => high = middle,
                },
                Ordering::Equal => return Ok(middle),
                Ordering::Greater => match self.order_type {
                    OrderType::Asc => high = middle,
                    OrderType::Desc => low = middle + 1,
                },
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
        unsafe {
            self.data[pos].set_len(split_at);
            new_vec.set_len(new_len);
            ptr::copy_nonoverlapping(
                self.data[pos].as_ptr().add(split_at),
                new_vec.as_mut_ptr(),
                new_vec.len(),
            );
        }
        self.maxes
            .insert(pos + 1, new_vec[new_vec.len() - 1].clone());
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
            self.index.clear();
            return;
        }
        self.index.clear();
        self.index.push(0);
        for i in 0..self.data.len() {
            self.index
                .push(self.index[self.index.len() - 1] + self.data[i].len());
        }
    }
    fn update_index(&mut self, pos: usize, values_len: i32) {
        if self.maxes.len() > 1 {
            for i in (pos + 1)..self.index.len() {
                self.index[i] = (self.index[i] as i32 + values_len) as usize;
            }
        }
    }
}
impl<T: Ord + Clone> Index<usize> for SortedContainers<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        assert!(index < self.len(), "index out of bound");
        let pos = self.tuple_from_index(index);
        &self.data[pos.0][pos.1]
    }
}
#[cfg(test)]
mod test {
    use rand::prelude::SliceRandom;
    use rand::{random, thread_rng};
    use crate::sorted_containers::{OrderType, SortedContainers};

    #[test]
    fn asc_ordered_insertion() {
        let mut vec: SortedContainers<i32> = SortedContainers::default();
        let mut rng = thread_rng();
        for i in -5_000..5_000 {
            match vec.insert(i) {
                Ok(_) => assert!(true),
                Err(_) => assert!(false),
            }
        }
        for i in 0..10_000 {
            let v = vec[i];
            let expected_value = i as i32 - 5000;
            assert!(expected_value == v);
        }
        vec.clear();
        let mut random_vec: Vec<i32> = (-5_000..5_000).collect();
        random_vec.shuffle(&mut rng);
        for el in random_vec {
            match vec.insert(el) {
                Ok(_) => assert!(true),
                Err(_) => assert!(false),
            }
        }
        let mut prev_element = vec[0];
        for el in 1..10_0000 {
            assert!(prev_element < el);
            prev_element = el;
        }
    }
    #[test]
    fn desc_ordered_insertion() {
        let mut rng = thread_rng();
        let mut vec: SortedContainers<i32> = SortedContainers::new(OrderType::Desc);
        for i in -5_000..5_000 {
            match vec.insert(i) {
                Ok(_) => assert!(true),
                Err(_) => assert!(false),
            }
        }
        let mut expected_value = 4_999;
        for i in 0..10_000 {
            let v = vec[i];
            assert!(expected_value == v);
            expected_value -= 1;
        }
        vec.clear();
        let mut random_vec: Vec<i32> = (-5_000..5_000).collect();
        random_vec.shuffle(&mut rng);
        for el in random_vec {
            match vec.insert(el) {
                Ok(_) => assert!(true),
                Err(_) => assert!(false),
            }
        }
        let mut prev_element = vec[0];
        for i in 1..10_000 {
            assert!(prev_element > vec[i]);
            prev_element = vec[i];
        }
    }
    #[test]
    fn remove_element() {
        let mut vec: SortedContainers<i32> = SortedContainers::default();
        for i in -5_000..5_000 {
            match vec.insert(i) {
                Ok(_) => assert!(true),
                Err(_) => assert!(false),
            }
        }
        for i in 0..10_000 {
            let to_remove = i - 5000;
            match vec.remove(&to_remove) {
                Ok(removed_value) => assert!(to_remove == removed_value),
                Err(_) => assert!(false),
            }
        }
    }
    #[test]
    fn find_element() {
        let mut vec: SortedContainers<i32> = SortedContainers::default();
        for i in -5_000..5_000 {
            match vec.insert(i) {
                Ok(_) => assert!(true),
                Err(_) => assert!(false),
            }
        }
        let mut expected_element = -5000;
        for i in 0..10_000 {
            match vec.find(&expected_element) {
                Ok(pos) => assert!(i == pos),
                Err(_) => assert!(false),
            }
            expected_element += 1;
        }
    }
    #[test]
    fn remove_elements() {
        let mut rng = thread_rng();
        let mut vec: SortedContainers<i32> = SortedContainers::default();
        let mut random_vec: Vec<i32> = (-5_000..5_000).collect();
        random_vec.shuffle(&mut rng);
        for el in random_vec {
            match vec.insert(el) {
                Ok(_) => assert!(true),
                Err(_) => assert!(false),
            }
        }
        for i in -5_000..5_000 {
            match vec.remove(&i) {
                Ok(removed_el) => assert!(removed_el == i),
                Err(_) => assert!(false),
            }
            if i % 1_000 == 0 {
                let mut prev_el = vec[0];
                for idx in 1..vec.len() {
                    assert!(prev_el < vec[idx]);
                    prev_el = vec[idx];
                }
            }
        }
        assert!(vec.len() == 0);
        assert!(vec.data.len() == 0);
        assert!(vec.maxes.len() == 0);
    }
}

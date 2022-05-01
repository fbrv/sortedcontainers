use crate::errors::SortedContainersError;
use std::cmp::Ordering;
use std::ops::Index;
use std::ptr;

pub enum OrderType {
    Asc,
    Desc,
}
#[derive(PartialEq)]
enum ProcessType {
    Insert,
    Update,
    InsertOrUpdate,
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
            expand_strategy: |len, _pos| len > 2000,
            shrink_strategy: |len, _pos| len < 500,
        }
    }
}
impl<T: Ord + Clone> SortedContainers<T> {
    /// Constructs a new empty `SortedContainers<T>` with the specified order type
    ///
    /// The collection will store in ascending or descending order the elements later inserted.
    ///
    /// # Examples
    /// let mut sorted_containers = SortedContainers::new(OrderType::Asc);
    /// // the sorted collection will store in ascending order the input elements
    /// let mut sorted_containers = SortedContainers::new(OrderType::Desc);
    /// // the sorted collection will store in descending order the input elements
    pub fn new(order_type: OrderType) -> SortedContainers<T> {
        SortedContainers {
            data: vec![Vec::new()],
            maxes: Vec::new(),
            index: Vec::new(),
            order_type,
            len: 0,
            expand_strategy: |len, _pos| len > 2000,
            shrink_strategy: |len, _pos| len < 500,
        }
    }
    /// Returns the number of elements in the sortedcontainers, also referred as its 'length'.
    pub fn len(&self) -> usize {
        self.len
    }
    /// Returns `true` if the vectors contains no elements
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    /// Remove all the elements inside the sortedcontainers.
    pub fn clear(&mut self) {
        self.data.clear();
        self.maxes.clear();
        self.index.clear();
        self.len = 0;
    }
    /// Search an element inside the collection.
    /// Complexity is O(log(M)) + O(log(N))
    /// If the element exists in the collection the actual position is returned otherwise
    /// an error is returned
    pub fn find(&self, element: &T) -> Result<usize, SortedContainersError<T>> {
        match self.search_element(element) {
            Ok(pos) => Ok(self.index_from_tuple(pos)),
            Err(_) => Err(SortedContainersError::ElementNotFound(element.clone())),
        }
    }
    /// Insert an element inside the collection.
    ///
    /// If the element is not currently inside the collection, the element is inserted
    /// and the actual position is returned.
    /// Complexity is O(log(M)) + O(N)
    /// If the element already exists, an error is returned.
    pub fn insert(&mut self, value: T) -> Result<usize, SortedContainersError<T>> {
        self.process_element(value, ProcessType::Insert)
    }
    /// Update an element inside the collection.
    /// Complexity is O(log(M)) + O(log(N))
    /// If the element exists in the collection, the element will be updated returning the actual
    /// position, otherwise an error is returned.
    pub fn update(&mut self, value: T) -> Result<usize, SortedContainersError<T>> {
        self.process_element(value, ProcessType::Update)
    }
    /// Insert or update an element inside the collection.
    /// Given an element is input, a search if performed. If the element already exists, the element
    /// is updated, otherwise is inserted. The actual element position is then returned.
    pub fn insert_or_update(&mut self, value: T) -> Result<usize, SortedContainersError<T>> {
        self.process_element(value, ProcessType::InsertOrUpdate)
    }
    /// Remove an element that is stored inside the collection.
    /// Time complexity O(log(M)) + O(log(N)) + O(N)
    /// Given an element in input, a search is perfoemd. If the element exists inside the collection,
    /// the element is removed and returned. Otherwise an error is returned.
    pub fn remove(&mut self, value: &T) -> Result<T, String> {
        match self.search_element(value) {
            Ok((pos, idx)) => {
                let removed_val = self.data[pos].remove(idx);
                self.update_index(pos, -1);
                self.len -= 1;
                if self.is_empty() {
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
    /// given an position in input, the element at `self.data[position]` is splitted in half and the
    /// second part is inserted at `position + 1` inside the `self.data`
    #[inline]
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
        match self.order_type {
            OrderType::Asc => {
                self.maxes
                    .insert(pos + 1, new_vec[new_vec.len() - 1].clone());
                self.maxes[pos] = self.data[pos][self.data[pos].len() - 1].clone();
            }
            OrderType::Desc => {
                self.maxes.insert(pos + 1, new_vec[0].clone());
                self.maxes[pos] = self.data[pos][0].clone();
            }
        }
        // add the second half part of the vector at position + 1
        self.data.insert(pos + 1, new_vec);
        self.build_index();
    }
    #[inline]
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
    /// search an element inside the collection and return the actual position
    /// or the expected position.
    /// Time complexity O(log(M)) + O(log(N))
    #[inline]
    fn search_element(&self, value: &T) -> Result<(usize, usize), (usize, usize)> {
        let mut pos: usize = 0;
        if self.maxes.len() > 1 {
            pos = self.bisect(&self.maxes, value, true).unwrap();
        }
        if self.data.len() == pos {
            pos -= 1;
        }
        match self.bisect(&self.data[pos], value, false) {
            Ok(idx) => Ok((pos, idx)),
            Err(idx) => Err((pos, idx)),
        }
    }
    /// Perform binary search to a given input vector and the element to search.
    /// If the element does not exists, the expected position is returned.
    #[inline]
    fn bisect(&self, values: &[T], value: &T, bisect_maxes: bool) -> Result<usize, usize> {
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
        if bisect_maxes {
            match self.order_type {
                OrderType::Asc => {}
                OrderType::Desc => {
                    if low == self.maxes.len() {
                        low -= 1;
                    }
                    if low > 0 {
                        match self.maxes[low].cmp(value) {
                            Ordering::Less => low -= 1,
                            Ordering::Equal => {}
                            Ordering::Greater => {}
                        }
                    }
                }
            }
            Ok(low)
        } else {
            match self.order_type {
                OrderType::Asc => Err(low),
                OrderType::Desc => Err(high),
            }
        }
    }
    /// Perform a binary search on the index using the position in input.
    #[inline]
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
    #[inline]
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
    #[inline]
    fn index_from_tuple(&self, pos: (usize, usize)) -> usize {
        if self.data.len() > 1 {
            return self.index[pos.0] + pos.1;
        }
        pos.1
    }
    #[inline]
    fn build_index(&mut self) {
        self.index.clear();
        if self.is_empty() || self.maxes.len() < 2 {
            return;
        }
        self.index.push(0);
        for i in 0..self.data.len() {
            self.index
                .push(self.index[self.index.len() - 1] + self.data[i].len());
        }
    }
    #[inline]
    fn update_index(&mut self, pos: usize, values_len: i32) {
        if self.maxes.len() > 1 {
            for i in (pos + 1)..self.index.len() {
                self.index[i] = (self.index[i] as i32 + values_len) as usize;
            }
        }
    }
    /// process the element in input based on the ProcessType
    #[inline]
    fn process_element(
        &mut self,
        value: T,
        process_type: ProcessType,
    ) -> Result<usize, SortedContainersError<T>> {
        if self.maxes.is_empty()
            && (process_type == ProcessType::Insert || process_type == ProcessType::InsertOrUpdate)
        {
            // no data inside the collection and process_type is insert. If data is empty is needed
            // to append an empty Vec, after that the element is pushed into data and into maxes vec
            if self.data.is_empty() {
                self.data.push(Vec::new());
            }
            self.data[0].push(value.clone());
            self.maxes.push(value);
            self.len += 1;
            Ok(0)
        } else if self.maxes.is_empty() && process_type == ProcessType::Update {
            // the collection is empty and process_type is update. An error is returned.
            Err(SortedContainersError::ElementNotFound(value))
        } else {
            match self.search_element(&value) {
                Ok(pos) => {
                    if process_type == ProcessType::Update
                        || process_type == ProcessType::InsertOrUpdate
                    {
                        // element exist and process_type is update, the element in input will be
                        // update at the position found.
                        self.data[pos.0][pos.1] = value;
                        Ok(self.index_from_tuple(pos))
                    } else {
                        // element exists and process_type is insert, an error is raised.
                        Err(SortedContainersError::ElementAlreadyExist(value))
                    }
                }
                Err(pos) => {
                    if process_type == ProcessType::Insert
                        || process_type == ProcessType::InsertOrUpdate
                    {
                        //element does not exists and process_type is insert. The element must be
                        // inserted.

                        // if the position is equal to the last element in the vector, the max
                        // element must be updated
                        if value > self.maxes[pos.0] {
                            self.maxes[pos.0] = value.clone();
                        }
                        // the new element is inserted and the len is increased.
                        self.data[pos.0].insert(pos.1, value);
                        self.len += 1;
                        // update the index
                        self.update_index(pos.0, 1);
                        // the inserted position is computed before the eventual expansion
                        let final_pos = self.index_from_tuple((pos.0, pos.1));
                        // if the expand strategy return true, the expand method will be called,
                        // the old vector is splitted in two and the new vector is pushed into data
                        if (self.expand_strategy)(self.data[pos.0].len(), pos.1) {
                            self.expand(pos.0);
                        }
                        // the inserted position is returned
                        Ok(final_pos)
                    } else {
                        //element not found and process_type is update. An error is returned
                        Err(SortedContainersError::ElementNotFound(value))
                    }
                }
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
    use crate::errors::SortedContainersError;
    use crate::sorted_containers::{OrderType, SortedContainers};
    use more_asserts::assert_le;
    use rand::prelude::SliceRandom;
    use rand::{thread_rng, Rng};

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
            assert_eq!(expected_value, v);
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
    fn asc_random_insertion() {
        let mut vec: SortedContainers<i32> = gen_random_sorted_containers(OrderType::Asc, 100_000);
        for i in 0..vec.data.len() {
            let last_val = vec.data[i].last().unwrap();
            assert_eq!(*last_val, vec.maxes[i]);
        }
        let mut prev_element = vec[0];
        for i in 1..vec.len() {
            assert_le!(prev_element, vec[i]);
            prev_element = vec[i];
        }
    }
    #[test]
    fn desc_random_insertion() {
        let mut vec: SortedContainers<i32> = gen_random_sorted_containers(OrderType::Desc, 100_000);
        for i in 0..vec.data.len() {
            let last_val = vec.data[i][0];
            assert_eq!(vec.maxes[i], last_val);
        }
        let mut prev_element = vec[0];
        for i in 1..vec.len() {
            assert_le!(vec[i], prev_element);
            prev_element = vec[i];
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
            assert_eq!(expected_value, v);
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
        let mut vec = gen_random_vec(OrderType::Asc);
        for i in 0..10_000 {
            let to_remove = i - 5000;
            match vec.remove(&to_remove) {
                Ok(removed_value) => assert_eq!(to_remove, removed_value),
                Err(_) => assert!(false),
            }
        }
    }
    #[test]
    fn find_element() {
        let vec = gen_random_vec(OrderType::Asc);
        let mut expected_element = -5000;
        for i in 0..10_000 {
            match vec.find(&expected_element) {
                Ok(pos) => assert_eq!(i, pos),
                Err(_) => assert!(false),
            }
            expected_element += 1;
        }
    }
    #[test]
    fn remove_elements() {
        let mut vec = gen_random_vec(OrderType::Asc);
        for i in -5_000..5_000 {
            match vec.remove(&i) {
                Ok(removed_el) => assert_eq!(removed_el, i),
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
        assert_eq!(vec.len(), 0);
        assert_eq!(vec.data.len(), 0);
        assert_eq!(vec.maxes.len(), 0);
    }
    fn gen_random_sorted_containers(order_type: OrderType, len: usize) -> SortedContainers<i32> {
        let mut vec: SortedContainers<i32> = SortedContainers::new(order_type);
        let mut rng = thread_rng();
        for _i in 0..len {
            match vec.insert_or_update(rng.gen_range(-1_000_000..1_000_000)) {
                Ok(_) => assert!(true),
                Err(_) => assert!(false),
            }
        }
        vec
    }
    fn gen_random_vec(order_type: OrderType) -> SortedContainers<i32> {
        let mut rng = thread_rng();
        let mut vec: SortedContainers<i32> = SortedContainers::new(order_type);
        let mut elements: Vec<i32> = (-5_000..5_000).collect();
        elements.shuffle(&mut rng);
        for el in elements {
            match vec.insert(el) {
                Ok(_) => assert!(true),
                Err(_) => assert!(false),
            }
        }
        vec
    }
}

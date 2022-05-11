use crate::errors::SortedContainersError;
use crate::sorted_container_iter::SortedContainerIter;
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
    pub fn new_with_strategies(
        order_type: OrderType,
        expand_strategy: fn(usize, usize) -> bool,
        shrink_strategy: fn(usize, usize) -> bool,
    ) -> SortedContainers<T> {
        SortedContainers {
            data: vec![Vec::new()],
            maxes: Vec::new(),
            index: Vec::new(),
            order_type,
            len: 0,
            expand_strategy,
            shrink_strategy,
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
    /// Returns the current number of sub-vectors
    pub fn depth(&self) -> usize {
        self.data.len()
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
    pub fn find(&self, element: &T) -> Option<usize> {
        match self.search_element(element) {
            Ok(pos) => Some(self.index_from_tuple(pos)),
            Err(_) => None,
        }
    }
    /// Insert an element inside the collection.
    ///
    /// If the element is not currently inside the collection, the element is inserted
    /// and the actual position is returned.
    /// Complexity is O(log(M)) + O(log(N)) + O(N)
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
    pub fn remove(&mut self, value: &T) -> Option<T> {
        match self.search_element(value) {
            Ok((pos, idx)) => {
                let removed_val = self.data[pos].remove(idx);
                self.update_index(pos, -1);
                self.len -= 1;
                if self.is_empty() {
                    self.maxes.clear();
                    self.data.clear();
                    self.index.clear();
                    return Some(removed_val);
                }
                if self.maxes.len() > 1 && (self.shrink_strategy)(self.data[pos].len(), pos) {
                    self.shrink(pos);
                }
                Some(removed_val)
            }
            Err(_) => None,
        }
    }
    /// Return a vector of elements in a specified range.
    /// Panics in the following scenarios:
    /// 1. start > end
    /// 2. start >= collection length
    /// 3. end >= collection length
    pub fn range(&self, start: usize, end: usize) -> Option<Vec<T>> {
        if start > end {
            panic!("start position is greater than end position");
        }
        if start >= self.len() {
            panic!("start is greater than total len");
        }
        if end >= self.len() {
            panic!("end is greater than total len");
        }
        let mut vec = Vec::new();
        for i in start..end {
            let pos = self.tuple_from_index(i);
            vec.push(self.data[pos.0][pos.1].clone());
        }
        if !vec.is_empty() {
            Some(vec)
        } else {
            None
        }
    }
    /// Apply a filter function to the collection and returns the filtered entries, if any
    pub fn filter(&self, predicate: fn(&T) -> bool) -> Option<Vec<T>> {
        let mut vec = Vec::new();
        for i in 0..self.len() {
            let pos = self.tuple_from_index(i);
            if (predicate)(&self.data[pos.0][pos.1]) {
                vec.push(self.data[pos.0][pos.1].clone());
            }
        }
        if vec.is_empty() {
            None
        } else {
            Some(vec)
        }
    }
    // Apply a map function to the collection and returns the new objects
    pub fn map<K>(&self, predicate: fn(&T) -> K) -> Option<Vec<K>> {
        if self.is_empty() {
            return None;
        }
        let mut vec = Vec::new();
        for i in 0..self.len() {
            let pos = self.tuple_from_index(i);
            vec.push((predicate)(&self.data[pos.0][pos.1]));
        }
        Some(vec)
    }
    // Returns an iterator over the collection
    pub fn iter(&self) -> SortedContainerIter<'_, T> {
        SortedContainerIter {
            data: &self.data,
            pos: 0,
            idx: 0,
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
    ///given a position in input, the element at `self.data[position]` is merged to the previous
    /// or next element present inside `self.data` depending on the the length of the two elements
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
    /// given an index, the function returns the actual position in the form `(usize, usize)`
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
    /// given a position in the form `(usize, usize)`, returns an index
    #[inline]
    fn index_from_tuple(&self, pos: (usize, usize)) -> usize {
        if self.data.len() > 1 {
            return self.index[pos.0] + pos.1;
        }
        pos.1
    }
    /// compute a positional index to transform `usize` position into `(usize, usize)`
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
    /// update the positional index whenever a new element is inserted or removed
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
impl<'a, T: Ord + Clone> IntoIterator for &'a SortedContainers<T> {
    type Item = &'a T;

    type IntoIter = SortedContainerIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
#[cfg(test)]
mod test {
    use crate::sorted_containers::{OrderType, SortedContainers};
    use more_asserts::{assert_gt, assert_lt};
    use rand::prelude::SliceRandom;
    use rand::{thread_rng, Rng};

    #[test]
    fn asc_ordered_insertion() {
        let mut vec = gen_sorted_container(100_000, OrderType::Asc, false);
        let mut prev_element = vec[0];
        for i in 1..vec.len() {
            assert_lt!(prev_element, vec[i]);
            prev_element = vec[i];
        }
        check_maxes(&vec, OrderType::Asc);
        vec.clear();
        assert_eq!(vec.len(), 0);
        assert_eq!(vec.data.len(), 0);
        assert_eq!(vec.maxes.len(), 0);
        assert_eq!(vec.index.len(), 0);
    }
    #[test]
    fn asc_random_insertion() {
        let mut vec = gen_sorted_container(100_000, OrderType::Asc, true);
        let mut prev_element = vec[0];
        for i in 1..vec.len() {
            assert_lt!(prev_element, vec[i]);
            prev_element = vec[i];
        }
        check_maxes(&vec, OrderType::Asc);
        vec.clear();
        assert_eq!(vec.len(), 0);
        assert_eq!(vec.data.len(), 0);
        assert_eq!(vec.maxes.len(), 0);
        assert_eq!(vec.index.len(), 0);
    }
    #[test]
    fn desc_ordered_insertion() {
        let mut vec = gen_sorted_container(100_000, OrderType::Desc, false);
        let mut prev_element = vec[0];
        for i in 1..vec.len() {
            assert_gt!(prev_element, vec[i]);
            prev_element = vec[i];
        }
        check_maxes(&vec, OrderType::Desc);
        vec.clear();
        assert_eq!(vec.len(), 0);
        assert_eq!(vec.is_empty(), true);
        assert_eq!(vec.data.len(), 0);
        assert_eq!(vec.maxes.len(), 0);
        assert_eq!(vec.index.len(), 0);
    }
    #[test]
    fn desc_random_insertion() {
        let mut vec = gen_sorted_container(100_000, OrderType::Desc, true);
        let mut prev_element = vec[0];
        for i in 1..vec.len() {
            assert_gt!(prev_element, vec[i]);
            prev_element = vec[i];
        }
        check_maxes(&vec, OrderType::Desc);
        vec.clear();
        assert_eq!(vec.len(), 0);
        assert_eq!(vec.is_empty(), true);
        assert_eq!(vec.data.len(), 0);
        assert_eq!(vec.maxes.len(), 0);
        assert_eq!(vec.index.len(), 0);
    }

    #[test]
    fn test_insertion() {
        let mut vec = SortedContainers::default();
        match vec.insert(42) {
            Ok(_) => assert!(true),
            Err(_) => assert!(false),
        }
        match vec.insert(42) {
            Ok(_) => assert!(false),
            Err(_) => assert!(true),
        }
    }
    #[test]
    fn test_insertion_desc() {
        let mut vec = SortedContainers::new(OrderType::Desc);
        match vec.insert(42) {
            Ok(_) => assert!(true),
            Err(_) => assert!(false),
        }
        match vec.insert(42) {
            Ok(_) => assert!(false),
            Err(_) => assert!(true),
        }
    }
    #[test]
    fn test_update() {
        let mut vec = SortedContainers::new(OrderType::Desc);
        match vec.update(42) {
            Ok(_) => assert!(false),
            Err(_) => assert!(true),
        }
        match vec.insert(42) {
            Ok(_) => assert!(true),
            Err(_) => assert!(false),
        }
        match vec.update(42) {
            Ok(_) => assert!(true),
            Err(_) => assert!(false),
        }
    }
    #[test]
    fn test_remove() {
        let mut rng = thread_rng();
        let mut vec = gen_sorted_container(10_000, OrderType::Asc, true);
        while vec.len() > 0 {
            let idx = rng.gen_range(0..vec.len());
            let el = vec[idx];
            match vec.remove(&el) {
                Some(removed_element) => assert_eq!(el, removed_element),
                None => assert!(false),
            }
            if !vec.is_empty() && vec.len() % 100 == 0 {
                let mut prev_el = vec[0];
                for i in 1..vec.len() {
                    assert_lt!(prev_el, vec[i]);
                    prev_el = vec[i];
                }
            }
        }
    }
    #[test]
    fn test_index() {
        let vec = gen_sorted_container(100_000, OrderType::Asc, true);
        test_index_check_trait(&vec);
        let vec = gen_sorted_container(100_000, OrderType::Desc, true);
        test_index_check_trait(&vec);
    }
    #[test]
    fn test_range() {
        let vec = gen_sorted_container(5_000, OrderType::Asc, true);
        let rng = vec.range(2500, 7500).unwrap();
        for i in -2_500..2_500 {
            let idx = (i + 2500) as usize;
            assert_eq!(i, rng[idx]);
        }
    }
    #[test]
    fn test_iter() {
        let vec = gen_sorted_container(5_000, OrderType::Asc, false);
        let mut c_element = -5_000;
        for el in &vec {
            assert_eq!(c_element, *el);
            c_element += 1;
        }
    }
    #[test]
    fn test_filter() {
        let vec = gen_sorted_container(5_000, OrderType::Asc, false);
        let filtered_elements = vec.filter(|x| x % 2 == 0);
        assert!(filtered_elements.unwrap().len() == 5_000);
    }
    #[test]
    fn test_map() {
        let mut vec = SortedContainers::default();
        let mut expected_sum = 0;
        for i in 0..10 {
            expected_sum += i * 2;
            vec.insert(i);
        }
        let mapped_elements = vec.map(|x| x * 2);
        let mut sum_mapped_elements = 0;
        for el in mapped_elements.unwrap() {
            sum_mapped_elements += el;
        }
        assert_eq!(sum_mapped_elements, expected_sum);
    }

    fn test_index_check_trait(vec: &SortedContainers<i32>) {
        let mut idx = 0;
        let mut pos = 0;
        for i in 0..vec.len() {
            if pos == vec.data[idx].len() {
                pos = 0;
                idx += 1;
            }
            assert_eq!(vec[i], vec.data[idx][pos]);
            pos += 1;
        }
    }
    fn check_maxes(vec: &SortedContainers<i32>, order_type: OrderType) {
        for i in 0..vec.data.len() {
            match order_type {
                OrderType::Asc => {
                    let last_el = vec.data[i].last().unwrap();
                    assert_eq!(*last_el, vec.maxes[i]);
                }
                OrderType::Desc => {
                    let first_el = vec.data[i].first().unwrap();
                    assert_eq!(*first_el, vec.maxes[i]);
                }
            }
        }
    }
    fn gen_sorted_container(
        len: usize,
        order_type: OrderType,
        shuffle: bool,
    ) -> SortedContainers<i32> {
        let mut rng = thread_rng();
        let mut sorted_vec = SortedContainers::new(order_type);
        let mut elements: Vec<i32> = (-(len as i32)..len as i32).collect();
        if shuffle {
            elements.shuffle(&mut rng);
        }
        for el in elements {
            match sorted_vec.insert_or_update(el) {
                Ok(_) => assert!(true),
                Err(_) => assert!(false),
            }
        }
        sorted_vec
    }
}

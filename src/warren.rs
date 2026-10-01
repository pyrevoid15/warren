use std::{fmt::Debug, mem::MaybeUninit};

type MaskType = u64;
pub const REGION_SIZE: usize = MaskType::BITS as usize;

const MAX_WARREN_ITEMS_PRINTED: usize = 128;

/// A growable data structure that reuses entry spaces in continuous memory.
/// Has constant-time insertion and removal, and fast iteration. 
/// Since elements are stored contiguously, this structure lacks pointer stability.
/// An index will point to the same element until removal, however.
/// 
/// Inspired by plf::hive, boost::container::hub, and colony-rs.
/// 
pub struct Warren<T> {
    data_regions: Vec<[MaybeUninit<T>; REGION_SIZE]>,
    guard: Vec<MaskType>,
    region_idx_stack: Vec<usize>,
    size: usize,
    capacity: usize
}

impl<T> Warren<T> {
    
    /// Creates a Warren with the default number of regions
    /// 
    /// This function is equivalent to: `Warren::<T>::with_region_count(1)`
    #[inline] 
    #[allow(unused)]
    pub fn new() -> Self {
        Self::with_region_count(1)
    }

    /// Creates a Warren with the minimum number of regions which is
    /// 
    /// (a) greater than or equal to capacity / REGION_SIZE and
    /// (b) a power of two
    /// 
    /// This function is equivalent to: `Warren::<T>::with_region_count(ceil(capacity as f32 / REGION_SIZE as f32))`
    #[inline]
    #[allow(unused)]    
    pub fn with_capacity(capacity: usize) -> Self {
        assert!(capacity > 0);
        let num_regions = capacity / REGION_SIZE + (capacity % REGION_SIZE > 0) as usize;
        Self::with_region_count(num_regions)
    }
    
    /// Creates a Warren with a number of regions which is the next highest power of two to the given `region_count`.
    #[inline]
    #[allow(unused)]
    pub fn with_region_count(region_count: usize) -> Self {
        let real_region_count = region_count.next_power_of_two();
        Self::with_region_count_strict(real_region_count)
    }

    /// Creates a Warren with the exact number of regions `region_count`.
    #[allow(unused)]
    pub fn with_region_count_strict(region_count: usize) -> Self {
        assert!(region_count > 0);

        let capacity = region_count * REGION_SIZE;

        let mut data_regions = Vec::with_capacity(region_count);
        data_regions.resize_with(region_count, || unsafe{ std::mem::zeroed() });

        let mut guard = Vec::with_capacity(region_count);
        guard.resize_with(region_count, || 0);

        let region_idx_stack = (0..region_count).rev().collect();

        Self { data_regions, guard, region_idx_stack, size: 0, capacity }
    }

    //#[inline(always)] 
    fn _region_is_empty(&self, region_idx: usize) -> bool { self.guard[region_idx] == 0 }

    //#[inline(always)] 
    fn _region_is_full(&self, region_idx: usize) -> bool { self.guard[region_idx] == MaskType::MAX }

    fn _find_insertion_region(&mut self) -> Option<usize> {
        while let Some(&region_idx) = self.region_idx_stack.last() {
            if self._region_is_full(region_idx) { 
                self.region_idx_stack.pop(); continue;
            }
            
            return Some(region_idx); 
        }
        return None;
    }

    fn _add_new_region(&mut self) -> usize {
        debug_assert!(self.size == self.capacity);

        self.guard.push(0);

        let region_idx = self.data_regions.len();
        self.data_regions.push(unsafe { std::mem::zeroed() });

        self.capacity += REGION_SIZE;
        self.region_idx_stack.push(region_idx);

        return region_idx;
    } 

    fn _insert_in_region(&mut self, region_idx: usize, value: T) -> usize {
        debug_assert!(!self._region_is_full(region_idx));
        
        let internal_index = self.guard[region_idx].trailing_ones() as usize;
        self.guard[region_idx] |= 1 << internal_index;

        let real_index = internal_index + REGION_SIZE * region_idx;
        self.data_regions.as_flattened_mut()[real_index] = MaybeUninit::new(value);

        self.size += 1;

        return real_index;
    }

    /// Inserts a `value` into the Warren and returns the index it was placed into.
    /// 
    /// The index that is chosen can be anywhere in the range [0, capacity). If there are no empty indices, a new region is created.
    /// ```
    /// let mut warren = warren::Warren::new();
    /// let index = warren.insert(9u32);
    /// assert_eq!(warren.get(index), Some(9).as_ref());
    /// ```
    #[allow(unused)]
    pub fn insert(&mut self, value: T) -> usize {
        let region_index = self._find_insertion_region()
            .unwrap_or_else(|| { self._add_new_region() });

        self._insert_in_region(region_index, value)
    }

    /// Inserts a `value` into the Warren and returns the index and a mutable reference to the inserted `value` without needing to also call `Warren::get_mut`.
    /// 
    /// See `Warren::insert` for more detail on insertion.
    /// 
    /// ```
    /// let mut warren = warren::Warren::new();
    /// let (index, value) = warren.insert_mut(9u32);
    /// *value = 10;
    /// assert_eq!(warren.get(index), Some(10).as_ref());
    /// ```
    pub fn insert_mut(&mut self, value: T) -> (usize, &mut T) {
        let index = self.insert(value);
        let mref = unsafe { self.data_regions.as_flattened_mut()[index].assume_init_mut() };
        (index, mref)
    }
    
    fn _remove_from_region(&mut self, region_idx: usize, internal_index: usize, real_index: usize) -> bool {
        let mask = (1 as MaskType) << internal_index;
        if self.guard[region_idx] == 0 || self.guard[region_idx] & mask == 0 { 
            return false; 
        }

        self.guard[region_idx] &= !mask;
        unsafe { self.data_regions.as_flattened_mut()[real_index].assume_init_drop(); }

        return true;
    }

    /// Removes the value at the given `index`.
    /// 
    /// If there is a value at that `index`, this function drops that value, sets its flag to inactive, and returns `true`.
    /// Otherwise, returns `false`.
    /// 
    /// ```
    /// let mut warren = warren::Warren::new();
    /// let (index, value) = warren.insert_mut(9u32);
    /// *value = 10;
    /// assert_eq!(warren.get(index), Some(10).as_ref());
    /// assert_eq!(warren.contains(index), true);
    /// 
    /// assert_eq!(warren.remove(index), true);
    /// assert_eq!(warren.contains(index), false);
    /// 
    /// assert_eq!(warren.remove(index), false);
    /// assert_eq!(warren.contains(index), false);
    /// ```
    #[allow(unused)]
    pub fn remove(&mut self, index: usize) -> bool {
        if index >= self.capacity { return false; }

        let region_index = index / REGION_SIZE;
        let internal_index = index % REGION_SIZE;

        let was_full = self._region_is_full(region_index);
        let success = self._remove_from_region(region_index, internal_index, index);

        if success && was_full { self.region_idx_stack.push(region_index); }

        self.size -= success as usize;
        return success;
    }

    /// Returns `true` if there is a value at that `index`. False otherwise.
    /// ```
    /// let mut warren = warren::Warren::<u32>::from_iter(vec![1, 3, 5]);
    /// assert_eq!(warren.contains(0), true);
    /// assert_eq!(warren.contains(1), true);
    /// assert_eq!(warren.contains(3), false);
    /// ```
    pub fn contains(&self, index: usize) -> bool {
        let ri = index / REGION_SIZE;
        let ii = index % REGION_SIZE;
        return index < self.capacity && self.guard[ri] & (1 << ii) != 0;
    }

    /// Returns an immutable reference if there is a value at that `index` inside an `Option`. Otherwise, returns `None`.
    /// ```
    /// let mut warren = warren::Warren::<u32>::from_iter(vec![1, 3, 5]);
    /// assert_eq!(warren.get(0), Some(1).as_ref());
    /// assert_eq!(warren.get(1), Some(3).as_ref());
    /// assert_eq!(warren.get(2), Some(5).as_ref());
    /// ```
    pub fn get(&self, index: usize) -> Option<&T> {
        let ri = index / REGION_SIZE;
        let ii = index % REGION_SIZE;
        if self.guard[ri] & (1 << ii) != 0 {
            self.data_regions.as_flattened().get(index)
                .and_then(|x| unsafe { Some(x.assume_init_ref()) })
        } else { None }
    }

    /// Returns a mutable reference if there is a value at that `index` inside an `Option`. Otherwise, returns `None`.
    /// ```
    /// let mut warren = warren::Warren::<u32>::from_iter(vec![1, 3, 5]);
    /// if let Some(x) = warren.get_mut(2) { 
    ///     *x = 0;
    /// };
    /// assert_eq!(warren.get(2), Some(0).as_ref());
    /// ```
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        let ri = index / REGION_SIZE;
        let ii = index % REGION_SIZE;
        if self.guard[ri] & (1 << ii) != 0 {
            self.data_regions.as_flattened_mut().get_mut(index)
                .and_then(|x| unsafe { Some(x.assume_init_mut()) })
        } else { None }
    }
    
    /// Returns N mutable reference options for each index in `indices`. 
    /// When there is no value at that index, the corresponding option will be `None`.
    /// Unlike `Vec::get_disjoint_mut`, this function does not ensure that indices are deduplicated.
    /// 
    /// ```
    /// let mut warren = warren::Warren::<u32>::from_iter(vec![1, 3, 5, 6, 8]);
    /// let mut opts = warren.get_disjoint_mut([0, 6]);
    /// 
    /// assert!(opts[0].is_some());
    /// assert!(opts[1].is_none());
    /// 
    /// if let Some(x) = &mut opts[0] {
    ///     **x = 0;
    /// }
    /// 
    /// assert_eq!(warren.get(0), Some(0).as_ref());
    /// 
    /// ```
    pub fn get_disjoint_mut<const N: usize>(&mut self, indices: [usize; N]) -> [Option<&mut T>; N] {
        let ptr = self as *mut Warren<T>;

        let result = indices.map(|index| {
            unsafe { (*ptr).get_mut(index) }
        });

        result
    }

    /// Removes all elements in the Warren for which `f`` is false.
    pub fn retain(&mut self, mut f: impl FnMut(&T) -> bool) {
        let ptr = self as *mut Warren<T>;
        self.iter()
            .for_each(|(i, x)| {
                let keep = f(x);
                if !keep { 
                    unsafe { (*ptr).remove(i); } 
                }
            });
    }

    /// Removes all elements in the Warren for which `f`` is false. 
    /// `f` may also mutate the elements.
    pub fn retain_mut(&mut self, mut f: impl FnMut(&mut T) -> bool) {
        let ptr = self as *mut Warren<T>;
        self.iter_mut()
            .for_each(|(i, x)| {
                let keep = f(x);
                if !keep { 
                    unsafe { (*ptr).remove(i); } 
                }
            });
    }

    /// Returns the number of elements in this Warren.
    #[inline] pub fn size(&self) -> usize { self.size }

    /// Returns the number of entries (active and inactive) in this Warren.
    #[inline] pub fn capacity(&self) -> usize { self.capacity }

    fn _get_first_iter_location(&self) -> Option<(usize, usize)> {
        let region_index = self.guard.iter()
            .position(|x|{ *x != 0 });

        region_index
            .and_then(|i| {
                Some((i, self.guard[i].trailing_zeros() as usize))
            })
    }

    fn _get_next_iter_location(&self, (region_index, internal_index): (usize, usize)) -> Option<(usize, usize)> {
        if internal_index < REGION_SIZE {
            let next_internal_index = internal_index + 1;
            if next_internal_index < REGION_SIZE {
                let lookup = self.guard[region_index] >> next_internal_index;
                if lookup != 0 {
                    let next_index = next_internal_index + lookup.trailing_zeros() as usize;
                    return Some((region_index, next_index));
                }
            }
        }

        for next_region_index in (region_index + 1)..(self.data_regions.len()) {
            if self._region_is_empty(next_region_index) { continue; }

            let internal_index = self.guard[next_region_index].trailing_zeros() as usize;
            return Some((next_region_index, internal_index));
        }
        
        return None;
    }

    /// Creates an iterator for this Warren.
    pub fn iter(&self) -> WarrenIter<'_, T> { 
        let (region_index, internal_index) = self._get_first_iter_location()
            .unwrap_or_else(|| { (self.data_regions.len(), 0) });
        WarrenIter { warren: &self, region_index, internal_index }
    }

    /// Creates an iterator for this Warren that can be used to mutate the elements inside the Warren.
    pub fn iter_mut(&mut self) -> WarrenIterMut<'_, T> {
        let (region_index, internal_index) = self._get_first_iter_location()
            .unwrap_or_else(|| { (self.data_regions.len(), 0) });
        WarrenIterMut { warren: self, region_index, internal_index }
    }
}

impl<T> Default for Warren<T> {
    fn default() -> Self { Self::new() }
}


impl<T: std::fmt::Debug> std::fmt::Debug for Warren<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Warren ")?;
        f.debug_list().entries(self.iter().take(MAX_WARREN_ITEMS_PRINTED)).finish_non_exhaustive()
    }
}

impl<T> FromIterator<T> for Warren<T> {
    /// Note: This does not return indices. If you want indices, may be better to call the following instead.  
    /// ```
    /// let mut warren = Warrem::new();
    /// iter.into_iter().map(|x| { warren.insert(x) })
    /// ``` 
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut warren = Warren::with_region_count_strict(8);
        iter.into_iter().for_each(|x|{ warren.insert(x); });
        warren
    }
}

impl<T> Extend<T> for Warren<T> {
    /// Note: This does not return indices. If you want indices, may be better to call the following instead.  
    /// ```
    /// let mut warren = Warrem::new();
    /// iter.into_iter().map(|x| { warren.insert(x) })
    /// ``` 
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        iter.into_iter().for_each(|x|{ self.insert(x); });
    }
}

pub struct WarrenIter<'a, T> {
    warren: &'a Warren<T>,
    region_index: usize,
    internal_index: usize,
}

impl<'a, T> Iterator for WarrenIter<'a, T> {
    type Item = (usize, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        if self.region_index >= self.warren.data_regions.len() {
            return None;
        }

        let real_index = self.region_index * REGION_SIZE + self.internal_index;
        let next_location = self.warren._get_next_iter_location((self.region_index, self.internal_index));
        (self.region_index, self.internal_index) = next_location
            .unwrap_or((self.warren.data_regions.len(), 0));

        Some((real_index, unsafe { self.warren.data_regions.as_flattened()[real_index].assume_init_ref() }))
    }
}

impl<T: Debug> Debug for WarrenIter<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WarrenIter").field("warren", &self.warren).field("region_index", &self.region_index).field("internal_index", &self.internal_index).finish()
    }
}

pub struct WarrenIterMut<'a, T> {
    warren: &'a mut Warren<T>,
    region_index: usize,
    internal_index: usize,
}

impl<'a, T> Iterator for WarrenIterMut<'a, T> {
    type Item = (usize, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        if self.region_index >= self.warren.data_regions.len() {
            return None;
        }

        let real_index = self.region_index * REGION_SIZE + self.internal_index;
        let next_location = self.warren._get_next_iter_location((self.region_index, self.internal_index));
        (self.region_index, self.internal_index) = next_location
            .unwrap_or((self.warren.data_regions.len(), 0));

        Some((real_index, unsafe { &mut *self.warren.data_regions.as_flattened_mut()[real_index].as_mut_ptr() }))
    }
}

impl<T: Debug> Debug for WarrenIterMut<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WarrenIterMut").field("warren", &self.warren).field("region_index", &self.region_index).field("internal_index", &self.internal_index).finish()
    }
}
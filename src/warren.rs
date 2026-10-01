use std::{fmt::Debug, mem::MaybeUninit};

type MaskType = u64;
pub const REGION_SIZE: usize = MaskType::BITS as usize;

pub trait WarrenIndex: private::Sealed + Sized + Eq + Debug {
    fn real_index(&self) -> usize;
}

impl WarrenIndex for usize {
    fn real_index(&self) -> usize { *self }
}

impl WarrenIndex for (usize, u8) {
    fn real_index(&self) -> usize { self.0 }
}

/// A trait determining the capabilities of a Warren.
/// Types with this trait track which slots in each region are active and how indices are reused.
///
/// Implementations decide:
/// - which index type is returned
/// - how insertion/removal updates occupancy
/// - how active entries are validated
///
/// `FlagGuard` is the default, and `GenerationGuard` adds generation checks
/// so stale indices cannot accidentally access old values.
pub trait WarrenGuard: private::Sealed {
    type MaskType;
    type IndexType: WarrenIndex;

    const REGION_SIZE: usize = MaskType::BITS as usize;

    #[doc(hidden)]
    fn _with_region_count(region_count: usize) -> Self;

    #[doc(hidden)]
    fn _on_add_region(&mut self);

    #[doc(hidden)]
    fn _on_insert(&mut self, region_idx: usize) -> Self::IndexType;

    #[doc(hidden)]
    fn _region_is_full(&self, region_idx: usize) -> bool;

    #[doc(hidden)]
    fn _region_is_empty(&self, region_idx: usize) -> bool;

    #[doc(hidden)]
    fn _on_remove(&mut self, index: Self::IndexType) -> (bool, bool, usize);

    #[doc(hidden)]
    fn _check_active(&self, index: Self::IndexType) -> bool;
    
    #[doc(hidden)]
    fn _get_first_iter_location(&self) -> Option<(usize, usize)>;
    
    #[doc(hidden)]
    fn _get_next_iter_location(&self, ri_ii: (usize, usize)) -> Option<(usize, usize)>;

    #[doc(hidden)]
    fn _get_index(&self, region_idx: usize, internal_idx: usize) -> Self::IndexType;
}

/// Default guard that uses a bitmask per region.
/// Each set bit means a corresponding entry is currently occupied.
/// Has no generation checks.
pub struct FlagGuard {
    guard: Vec<<FlagGuard as WarrenGuard>::MaskType>
}

impl WarrenGuard for FlagGuard {
    type MaskType = MaskType;
    type IndexType = usize;

    fn _with_region_count(region_count: usize) -> Self {
        let mut guard = Vec::with_capacity(region_count);
        guard.resize_with(region_count, || 0);
        Self { guard }
    }

    fn _on_add_region(&mut self) {
        self.guard.push(0);
    }

    fn _on_insert(&mut self, region_idx: usize) -> Self::IndexType {
        let internal_index = self.guard[region_idx].trailing_ones() as usize;
        self.guard[region_idx] |= 1 << internal_index;

        let real_index = region_idx * Self::REGION_SIZE + internal_index;
        real_index
    }

    fn _region_is_full(&self, region_idx: usize) -> bool {
        self.guard[region_idx] == Self::MaskType::MAX
    }

    fn _region_is_empty(&self, region_idx: usize) -> bool {
        self.guard[region_idx] == 0
    }

    fn _on_remove(&mut self, index: Self::IndexType) -> (bool, bool, usize) {
        
        let region_index = index / Self::REGION_SIZE;
        let internal_index = index % Self::REGION_SIZE;

        let mask = (1 as MaskType) << internal_index;
        if self.guard[region_index] == 0 || self.guard[region_index] & mask == 0 { 
            return (false, false, 0); 
        }

        let was_full = self._region_is_full(region_index);
        self.guard[region_index] &= !mask;

        return (true, was_full, region_index);
    }

    fn _check_active(&self, index: Self::IndexType) -> bool {
        let ri = index / Self::REGION_SIZE;
        let ii = index % Self::REGION_SIZE;
        return self.guard[ri] & (1 << ii) != 0;
    }
    
    #[doc(hidden)]
    fn _get_first_iter_location(&self) -> Option<(usize, usize)> {
        let region_index = self.guard.iter()
            .position(|x|{ *x != 0 });

        region_index
            .and_then(|i| {
                Some((i, self.guard[i].trailing_zeros() as usize))
            })
    }

    #[doc(hidden)]
    fn _get_next_iter_location(&self, (region_index, internal_index): (usize, usize)) -> Option<(usize, usize)> {
        if internal_index < Self::REGION_SIZE {
            let next_internal_index = internal_index + 1;
            if next_internal_index < Self::REGION_SIZE {
                let lookup = self.guard[region_index] >> next_internal_index;
                if lookup != 0 {
                    let next_index = next_internal_index + lookup.trailing_zeros() as usize;
                    return Some((region_index, next_index));
                }
            }
        }

        for next_region_index in (region_index + 1)..(self.guard.len()) {
            if self._region_is_empty(next_region_index) { continue; }

            let internal_index = self.guard[next_region_index].trailing_zeros() as usize;
            return Some((next_region_index, internal_index));
        }
        
        return None;
    }

    fn _get_index(&self, region_idx: usize, internal_idx: usize) -> Self::IndexType {
        region_idx * REGION_SIZE + internal_idx
    }
}

/// Guard that stores a generation counter per entry to reject stale indices.
pub struct GenerationGuard {
    guard: FlagGuard,
    generation: Vec<[u8; REGION_SIZE]> //TODO: Change "64" to Self::REGION_SIZE after implementing WarrenGuard.
}

impl WarrenGuard for GenerationGuard {
    type MaskType = MaskType;

    type IndexType = (usize, u8);

    fn _with_region_count(region_count: usize) -> Self {
        let guard = FlagGuard::_with_region_count(region_count);

        let mut generation = Vec::with_capacity(region_count);
        generation.resize(region_count, [0; REGION_SIZE]);

        Self { guard, generation }
    }

    fn _on_add_region(&mut self) {
        self.guard._on_add_region();
        self.generation.push([0; REGION_SIZE]);
    }

    fn _on_insert(&mut self, region_idx: usize) -> Self::IndexType {
        let real_index = self.guard._on_insert(region_idx);
        let next_generation = self.generation.as_flattened()[real_index].checked_add(1).unwrap_or(0);
        self.generation.as_flattened_mut()[real_index] = next_generation;
        (real_index, next_generation)
    }

    fn _region_is_full(&self, region_idx: usize) -> bool {
        self.guard._region_is_full(region_idx)
    }

    fn _region_is_empty(&self, region_idx: usize) -> bool {
        self.guard._region_is_empty(region_idx)
    }

    fn _on_remove(&mut self, index: Self::IndexType) -> (bool, bool, usize) {
        self.guard._on_remove(index.real_index())
    }

    fn _check_active(&self, index: Self::IndexType) -> bool {
        let ri = index.0 / Self::REGION_SIZE;
        let ii = index.0 % Self::REGION_SIZE;
        return self.guard.guard[ri] & (1 << ii) != 0 && self.generation.as_flattened()[index.0] == index.1;
    }

    fn _get_first_iter_location(&self) -> Option<(usize, usize)> {
        self.guard._get_first_iter_location()
    }

    fn _get_next_iter_location(&self, ri_ii: (usize, usize)) -> Option<(usize, usize)> {
        self.guard._get_next_iter_location(ri_ii)
    }

    fn _get_index(&self, region_idx: usize, internal_idx: usize) -> Self::IndexType {
        let real_index = self.guard._get_index(region_idx, internal_idx);
        let generation = self.generation.as_flattened()[real_index];
        (real_index, generation)
    }
}

mod private {
    use crate::warren::{FlagGuard, GenerationGuard};

    pub trait Sealed {}

    impl Sealed for FlagGuard {}
    impl Sealed for GenerationGuard {}

    impl Sealed for usize {}
    impl Sealed for (usize, u8) {}
}


const MAX_WARREN_ITEMS_PRINTED: usize = 128;

/// A growable data structure that reuses entry spaces in continuous memory.
/// Has constant-time insertion and removal, and fast iteration. 
/// Since elements are stored contiguously, this structure lacks pointer stability.
/// An index will point to the same element until removal, however.
/// 
/// /// The `G` parameter controls how active slots and indices are tracked.
/// Use `FlagGuard` for the default behavior, or `GenerationGuard` when you
/// need stale-index protection.
/// 
/// Inspired by plf::hive, boost::container::hub, and colony-rs.
/// 
pub struct Warren<T, G: WarrenGuard = FlagGuard> {
    data_regions: Vec<[MaybeUninit<T>; REGION_SIZE]>,
    guard: G,
    region_idx_stack: Vec<usize>,
    size: usize,
    capacity: usize
}

impl<T, G: WarrenGuard> Warren<T, G> {
    
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

        let guard = G::_with_region_count(region_count);

        let region_idx_stack = (0..region_count).rev().collect();

        Self { data_regions, guard, region_idx_stack, size: 0, capacity }
    }

    #[doc(hidden)]
    fn _find_insertion_region(&mut self) -> Option<usize> {
        while let Some(&region_idx) = self.region_idx_stack.last() {
            if self.guard._region_is_full(region_idx) { 
                self.region_idx_stack.pop(); continue;
            }
            
            return Some(region_idx); 
        }
        return None;
    }

    #[doc(hidden)]
    fn _add_new_region(&mut self) -> usize {
        debug_assert!(self.size == self.capacity);

        self.guard._on_add_region();

        let region_idx = self.data_regions.len();
        self.data_regions.push(unsafe { std::mem::zeroed() });

        self.capacity += REGION_SIZE;
        self.region_idx_stack.push(region_idx);

        return region_idx;
    } 

    #[doc(hidden)]
    fn _insert_in_region(&mut self, region_idx: usize, value: T) -> G::IndexType {
        debug_assert!(!self.guard._region_is_full(region_idx));
        
        let index = self.guard._on_insert(region_idx);

        let real_index = index.real_index();
        self.data_regions.as_flattened_mut()[real_index] = MaybeUninit::new(value);

        self.size += 1;

        return index;
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
    pub fn insert(&mut self, value: T) -> G::IndexType {
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
    pub fn insert_mut(&mut self, value: T) -> (G::IndexType, &mut T) {
        let index = self.insert(value);
        let mref = unsafe { self.data_regions.as_flattened_mut()[index.real_index()].assume_init_mut() };
        (index, mref)
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
    pub fn remove(&mut self, index: G::IndexType) -> bool {
        let real_index = index.real_index();
        if real_index >= self.capacity { return false; }

        let (success, was_full, region_index) = self.guard._on_remove(index);

        if success {
            unsafe { self.data_regions.as_flattened_mut()[real_index].assume_init_drop(); }
            if was_full {
                self.region_idx_stack.push(region_index);
            }
        }

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
    pub fn contains(&self, index: G::IndexType) -> bool {
        return index.real_index() < self.capacity && self.guard._check_active(index);
    }

    /// Returns an immutable reference if there is a value at that `index` inside an `Option`. Otherwise, returns `None`.
    /// ```
    /// let mut warren = warren::Warren::<u32>::from_iter(vec![1, 3, 5]);
    /// assert_eq!(warren.get(0), Some(1).as_ref());
    /// assert_eq!(warren.get(1), Some(3).as_ref());
    /// assert_eq!(warren.get(2), Some(5).as_ref());
    /// ```
    pub fn get(&self, index: G::IndexType) -> Option<&T> {
        let real_index = index.real_index();
        if self.guard._check_active(index) {
            self.data_regions.as_flattened().get(real_index)
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
    pub fn get_mut(&mut self, index: G::IndexType) -> Option<&mut T> {
        let real_index = index.real_index();
        if self.guard._check_active(index) {
            self.data_regions.as_flattened_mut().get_mut(real_index)
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
    pub fn get_disjoint_mut<const N: usize>(&mut self, indices: [G::IndexType; N]) -> [Option<&mut T>; N] {
        let ptr = self as *mut Warren<T, G>;

        let result = indices.map(|index| {
            unsafe { (*ptr).get_mut(index) }
        });

        result
    }

    /// Removes all elements in the Warren for which `f`` is false.
    pub fn retain(&mut self, mut f: impl FnMut(&T) -> bool) {
        let ptr = self as *mut Warren<T, G>;
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
        let ptr = self as *mut Warren<T, G>;
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

    

    /// Creates an iterator for this Warren.
    pub fn iter(&self) -> WarrenIter<'_, T, G> { 
        let (region_index, internal_index) = self.guard._get_first_iter_location()
            .unwrap_or_else(|| { (self.data_regions.len(), 0) });
        WarrenIter { warren: &self, region_index, internal_index }
    }

    /// Creates an iterator for this Warren that can be used to mutate the elements inside the Warren.
    pub fn iter_mut(&mut self) -> WarrenIterMut<'_, T, G> {
        let (region_index, internal_index) = self.guard._get_first_iter_location()
            .unwrap_or_else(|| { (self.data_regions.len(), 0) });
        WarrenIterMut { warren: self, region_index, internal_index }
    }
}

impl<T, G: WarrenGuard> Default for Warren<T, G> {
    fn default() -> Self { Self::new() }
}


impl<T: std::fmt::Debug, G: WarrenGuard> std::fmt::Debug for Warren<T, G> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Warren ")?;
        f.debug_list().entries(self.iter().take(MAX_WARREN_ITEMS_PRINTED)).finish_non_exhaustive()
    }
}

impl<T, G: WarrenGuard> FromIterator<T> for Warren<T, G> {
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

impl<T, G: WarrenGuard> Extend<T> for Warren<T, G> {
    /// Note: This does not return indices. If you want indices, may be better to call the following instead.  
    /// ```
    /// let mut warren = Warrem::new();
    /// iter.into_iter().map(|x| { warren.insert(x) })
    /// ``` 
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        iter.into_iter().for_each(|x|{ self.insert(x); });
    }
}

pub struct WarrenIter<'a, T, G: WarrenGuard> {
    warren: &'a Warren<T, G>,
    region_index: usize,
    internal_index: usize,
}

impl<'a, T, G: WarrenGuard> Iterator for WarrenIter<'a, T, G> {
    type Item = (G::IndexType, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        if self.region_index >= self.warren.data_regions.len() {
            return None;
        }

        let index = self.warren.guard._get_index(self.region_index, self.internal_index);
        let next_location = self.warren.guard._get_next_iter_location((self.region_index, self.internal_index));
        (self.region_index, self.internal_index) = next_location
            .unwrap_or((self.warren.data_regions.len(), 0));

        let real_index = index.real_index();
        Some((index, unsafe { self.warren.data_regions.as_flattened()[real_index].assume_init_ref() }))
    }
}

impl<T: Debug, G: WarrenGuard> Debug for WarrenIter<'_, T, G> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WarrenIter").field("warren", &self.warren).field("region_index", &self.region_index).field("internal_index", &self.internal_index).finish()
    }
}

pub struct WarrenIterMut<'a, T, G: WarrenGuard> {
    warren: &'a mut Warren<T, G>,
    region_index: usize,
    internal_index: usize,
}

impl<'a, T, G: WarrenGuard> Iterator for WarrenIterMut<'a, T, G> {
    type Item = (G::IndexType, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        if self.region_index >= self.warren.data_regions.len() {
            return None;
        }

        let index = self.warren.guard._get_index(self.region_index, self.internal_index);
        let next_location = self.warren.guard._get_next_iter_location((self.region_index, self.internal_index));
        (self.region_index, self.internal_index) = next_location
            .unwrap_or((self.warren.data_regions.len(), 0));

        let real_index = index.real_index();
        Some((index, unsafe { &mut *self.warren.data_regions.as_flattened_mut()[real_index].as_mut_ptr() }))
    }
}

impl<T: Debug, G: WarrenGuard> Debug for WarrenIterMut<'_, T, G> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WarrenIterMut").field("warren", &self.warren).field("region_index", &self.region_index).field("internal_index", &self.internal_index).finish()
    }
}
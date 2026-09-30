use std::mem::MaybeUninit;

type MaskType = u32;
pub const REGION_SIZE: usize = MaskType::BITS as usize;

pub struct Warren<T> {
    data_regions: Vec<[MaybeUninit<T>; REGION_SIZE]>,
    guard: Vec<MaskType>,
    region_idx_stack: Vec<usize>,
    size: usize,
    capacity: usize
}

impl<T> Warren<T> {
    #[inline] 
    #[allow(unused)]
    pub fn new() -> Self {
        Self::with_region_count(1)
    }

    #[inline]
    #[allow(unused)]
    pub fn with_capacity(capacity: usize) -> Self {
        assert!(capacity > 0);
        let num_regions = capacity / REGION_SIZE + (capacity % REGION_SIZE > 0) as usize;
        Self::with_region_count(num_regions)
    }

    #[allow(unused)]
    pub fn with_region_count(region_count: usize) -> Self {
        assert!(region_count > 0);

        let real_region_count = region_count.next_power_of_two();

        let capacity = real_region_count * REGION_SIZE;

        let mut data_regions = Vec::with_capacity(real_region_count);
        data_regions.resize_with(real_region_count, || unsafe{ std::mem::zeroed() });

        let mut guard = Vec::with_capacity(real_region_count);
        guard.resize_with(real_region_count, || 0);

        let region_idx_stack = (0..real_region_count).rev().collect();

        Self { data_regions, guard, region_idx_stack, size: 0, capacity }
    }

    #[inline] fn _region_is_empty(&self, region_idx: usize) -> bool { self.guard[region_idx] == 0 }
    #[inline] fn _region_is_full(&self, region_idx: usize) -> bool { self.guard[region_idx] == MaskType::MAX }

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

    #[allow(unused)]
    pub fn insert(&mut self, value: T) -> usize {
        let region_index = self._find_insertion_region()
            .unwrap_or_else(|| { self._add_new_region() });

        self._insert_in_region(region_index, value)
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

    pub fn get(&self, index: usize) -> Option<&T> {
        let ri = index / REGION_SIZE;
        let ii = index % REGION_SIZE;
        if self.guard[ri] & (1 << ii) != 0 {
            self.data_regions.as_flattened().get(index)
                .and_then(|x| unsafe { Some(x.assume_init_ref()) })
        } else { None }
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        let ri = index / REGION_SIZE;
        let ii = index % REGION_SIZE;
        if self.guard[ri] & (1 << ii) != 0 {
            self.data_regions.as_flattened_mut().get_mut(index)
                .and_then(|x| unsafe { Some(x.assume_init_mut()) })
        } else { None }
    }

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

    #[inline] pub fn size(&self) -> usize { self.size }
    #[inline] pub fn capacity(&self) -> usize { self.capacity }

    pub fn iter(&self) -> WarrenIter<'_, T> { 
        let (region_index, internal_index) = self._get_first_iter_location()
            .unwrap_or_else(|| { (self.data_regions.len(), 0) });
        WarrenIter { warren: &self, region_index, internal_index }
    }

    pub fn iter_mut(&mut self) -> WarrenIterMut<'_, T> {
        let (region_index, internal_index) = self._get_first_iter_location()
            .unwrap_or_else(|| { (self.data_regions.len(), 0) });
        WarrenIterMut { warren: self, region_index, internal_index }
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


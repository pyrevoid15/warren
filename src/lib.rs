//! # Warren
//!
//! A compact, index-based container designed for fast insertion, fast reuse of
//! freed slots, and efficient iteration over active entries.
//!
//! `Warren` stores values in contiguous regions and reuses emptied positions
//! in constant time. This makes it a good fit for
//! workloads with frequent insert/remove cycles and a need to keep iteration
//! over live elements straightforward.
//!
//! ## Guards
//!
//! The storage policy is controlled by types with the `WarrenGuard` trait. The default
//! `FlagGuard` tracks occupancy with a bitmask, while `GenerationGuard` adds a
//! generation counter so stale indices cannot accidentally mutate a newly
//! inserted value in the same index.
//!
//! ```rust
//! use warren::{Warren, GenerationGuard};
//!
//! let mut warren: Warren<u32, GenerationGuard> = Warren::new();
//! let (index, generation) = warren.insert(42);
//!
//! assert_eq!(warren.get((index.0, generation)), Some(&42));
//! ```
//!

pub mod warren;

#[doc(inline)]
pub use warren::{Warren, FlagGuard, GenerationGuard};

#[cfg(test)]
mod tests {

use crate::warren::GenerationGuard;

use super::*;
    use warren::REGION_SIZE;
    use rand::Rng;

    #[test]
    fn test_hive_iter_skips_removed_slots() {
        let mut warren: Warren<u32> = Warren::with_capacity(REGION_SIZE + 2);
        let removed = [1, 3, REGION_SIZE - 1, REGION_SIZE];

        for index in 0..(REGION_SIZE + 2) {
            warren.insert(index as u32);
        }
        for index in removed {
            assert!(warren.remove(index));
        }

        let expected: Vec<_> = (0..(REGION_SIZE + 2))
            .filter(|index| !removed.contains(index))
            .map(|index| (index, index as u32))
            .collect();

        let actual: Vec<_> = warren.iter().map(|(index, value)| (index, *value)).collect();
        assert_eq!(actual, expected);

        for (index, value) in warren.iter_mut() {
            *value += 1;
            assert_eq!(*value, index as u32 + 1);
        }
    }

    #[test]
    fn test_hive_slots_replaced() {
        let mut warren: Warren<u32> = Warren::with_capacity(REGION_SIZE + 2);
        let removed = [1, 3, REGION_SIZE - 1, REGION_SIZE];

        for index in 0..(REGION_SIZE + 2) {
            warren.insert(index as u32);
        }

        for index in removed {
            assert!(warren.remove(index));
        }

        for _index in removed {
            warren.insert(69420);
        }
        
        removed.iter().for_each(|&i| {
            assert_eq!(*warren.get(i).expect("Found index was not reused."), 69420);
        });
    }

    #[test]
    fn test_hive_disjoint_selection() {
        
        let mut warren: Warren<u32> = Warren::with_capacity(REGION_SIZE + 2);
        let removed = [1, 3, REGION_SIZE - 1, REGION_SIZE];

        for index in 0..(REGION_SIZE + 2) {
            warren.insert(index as u32);
        }

        for index in removed {
            assert!(warren.remove(index));
        }

        let selection = [0, 1, 5, 6, REGION_SIZE - 1];
        let actual = warren.get_disjoint_mut(selection);

        let expected = [Some(0u32), None, Some(5), Some(6), None];

        for i in 0..5 {
            assert_eq!(actual[i].is_some(), expected[i].is_some());
            if actual[i].is_none() { continue; }

            let a = actual[i].as_deref().unwrap();
            let b = &expected[i].unwrap();
            assert_eq!(a, b);
        }
    }

    #[test]
    fn test_hive_retain() {
        
        let mut warren: Warren<u32> = Warren::with_capacity(REGION_SIZE + 2);
        let removed: Vec<_> = (0..(REGION_SIZE + 2)).filter(|x| x % 2 == 1).collect();

        for index in 0..(REGION_SIZE + 2) {
            warren.insert(index as u32);
        }
        
        warren.retain(|x| x % 2 == 0);

        let expected: Vec<_> = (0..(REGION_SIZE + 2))
            .filter(|index| !removed.contains(index))
            .map(|index| (index, index as u32))
            .collect();

        let actual: Vec<_> = warren.iter().map(|(index, value)| (index, *value)).collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_hive_retain_mut() {
        
        let mut warren: Warren<u32> = Warren::with_capacity(REGION_SIZE + 2);
        let removed: Vec<_> = (0..(REGION_SIZE + 2)).filter(|x| x % 2 == 1).collect();

        for index in 0..(REGION_SIZE + 2) {
            warren.insert(index as u32);
        }
        
        warren.retain_mut(|x| { 
            let keep = *x % 2 == 0; 
            *x = 0; 
            keep 
        });

        let expected: Vec<_> = (0..(REGION_SIZE + 2))
            .filter(|index| !removed.contains(index))
            .map(|index| (index, 0))
            .collect();

        let actual: Vec<_> = warren.iter().map(|(index, value)| (index, *value)).collect();
        assert_eq!(actual, expected);

        for (_i, x) in warren.iter() {
            assert_eq!(*x, 0);
        }
    }

    #[test]
    fn test_hive_generation_correctly_increments_generation() {
        let mut warren : Warren<u32, GenerationGuard> = Warren::new();

        for i in 1..9 {
            let (index, g) = warren.insert(0);
            assert_eq!(g, i);

            assert_eq!(warren.remove((index, g)), true);
            assert_eq!(warren.remove((index, g - 1)), false);
            assert_eq!(warren.remove((index, g + 1)), false);
        }
    }

    #[test]
    fn test_hive_generation_iter_skips_removed_slots() {
        let mut warren: Warren<u32, GenerationGuard> = Warren::with_capacity(REGION_SIZE + 2);
        let removed = [1, 3, REGION_SIZE - 1, REGION_SIZE];

        for index in 0..(REGION_SIZE + 2) {
            warren.insert(index as u32);
        }
        for index in removed {
            assert!(warren.remove((index, 1)));
        }

        let expected: Vec<_> = (0..(REGION_SIZE + 2))
            .filter(|index| !removed.contains(index))
            .map(|index| ((index, 1), index as u32))
            .collect();

        let actual: Vec<_> = warren.iter().map(|(index, value)| (index, *value)).collect();
        assert_eq!(actual, expected);

        for (index, value) in warren.iter_mut() {
            *value += 1;
            assert_eq!(*value, index.0 as u32 + 1);
        }
    }

    #[test]
    fn test_hive_generation_slots_replaced() {
        let mut warren: Warren<u32, GenerationGuard> = Warren::with_capacity(REGION_SIZE + 2);
        let removed = [1, 3, REGION_SIZE - 1, REGION_SIZE];

        for index in 0..(REGION_SIZE + 2) {
            warren.insert(index as u32);
        }

        for index in removed {
            assert!(warren.remove((index, 1)));
        }

        for _index in removed {
            warren.insert(69420);
        }
        
        removed.iter().for_each(|&i| {
            assert_eq!(*warren.get((i, 2)).expect("Found index was not reused."), 69420);
        });
    }

    #[test]
    fn test_hive_generation_disjoint_selection() {
        
        let mut warren: Warren<u32, GenerationGuard> = Warren::with_capacity(REGION_SIZE + 2);
        let removed = [1, 3, REGION_SIZE - 1, REGION_SIZE];

        for index in 0..(REGION_SIZE + 2) {
            warren.insert(index as u32);
        }

        for index in removed {
            assert!(warren.remove((index, 1)));
        }

        let selection = [(0, 1), (1, 1), (5, 1), (6, 1), (REGION_SIZE - 1, 1)];
        let actual = warren.get_disjoint_mut(selection);

        let expected = [Some(0u32), None, Some(5), Some(6), None];

        for i in 0..5 {
            assert_eq!(actual[i].is_some(), expected[i].is_some());
            if actual[i].is_none() { continue; }

            let a = actual[i].as_deref().unwrap();
            let b = &expected[i].unwrap();
            assert_eq!(a, b);
        }
    }

    #[test]
    fn test_hive_generation_retain() {
        
        let mut warren: Warren<u32, GenerationGuard> = Warren::with_capacity(REGION_SIZE + 2);
        let removed: Vec<_> = (0..(REGION_SIZE + 2)).filter(|x| x % 2 == 1).collect();

        for index in 0..(REGION_SIZE + 2) {
            warren.insert(index as u32);
        }
        
        warren.retain(|x| x % 2 == 0);

        let expected: Vec<_> = (0..(REGION_SIZE + 2))
            .filter(|index| !removed.contains(index))
            .map(|index| ((index, 1), index as u32))
            .collect();

        let actual: Vec<_> = warren.iter().map(|(index, value)| (index, *value)).collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_hive_generation_retain_mut() {
        
        let mut warren: Warren<u32, GenerationGuard> = Warren::with_capacity(REGION_SIZE + 2);
        let removed: Vec<_> = (0..(REGION_SIZE + 2)).filter(|x| x % 2 == 1).collect();

        for index in 0..(REGION_SIZE + 2) {
            warren.insert(index as u32);
        }
        
        warren.retain_mut(|x| { 
            let keep = *x % 2 == 0; 
            *x = 0; 
            keep 
        });

        let expected: Vec<_> = (0..(REGION_SIZE + 2))
            .filter(|index| !removed.contains(index))
            .map(|index| ((index, 1), 0))
            .collect();

        let actual: Vec<_> = warren.iter().map(|(index, value)| (index, *value)).collect();
        assert_eq!(actual, expected);

        for (_i, x) in warren.iter() {
            assert_eq!(*x, 0);
        }
    }

    #[test]
    fn test_hive_correctness_u64() {
        const CEILING: usize = 1000000;
        const REMOVAL: usize = 500000;

        let mut warren: Warren<u64> = Warren::with_capacity(CEILING);
        let mut rng = rand::rng();

        let mut timestamps = Vec::with_capacity(16);

        // Segment 0: Test insertion.
        timestamps.push(std::time::Instant::now());

        for i in 0..CEILING {
            warren.insert(i as u64);
        }
        
        assert_eq!(warren.size(), CEILING, "Did not register the correct number of elements in the hive.");
        
        println!(".");
        timestamps.push(std::time::Instant::now());
        
        // Section 1: Test iteration reaches every slot.

        let mut iter_count = 0;
        for item in warren.iter() {
            assert_eq!(*item.1, iter_count as u64);
            iter_count += 1;
        }

        let capacity1 = warren.capacity();
        assert_eq!(warren.size(), CEILING, "Did not register the correct number of elements in the hive.");
        assert_eq!(iter_count, CEILING, "Did not iterate through the same number of elements which had been inserted.");
        assert!(capacity1 >= CEILING);

        println!(".");
        timestamps.push(std::time::Instant::now());

        // Section 2: Test removal of elements

        let mut num_removed_1 = 0;
        for _ in 0..REMOVAL {
            let i = rng.next_u32() as usize % CEILING;
            num_removed_1 += warren.remove(i) as usize;
        }

        assert!(num_removed_1 <= REMOVAL, "Duplicate removal?");
        assert_eq!(warren.capacity(), capacity1, "Capacity unexpectedly changed during removals.");
        assert_eq!(warren.size(), CEILING - num_removed_1, "Expected size of {}. Got {}", CEILING - num_removed_1, warren.size());

        println!(".");
        timestamps.push(std::time::Instant::now());

        // Section 3: Test that all slots can be accessed by Hive::get.

        let mut num_removed = 0;
        for i in 0..CEILING {
            let opt = warren.get(i);
            if let Some(val) = opt {
                assert_eq!(*val as usize, i, "Expected {} at index {}. Got {} instead.", i, i, *val);
            }
            else {
                num_removed += 1;
            }
        }

        assert_eq!(num_removed_1, num_removed,
            "Expected {} omissions while iterating. Got {} omissions.", num_removed_1, num_removed);

        println!(".");
        timestamps.push(std::time::Instant::now());

        // Section 4: Test iteration reaches every element after removing elements.

        let mut iter_count1 = 0;
        for _item in warren.iter() {
            iter_count1 += 1;
        }
        
        assert_eq!(iter_count1, warren.size(), 
            "Did not iterate through the expected number of elements. There are {} elements. We iterated {} times.",
            warren.size(), iter_count1);

        println!(".");
        timestamps.push(std::time::Instant::now());

        // Section 5: Test space reuse.

        for _i in 0..num_removed_1 {
            let value = 69420;
            warren.insert(value);
        }
        
        assert!(warren.capacity() == capacity1, "Element does not reuse spaces correctly.");
        assert_eq!(warren.size(), CEILING, "Element count is incorrect.");

        println!(".");
        timestamps.push(std::time::Instant::now());

        // Section 6: Test removal of all elements.

        for i in 0..CEILING {
            warren.remove(i);
        }

        assert!(warren.capacity() == capacity1, "Capacity unexpectedly changed during removals.");
        assert_eq!(warren.size(), 0, "Element removal not registered.");
        
        timestamps.push(std::time::Instant::now());

        // Section 7: Test reuse of element indices.

        for i in 0..CEILING {
            warren.insert(i as u64);
        }

        assert!(warren.capacity() == capacity1, "Capacity unexpectedly changed during removals.");
        assert_eq!(warren.size(), CEILING, "Element size incorrect. Expected {}.", CEILING);

        println!(".");
        timestamps.push(std::time::Instant::now());

        // Section 8: Test mutable iterator.
        
        let mut iter_count2 = 0;
        for item in warren.iter_mut() {
            *item.1 *= 2;
            
            iter_count2 += 1;
        }
        
        assert_eq!(iter_count2, warren.size(), "Did not iterate through the expected number of elements.");

        println!(".");
        timestamps.push(std::time::Instant::now());
        
        // Section 9: Verifying mutable iterator (must be analyzed manually).
        
        let mut iter_count3 = 0;
        for _item in warren.iter() {
            iter_count3 += 1;
        }
        
        assert_eq!(iter_count3, warren.size(), "Did not iterate through the expected number of elements.");

        println!(".");
        timestamps.push(std::time::Instant::now());

        // Section 10

        let mut v = Vec::new();
        for i in 0..CEILING {
            v.push(i);
        }

        timestamps.push(std::time::Instant::now());

        // Section 11

        for x in v.iter_mut() {
            *x *= 2;
        }

        timestamps.push(std::time::Instant::now());

        // Check times.

        for i in 0..(timestamps.len() - 1) {
            let diff = timestamps[i+1].checked_duration_since(timestamps[i]).unwrap().as_micros();
            println!("Segment {i}: {diff} μs lapsed.");
        }

        let diff = timestamps[timestamps.len() - 1].checked_duration_since(timestamps[0]).unwrap().as_micros();
            println!("Total: {diff} μs lapsed.");
    }

}

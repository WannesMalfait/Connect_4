use std::sync::atomic::{AtomicU16, AtomicU32, Ordering};

use crate::position::Position;

/// The following are functions to find the next prime factor at compile time

const fn med(min: u64, max: u64) -> u64 {
    (min + max) / 2
}

/// Checks if `n` has a a divisor between `min` (inclusive) and `max` (exclusive)
const fn has_factor(n: u64, min: u64, max: u64) -> bool {
    if min * min > n {
        false
    }
    // do not search for factor above sqrt(n)
    else if min + 1 >= max {
        n % min == 0
    } else {
        has_factor(n, min, med(min, max)) || has_factor(n, med(min, max), max)
    }
}

/// Return the next prime number greater or equal to `n`.
/// `n` must be >= 2
const fn next_prime(n: u64) -> u64 {
    if has_factor(n, 2, n) {
        next_prime(n + 1)
    } else {
        n
    }
}

type KeyType = u64;
type PartialKeyType = u32;
type AtomicPartialKeyType = AtomicU32;
type AtomicValueType = AtomicU16;

#[derive(Debug, PartialEq, Eq)]
pub struct PosInfo {
    /// The score in this position (either a lower bound or an upper bound)
    score: u8,
    /// The column from which we got this bound.
    column: u8,
}

impl PosInfo {
    fn new(score: u8, column: u8) -> Self {
        Self { score, column }
    }

    fn zero() -> Self {
        Self {
            score: 0,
            column: 0,
        }
    }

    pub fn score(&self) -> isize {
        self.score as isize
    }

    pub fn column(&self) -> u8 {
        self.column
    }
}

/// Transposition Table is a simple hash map with fixed storage size.
/// In case of collision we keep the last entry and overide the previous one.
/// We keep only part of the key to reduce storage, but no error is possible due
/// to the Chinese Remainder theorem.
///
/// The number of stored entries is the next prime after a power of two that is defined
/// at compile time. We also define size of the entries and keys to allow optimization at
/// compile time.
///
/// The Transposition table is also thread safe, due to a simple trick with xor. Instead of
/// storing the key directly, we xor it with the data. When we then call `get(key)` we compare
/// the key stored to the result of xor-ing the key with the value stored. If the value had been
/// written by another thread, these will be different, and we know that we shouldn't return this
/// data. See [this page](https://www.chessprogramming.org/Shared_Hash_Table#Xor) for more info.
pub struct TranspositionTable {
    keys: Box<[AtomicPartialKeyType]>,
    values: Box<[AtomicValueType]>,
}
impl TranspositionTable {
    /// Base 2 log of the size of the Transposition Table.
    const LOG_SIZE: usize = 24;
    const SIZE: u64 = next_prime(1 << Self::LOG_SIZE);
}

impl Default for TranspositionTable {
    fn default() -> Self {
        Self::new()
    }
}

impl TranspositionTable {
    /// Create a new `TranspositionTable` with no stored entries.
    /// ```
    /// use connect_4::transposition_table::TranspositionTable;
    /// use connect_4::transposition_table::PosInfo;
    /// let mut table = TranspositionTable::new();
    /// assert_eq!(table.get(5), None);
    /// table.put(5, 2, 0);
    /// assert_eq!(table.get(5).unwrap().score() , 2);
    /// ```
    pub fn new() -> Self {
        println!("Initialized transposition table with size: {}", Self::SIZE);
        // Initialize with `Self::SIZE + 1` to guarantee that we will always see
        // uninitialized entries as uninitialized. Using `Option<PartialKeyType>`
        // was too slow.
        Self {
            keys: (0..Self::SIZE)
                .map(|_| AtomicPartialKeyType::new(Self::SIZE as PartialKeyType + 1))
                .collect(),
            values: (0..Self::SIZE)
                .map(|_| AtomicValueType::new(unsafe { std::mem::transmute(PosInfo::zero()) }))
                .collect(),
        }
    }
    /// Get rid of all stored entries.
    pub fn reset(&self) {
        for i in 0..Self::SIZE {
            // Initialize with `Self::SIZE + 1` to guarantee that we will always see
            // uninitialized entries as uninitialized.
            self.keys[i as usize].store((Self::SIZE + 1) as PartialKeyType, Ordering::Relaxed);
            self.values[i as usize].store(
                unsafe { std::mem::transmute(PosInfo::zero()) },
                Ordering::Relaxed,
            );
        }
    }
    /// Get the associated value of the given `key`. If no entry was found
    /// it returns `None`, otherwise it returns `Some(value)`.
    #[must_use]
    pub fn get(&self, key: KeyType) -> Option<PosInfo> {
        let index = Self::index(key);
        let r_key;
        let value;
        unsafe {
            r_key = self.keys.get_unchecked(index).load(Ordering::Relaxed);
            value = self.values.get_unchecked(index).load(Ordering::Relaxed);
        }
        // We need to use the xor trick to ensure that key and value were set by the same thread.
        if r_key == key as PartialKeyType ^ value as PartialKeyType {
            Some(unsafe { std::mem::transmute(value) })
        } else {
            None
        }
    }
    /// Store a key value pair in the table. Previous entries are overwritten on collision.
    pub fn put(&self, key: KeyType, score: u8, column: u8) {
        let index = Self::index(key);
        let value = unsafe { std::mem::transmute(PosInfo::new(score, column)) };
        // Importantly, we xor with the value. This allows us to verify that we got the correct
        // entry upon retrieval, by xor-ing again.
        unsafe {
            self.keys.get_unchecked(index).store(
                key as PartialKeyType ^ value as PartialKeyType,
                Ordering::Relaxed,
            );
            self.values
                .get_unchecked(index)
                .store(value, Ordering::Relaxed);
        }
    }

    /// Same as put, but we first query the hashtable to see if this is actually a better bound.
    pub fn put_checked(&self, key: KeyType, score: u8, column: u8, is_upper_bound: bool) {
        if let Some(pos_info) = self.get(key) {
            // Check if the new score is a better bound.
            let val = pos_info.score();
            if val > Position::MAX_SCORE - Position::MIN_SCORE + 1 {
                // Lower bound was stored.
                if ! is_upper_bound && val >= score as isize{
                    // The lower bound stored was better.
                    return;
                }
            } else if val <= score as isize {
                // The upper bound stored was better.
                return;
            }
        }
        self.put(key, score, column)
    }

    /// Get the index for the given `key`.
    fn index(key: KeyType) -> usize {
        (key % Self::SIZE) as usize
    }
}

#[cfg(test)]
mod tests {
    use position::{Column, Position};
    use std::sync::Arc;

    use crate::{
        position,
        transposition_table::{KeyType, PosInfo},
    };

    use super::TranspositionTable;
    #[test]
    fn inserts_and_gets() {
        let tb = TranspositionTable::new();
        let mut pos = position::Position::new();
        assert_eq!(tb.get(pos.key()), None);
        for j in 0..20 {
            pos.play_col(j * 5 % Position::WIDTH);
            let key = pos.key();
            assert_eq!(tb.get(key), None);
            Position::display_bitboard(pos.key());
            for i in 0..Position::WIDTH {
                let bmove = pos.possible_non_losing_moves() & Position::column_mask(i);
                let score = pos.move_score(bmove) as Column;
                tb.put(key, score, i);
                assert_eq!(tb.get(key), Some(PosInfo::new(score, i)));
            }
        }
    }

    #[test]
    fn uninitialized() {
        let tb = TranspositionTable::new();
        for p in [3, 5, 11, 37, 53, 137] {
            let mut pos = position::Position::new();
            assert_eq!(tb.get(pos.key()), None);
            for k in 0..100 {
                let col = (k * p) as u8 % Position::WIDTH;
                if !pos.can_play(col) {
                    continue;
                }
                pos.play_col(col);
                // Should register as uninitialized.
                assert_eq!(tb.get(pos.key()), None);
            }
        }
    }

    #[test]
    fn threaded() {
        let table = Arc::new(TranspositionTable::new());
        let table1 = table.clone();
        let table2 = table.clone();
        const NUM_TRIES: usize = 1000;
        // Both i and TranspositionTable::SIZE + i will have the same index
        // into the transposition table. If two threads store at the same time
        // and one ends up storing the key, while the other stores the value,
        // then the table should return that as if there was nothing stored.
        let join_handle1 = std::thread::spawn(move || {
            for i in 0..NUM_TRIES {
                table1.put(i as KeyType, 1, 0);
                std::thread::sleep(std::time::Duration::from_micros(500));
            }
        });
        let join_handle2 = std::thread::spawn(move || {
            for i in 0..NUM_TRIES {
                table2.put(TranspositionTable::SIZE + i as KeyType, 2, 0);
                std::thread::sleep(std::time::Duration::from_micros(500));
            }
        });
        join_handle1.join().unwrap();
        join_handle2.join().unwrap();
        let mut values = [0; NUM_TRIES];
        for (i, value) in values.iter_mut().enumerate() {
            match table.get(i as KeyType) {
                Some(v) => {
                    assert_eq!(v, PosInfo::new(1, 0));
                    *value = 1;
                }
                None => {
                    if let Some(v) = table.get(TranspositionTable::SIZE + i as KeyType) {
                        assert_eq!(v, PosInfo::new(2, 0));
                        *value = 2;
                    }
                }
            }
        }
        // 0: collision (key and value come from different threads),
        // 1: value stored by thread 1
        // 2: value stored by thread 2
        println!("{values:?}");
    }
}

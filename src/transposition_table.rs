use crate::position;

/// The following are functions to dind the next prime factor at compile time

const fn med(min: u64, max: u64) -> u64 {
    (min + max) / 2
}

/// Checks if `n` has a a divisor between `min` (inclusive) and `max` (exclusive)
const fn has_factor(n: u64, min: u64, max: u64) -> bool {
    if min * min > n {
        false
    }
    // do not search for factor above sqrt(n)
    else {
        if min + 1 >= max {
            n % min == 0
        } else {
            has_factor(n, min, med(min, max)) || has_factor(n, med(min, max), max)
        }
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
type ValueType = position::Column;

/// Transposition Table is a simple hash map with fixed storage size.
/// In case of collision we keep the last entry and overide the previous one.
/// We keep only part of the key to reduce storage, but no error is possible due
/// to the Chinese Remainder theorem.
///
/// The number of stored entries is a power of two that is defined at compile time.
/// We also define size of the entries and keys to allow optimization at compile time.
pub struct TranspositionTable {
    keys: Vec<Option<PartialKeyType>>,
    values: Vec<ValueType>,
}
impl TranspositionTable {
    /// Base 2 log of the size of the Transposition Table.
    const LOG_SIZE: usize = 23;
    const SIZE: u64 = next_prime(1 << Self::LOG_SIZE);
}
impl TranspositionTable {
    /// Create a new `TranspositionTable` with no stored entries.
    /// ```
    /// use connect_4::transposition_table::TranspositionTable;
    /// let mut table = TranspositionTable::new();
    /// assert_eq!(table.get(5), None);
    /// table.put(5, 2);
    /// assert_eq!(table.get(5), Some(2));
    /// ```
    pub fn new() -> Self {
        println!("Initialized transposition table with size: {}", Self::SIZE);
        // If no entries are in the table and we call it from the starting position
        // then the key is zero, which would make it seem like there is a value stored
        // there even though there isn't.
        Self {
            keys: vec![None; Self::SIZE as usize],
            values: vec![0; Self::SIZE as usize],
        }
    }
    /// Get rid of all stored entries.
    pub fn reset(&mut self) {
        for i in 0..Self::SIZE {
            self.keys[i as usize] = None;
            self.values[i as usize] = 0;
        }
    }
    /// Get the associated value of the given `key`. If no entry was found
    /// it returns `None`, otherwise it returns `Some(value)`.
    pub fn get(&self, key: KeyType) -> Option<ValueType> {
        let pos = Self::index(key);
        let r_key = self.keys[pos]?;
        if r_key == key as PartialKeyType {
            Some(self.values[pos])
        } else {
            None
        }
    }
    /// Store a key value pair in the table. Previous entries are overwritten on collision.
    pub fn put(&mut self, key: KeyType, value: ValueType) {
        let pos = Self::index(key);
        self.keys[pos] = Some(key as PartialKeyType); // key is possibly truncated as key_t is possibly less than key_size bits.
        self.values[pos] = value;
    }
    /// Get the index for the given `key`.
    fn index(key: KeyType) -> usize {
        (key % Self::SIZE) as usize
    }
}

#[cfg(test)]
mod tests {
    use position::{Column, Position};

    use crate::position;

    use super::TranspositionTable;
    #[test]
    fn inserts_and_gets() {
        let mut tb = TranspositionTable::new();
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
                tb.put(key, score);
                assert_eq!(tb.get(key), Some(score));
            }
        }
    }
}

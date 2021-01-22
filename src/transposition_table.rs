use crate::position;

/**
 * util functions to compute next prime at compile time
 */
const fn med(min: u64, max: u64) -> u64 {
    (min + max) / 2
}
/**
 * tells if an integer n has a a divisor between min (inclusive) and max (exclusive)
 */
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

// return next prime number greater or equal to n.
// n must be >= 2
const fn next_prime(n: u64) -> u64 {
    if has_factor(n, 2, n) {
        next_prime(n + 1)
    } else {
        n
    }
}

// // log2(1) = 0; log2(2) = 1; log2(3) = 1; log2(4) = 2; log2(8) = 3
// const fn log2(n: usize) -> usize {
//     if n <= 1 {
//         0
//     } else {
//         log2(n / 2) + 1
//     }
// }

type KeyType = u64;
type PartialKeyType = u32;
type ValueType = position::Column;

/**
 * Transposition Table is a simple hash map with fixed storage size.
 * In case of collision we keep the last entry and overide the previous one.
 * We keep only part of the key to reduce storage, but no error is possible thanks to Chinese theorem.
 *
 * The number of stored entries is a power of two that is defined at compile time.
 * We also define size of the entries and keys to allow optimization at compile time.
 *
 * key_size:   number of bits of the key
 * value_size: number of bits of the value
 * log_size:   base 2 log of the size of the Transposition Table.
 *             The table will contain 2^log_size elements
 */

pub struct TranspositionTable {
    keys: Vec<PartialKeyType>,
    values: Vec<ValueType>,
}
impl TranspositionTable {
    const LOG_SIZE: usize = 27;
    const SIZE: u64 = next_prime(1 << Self::LOG_SIZE);
}
impl TranspositionTable {
    pub fn new() -> Self {
        println!("Initialized transposition table with size: {}", Self::SIZE);
        let mut temp = vec![0; Self::SIZE as usize];
        // If no entries are in the table and we call it from the starting position
        // then the key is zero, which would make it seem like there is a value stored
        // there even though there isn't.
        temp[0] = 1;
        Self {
            keys: temp,
            values: vec![0; Self::SIZE as usize],
        }
    }
    pub fn reset(&mut self) {
        for i in 0..Self::SIZE {
            self.keys[i as usize] = 0;
            self.values[i as usize] = 0;
        }
    }

    pub fn get(&self, key: KeyType) -> Option<ValueType> {
        let pos = Self::index(key);
        if self.keys[pos] == (key as PartialKeyType) {
            Some(self.values[pos])
        } else {
            None
        }
    }
    pub fn put(&mut self, key: KeyType, value: ValueType) {
        let pos = Self::index(key);
        self.keys[pos] = key as PartialKeyType; // key is possibly truncated as key_t is possibly less than key_size bits.
        self.values[pos] = value;
    }
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

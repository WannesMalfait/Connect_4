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

impl Default for TranspositionTable {
    fn default() -> Self {
        Self::new()
    }
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
    #[must_use]
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
    #[must_use]
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

#[derive(Clone, Copy)]
struct BookEntry {
    /// The key of the position
    pos: KeyType,
    /// A move with the best possible score
    bmove: position::Bitboard,
    /// The best possible score in the position
    score: isize,
}

/// An `OpeningBook` is a way to store the best moves in common positions
/// in the opening, which might take a long time to solve. For each position
/// in the opening book we store the best move and the score associated with
/// this move.
///
/// **Warning**: Only one entry is stored per position.
pub struct OpeningBook {
    entries: Vec<BookEntry>,
}

impl From<Vec<BookEntry>> for OpeningBook {
    fn from(vec: Vec<BookEntry>) -> Self {
        let mut book = OpeningBook { entries: vec };
        if book.is_valid() {
            return book;
        }
        book.entries.sort_unstable_by_key(|entry| entry.pos);
        book.entries.dedup_by_key(|entry| entry.pos);
        assert!(book.is_valid());
        book
    }
}

/// A struct which helps to iterate over the possible book moves in a given position.
pub struct BookMoves<'a> {
    book: &'a OpeningBook,
    pos: position::Position,
    next_col: position::Column,
}

impl Iterator for BookMoves<'_> {
    type Item = position::Column;

    fn next(&mut self) -> Option<Self::Item> {
        for col in self.next_col..(position::Position::WIDTH) {
            if !self.pos.can_play(col) {
                continue;
            }
            let mut next_pos = self.pos.clone();
            next_pos.play_col(col);
            let key = next_pos.key();
            if self.book.get(key).is_some() {
                self.next_col = col + 1;
                return Some(col);
            }
        }
        self.next_col = position::Position::WIDTH;
        None
    }
}

impl Default for OpeningBook {
    fn default() -> Self {
        Self::new()
    }
}

impl OpeningBook {
    #[must_use]
    pub fn new() -> Self {
        OpeningBook {
            entries: Vec::new(),
        }
    }

    fn is_valid(&self) -> bool {
        // `is_sorted()` is unstable, so create our own version.
        let mut prev = match self.entries.first() {
            Some(e) => e.pos,
            None => return true,
        };
        return self
            .entries
            .iter()
            .skip(1)
            .map(|entry| entry.pos)
            .all(move |curr| {
                if prev >= curr {
                    // A position can appear at most once!
                    return false;
                };
                prev = curr;
                true
            });
    }

    /// Load an opening book from a file. If errors occured while
    /// loading or parsing the file an `Err` is returned.
    pub fn load_book(path: String) -> std::io::Result<Self> {
        todo!("Read file in path: {} and store keys and values", path);
    }

    /// Get the associated value of the given `key`. If no entry was found
    /// it returns `None`, otherwise it returns `Some(bmove, score)`.
    #[must_use]
    pub fn get(&self, key: KeyType) -> Option<(position::Bitboard, isize)> {
        if let Ok(pos) = self.entries.binary_search_by_key(&key, |entry| entry.pos) {
            let entry = self.entries[pos];
            Some((entry.bmove, entry.score))
        } else {
            None
        }
    }

    /// Insert an entry in the book for the given key of the position.
    /// If the position is already in the book, it is overwritten.
    pub fn put(&mut self, key: KeyType, bmove: position::Bitboard, score: isize) {
        let entry = BookEntry {
            pos: key,
            bmove,
            score,
        };
        match self.entries.binary_search_by_key(&key, |entry| entry.pos) {
            Ok(index) => {
                // We already have an entry, so just overwrite it.
                self.entries[index] = entry;
            }
            Err(index) => self.entries.insert(index, entry),
        }
    }

    /// Insert an entry in the book for the given position.
    /// If the position is already in the book, it is overwritten.
    #[inline]
    pub fn put_pos(&mut self, pos: &position::Position, bmove: position::Bitboard, score: isize) {
        self.put(pos.key(), bmove, score);
    }

    /// Get the playable moves from this position that are in the book.
    /// The moves are sorted by collumn.
    #[must_use]
    pub fn book_moves_from_position(&self, pos: position::Position) -> BookMoves {
        BookMoves {
            book: self,
            pos,
            next_col: 0,
        }
    }
}
#[cfg(test)]
mod tests {
    use position::{Column, Position};

    use crate::position;

    use super::BookEntry;
    use super::OpeningBook;
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

    #[test]
    fn adding_book_entries() {
        let mut book = OpeningBook::new();
        let mut pos = position::Position::new();
        assert_eq!(book.get(pos.key()), None);
        for j in 0..20 {
            pos.play_col(j * 5 % Position::WIDTH);
            let key = pos.key();
            assert_eq!(book.get(key), None);
            for i in 0..Position::WIDTH {
                println!("Inserting {key}");
                let bmove = pos.possible_non_losing_moves() & Position::column_mask(i);
                let score = pos.move_score(bmove) as Column;
                // Just for testing we put in dummy score and best move.
                book.put(key, bmove, score.into());
                assert_eq!(book.get(key), Some((bmove, isize::from(score))));
            }
        }
        assert!(book.is_valid());
    }
    #[test]
    fn adding_book_entries_at_once() {
        let mut pos = position::Position::new();
        let mut entries = Vec::new();
        for j in 0..20 {
            pos.play_col(j * 5 % Position::WIDTH);
            let key = pos.key();
            // Opening book can only store one entry per position, so only one of these will get added.
            // Since the sorting is unstable, there is no guarantee about which entry is added.
            for i in 0..Position::WIDTH {
                println!("Inserting {key}");
                let bmove = pos.possible_non_losing_moves() & Position::column_mask(i);
                let score = pos.move_score(bmove) as Column;
                // Just for testing we put in dummy score and best move.
                entries.push(BookEntry {
                    pos: key,
                    bmove,
                    score: score.into(),
                });
            }
        }
        let book = OpeningBook::from(entries);
        assert!(book.is_valid());
    }

    #[test]
    fn book_moves() {
        let mut pos = position::Position::new();
        let mut book = OpeningBook::new();
        book.put_pos(&pos, 0u64, 0);
        let mut moves = book.book_moves_from_position(pos.clone());
        // No book moves in starting position.
        assert_eq!(moves.next(), None);
        // Add 2 moves in the book.
        pos.play_col(0);
        book.put_pos(&pos, 0u64, 0);
        pos = position::Position::new();
        pos.play_col(2);
        book.put_pos(&pos, 0u64, 0);
        pos = position::Position::new();
        // Look at moves from the starting position
        let mut moves = book.book_moves_from_position(pos);
        assert_eq!(moves.next(), Some(0));
        assert_eq!(moves.next(), Some(2));
        assert_eq!(moves.next(), None);
    }
}

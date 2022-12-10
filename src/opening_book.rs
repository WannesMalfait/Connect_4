use crate::position::{Column, Position};
use std::{
    error::Error,
    fs::File,
    io::{BufRead, BufReader, BufWriter, Write},
    path::Path,
};

#[derive(Clone, Copy)]
struct BookEntry {
    /// The key of the position
    pos: u64,
    /// The best possible score in the position
    score: isize,
}
#[derive(Debug)]
enum ParseBookEntryError {
    /// The number of values in the string is not 3
    NumValues,
    /// The position key stored was not valid
    Pos,
    /// The score stored was not valid
    Score,
}

impl std::fmt::Display for ParseBookEntryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NumValues => write!(f, "Expected 2 values in the entry"),
            Self::Pos => write!(f, "Could not parse first value into a valid position"),
            Self::Score => write!(f, "Could not parse third value into a valid score"),
        }
    }
}

impl Error for ParseBookEntryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

impl From<ParseBookEntryError> for std::io::Error {
    fn from(err: ParseBookEntryError) -> Self {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, err)
    }
}

impl BookEntry {
    pub fn from_string(str: &str) -> Result<Self, ParseBookEntryError> {
        let v: Vec<&str> = str.split(' ').collect();
        if v.len() != 2 {
            return Err(ParseBookEntryError::NumValues);
        }
        let pos = match v[0].parse::<u64>() {
            Ok(p) => p,
            Err(_) => return Err(ParseBookEntryError::Pos),
        };
        let score = match v[1].parse::<isize>() {
            Ok(s) => s,
            Err(_) => return Err(ParseBookEntryError::Score),
        };
        Ok(Self { pos, score })
    }
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
    pos: Position,
    next_col: Column,
}

impl Iterator for BookMoves<'_> {
    type Item = Column;

    fn next(&mut self) -> Option<Self::Item> {
        for col in self.next_col..(Position::WIDTH) {
            if !self.pos.can_play(col) {
                continue;
            }
            let mut next_pos = self.pos.clone();
            next_pos.play_col(col);
            if self.book.get(&next_pos).is_some() {
                self.next_col = col + 1;
                return Some(col);
            }
        }
        self.next_col = Position::WIDTH;
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

    #[must_use]
    pub fn num_entries(&self) -> usize {
        self.entries.len()
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
    pub fn load(path: &Path) -> Result<Self, std::io::Error> {
        let file = File::open(path)?;
        let file = BufReader::new(file);
        let mut entries = Vec::new();
        for line in file.lines() {
            let line = line?;
            let entry = BookEntry::from_string(&line)?;
            entries.push(entry);
        }
        Ok(Self::from(entries))
    }

    pub fn store(&self, path: &Path) -> Result<(), std::io::Error> {
        let file = File::options().write(true).create(true).open(path)?;
        file.set_len(0)?;
        let mut file = BufWriter::new(file);
        for entry in &self.entries {
            writeln!(&mut file, "{} {}", entry.pos, entry.score)?;
        }
        file.flush()?;
        Ok(())
    }

    /// Get the associated value of the given position. If no entry was found
    /// it returns `None`, otherwise it returns `Some(score)`.
    #[must_use]
    pub fn get(&self, pos: &Position) -> Option<isize> {
        self.get_by_key(pos.key3())
    }

    /// Get the associated value of the given `key`. If no entry was found
    /// it returns `None`, otherwise it returns `Some(score)`.
    ///
    /// WARNING: the key should be the symmetric base 3 key of the position.
    #[must_use]
    fn get_by_key(&self, key: u64) -> Option<isize> {
        if let Ok(pos) = self.entries.binary_search_by_key(&key, |entry| entry.pos) {
            let entry = self.entries[pos];
            Some(entry.score)
        } else {
            None
        }
    }

    /// Insert an entry in the book for the given key of the position.
    /// If the position is already in the book, it is overwritten.
    ///
    /// WARNING: the key should be the symmetric base 3 key of the position.
    fn put_by_key(&mut self, key: u64, score: isize) {
        let entry = BookEntry { pos: key, score };
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
    pub fn put(&mut self, pos: &Position, score: isize) {
        self.put_by_key(pos.key3(), score);
    }

    /// Get the playable moves from this position that are in the book.
    /// The moves are sorted by column.
    #[must_use]
    pub fn book_moves_from_position(&self, pos: Position) -> BookMoves {
        BookMoves {
            book: self,
            pos,
            next_col: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::position::{Column, Position};

    use super::BookEntry;
    use super::OpeningBook;
    #[test]
    fn adding_book_entries() {
        let mut book = OpeningBook::new();
        let mut pos = Position::new();
        assert_eq!(book.get(&pos), None);
        for j in 0..20 {
            pos.play_col(j * 5 % Position::WIDTH);
            assert_eq!(book.get(&pos), None);
            for i in 0..Position::WIDTH {
                let bmove = pos.possible_non_losing_moves() & Position::column_mask(i);
                let score = pos.move_score(bmove) as Column;
                // Just for testing we put in dummy score and best move.
                book.put(&pos, score.into());
                assert_eq!(book.get(&pos), Some(isize::from(score)));
            }
        }
        assert!(book.is_valid());
    }
    #[test]
    fn adding_book_entries_at_once() {
        let mut pos = Position::new();
        let mut entries = Vec::new();
        for j in 0..20 {
            pos.play_col(j * 5 % Position::WIDTH);
            // Opening book can only store one entry per position, so only one of these will get added.
            // Since the sorting is unstable, there is no guarantee about which entry is added.
            for i in 0..Position::WIDTH {
                let bmove = pos.possible_non_losing_moves() & Position::column_mask(i);
                let score = pos.move_score(bmove) as Column;
                // Just for testing we put in dummy score and best move.
                entries.push(BookEntry {
                    pos: pos.key3(),
                    score: score.into(),
                });
            }
        }
        let book = OpeningBook::from(entries);
        assert!(book.is_valid());
    }

    #[test]
    fn book_moves() {
        let mut pos = Position::new();
        let mut book = OpeningBook::new();
        book.put(&pos, 0);
        let mut moves = book.book_moves_from_position(pos.clone());
        // No book moves in starting position.
        assert_eq!(moves.next(), None);
        // Add 2 moves in the book.
        pos.play_col(0);
        book.put(&pos, 0);
        pos = Position::new();
        pos.play_col(2);
        book.put(&pos, 0);
        pos = Position::new();
        // Look at moves from the starting position
        let mut moves = book.book_moves_from_position(pos);
        assert_eq!(moves.next(), Some(0));
        assert_eq!(moves.next(), Some(2));
        // Key is symmetric!
        assert_eq!(moves.next(), Some(4));
        assert_eq!(moves.next(), Some(6));
        assert_eq!(moves.next(), None);
    }

    #[test]
    fn store_load_book() {
        let mut pos = Position::new();
        let mut book = OpeningBook::new();
        book.put(&pos, 0);
        let mut moves = book.book_moves_from_position(pos.clone());
        // No book moves in starting position.
        assert_eq!(moves.next(), None);
        // Add 2 moves in the book.
        pos.play_col(0);
        println!("Played col 0");
        pos.display_position();
        book.put(&pos, 0);
        pos = Position::new();
        pos.play_col(2);
        println!("Played col 2");
        pos.display_position();
        book.put(&pos, 0);
        pos = Position::new();
        pos.play_col(3);
        println!("Played col 3");
        pos.display_position();
        book.put(&pos, 0);

        let book_path = std::path::Path::new("test_book.book");
        book.store(book_path).unwrap();
        let book = OpeningBook::load(book_path).unwrap();

        // Get rid of the test book again.
        std::fs::remove_file(book_path).unwrap();

        pos = Position::new();
        // Look at moves from the starting position
        let mut moves = book.book_moves_from_position(pos);
        assert_eq!(moves.next(), Some(0));
        assert_eq!(moves.next(), Some(2));
        assert_eq!(moves.next(), Some(3));
        // Key is symmetric!
        assert_eq!(moves.next(), Some(4));
        assert_eq!(moves.next(), Some(6));
        assert_eq!(moves.next(), None);
    }
}

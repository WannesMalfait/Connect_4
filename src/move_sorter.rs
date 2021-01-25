use crate::position;
use position::Position;

#[derive(Clone, Copy)]
struct Inner {
    score: u8,
    bmove: position::Bitboard,
}
impl Inner {
    fn new() -> Self {
        Inner { score: 0, bmove: 0 }
    }
}
/// This struct helps sorting the next moves
///
/// You have to add moves first with their score
/// then you can get them back in decreasing score
///
/// This class implement an insertion sort that is in practice very
/// efficient for small number of move to sort (max is `Position::WIDTH`)
/// and also efficient if the move are pushed in approximatively increasing
/// order which can be acheived by using a simpler column ordering heuristic.
pub struct MoveSorter {
    size: usize,
    moves: [Inner; Position::WIDTH as usize],
}
impl MoveSorter {
    /// Create a new sorter.
    pub fn new() -> Self {
        MoveSorter {
            size: 0,
            moves: [Inner::new(); Position::WIDTH as usize],
        }
    }
    /// Add a move in the container with its score.
    /// You cannot add more than `Position::WIDTH` moves
    pub fn add(&mut self, bmove: position::Bitboard, score: u8) {
        let mut pos = self.size;
        let new = Inner { score, bmove };
        // Shift elements to the right until we are in the right place.
        while pos != 0 && self.moves[pos - 1].score > score {
            self.moves[pos] = self.moves[pos - 1];
            pos = pos - 1;
        }
        self.moves[pos] = new;
        self.size += 1;
    }

    /**
     * reset (empty) the container
     */
    pub fn reset(&mut self) {
        self.size = 0;
    }
}

impl Iterator for MoveSorter {
    type Item = position::Bitboard;
    /// Get the next move and remove it from the collection. Moves are ordered by decreasing scores.
    /// If there are no more moves, return `None`.
    fn next(&mut self) -> Option<position::Bitboard> {
        match self.size {
            0 => None,
            n => {
                self.size -= 1;
                Some(self.moves[n - 1].bmove)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use position::Position;

    use crate::position;

    use super::MoveSorter;

    #[test]
    fn correct_insertion_sort() {
        let mut ms = MoveSorter::new();
        for i in 0..Position::WIDTH {
            ms.add(i as u64, Position::WIDTH - i + 4);
        }
        for (i, bmove) in ms.into_iter().enumerate() {
            assert_eq!(bmove, i as position::Bitboard);
        }
    }
}

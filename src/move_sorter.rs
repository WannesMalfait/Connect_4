use crate::position;
use position::Position;

/**
 * This class helps sorting the next moves
 *
 * You have to add moves first with their score
 * then you can get them back in decreasing score
 *
 * This class implement an insertion sort that is in practice very
 * efficient for small number of move to sort (max is Position::WIDTH)
 * and also efficient if the move are pushed in approximatively increasing
 * order which can be acheived by using a simpler column ordering heuristic.
 */
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
pub struct MoveSorter {
    size: usize,
    moves: [Inner; Position::WIDTH as usize],
}
impl MoveSorter {
    // public:
    pub fn new() -> Self {
        MoveSorter {
            size: 0,
            moves: [Inner::new(); Position::WIDTH as usize],
        }
    }
    /**
     * Add a move in the container with its score.
     * You cannot add more than Position::WIDTH moves
     */
    pub fn add(&mut self, bmove: position::Bitboard, score: u8) {
        let mut pos = self.size;
        let new = Inner { score, bmove };
        // Add the new element to the end of the array
        self.moves[pos] = new;
        // Swap until it's in the right place
        while pos != 0 && self.moves[pos - 1].score > score {
            self.moves.swap(pos - 1, pos);
            pos = pos - 1;
        }
        self.size += 1;
    }

    /**
     * Get next move
     * @return next remaining move with max score and remove it from the container.
     * If no more move is available return 0
     */
    pub fn get_next(&mut self) -> Option<position::Bitboard> {
        match self.size {
            0 => None,
            n => {
                self.size -= 1;
                Some(self.moves[n - 1].bmove)
            }
        }
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
        for i in 0..Position::WIDTH {
            assert_eq!(ms.get_next(), Some(i as position::Bitboard));
        }
        assert_eq!(ms.get_next(), None);
    }
}

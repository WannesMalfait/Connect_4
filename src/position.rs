//
// Example of bit order to encode for a 7x6 board
// .  .  .  .  .  .  .
// 5 12 19 26 33 40 47
// 4 11 18 25 32 39 46
// 3 10 17 24 31 38 45
// 2  9 16 23 30 37 44
// 1  8 15 22 29 36 43
// 0  7 14 21 28 35 42
//
// Position is stored as
// - a bitboard "mask" with 1 on any color stones
// - a bitboard "current_player" with 1 on stones of current player
//
// "current_player" bitboard can be transformed into a compact and non ambiguous key
// by adding an extra bit on top of the last non empty cell of each column.
// This allow to identify all the empty cells whithout needing "mask" bitboard
//
// current_player "x" = 1, opponent "o" = 0
//
//
// board     position  mask      key       bottom
//           0000000   0000000   0000000   0000000
// .......   0000000   0000000   0001000   0000000
// ...o...   0000000   0001000   0010000   0000000
// ..xx...   0011000   0011000   0011000   0000000
// ..ox...   0001000   0011000   0001100   0000000
// ..oox..   0000100   0011100   0000110   0000000
// ..oxxo.   0001100   0011110   1101101   1111111
//
//
// current_player "o" = 1, opponent "x" = 0
//
// board     position  mask      key       bottom
//           0000000   0000000   0001000   0000000
// ...x...   0000000   0001000   0000000   0000000
// ...o...   0001000   0001000   0011000   0000000
// ..xx...   0000000   0011000   0000000   0000000
// ..ox...   0010000   0011000   0010100   0000000
// ..oox..   0011000   0011100   0011010   0000000
// ..oxxo.   0010010   0011110   1110011   1111111
//
//
// key is an unique representation of a board key = position + mask + bottom
// in practice, as bottom is constant, key = position + mask is also a
// non-ambigous representation of the position.
//

pub type Bitboard = u64;
pub type Column = u8;
///
/// A struct storing a Connect 4 position.
/// Functions are relative to the current player to play.
/// Position containing alignment are not supported by this class.
///
/// A binary bitboard representation is used.
/// Each column is encoded on `HEIGHT+1` bits.
#[derive(Clone)]
pub struct Position {
    /// bitboard of the current_player stones
    current_position: Bitboard,
    /// bitboard of all the already played spots
    mask: Bitboard,
    /// number of moves played since the beginning of the game.
    moves: u8,
}
/// Handle errors when playing a sequence of moves
pub enum PlayResult {
    Ok,
    TooSmall,
    TooBig(Column),
    Unplayable(Column),
    AlreadyWinning(Column),
}
/// Handle the enum type, and print appropriate error messages
#[must_use]
#[allow(clippy::needless_pass_by_value)]
pub fn play_result_ok(result: PlayResult) -> bool {
    match result {
        PlayResult::Ok => true,
        PlayResult::TooSmall => {
            eprintln!(
                "Input column was too small should be between 1 and {}",
                Position::WIDTH
            );
            false
        }
        PlayResult::TooBig(col) => {
            eprintln!(
                "Input column ({}) was too big should be between 1 and {}",
                col + 1,
                Position::WIDTH
            );
            false
        }
        PlayResult::Unplayable(col) => {
            eprintln!(
                "Input column ({}) is already full (board height: {})",
                col + 1,
                Position::HEIGHT
            );
            false
        }
        PlayResult::AlreadyWinning(col) => {
            eprintln!("Playing column {} leads to an already won position", col);
            false
        }
    }
}

impl Position {
    /// Width of the board
    pub const WIDTH: Column = 7;
    /// Height of the board
    pub const HEIGHT: Column = 6;
    /// For width and height of 7x6 min score is -18
    pub const MIN_SCORE: isize = -((Self::WIDTH * Self::HEIGHT) as isize) / 2 + 3;
    /// For width and height of 7x6 max score is 18
    pub const MAX_SCORE: isize = ((Self::WIDTH * Self::HEIGHT + 1) as isize) / 2 - 3;
    // Masks used for calculating possible moves.
    const BOTTOM_MASK: Bitboard = Self::bottom(Self::WIDTH, Self::HEIGHT);
    const BOARD_MASK: Bitboard = Self::BOTTOM_MASK * ((1u64 << Self::HEIGHT) - 1);
}

impl Position {
    /// Plays a possible move given by its bitboard representation
    ///
    /// `bmove`: a possible move given by its bitboard representation
    ///       only one bit of the bitboard should be set to 1
    ///       the move should be a valid possible move for the current player
    pub fn play(&mut self, bmove: Bitboard) {
        self.current_position ^= self.mask;
        self.mask |= bmove;
        self.moves += 1;
    }

    /// Plays a sequence of successive played columns, mainly used to initilize a board.
    /// `seq` is a sequence of digits corresponding to the 1-based index of the column played.
    ///
    /// You can check if the move sequence was valid by calling `play_result_ok()` on the
    /// returned value.
    pub fn play_sequence(&mut self, seq: &[Column]) -> PlayResult {
        for col_1_based in seq {
            if let Some(col) = col_1_based.checked_sub(1) {
                if col >= Position::WIDTH {
                    return PlayResult::TooBig(col);
                } else if !self.can_play(col) {
                    return PlayResult::Unplayable(col);
                } else if self.is_winning_move(col) {
                    return PlayResult::AlreadyWinning(col); // invalid move
                }
                self.play_col(col);
            } else {
                return PlayResult::TooSmall; // invalid move
            }
        }
        PlayResult::Ok
    }

    /// Create a position from a string of moves with no spaces in between
    /// If something went wrong with parsing `None` is returned.
    #[must_use]
    pub fn from_string(position_str: &str) -> Option<Self> {
        let mut pos = Position::new();
        let seq = position_str
            .chars()
            .map(|m| {
                m.to_digit(10).unwrap_or_else(|| {
                    println!("Invalid char {}, set to collumn 1 as default", m);
                    1
                }) as Column
            })
            .collect::<Vec<Column>>();
        if play_result_ok(pos.play_sequence(&seq)) {
            Some(pos)
        } else {
            None
        }
    }

    /// return true if the current player can win next move.
    #[must_use]
    pub fn can_win_next(&self) -> bool {
        (self.winning_position() & self.possible()) != 0
    }

    /// return the number of moves played since the beginning of the game.
    #[must_use]
    pub fn nb_moves(&self) -> u8 {
        self.moves
    }

    /// returns a compact representation of a position on WIDTH*(HEIGHT+1) bits.
    #[must_use]
    pub fn key(&self) -> Bitboard {
        self.current_position + self.mask
    }

    /// Build a symetric base 3 key. Two symetric positions will have the same key.
    ///
    /// This key is a base 3 representation of the sequence of played moves column per column,
    /// from bottom to top. The 3 digits are `top_of_colum(0)`, `current_player(1)`, `opponent(2)`.
    ///
    /// example: game "45" where player one played colum 4, then player two played column 5
    /// has a representation in base 3 digits : 0 0 0 1 0 2 0 0 0 or : 3*3^3 + 1*3^5
    ///
    /// The symetric key is the mimimum key of the two keys built iterating columns from left to right
    /// or right to left.
    ///
    /// as the last digit is always 0, we omit it and a base 3 key
    /// uses N = (nbMoves + nbColums - 1) base 3 digits or N*log2(3) bits.
    #[must_use]
    pub fn key3(&self) -> u64 {
        let mut key_forward = 0;
        for i in 0..Position::WIDTH {
            // compute key in increasing order of columns
            self.partial_key3(&mut key_forward, i);
        }
        let mut key_reverse = 0;
        // compute key in decreasing order of columns
        for i in (0..Position::WIDTH).rev() {
            self.partial_key3(&mut key_reverse, i);
        }
        // take the smallest key and divide per 3 as the last base3 digit is always 0
        if key_forward < key_reverse {
            key_forward / 3
        } else {
            key_reverse / 3
        }
    }

    /// Return a bitboard of all the possible next moves the do not lose in one turn.
    /// A losing move is a move leaving the possibility for the opponent to win directly.
    ///
    /// WARNING: this function is intended to test position where you cannot win in one turn
    /// If you have a winning move, this function can miss it and prefer to prevent the opponent
    /// to make an alignment.
    #[must_use]
    pub fn possible_non_losing_moves(&self) -> Bitboard {
        debug_assert!(!self.can_win_next());
        let possible_mask = self.possible();
        let opponent_win = self.opponent_winning_position();
        let forced_moves = possible_mask & opponent_win;
        if forced_moves == 0 {
            possible_mask & !(opponent_win >> 1) // avoid to play below an opponent winning spot
        } else if (forced_moves & (forced_moves - 1)) == 0 {
            forced_moves & !(opponent_win >> 1) // enforce to play the single forced move
        } else {
            // check if there is more than one forced move
            0 // the opponnent has two winning moves and you cannot stop him
        }
    }

    /// Score a possible move.
    ///
    /// `bmove` is a possible move given in a bitboard format.
    ///
    /// The score we are using is the number of winning spots
    /// the current player has after playing the move.
    #[must_use]
    pub fn move_score(&self, bmove: Bitboard) -> u8 {
        Self::popcount(Self::compute_winning_position(
            self.current_position | bmove,
            self.mask,
        ))
    }

    /// Default constructor, build an empty position.
    #[must_use]
    pub fn new() -> Position {
        Position {
            current_position: 0,
            mask: 0,
            moves: 0,
        }
    }

    /// Indicates whether a column is playable.
    /// `col` is a 0-based index of the column to play
    /// returns `true` if the column is playable, `false` if the column is already full.
    #[must_use]
    pub fn can_play(&self, col: Column) -> bool {
        (self.mask & Self::top_mask_col(col)) == 0
    }

    /// Plays a playable column.
    /// This function should not be called on a non-playable column or a column making an alignment.
    ///
    /// `col` is a 0-based index of a playable column.
    pub fn play_col(&mut self, col: Column) {
        self.play((self.mask + Self::bottom_mask_col(col)) & Self::column_mask(col));
    }

    /// Indicates whether the current player wins by playing a given column.
    /// This function should never be called on a non-playable column.
    /// `col` is a 0-based index of a playable column.
    /// returns `true` if the current player makes an alignment by playing the corresponding column `col`.
    #[must_use]
    pub fn is_winning_move(&self, col: Column) -> bool {
        (self.winning_position() & self.possible() & Self::column_mask(col)) != 0
    }

    /// Displays the bitboard, usefull for debugging
    pub fn display_bitboard(bb: Bitboard) {
        for col in (0..Self::HEIGHT).rev() {
            for row in 0..(Self::WIDTH) {
                if (bb & (1u64 << (col + row * (Self::HEIGHT + 1)))) == 0 {
                    print!(".");
                } else {
                    print!("x");
                }
            }
            println!();
        }
    }
    /// Returns either ("x", "o") or ("o", "x").
    /// The first element is the current player.
    #[must_use]
    pub fn current_player(&self) -> (&str, &str) {
        match self.moves % 2 {
            0 => ("x", "o"),
            _ => ("o", "x"),
        }
    }

    /// Prints the current position to `std_out()`.
    pub fn display_position(&self) {
        let (us, them) = match self.moves % 2 {
            0 => ("x", "o"),
            _ => ("o", "x"),
        };
        for col in (0..Self::HEIGHT).rev() {
            for row in 0..(Self::WIDTH) {
                if (self.mask & (1u64 << (col + row * (Self::HEIGHT + 1)))) == 0 {
                    print!(".");
                } else {
                    match self.current_position & (1u64 << (col + row * (Self::HEIGHT + 1))) {
                        0 => {
                            print!("{}", us);
                        }
                        _ => {
                            print!("{}", them);
                        }
                    }
                }
            }
            println!();
        }
    }

    /// Compute a partial base 3 key for a given column
    fn partial_key3(&self, key: &mut u64, col: Column) {
        let mut pos = 1 << (col * (Position::HEIGHT + 1));
        while (pos & self.mask) != 0 {
            *key *= 3;
            if (pos & self.current_position) == 0 {
                *key += 2;
            } else {
                *key += 1;
            }
            pos <<= 1;
        }
        *key *= 3;
    }

    /// Return a bitboard of the possible winning positions for the current player
    #[must_use]
    fn winning_position(&self) -> Bitboard {
        Self::compute_winning_position(self.current_position, self.mask)
    }

    /// Return a bitboard of the possible winning positions for the opponent
    #[must_use]
    fn opponent_winning_position(&self) -> Bitboard {
        Self::compute_winning_position(self.current_position ^ self.mask, self.mask)
    }

    /// Bitboard of the next possible valid moves for the current player
    /// including losing moves.
    #[must_use]
    fn possible(&self) -> Bitboard {
        (self.mask + Self::BOTTOM_MASK) & Self::BOARD_MASK
    }

    /// Counts the number of bits set to one in a `u64`.
    #[must_use]
    fn popcount(mut m: u64) -> u8 {
        let mut c: u8 = 0;
        while m != 0 {
            m &= m - 1;
            c += 1;
        }
        c
    }

    /// Returns a bitboard of all the winning free spots making an alignment in the
    /// position `position`. The `mask` has the bits set where a spot was already played.
    #[must_use]
    fn compute_winning_position(position: Bitboard, mask: Bitboard) -> Bitboard {
        // vertical;
        let mut r = (position << 1) & (position << 2) & (position << 3);

        //horizontal
        let mut p = (position << (Self::HEIGHT + 1)) & (position << (2 * (Self::HEIGHT + 1)));
        r |= p & (position << (3 * (Self::HEIGHT + 1)));
        r |= p & (position >> (Self::HEIGHT + 1));
        p = (position >> (Self::HEIGHT + 1)) & (position >> (2 * (Self::HEIGHT + 1)));
        r |= p & (position << (Self::HEIGHT + 1));
        r |= p & (position >> (3 * (Self::HEIGHT + 1)));

        //diagonal 1
        p = (position << Self::HEIGHT) & (position << (2 * Self::HEIGHT));
        r |= p & (position << (3 * Self::HEIGHT));
        r |= p & (position >> Self::HEIGHT);
        p = (position >> Self::HEIGHT) & (position >> (2 * Self::HEIGHT));
        r |= p & (position << Self::HEIGHT);
        r |= p & (position >> (3 * Self::HEIGHT));

        //diagonal 2
        p = (position << (Self::HEIGHT + 2)) & (position << (2 * (Self::HEIGHT + 2)));
        r |= p & (position << (3 * (Self::HEIGHT + 2)));
        r |= p & (position >> (Self::HEIGHT + 2));
        p = (position >> (Self::HEIGHT + 2)) & (position >> (2 * (Self::HEIGHT + 2)));
        r |= p & (position << (Self::HEIGHT + 2));
        r |= p & (position >> (3 * (Self::HEIGHT + 2)));

        r & (Self::BOARD_MASK ^ mask)
    }

    #[must_use]
    const fn bottom(width: Column, height: Column) -> Bitboard {
        if width == 0 {
            0
        } else {
            Self::bottom(width - 1, height) | 1u64 << ((width - 1) * (height + 1))
        }
    }

    // return a bitboard containg a single 1 corresponding to the top cel of a given column
    #[must_use]
    fn top_mask_col(col: Column) -> Bitboard {
        1u64 << ((Self::HEIGHT - 1) + col * (Self::HEIGHT + 1))
    }

    // return a bitboard containg a single 1 corresponding to the bottom cell of a given column
    #[must_use]
    fn bottom_mask_col(col: Column) -> Bitboard {
        1u64 << (col * (Self::HEIGHT + 1))
    }

    // return a bitboard 1 on all the cells of a given column
    #[must_use]
    pub fn column_mask(col: Column) -> Bitboard {
        ((1u64 << Self::HEIGHT) - 1) << (col * (Self::HEIGHT + 1))
    }
}

#[cfg(test)]
mod tests {
    use super::{play_result_ok, Position};
    #[test]
    fn simple_moves() {
        let mut pos = Position::new();
        assert_eq!(pos.nb_moves(), 0);
        for col in 0..Position::WIDTH {
            assert!(pos.can_play(col));
        }
        // play a move 0-based indices
        pos.play_col(1);
        assert_eq!(pos.nb_moves(), 1);
        pos.play_col(2);
        assert_eq!(pos.nb_moves(), 2);
        pos.play_col(1);
        assert_eq!(pos.nb_moves(), 3);
        // 1-based indices
        let result = pos.play_sequence(&[3, 2, 3]);
        // current position:
        // .......
        // .......
        // .......
        // .xo....
        // .xo....
        // .xo....

        assert!(play_result_ok(result));
        assert_eq!(pos.nb_moves(), 6);
        assert!(pos.is_winning_move(1));
    }
    #[test]
    fn find_all_moves() {
        let mut pos = Position::new();
        Position::display_bitboard(pos.possible_non_losing_moves());
        let result = pos.play_sequence(&[4, 4, 3, 3, 5]);
        assert!(play_result_ok(result));
        // Every move loses
        assert_eq!(0u64, pos.possible_non_losing_moves());
    }
}

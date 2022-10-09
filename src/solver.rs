use std::time::Instant;

use crate::move_sorter;
use crate::position;
use crate::transposition_table;
use move_sorter::MoveSorter;
use position::{Column, Position};

pub struct Solver {
    node_count: u64,
    column_order: [Column; Position::WIDTH as usize],
    trans_table: transposition_table::TranspositionTable,
}

impl Default for Solver {
    fn default() -> Self {
        Self::new()
    }
}

impl Solver {
    const INVALID_MOVE: isize = -1000;

    /// Nodes visited since last reset.
    #[must_use]
    pub fn get_node_count(&self) -> u64 {
        self.node_count
    }
    /// Reset the counter to 0.
    pub fn reset_node_count(&mut self) {
        self.node_count = 0;
    }
    /// Clear the transposition table of entries
    pub fn reset_transposition_table(&mut self) {
        self.trans_table.reset();
    }

    /// Initializes the solver with a transposition table and move
    /// ordering heuristics.
    #[must_use]
    pub fn new() -> Self {
        let mut column_order = [0; Position::WIDTH as usize];
        // initialize the column exploration order, starting with center columns
        for i in 0..Position::WIDTH {
            // example for WIDTH=7: column_order = {3, 2, 4, 1, 5, 0, 6}
            column_order[i as usize] = (Position::WIDTH as isize / 2
                + (1 - 2 * (i % 2) as isize) * (i as isize + 1) / 2)
                as Column;
        }
        Solver {
            node_count: 0,
            column_order,
            trans_table: transposition_table::TranspositionTable::new(),
        }
    }

    /// Get the number of stones left for one player in the given position offset by `addend` moves.
    #[inline]
    fn num_stones_left(addend: isize, pos: &Position) -> isize {
        ((Position::WIDTH * Position::HEIGHT) as isize + addend - pos.nb_moves() as isize) / 2
    }

    /// Main alpha-beta search function.
    fn negamax(&mut self, pos: &Position, mut alpha: isize, mut beta: isize) -> isize {
        debug_assert!(alpha < beta);
        debug_assert!(!pos.can_win_next());
        // increment number of explored nodes
        self.node_count += 1;

        let possible = pos.possible_non_losing_moves();
        // All moves lose
        if possible == 0 {
            return -Self::num_stones_left(0, pos);
        }
        // No stones left => draw
        if pos.nb_moves() >= Position::WIDTH * Position::HEIGHT - 2 {
            return 0;
        }
        // This is a lower bound on the score because they can't win next move
        let mut min = -Self::num_stones_left(-2, pos);
        if alpha < min {
            // We are searching in [alpha;beta] window but min > alpha, so we can instead search in [min; beta] window
            alpha = min;
            if alpha >= beta {
                // We can prune because the search window is empty
                return alpha;
            }
        }
        // Upper bound on the score because we can't win next move
        let mut max = Self::num_stones_left(-1, pos);
        if beta > max {
            // We are searching in [alpha;beta] window but beta > max, so we can instead search in [alpha; max] window
            beta = max;
            if alpha >= beta {
                // We can prune because the search window is empty
                return beta;
            }
        }

        let key = pos.key();
        if let Some(val) = self.trans_table.get(key) {
            // The node has been visited before
            let val = val as isize;
            if val > Position::MAX_SCORE - Position::MIN_SCORE + 1 {
                // Lower bound was stored
                min = val + 2 * Position::MIN_SCORE - Position::MAX_SCORE - 2;
                if alpha < min {
                    alpha = min;
                    if alpha >= beta {
                        return alpha;
                    }
                }
            } else {
                // Upper bound was stored
                max = val + Position::MIN_SCORE - 1;
                if beta > max {
                    beta = max;
                    if alpha >= beta {
                        return beta;
                    }
                }
            }
        }

        let mut moves = MoveSorter::new();
        // Add the moves to the sorter in reverse order, because the last moves
        // have a higher chance of getting good scores, this way the sorting
        // is faster
        for i in (0..Position::WIDTH).rev() {
            let bmove = possible & Position::column_mask(self.column_order[i as usize]);
            if bmove != 0 {
                moves.add(bmove, pos.move_score(bmove));
            }
        }
        for bmove in moves {
            let mut pos2 = *pos;
            pos2.play(bmove);
            let score = -self.negamax(&pos2, -beta, -alpha);
            if score >= beta {
                debug_assert!((score + Position::MAX_SCORE - 2 * Position::MIN_SCORE + 2) > 0);
                self.trans_table.put(
                    key,
                    (score + Position::MAX_SCORE - 2 * Position::MIN_SCORE + 2) as Column,
                );

                // Save a lower bound
                return score;
            }
            if score > alpha {
                // We only need to search for better moves than the best so far
                alpha = score;
            }
        }
        debug_assert!((alpha - Position::MIN_SCORE + 1) > 0);
        // Save an upper bound
        self.trans_table
            .put(key, (alpha - Position::MIN_SCORE + 1) as Column);
        alpha
    }

    /// Get a score for the current position, if `weak` is true, then only a weak solve
    /// is done, i.e. we only check if it is a win a draw or a loss, but without a score.
    /// Prints search info to `std_out` if `output` is set to `true`.
    ///
    /// A positive score means it's winning for the current player and a negative score means
    /// that it's losing. A score of zero means it's a draw with best play. A score of 1 means
    /// that the current player can win with his last stone, 2 with his second to last stone...
    pub fn solve(&mut self, pos: &Position, weak: bool, output: bool) -> isize {
        // check if win in one move as the Negamax function does not support this case.
        if pos.can_win_next() {
            return Self::num_stones_left(1, pos);
        }
        let mut min = -Self::num_stones_left(0, pos);
        let mut max = Self::num_stones_left(1, pos);
        if weak {
            min = -1;
            max = 1;
        }

        while min < max {
            let now = Instant::now();
            let nodes = self.get_node_count();
            // iteratively narrow the min-max exploration window
            let mut med = min + (max - min) / 2;
            if med <= 0 && min / 2 < med {
                med = min / 2;
            } else if med >= 0 && max / 2 > med {
                med = max / 2;
            }
            if output {
                println!("Searching: alpha {} beta {}", med, med + 1);
            }
            // use a null depth window to know if the actual score is greater or smaller than med
            let r = self.negamax(pos, med, med + 1);
            if r <= med {
                max = r;
            } else {
                min = r;
            }
            if output {
                println!(
                    "took: {:?} with {} nodes, kn/s: {:.1}",
                    now.elapsed(),
                    self.get_node_count() - nodes,
                    (self.get_node_count() - nodes) as f64 / now.elapsed().as_secs_f64() / 1000.0,
                );
            }
        }
        min
    }

    /// Get a score for all the columns that can be played by calling `solve()`.
    pub fn analyze(&mut self, pos: &Position, weak: bool) -> Vec<isize> {
        let mut scores = vec![Self::INVALID_MOVE; Position::WIDTH as usize];
        for col in 0..Position::WIDTH {
            if pos.can_play(col) {
                if pos.is_winning_move(col) {
                    scores[col as usize] = Self::num_stones_left(1, pos);
                } else {
                    let mut pos2 = *pos;
                    pos2.play_col(col);
                    scores[col as usize] = -self.solve(&pos2, weak, true);
                }
            }
        }
        scores
    }
    /// Convert a score to the number of moves till the winning player can win.
    /// Should not be called with score 0, which is a draw
    #[must_use]
    pub fn score_to_moves_to_win(pos: &Position, score: isize) -> isize {
        if score > 0 {
            Self::num_stones_left(1, pos) - score + 1
        } else {
            Self::num_stones_left(0, pos) + score + 1
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    #[test]
    fn column_order() {
        let s = Solver::new();
        if Position::WIDTH == 7 {
            assert_eq!(s.column_order, [3, 2, 4, 1, 5, 0, 6]);
        }
    }
    #[test]
    fn test_scores() {
        let mut pos = Position::new();
        pos.play_sequence(&[4, 4, 5]);
        assert_eq!(Solver::score_to_moves_to_win(&pos, 2), 19);
        pos.play_col(3);
        assert_eq!(Solver::score_to_moves_to_win(&pos, -2), 18);
        pos.play_col(7);
        assert_eq!(Solver::score_to_moves_to_win(&pos, 2), 18);
        pos.play_col(6);
        assert_eq!(Solver::score_to_moves_to_win(&pos, -2), 17);
        pos = Position::new();
        pos.play_sequence(&[4, 4, 5, 5]);
        assert_eq!(Solver::score_to_moves_to_win(&pos, 18), 2);
        pos.play_col(3);
        assert_eq!(Solver::score_to_moves_to_win(&pos, -18), 1);
    }
}

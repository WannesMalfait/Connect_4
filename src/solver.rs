use std::sync::{
    atomic::{AtomicBool, AtomicIsize, AtomicU64, Ordering},
    Arc,
};
use std::time::Instant;

use crate::move_sorter;
use crate::opening_book::OpeningBook;
use crate::position;
use crate::transposition_table::TranspositionTable;
use move_sorter::MoveSorter;
use position::{Column, Position};

struct Nodes(Arc<AtomicU64>);

impl Clone for Nodes {
    fn clone(&self) -> Self {
        Self(Arc::new(AtomicU64::new(0)))
    }
}

#[derive(Clone)]
struct NodeCounter {
    node_counters: Vec<Option<Arc<AtomicU64>>>,
}

impl NodeCounter {
    fn initialize_node_counters(&mut self, threads: usize) {
        self.node_counters = vec![None; threads];
    }

    fn add_node_counter(&mut self, thread: usize, node_counter: Arc<AtomicU64>) {
        self.node_counters[thread] = Some(node_counter);
    }

    fn get_node_count(&self) -> u64 {
        let mut total_nodes = 0;
        for nodes in self.node_counters.iter().flatten() {
            total_nodes += nodes.load(Ordering::Relaxed);
        }
        total_nodes
    }
}

#[derive(Clone)]
struct SharedContext {
    table: Arc<TranspositionTable>,
    abort_search: Arc<AtomicBool>,
    score: Arc<AtomicIsize>,
}

impl SharedContext {
    fn abort_search(&self) -> bool {
        self.abort_search.load(Ordering::SeqCst)
    }

    fn abort_now(&self) {
        self.abort_search.store(true, Ordering::SeqCst)
    }
}

#[derive(Clone)]
struct LocalContext {
    abort: bool,
    nodes: Nodes,
    tt_hits: u64,
}

impl LocalContext {
    pub fn reset_nodes(&self) {
        self.nodes.0.store(0, Ordering::Relaxed);
    }

    pub fn increment_nodes(&self) {
        self.nodes.0.fetch_add(1, Ordering::Relaxed);
    }

    pub fn nodes(&self) -> u64 {
        self.nodes.0.load(Ordering::Relaxed)
    }
}

struct Searcher {
    shared_context: SharedContext,
    local_context: LocalContext,
    node_counter: NodeCounter,
}

pub struct Solver {
    trans_table: Arc<TranspositionTable>,
    book: Option<OpeningBook>,
}

impl Default for Solver {
    fn default() -> Self {
        Self::new(None)
    }
}

impl Solver {
    /// Initializes the solver with a transposition table. A book can be
    /// added with the `set_book` method.
    #[must_use]
    pub fn new(book: Option<OpeningBook>) -> Self {
        Solver {
            trans_table: Arc::new(TranspositionTable::new()),
            book,
        }
    }

    /// Convert a score to the number of moves till the winning player can win.
    /// If the score is 0, then the position is a draw and the number returned is
    /// the number of moves left for the current player.
    #[must_use]
    pub fn score_to_moves_to_win(pos: &Position, score: isize) -> isize {
        if score > 0 {
            pos.num_stones_left(1) - score + 1
        } else {
            pos.num_stones_left(0) + score + 1
        }
    }

    /// Clear the transposition table of entries
    pub fn reset_transposition_table(&mut self) {
        self.trans_table.reset();
    }

    pub fn set_book(&mut self, book: OpeningBook) {
        self.book = Some(book)
    }

    /// Generate an opening book by adding all the positions up to a certain depth.
    /// This function does not store the opening book in a file.
    pub fn generate_book(&mut self, pos: &Position, depth: usize) {
        match &self.book {
            None => {
                self.book = Some(OpeningBook::new());
            }
            Some(book) => {
                if book.get(pos).is_some() {
                    return;
                }
            }
        };
        if pos.nb_moves() as usize > depth {
            return;
        }
        println!("\nAdding position to opening book...");
        pos.display_position();
        let (score, _) = self.solve(pos, false, true, 1);
        let book = self.book.as_mut().unwrap();
        println!("Added position with score {score}");
        book.put(pos, score);
        for col in 0..Position::WIDTH {
            let col = Searcher::COLUMN_ORDER1[col as usize];
            if !pos.can_play(col) || pos.is_winning_move(col) {
                continue;
            }
            let mut p2 = pos.clone();
            p2.play_col(col);
            self.generate_book(&p2, depth);
        }
    }

    /// Gets the solver's opening book. Panics if it has no book.
    pub fn get_book(&'_ self) -> &'_ OpeningBook {
        self.book.as_ref().unwrap()
    }

    /// Get a score for the current position, if `weak` is true, then only a weak solve
    /// is done, i.e. we only check if it is a win a draw or a loss, but without a score.
    /// Prints search info to `std_out` if `output` is set to `true`.
    ///
    /// A positive score means it's winning for the current player and a negative score means
    /// that it's losing. A score of zero means it's a draw with best play. A score of 1 means
    /// that the current player can win with their last stone, 2 with their second to last stone...
    pub fn solve(
        &mut self,
        pos: &Position,
        weak: bool,
        output: bool,
        num_threads: u8,
    ) -> (isize, u64) {
        // Check if we can win in one move as the negamax function does not support this case.
        if pos.can_win_next() {
            return (pos.num_stones_left(1), 0);
        }

        // Check if the position is in the opening book.
        if let Some(book) = &self.book {
            if let Some(score) = book.get(pos) {
                if output {
                    println!("Position in opening book");
                }
                return (score, 0);
            }
        }
        let mut searcher = Searcher::new(self.trans_table.clone());
        searcher.search(num_threads, output, pos, weak)
    }

    /// Get a score for all the columns that can be played by calling `solve()`.
    pub fn analyze(&mut self, pos: &Position, weak: bool) -> Vec<isize> {
        let mut scores = vec![Searcher::INVALID_MOVE; Position::WIDTH as usize];
        for col in 0..Position::WIDTH {
            if pos.can_play(col) {
                if pos.is_winning_move(col) {
                    scores[col as usize] = pos.num_stones_left(1);
                } else {
                    let mut pos2 = pos.clone();
                    pos2.play_col(col);
                    let (score, nodes) = self.solve(&pos2, weak, true, 1);
                    println!("Solved with {nodes} nodes.");
                    scores[col as usize] = -score;
                }
            }
        }
        scores
    }
}

impl Searcher {
    const INVALID_MOVE: isize = -1000;
    const COLUMN_ORDER1: [Column; Position::WIDTH as usize] =
        Self::column_order1(0, [0; Position::WIDTH as usize]);
    const COLUMN_ORDER2: [Column; Position::WIDTH as usize] =
        Self::column_order2(0, [0; Position::WIDTH as usize]);
    const fn column_order1(
        i: u8,
        mut temp_order: [Column; Position::WIDTH as usize],
    ) -> [Column; Position::WIDTH as usize] {
        if i == Position::WIDTH {
            return temp_order;
        }
        // initialize the column exploration order, starting with center columns
        // example for WIDTH=7: column_order = {3, 2, 4, 1, 5, 0, 6}
        temp_order[i as usize] = (Position::WIDTH as isize / 2
            + (1 - 2 * (i % 2) as isize) * (i as isize + 1) / 2)
            as Column;
        Self::column_order1(i + 1, temp_order)
    }
    const fn column_order2(
        i: u8,
        mut temp_order: [Column; Position::WIDTH as usize],
    ) -> [Column; Position::WIDTH as usize] {
        if i == Position::WIDTH {
            return temp_order;
        }
        // initialize the column exploration order, starting with center columns
        // example for WIDTH=7: column_order = {3, 4, 2, 5, 1, 6, 0}
        temp_order[i as usize] = (Position::WIDTH as isize / 2
            - (1 - 2 * (i % 2) as isize) * (i as isize + 1) / 2)
            as Column;
        Self::column_order2(i + 1, temp_order)
    }
}

impl Searcher {
    #[must_use]
    pub fn new(table: Arc<TranspositionTable>) -> Self {
        Self {
            shared_context: SharedContext {
                table,
                abort_search: Arc::new(AtomicBool::new(false)),
                score: Arc::new(AtomicIsize::new(0)),
            },
            local_context: LocalContext {
                abort: false,
                nodes: Nodes(Arc::new(AtomicU64::new(0))),
                tt_hits: 0,
            },
            node_counter: NodeCounter {
                node_counters: Vec::new(),
            },
        }
    }

    /// Main alpha-beta search function.
    fn negamax(
        local_context: &mut LocalContext,
        shared_context: &SharedContext,
        pos: &Position,
        mut alpha: isize,
        mut beta: isize,
        can_be_symmetric: bool,
        thread_id: u8,
    ) -> isize {
        debug_assert!(alpha < beta);
        debug_assert!(!pos.can_win_next());
        // increment number of explored nodes
        local_context.increment_nodes();

        if local_context.nodes() % 1024 == 0 && shared_context.abort_search() {
            local_context.abort = true;
            return 0;
        }

        let possible = pos.possible_non_losing_moves();
        // All moves lose
        if possible == 0 {
            return -pos.num_stones_left(0);
        }
        // No stones left => draw
        if pos.nb_moves() >= Position::WIDTH * Position::HEIGHT - 2 {
            return 0;
        }
        // This is a lower bound on the score because they can't win next move
        let min = -pos.num_stones_left(-2);
        if alpha < min {
            // We are searching in [alpha;beta] window but min > alpha, so we can instead search in [min; beta] window
            alpha = min;
            if alpha >= beta {
                // We can prune because the search window is empty
                return alpha;
            }
        }
        // Upper bound on the score because we can't win next move
        let max = pos.num_stones_left(-1);
        if beta > max {
            // We are searching in [alpha;beta] window but beta > max, so we can instead search in [alpha; max] window
            beta = max;
            if alpha >= beta {
                // We can prune because the search window is empty
                return beta;
            }
        }

        let key = pos.key();
        let mut best_column = None;
        if let Some(posinfo) = shared_context.table.get(key) {
            local_context.tt_hits += 1;
            // The node has been visited before
            let val = posinfo.score();
            if val > Position::MAX_SCORE - Position::MIN_SCORE + 1 {
                // Lower bound was stored
                let min = val + 2 * Position::MIN_SCORE - Position::MAX_SCORE - 2;
                if alpha < min {
                    alpha = min;
                    if alpha >= beta {
                        return alpha;
                    }
                }
            } else {
                // Upper bound was stored
                let max = val + Position::MIN_SCORE - 1;
                if beta > max {
                    beta = max;
                    if alpha >= beta {
                        return beta;
                    }
                }
            }
            best_column = Some(posinfo.column());
            debug_assert!(0 != possible & Position::column_mask(best_column.unwrap()));
        } else {
            // TODO: tt_miss counter, or some other way of getting a feel how useful our tb entries are.
        }

        let mut moves = MoveSorter::new();
        // Add the moves to the sorter in reverse order, because the last moves
        // have a higher chance of getting good scores, this way the sorting
        // is faster
        for i in (0..Position::WIDTH).rev() {
            // TODO: other way of making threads help each other.
            let col = if thread_id % 2 == 0 {
                Self::COLUMN_ORDER1[i as usize]
            } else {
                Self::COLUMN_ORDER2[i as usize]
            };
            let bmove = possible & Position::column_mask(col);
            if bmove != 0 && Some(col) != best_column {
                moves.add(bmove, col, pos.move_score(bmove));
            }
        }

        if let Some(col) = best_column {
            // This has a higher score, since there can be at most `Position::WIDTH` winning moves.
            let bmove = possible & Position::column_mask(col);
            moves.add(bmove, col, Position::WIDTH + 1);
        }

        let mut highest_score = None;
        for (bmove, col) in moves {
            let mut pos2 = pos.clone();
            pos2.play(bmove);
            let score = -Self::negamax(
                local_context,
                shared_context,
                &pos2,
                -beta,
                -alpha,
                can_be_symmetric,
                thread_id,
            );
            if score > alpha {
                // We only need to search for better moves than the best so far
                if score >= beta {
                    // TODO: Potentially only store if better bound.
                    debug_assert!((score + Position::MAX_SCORE - 2 * Position::MIN_SCORE + 2) > 0);
                    shared_context.table.put(
                        key,
                        (score + Position::MAX_SCORE - 2 * Position::MIN_SCORE + 2) as Column,
                        col,
                    );
                    if can_be_symmetric && pos.nb_moves() < 10 {
                        // Also store the mirrored position in the transposition table.
                        // If only a few moves have been made, the symmetric position is
                        // likely to be reached in another branch.
                        shared_context.table.put(
                            pos.mirrored_key(),
                            (score + Position::MAX_SCORE - 2 * Position::MIN_SCORE + 2) as Column,
                            col,
                        );
                    }

                    // Save a lower bound
                    return score;
                }
                alpha = score;
            }
            if highest_score.is_none() || score > highest_score.unwrap() {
                best_column = Some(col);
                highest_score = Some(score);
            }
        }
        debug_assert!((alpha - Position::MIN_SCORE + 1) > 0);
        // Save an upper bound
        shared_context.table.put(
            key,
            (alpha - Position::MIN_SCORE + 1) as Column,
            best_column.unwrap(),
        );
        alpha
    }

    /// Create a searcher that will solve the position.
    ///
    /// The searcher will return the number of nodes it searched,
    /// the score is stored in the shared context.
    fn launch_searcher(
        &mut self,
        output: bool,
        pos: &Position,
        weak: bool,
        thread_id: u8,
    ) -> impl FnMut() -> u64 {
        let thread_is_main = thread_id == 0;
        let shared_context = self.shared_context.clone();
        let mut local_context = self.local_context.clone();
        self.node_counter
            .add_node_counter(thread_id as usize, local_context.nodes.0.clone());
        let node_counter = if thread_is_main {
            Some(self.node_counter.clone())
        } else {
            None
        };

        // Essentially, we do a binary search for the actual score.
        let pos = pos.clone();
        let mut min = -pos.num_stones_left(0);
        let mut max = pos.num_stones_left(1);
        if weak {
            // We only need to know if the actual score is
            // < 0 ==> loss
            // > 0 ==> win
            // = 0 ==> draw
            min = -1;
            max = 1;
        }

        let can_be_symmetric = pos.can_become_symmetric();
        move || {
            let start = Instant::now();
            let mut nodes = 0;
            local_context.reset_nodes();
            while min < max {
                let local_timer = Instant::now();
                // Compute the middle of our search window.
                // TODO: explore making this value different for different threads.
                let mut med = min + (max - min) / 2;
                if med <= 0 && min / 2 < med {
                    med = min / 2;
                } else if med >= 0 && max / 2 > med {
                    med = max / 2;
                }
                if output && thread_is_main {
                    println!(
                        "Searching: alpha {} beta {} [min {min}, max {max}]",
                        med,
                        med + 1
                    );
                }
                // Is the actual score bigger or smaller than `med`?
                let r = Self::negamax(
                    &mut local_context,
                    &shared_context,
                    &pos,
                    med,
                    med + 1,
                    can_be_symmetric,
                    thread_id,
                );
                nodes = local_context.nodes();
                if local_context.abort {
                    return nodes;
                }
                if r <= med {
                    // Score was smaller, so update maximum.
                    max = r;
                } else {
                    // Score was bigger, so update minimum.
                    min = r;
                }
                if output && thread_is_main {
                    let total_nodes = node_counter.as_ref().unwrap().get_node_count();
                    let elapsed = start.elapsed();
                    println!(
                        "Took: {:?}, total nodes {}, kn/s: {}",
                        local_timer.elapsed(),
                        total_nodes,
                        (total_nodes as u128 * 1000) / elapsed.as_millis().max(1) / 1000,
                    );
                    // Try and output the principal variation.
                    print!("pv: ");
                    let mut pos = pos.clone();
                    while let Some(posinfo) = shared_context.table.get(pos.key()) {
                        let best_column = posinfo.column();
                        print!("{} ", best_column + 1);
                        pos.play_col(best_column);
                    }
                    println!();
                }
            }
            if shared_context.abort_search() {
                return nodes;
            }
            // We have solved the position. Alert the other threads that we are done.
            shared_context.abort_now();
            shared_context.score.store(min, Ordering::SeqCst);
            nodes
        }
    }

    fn search(
        &mut self,
        num_threads: u8,
        output: bool,
        pos: &Position,
        weak: bool,
    ) -> (isize, u64) {
        self.node_counter
            .initialize_node_counters(num_threads as usize);
        let mut join_handlers = vec![];
        for i in 1..num_threads {
            join_handlers.push(std::thread::spawn(
                self.launch_searcher(output, pos, weak, i),
            ));
        }
        let mut total_nodes = self.launch_searcher(output, pos, weak, 0)();
        for join_handler in join_handlers {
            let nodes = join_handler.join().unwrap();
            total_nodes += nodes;
        }

        (
            self.shared_context.score.load(Ordering::Relaxed),
            total_nodes,
        )
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    #[test]
    fn column_order() {
        if Position::WIDTH == 7 {
            assert_eq!(Searcher::COLUMN_ORDER1, [3, 2, 4, 1, 5, 0, 6]);
            assert_eq!(Searcher::COLUMN_ORDER2, [3, 4, 2, 5, 1, 6, 0]);
        }
    }

    #[test]
    fn test_scores() {
        let mut pos = Position::new();
        pos.play_sequence(&[4, 4, 5]);
        assert_eq!(Solver::score_to_moves_to_win(&pos, 2), 19);
        pos.play_col(3);
        assert_eq!(Solver::score_to_moves_to_win(&pos, -2), 18);
        pos.play_col(6);
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

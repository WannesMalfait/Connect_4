pub mod move_sorter;
pub mod position;
pub mod solver;
pub mod transposition_table;

pub mod game_solver {

    use crate::position::{self, Position};
    use crate::solver::Solver;
    use std::time::Instant;
    use std::{io, num::ParseIntError};
    pub struct Parser {
        solver: Solver,
        weak: bool,
        commands: Vec<String>,
    }

    impl Parser {
        pub fn new(weak: bool) -> Self {
            Self {
                solver: Solver::new(),
                weak,
                commands: vec![
                    "moves/play/move",
                    "position",
                    "solve",
                    "analyze",
                    "toggle-weak",
                    "commands/help",
                    "clear-tt",
                    "quit",
                ]
                .iter()
                .map(|&x| x.to_string())
                .collect(),
            }
        }
        pub fn run(&mut self) -> io::Result<()> {
            let mut pos = Position::new();
            let mut input = String::new();
            while let Ok(_) = io::stdin().read_line(&mut input) {
                let mut args = input.trim().split(' ');
                if let Some(first) = args.next() {
                    match &first.to_lowercase() as &str {
                        "moves" | "play" | "move" => {
                            if let Err(e) = self.handle_moves(args, &mut pos) {
                                println!("Moves should be numbers, got: {}", e);
                            }
                            pos.display_position();
                        }
                        "position" => {
                            pos = Position::new();
                            if let Err(e) = self.handle_moves(args, &mut pos) {
                                println!("Moves should be numbers, got: {}", e);
                            }
                            pos.display_position();
                        }
                        "solve" => {
                            let now = Instant::now();
                            self.solver.reset_node_count();
                            self.solve(&pos);
                            println!("Nodes searched: {}", self.solver.get_node_count());
                            println!("Took {:?}", now.elapsed());
                        }
                        "analyze" => {
                            let now = Instant::now();
                            self.solver.reset_node_count();
                            self.analyze(&pos);
                            println!("Nodes searched: {}", self.solver.get_node_count());
                            println!("Took {:?}", now.elapsed());
                        }
                        "toggle-weak" => {
                            self.weak = !self.weak;
                            println!("Weak set to {}", self.weak);
                        }
                        "commands" | "help" => {
                            println!("Valid commands are: {:?}", self.commands);
                        }
                        "clear-tt" => {
                            self.solver.reset_transposition_table();
                        }
                        "quit" => {
                            break;
                        }
                        _ => {
                            println!("Don't know the command: {}", first);
                        }
                    }
                }

                input = String::from("");
            }
            Ok(())
        }

        fn handle_moves(
            &mut self,
            args: std::str::Split<char>,
            pos: &mut Position,
        ) -> Result<(), ParseIntError> {
            let moves = args
                .map(|m| m.parse())
                .collect::<Result<Vec<position::Column>, _>>()?;
            if position::play_result_ok(pos.play_sequence(&moves)) {
                println!("Played columns: {:?}", moves);
            }
            Ok(())
        }
        fn analyze(&mut self, pos: &Position) {
            let scores = self.solver.analyze(&pos, self.weak);
            print!("Scores for the columns: ");
            for score in scores {
                print!(" {} ", score);
            }
            println!();
        }

        fn solve(&mut self, pos: &Position) {
            let score = self.solver.solve(&pos, self.weak);
            if score > 0 {
                print!(
                    "Score: {}, which means '{}' can win",
                    score,
                    pos.current_player().1
                );
            } else if score < 0 {
                print!(
                    "Score: {}, which means '{}' can win",
                    score,
                    pos.current_player().0,
                );
            }
            if !self.weak {
                print!(" in {} move(s)", Solver::score_to_moves_to_win(&pos, score),);
            }
            if score == 0 {
                print!("Score: {}, which means it's a draw", score);
            }
            println!();
        }
    }
}

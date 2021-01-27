pub mod move_sorter;
pub mod position;
pub mod solver;
pub mod transposition_table;

pub mod game_solver {

    use std::io::{self, BufReader, Write};
    use std::{
        fs::{self, File},
        path::PathBuf,
    };
    use std::{io::prelude::*, vec};

    use std::num::ParseIntError;
    use std::time::Instant;

    use crate::position::{self, Position};
    use crate::solver::Solver;

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
                    "bench",
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
            print!("> ");
            io::stdout().flush()?;
            while let Ok(_) = io::stdin().read_line(&mut input) {
                let mut args = input.trim().split(' ');
                if let Some(first) = args.next() {
                    match &first.to_lowercase() as &str {
                        "moves" | "play" | "move" => {
                            if let Err(e) = self.handle_moves(args, &mut pos) {
                                eprintln!("Moves should be numbers, got: {}", e);
                            }
                            println!("\nCurrent position:");
                            pos.display_position();
                        }
                        "position" => {
                            pos = Position::new();
                            if let Err(e) = self.handle_moves(args, &mut pos) {
                                eprintln!("Moves should be numbers, got: {}", e);
                            }
                            println!("\nCurrent position:");
                            pos.display_position();
                        }
                        "solve" => {
                            let now = Instant::now();
                            self.solver.reset_node_count();
                            self.solve(&pos);
                            println!("\nNodes searched: {}", self.solver.get_node_count());
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
                            println!("Cleared transposition table");
                        }
                        "bench" => {
                            if let Err(e) = Self::handle_bench(args, self.weak) {
                                eprintln!("Problem running benches: {}", e);
                            }
                        }
                        "quit" => {
                            break;
                        }
                        _ => {
                            eprintln!("Don't know the command: {}", first);
                            println!("Valid commands are: {:?}", self.commands);
                        }
                    }
                }
                input = String::from("");
                print!("\n> ");
                io::stdout().flush()?;
            }
            Ok(())
        }

        fn handle_moves(
            &mut self,
            args: std::str::Split<char>,
            pos: &mut Position,
        ) -> std::result::Result<(), ParseIntError> {
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
            if let Some(mut max) = scores.get(0) {
                print!("\nScores for the playable columns: ");
                for score in &scores {
                    print!(" {} ", score);
                    if score > max {
                        max = score;
                    }
                }
                print!("\nThe best score is: {}", max);
                self.explain_score(pos, *max);
            } else {
                println!("No playable columns")
            }
            println!("\n");
        }

        fn solve(&mut self, pos: &Position) {
            let score = self.solver.solve(&pos, self.weak, true);
            print!("\nScore is {}", score);
            self.explain_score(pos, score);
            println!();
        }

        fn explain_score(&mut self, pos: &Position, score: isize) {
            if score > 0 {
                print!(", which means '{}' can win", pos.current_player().1);
            } else if score < 0 {
                print!(", which means '{}' can win", pos.current_player().0,);
            }
            if !self.weak {
                print!(" in {} move(s)", Solver::score_to_moves_to_win(&pos, score),);
            }
            if score == 0 {
                print!(", which means it's a draw");
            }
        }

        fn handle_bench(mut args: std::str::Split<char>, weak: bool) -> std::io::Result<()> {
            let first = args.next().ok_or(std::io::ErrorKind::InvalidInput)?;
            if first == "all" {
                let paths = fs::read_dir("./benchmark_files")?;
                for dir in paths {
                    bench_file(dir?.path(), args.next(), weak)?;
                }
            } else {
                bench_file(PathBuf::from(first), args.next(), weak)?;
            }
            Ok(())
        }
    }
    fn conv_score(score: isize, weak: bool) -> isize {
        if weak {
            if score > 0 {
                1
            } else if score < 0 {
                -1
            } else {
                0
            }
        } else {
            score
        }
    }

    fn average<T>(list: Vec<T>) -> f64
    where
        f64: std::convert::From<T>,
    {
        let mut sum = 0.0;
        let length = list.len();
        if length == 0 {
            return 0.0;
        }
        for el in list {
            let el: f64 = el.into();
            sum += el;
        }
        sum / (length as f64)
    }

    /// Calls solve on the positions in the file. Returns `Err` if
    /// the file couldn't be read. If `max_lines` is not `None`, it
    /// will only run the lines upto `max_lines`.
    ///
    /// The recorded times are averaged, as well as the number of nodes.
    /// These are then printed to std_out. If the solver returns the wrong
    /// score, an error message is printed, but the benchmark continues.
    pub fn bench_file(path: PathBuf, max_lines: Option<&str>, weak: bool) -> std::io::Result<()> {
        println!("\nStarting benchmark: {}", path.display());
        let file = File::open(path)?;
        let file = BufReader::new(file);
        if let Ok(max_lines) = max_lines.unwrap_or("0").parse() {
            let mut solver = Solver::new();
            let mut times = Vec::with_capacity(max_lines);
            let mut nodes = Vec::with_capacity(max_lines);
            for (i, line) in file.lines().enumerate() {
                let line = line?;
                let mut parts = line.trim().split(' ');
                if let Some(position_str) = parts.next() {
                    if let Some(pos) = Position::from_string(position_str) {
                        print!("\rProcessing line: {}...", i + 1);
                        io::stdout().flush().unwrap();
                        solver.reset_node_count();
                        let now = Instant::now();
                        let score = conv_score(solver.solve(&pos, weak, false), weak);
                        times.push(now.elapsed().as_secs_f64());
                        nodes.push(solver.get_node_count() as f64);
                        if let Some(expected_result) = parts.next() {
                            if let Ok(expected_result) = expected_result.parse::<isize>() {
                                if score != conv_score(expected_result, weak) {
                                    eprintln!(
                                        "Expected score: {}, got: {} in pos {} on line {}",
                                        conv_score(expected_result, weak),
                                        score,
                                        position_str,
                                        i
                                    );
                                }
                            }
                        }
                    }
                } else {
                    eprintln!("Couldn't parse line {}: {}", i, line);
                }
                if i + 1 == max_lines {
                    break;
                }
            }
            println!("\n\nFinished benchmark");
            println!("Average time: {:?}", average(times));
            println!("Average number of nodes: {:?}", average(nodes));
            Ok(())
        } else {
            eprintln!("Invalid number of max lines");
            Err(io::Error::from(io::ErrorKind::InvalidInput))
        }
    }
}

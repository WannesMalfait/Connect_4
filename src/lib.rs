#![allow(
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss
)]
pub mod move_sorter;
pub mod position;
pub mod solver;
pub mod transposition_table;

pub mod game_solver {

    use std::cmp::Ordering;
    use std::io::{self, BufReader, Write};
    use std::{
        fs::{self, File},
        path::PathBuf,
    };
    use std::{io::prelude::*, vec};

    use std::time::Instant;

    use crate::position::{self, Position};
    use crate::solver::Solver;
    use crate::transposition_table;

    pub struct Parser {
        solver: Solver,
        weak: bool,
    }

    enum Command {
        PlayMoves(Vec<position::Column>),
        SetPosition(Vec<position::Column>),
        Solve,
        Analyze,
        ToggleWeak,
        Help(Option<Box<Command>>),
        ClearTT,
        Bench(Option<PathBuf>, Option<usize>),
        LoadBook(PathBuf),
        Quit,
    }

    impl Parser {
        #[must_use]
        pub fn new(weak: bool) -> Self {
            Self {
                solver: Solver::new(None),
                weak,
            }
        }

        /// Parse the arguments into a [`Command`]. If `recurse` is true,
        /// also recursively parse the argument to [`Command::Help`] as a [`Command`].
        fn parse_command(mut args: std::str::Split<char>, recurse: bool) -> Option<Command> {
            let first = args.next()?;
            match &first.to_lowercase() as &str {
                "moves" | "play" | "move" => {
                    let moves = match args
                        .map(str::parse)
                        .collect::<Result<Vec<position::Column>, _>>()
                    {
                        Err(e) => {
                            eprintln!("Moves should be numbers, got: {}", e);
                            return None;
                        }
                        Ok(v) => v,
                    };
                    Some(Command::PlayMoves(moves))
                }
                "position" => {
                    let moves = match args
                        .map(str::parse)
                        .collect::<Result<Vec<position::Column>, _>>()
                    {
                        Err(e) => {
                            eprintln!("Moves should be numbers, got: {}", e);
                            return None;
                        }
                        Ok(v) => v,
                    };
                    Some(Command::SetPosition(moves))
                }
                "solve" => Some(Command::Solve),
                "analyze" => Some(Command::Analyze),
                "toggle-weak" => Some(Command::ToggleWeak),
                "help" => {
                    if recurse {
                        let command = Self::parse_command(args, false);
                        match command {
                            Some(command) => Some(Command::Help(Some(Box::new(command)))),
                            None => Some(Command::Help(None)),
                        }
                    } else {
                        Some(Command::Help(None))
                    }
                }
                "clear-tt" => Some(Command::ClearTT),
                "bench" => match args.next() {
                    None => {
                        if recurse {
                            eprintln!("Expected bench file path or 'all'");
                            None
                        } else {
                            Some(Command::Bench(None, None))
                        }
                    }
                    Some(path) => {
                        let max_lines = match args.next() {
                            None => None,
                            Some(num) => match num.parse::<usize>() {
                                Ok(n) => Some(n),
                                Err(e) => {
                                    eprintln!("Expected maximum number of lines to run ({e})");
                                    return None;
                                }
                            },
                        };
                        if path == "all" {
                            Some(Command::Bench(None, max_lines))
                        } else if std::path::Path::new(path).exists() {
                            Some(Command::Bench(Some(PathBuf::from(path)), max_lines))
                        } else {
                            eprintln!("Invalid path {path}");
                            None
                        }
                    }
                },
                "load-book" => {
                    if !recurse {
                        return Some(Command::LoadBook(PathBuf::from("")));
                    }
                    match args.next() {
                        None => {
                            let path = std::path::Path::new("./opening_book.book");
                            if path.exists() {
                                Some(Command::LoadBook(path.to_path_buf()))
                            } else {
                                eprintln!(
                                    "No opening book found in default path, please provide a path."
                                );
                                None
                            }
                        }
                        Some(p) => {
                            let path = std::path::Path::new(p);
                            if path.exists() {
                                Some(Command::LoadBook(path.to_path_buf()))
                            } else {
                                eprintln!("Invalid path to book: {p}");
                                None
                            }
                        }
                    }
                }
                "quit" => Some(Command::Quit),
                _ => {
                    eprintln!("Don't know the command: {}", first);
                    if recurse {
                        Some(Command::Help(None))
                    } else {
                        None
                    }
                }
            }
        }

        pub fn run(&mut self) -> io::Result<()> {
            let mut pos = Position::new();
            let mut input = String::new();
            print!("> ");
            io::stdout().flush()?;
            while io::stdin().read_line(&mut input).is_ok() {
                let args = input.trim().split(' ');
                if let Some(command) = Self::parse_command(args, true) {
                    match command {
                        Command::PlayMoves(moves) => {
                            if position::play_result_ok(pos.play_sequence(&moves)) {
                                println!("Played columns: {:?}", moves);
                            }
                            println!("\nCurrent position:");
                            pos.display_position();
                        }
                        Command::SetPosition(moves) => {
                            pos = Position::new();
                            if position::play_result_ok(pos.play_sequence(&moves)) {
                                println!("Played columns: {:?}", moves);
                            }
                            println!("\nCurrent position:");
                            pos.display_position();
                        }
                        Command::Solve => {
                            let now = Instant::now();
                            self.solver.reset_node_count();
                            self.solve(&pos);
                            println!("\nNodes searched: {}", self.solver.get_node_count());
                            println!("Took {:?}", now.elapsed());
                        }
                        Command::Analyze => {
                            let now = Instant::now();
                            self.solver.reset_node_count();
                            self.analyze(&pos);
                            println!("Nodes searched: {}", self.solver.get_node_count());
                            println!("Took {:?}", now.elapsed());
                        }
                        Command::ToggleWeak => {
                            self.weak = !self.weak;
                            println!("Weak set to {}", self.weak);
                        }
                        Command::Help(command) => {
                            if let Some(command) = command {
                                match *command {
                                    Command::PlayMoves(_) => {
                                        println!("moves/play/move <column> <column> ...");
                                        println!(
                                            "Play a sequence of moves from the current position"
                                        );
                                    }
                                    Command::SetPosition(_) => {
                                        println!("position <column> <column> ...");
                                        println!("Set up a position by playing a sequence of moves from the starting position");
                                    }
                                    Command::Solve => {
                                        println!("Solve the current position");
                                    }
                                    Command::Analyze => {
                                        println!("Analyze all the possible moves in the current position");
                                    }
                                    Command::ToggleWeak => {
                                        println!("Toggle using the weak or strong solver.");
                                        println!("A weak solver only calculates win/draw/loss but not in how many moves");
                                    }
                                    Command::Help(_) => {
                                        println!("help <command>");
                                        println!("Get help about a specific command");
                                    }
                                    Command::ClearTT => {
                                        println!(
                                            "Clear the transposition table used by the solver."
                                        );
                                    }
                                    Command::Bench(_, _) => {
                                        println!("bench <path> | 'all' [max_lines] ");
                                        println!("Run the benchmarks in the given file.");
                                        println!(
                                            "Use 'all' instead of a path to run all benchmarks."
                                        );
                                        println!("A number max_lines can be specified to only solve at most that many positions per file.");
                                    }
                                    Command::LoadBook(_) => {
                                        println!("load-book [path]");
                                        println!("Load opening book from file.");
                                        println!("If path is not given the default path './opening_book.book' is used.");
                                    }
                                    Command::Quit => {
                                        println!("Quit the program.");
                                    }
                                }
                            } else {
                                println!(
                                    "Valid commands are: {:?}",
                                    vec![
                                        "moves/play/move",
                                        "position",
                                        "solve",
                                        "analyze",
                                        "toggle-weak",
                                        "help",
                                        "clear-tt",
                                        "bench",
                                        "load-book",
                                        "quit",
                                    ]
                                );
                                println!(
                                    "Type 'help <command>' for more info about a specific command"
                                );
                            }
                        }
                        Command::ClearTT => {
                            self.solver.reset_transposition_table();
                            println!("Cleared transposition table");
                        }
                        Command::Bench(path, max_lines) => {
                            if let Err(e) = Self::handle_bench(path, max_lines, self.weak) {
                                eprintln!("Error while running bench: '{e}'");
                            }
                        }
                        Command::LoadBook(path) => {
                            match transposition_table::OpeningBook::load(&path) {
                                Ok(book) => self.solver.set_book(book),
                                Err(e) => eprintln!("Error while loading book: '{e}'"),
                            }
                        }
                        Command::Quit => {
                            break;
                        }
                    }
                };
                input = String::from("");
                print!("\n> ");
                io::stdout().flush()?;
            }
            Ok(())
        }

        fn analyze(&mut self, pos: &Position) {
            let scores = self.solver.analyze(pos, self.weak);
            if let Some(mut max) = scores.first() {
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
                println!("No playable columns");
            }
            println!("\n");
        }

        fn solve(&mut self, pos: &Position) {
            let score = self.solver.solve(pos, self.weak, true);
            print!("\nScore is {}", score);
            self.explain_score(pos, score);
            println!();
        }

        fn explain_score(&mut self, pos: &Position, score: isize) {
            match score.cmp(&0) {
                Ordering::Greater => print!(", which means '{}' can win", pos.current_player().1),
                Ordering::Less => print!(", which means '{}' can win", pos.current_player().0),
                Ordering::Equal => (),
            }
            if !self.weak {
                print!(" in {} move(s)", Solver::score_to_moves_to_win(pos, score),);
            }
            if score == 0 {
                print!(", which means it's a draw");
            }
        }

        fn handle_bench(
            path: Option<PathBuf>,
            max_lines: Option<usize>,
            weak: bool,
        ) -> std::io::Result<()> {
            if let Some(path) = path {
                bench_file(path, max_lines, weak)?;
            } else {
                let paths = fs::read_dir("./benchmark_files")?;
                for dir in paths {
                    bench_file(dir?.path(), max_lines, weak)?;
                }
            }
            Ok(())
        }
    }
    fn conv_score(score: isize, weak: bool) -> isize {
        if weak {
            match score.cmp(&0) {
                Ordering::Greater => 1,
                Ordering::Less => -1,
                Ordering::Equal => 0,
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
    /// These are then printed to `std_out`. If the solver returns the wrong
    /// score, an error message is printed, but the benchmark continues.
    pub fn bench_file(path: PathBuf, max_lines: Option<usize>, weak: bool) -> std::io::Result<()> {
        println!("\nStarting benchmark: {}", path.display());
        let file = File::open(path)?;
        let file = BufReader::new(file);
        let max_lines = max_lines.unwrap_or_default();
        let mut solver = Solver::new(None);
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
    }
}

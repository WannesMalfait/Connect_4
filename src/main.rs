use connect_4::game_solver;
fn main() -> std::io::Result<()> {
    println!("Connect 4 solver by Pascal Pons ported to rust by Wannes Malfait");
    let mut parser = game_solver::Parser::new(false);
    parser.run()
}

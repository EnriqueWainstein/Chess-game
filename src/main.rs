fn main() {
    if let Err(error) = chess_game::uci::run_stdio() {
        eprintln!("UCI error: {error}");
        std::process::exit(1);
    }
}

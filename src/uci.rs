use std::io::{self, BufRead, Write};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::board::Board;
use crate::eval::{MATE_SCORE, MAX_PLY};
use crate::fen::{parse_fen, starting_position, to_fen};
use crate::history::{HistoryMoveError, HistoryOverflow, PositionHistory, make_move_with_history};
use crate::movegen::{MoveGenError, generate_legal, perft};
use crate::moves::Move;
use crate::search::{SearchLimits, SearchResult, search_with_info};
use crate::state::PositionState;
use crate::time::allocate_time;
use crate::types::Color;

const ENGINE_NAME: &str = "Chess Game";
const ENGINE_AUTHOR: &str = "Chess-game contributors";
const DEFAULT_DEPTH: u8 = 64;

pub fn run_stdio() -> io::Result<()> {
    run(io::stdin().lock(), io::stdout())
}

pub fn run<R: BufRead, W: Write + Send + 'static>(input: R, output: W) -> io::Result<()> {
    run_loop(input, Arc::new(Mutex::new(output)))
}

fn run_loop<R: BufRead, W: Write + Send + 'static>(
    input: R,
    output: Arc<Mutex<W>>,
) -> io::Result<()> {
    let (mut board, mut state) = starting_position();
    let mut history = PositionHistory::new(state.hash());
    let mut active: Option<ActiveSearch> = None;
    let mut quit = false;

    for line in input.lines() {
        let line = match line {
            Ok(line) => line,
            Err(error) => {
                eprintln!("UCI stdin error: {error}");
                break;
            }
        };
        let command = line.split_whitespace().collect::<Vec<_>>();
        if command.is_empty() {
            continue;
        }
        match command[0] {
            "uci" => {
                emit_line(&output, &format!("id name {ENGINE_NAME}"))?;
                emit_line(&output, &format!("id author {ENGINE_AUTHOR}"))?;
                emit_line(&output, "uciok")?;
            }
            "isready" => emit_line(&output, "readyok")?,
            "ucinewgame" => {
                finish_search(&mut active, true, &output)?;
                (board, state) = starting_position();
                history = PositionHistory::new(state.hash());
            }
            "position" => match parse_position(&command[1..]) {
                Ok((new_board, new_state, new_history)) => {
                    board = new_board;
                    state = new_state;
                    history = new_history;
                }
                Err(error) => eprintln!("position error: {error}"),
            },
            "go" if command.get(1) == Some(&"perft") => {
                finish_search(&mut active, true, &output)?;
                match command.get(2).and_then(|depth| depth.parse::<u8>().ok()) {
                    Some(depth) if command.len() == 3 => {
                        let mut perft_board = board;
                        let mut perft_state = state;
                        match perft(&mut perft_board, &mut perft_state, depth) {
                            Ok(nodes) => {
                                emit_line(&output, &format!("info string perft nodes {nodes}"))?;
                            }
                            Err(error) => eprintln!("perft error: {error}"),
                        }
                    }
                    _ => eprintln!("go perft requires exactly one depth from 0 to 255"),
                }
            }
            "go" => {
                finish_search(&mut active, true, &output)?;
                match parse_go(&command[1..], state.side_to_move()) {
                    Ok(go) => match start_search(board, state, history, go, Arc::clone(&output)) {
                        Ok(search) => active = Some(search),
                        Err(error) => eprintln!("search setup error: {error}"),
                    },
                    Err(error) => eprintln!("go error: {error}"),
                }
            }
            "stop" => finish_search(&mut active, true, &output)?,
            "d" => emit_diagnostic(&output, &board, &state)?,
            "quit" => {
                finish_search(&mut active, true, &output)?;
                quit = true;
                break;
            }
            "setoption" | "ponderhit" => {}
            _ => {}
        }
    }

    if !quit {
        finish_search(&mut active, true, &output)?;
    }
    Ok(())
}

#[derive(Debug)]
struct ActiveSearch {
    stop: Arc<AtomicBool>,
    fallback: Option<Move>,
    handle: JoinHandle<()>,
}

fn start_search<W: Write + Send + 'static>(
    board: Board,
    state: PositionState,
    history: PositionHistory,
    go: GoCommand,
    output: Arc<Mutex<W>>,
) -> Result<ActiveSearch, MoveGenError> {
    let stop = Arc::new(AtomicBool::new(false));
    let worker_stop = Arc::clone(&stop);
    let fallback = generate_legal(&board, &state)?.get(0);
    let handle = thread::spawn(move || {
        let started = Instant::now();
        let result = search_with_info(
            &board,
            &state,
            &history,
            go.search_limits,
            &worker_stop,
            |result| {
                let elapsed = started.elapsed();
                if let Err(error) = emit_info(&output, result, elapsed) {
                    eprintln!("UCI output error: {error}");
                    worker_stop.store(true, Ordering::Relaxed);
                }
                if go.soft_time.is_some_and(|soft| elapsed >= soft) {
                    worker_stop.store(true, Ordering::Relaxed);
                }
            },
        );
        let best_move = match result {
            Ok(result) => result.best_move.or(fallback),
            Err(error) => {
                eprintln!("search error: {error}");
                fallback
            }
        };
        let line =
            best_move.map_or_else(|| "bestmove 0000".to_owned(), |mv| format!("bestmove {mv}"));
        if let Err(error) = emit_line(&output, &line) {
            eprintln!("UCI output error: {error}");
        }
    });
    Ok(ActiveSearch {
        stop,
        fallback,
        handle,
    })
}

fn finish_search<W: Write + Send + 'static>(
    active: &mut Option<ActiveSearch>,
    request_stop: bool,
    output: &Arc<Mutex<W>>,
) -> io::Result<()> {
    let Some(search) = active.take() else {
        return Ok(());
    };
    if request_stop {
        search.stop.store(true, Ordering::Relaxed);
    }
    if search.handle.join().is_err() {
        eprintln!("search thread panicked");
        let line = search
            .fallback
            .map_or_else(|| "bestmove 0000".to_owned(), |mv| format!("bestmove {mv}"));
        emit_line(output, &line)?;
    }
    Ok(())
}

fn emit_info<W: Write>(
    output: &Mutex<W>,
    result: SearchResult,
    elapsed: Duration,
) -> io::Result<()> {
    let score = if result.score.abs() >= MATE_SCORE - i32::from(MAX_PLY) {
        let distance = if result.score > 0 {
            (MATE_SCORE - result.score + 1) / 2
        } else {
            -((MATE_SCORE + result.score + 1) / 2)
        };
        format!("score mate {distance}")
    } else {
        format!("score cp {}", result.score)
    };
    emit_line(
        output,
        &format!(
            "info depth {} {score} nodes {} time {}",
            result.depth,
            result.nodes,
            elapsed.as_millis()
        ),
    )
}

fn emit_diagnostic<W: Write>(
    output: &Mutex<W>,
    board: &Board,
    state: &PositionState,
) -> io::Result<()> {
    for line in board.to_string().lines() {
        emit_line(output, &format!("info string {line}"))?;
    }
    emit_line(output, &format!("info string fen {}", to_fen(board, state)))?;
    emit_line(output, &format!("info string hash {:016x}", state.hash()))
}

fn emit_line<W: Write>(output: &Mutex<W>, line: &str) -> io::Result<()> {
    let mut output = output
        .lock()
        .map_err(|_| io::Error::other("UCI output lock poisoned"))?;
    writeln!(output, "{line}")?;
    output.flush()
}

#[derive(Debug)]
struct GoCommand {
    search_limits: SearchLimits,
    soft_time: Option<Duration>,
}

fn parse_go(tokens: &[&str], side_to_move: Color) -> Result<GoCommand, String> {
    let mut depth = None;
    let mut movetime = None;
    let mut white_time = None;
    let mut black_time = None;
    let mut white_increment = Duration::ZERO;
    let mut black_increment = Duration::ZERO;
    let mut moves_to_go = None;
    let mut infinite = false;
    let mut index = 0;
    while index < tokens.len() {
        let token = tokens[index];
        if token == "infinite" {
            infinite = true;
            index += 1;
            continue;
        }
        if !matches!(
            token,
            "depth" | "movetime" | "wtime" | "btime" | "winc" | "binc" | "movestogo"
        ) {
            index += 1;
            continue;
        }
        let value = tokens
            .get(index + 1)
            .ok_or_else(|| format!("missing value after {token}"))?
            .parse::<u64>()
            .map_err(|_| format!("invalid numeric value for {token}"))?;
        match token {
            "depth" if value > 0 && value <= u64::from(MAX_PLY) => depth = Some(value as u8),
            "depth" => return Err(format!("depth must be from 1 to {MAX_PLY}")),
            "movetime" => movetime = Some(Duration::from_millis(value)),
            "wtime" => white_time = Some(Duration::from_millis(value)),
            "btime" => black_time = Some(Duration::from_millis(value)),
            "winc" => white_increment = Duration::from_millis(value),
            "binc" => black_increment = Duration::from_millis(value),
            "movestogo" if value > 0 && value <= u64::from(u32::MAX) => {
                moves_to_go = Some(value as u32)
            }
            "movestogo" => return Err("movestogo must be a positive integer".to_owned()),
            _ => {}
        }
        index += 2;
    }

    let (time_budget, infinite) = if let Some(movetime) = movetime {
        (
            Some(allocate_time(movetime, Duration::ZERO, Some(1))),
            false,
        )
    } else if infinite {
        (None, true)
    } else {
        let (remaining, increment) = match side_to_move {
            Color::White => (white_time.unwrap_or(Duration::ZERO), white_increment),
            Color::Black => (black_time.unwrap_or(Duration::ZERO), black_increment),
        };
        if white_time.is_some() || black_time.is_some() {
            (
                Some(allocate_time(remaining, increment, moves_to_go)),
                false,
            )
        } else {
            (None, false)
        }
    };
    Ok(GoCommand {
        search_limits: SearchLimits {
            max_depth: if infinite {
                MAX_PLY
            } else {
                depth.unwrap_or(DEFAULT_DEPTH)
            },
            time_limit: time_budget.map(|budget| budget.hard),
        },
        soft_time: time_budget.map(|budget| budget.soft),
    })
}

fn parse_position(tokens: &[&str]) -> Result<(Board, PositionState, PositionHistory), String> {
    let (mut board, mut state, mut index) = match tokens.first().copied() {
        Some("startpos") => {
            let (board, state) = starting_position();
            (board, state, 1)
        }
        Some("fen") if tokens.len() >= 7 => {
            let fen = tokens[1..7].join(" ");
            let (board, state) = parse_fen(&fen).map_err(|error| error.to_string())?;
            (board, state, 7)
        }
        Some("fen") => return Err("position fen requires six FEN fields".to_owned()),
        _ => return Err("position requires startpos or fen".to_owned()),
    };
    let mut history = PositionHistory::new(state.hash());
    if index == tokens.len() {
        return Ok((board, state, history));
    }
    if tokens[index] != "moves" {
        return Err(format!("unexpected position argument: {}", tokens[index]));
    }
    index += 1;
    for text in &tokens[index..] {
        let mv = parse_uci_move(&board, &state, text)?;
        make_move_with_history(&mut board, &mut state, mv, &mut history).map_err(|error| {
            match error {
                HistoryMoveError::HistoryOverflow => HistoryOverflow.to_string(),
                other => other.to_string(),
            }
        })?;
    }
    Ok((board, state, history))
}

pub fn parse_uci_move(board: &Board, state: &PositionState, text: &str) -> Result<Move, String> {
    generate_legal(board, state)
        .map_err(|error| error.to_string())?
        .iter()
        .find(|mv| mv.to_string() == text)
        .copied()
        .ok_or_else(|| format!("illegal move: {text}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fen::STARTING_FEN;
    use crate::types::{PieceType, Square};

    #[test]
    fn position_commands_are_transactional_and_track_moves() {
        let (board, state, history) =
            parse_position(&["startpos", "moves", "e2e4", "e7e5"]).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(state.side_to_move(), Color::White);
        assert_eq!(
            board.piece_at(Square::E4),
            Some((Color::White, PieceType::Pawn))
        );
        assert_eq!(
            board.piece_at(Square::E5),
            Some((Color::Black, PieceType::Pawn))
        );
        assert!(parse_position(&["startpos", "moves", "e2e5"]).is_err());
        assert_eq!(
            STARTING_FEN,
            crate::fen::to_fen(&starting_position().0, &starting_position().1)
        );
    }

    #[test]
    fn uci_move_parser_handles_castling_and_promotions_by_legal_generation() {
        let (board, state) = parse_fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1").unwrap();
        assert_eq!(
            parse_uci_move(&board, &state, "e1g1").unwrap().to_string(),
            "e1g1"
        );
        let (board, state) = parse_fen("k7/4P3/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        assert_eq!(
            parse_uci_move(&board, &state, "e7e8q").unwrap().to_string(),
            "e7e8q"
        );
    }

    #[test]
    fn go_parser_handles_depth_infinite_and_clock_limits() {
        assert_eq!(
            parse_go(&["depth", "4"], Color::White)
                .unwrap()
                .search_limits
                .max_depth,
            4
        );
        assert_eq!(
            parse_go(&["infinite"], Color::White)
                .unwrap()
                .search_limits
                .max_depth,
            MAX_PLY
        );
        let timed = parse_go(
            &["wtime", "2000", "btime", "1000", "winc", "50"],
            Color::Black,
        )
        .unwrap();
        assert!(timed.search_limits.time_limit.is_some());
        assert!(timed.soft_time.unwrap() <= timed.search_limits.time_limit.unwrap());
    }
}

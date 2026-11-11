use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::board::Board;
use crate::eval::{INFINITY, MATE_SCORE, MAX_PLY, MAX_QUIESCENCE_PLY, evaluate, piece_value};
use crate::history::{
    HistoryMoveError, PositionHistory, make_move_with_history, unmake_move_with_history,
};
use crate::movegen::{MOVE_LIST_CAPACITY, MoveGenError, MoveList, generate_legal, is_in_check};
use crate::moves::{Move, MoveFlag};
use crate::rules::is_insufficient_material;
use crate::state::PositionState;
use crate::types::{PieceType, Square};

const TIME_CHECK_INTERVAL: u64 = 256;
const QUIET_MOVE_ORDER: i32 = 0;
const PROMOTION_MOVE_ORDER: i32 = 100_000;
const CAPTURE_MOVE_ORDER: i32 = 1_000_000;
const EMPTY_MOVE: Move = Move::new(Square::A1, Square::A1, MoveFlag::Quiet);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchLimits {
    pub max_depth: u8,
    pub time_limit: Option<Duration>,
}

impl SearchLimits {
    pub const fn new(max_depth: u8) -> Self {
        Self {
            max_depth,
            time_limit: None,
        }
    }

    pub const fn with_time_limit(mut self, time_limit: Duration) -> Self {
        self.time_limit = Some(time_limit);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchResult {
    pub best_move: Option<Move>,
    pub score: i32,
    pub depth: u8,
    pub nodes: u64,
}

#[derive(Debug)]
pub enum SearchError {
    Interrupted,
    MoveGeneration(MoveGenError),
    History(HistoryMoveError),
}

impl fmt::Display for SearchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Interrupted => write!(formatter, "search interrupted"),
            Self::MoveGeneration(error) => error.fmt(formatter),
            Self::History(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for SearchError {}

impl From<MoveGenError> for SearchError {
    fn from(error: MoveGenError) -> Self {
        Self::MoveGeneration(error)
    }
}

impl From<HistoryMoveError> for SearchError {
    fn from(error: HistoryMoveError) -> Self {
        Self::History(error)
    }
}

pub fn search(
    board: &Board,
    state: &PositionState,
    history: &PositionHistory,
    limits: SearchLimits,
    stop: &AtomicBool,
) -> Result<SearchResult, SearchError> {
    search_with_info(board, state, history, limits, stop, |_| {})
}

pub fn search_with_info(
    board: &Board,
    state: &PositionState,
    history: &PositionHistory,
    limits: SearchLimits,
    stop: &AtomicBool,
    mut on_iteration: impl FnMut(SearchResult),
) -> Result<SearchResult, SearchError> {
    let deadline = limits
        .time_limit
        .and_then(|duration| Instant::now().checked_add(duration));
    let mut context = SearchContext {
        deadline,
        stop,
        nodes: 0,
        alpha_beta: true,
        check_deadline: true,
    };
    let mut board = *board;
    let mut state = *state;
    let mut history = *history;
    let legal = generate_legal(&board, &state)?;
    if legal.is_empty() {
        return Ok(SearchResult {
            best_move: None,
            score: if is_in_check(&board, state.side_to_move()) {
                -MATE_SCORE
            } else {
                0
            },
            depth: 0,
            nodes: 0,
        });
    }
    if is_draw(&board, &state, &history) {
        return Ok(SearchResult {
            best_move: None,
            score: 0,
            depth: 0,
            nodes: 0,
        });
    }

    let fallback = ordered_moves(&board, &legal).get(0);
    let mut result = SearchResult {
        best_move: fallback,
        score: evaluate(&board, &state),
        depth: 0,
        nodes: 0,
    };
    let max_depth = limits.max_depth.clamp(1, MAX_PLY);
    for depth in 1..=max_depth {
        context.check_deadline = depth > 1;
        match search_root(&mut board, &mut state, &mut history, depth, &mut context) {
            Ok((best_move, score)) => {
                result.best_move = best_move;
                result.score = score;
                result.depth = depth;
                result.nodes = context.nodes;
                on_iteration(result);
            }
            Err(SearchError::Interrupted) => break,
            Err(error) => return Err(error),
        }
        if stop.load(Ordering::Relaxed)
            || context
                .deadline
                .is_some_and(|deadline| Instant::now() >= deadline)
        {
            break;
        }
    }
    result.nodes = context.nodes;
    Ok(result)
}

pub fn search_fixed_depth(
    board: &Board,
    state: &PositionState,
    history: &PositionHistory,
    depth: u8,
) -> Result<SearchResult, SearchError> {
    let stop = AtomicBool::new(false);
    search(board, state, history, SearchLimits::new(depth), &stop)
}

fn search_root(
    board: &mut Board,
    state: &mut PositionState,
    history: &mut PositionHistory,
    depth: u8,
    context: &mut SearchContext<'_>,
) -> Result<(Option<Move>, i32), SearchError> {
    context.poll()?;
    let moves = generate_legal(board, state)?;
    if moves.is_empty() {
        return Ok((
            None,
            if is_in_check(board, state.side_to_move()) {
                -MATE_SCORE
            } else {
                0
            },
        ));
    }
    if is_draw(board, state, history) {
        return Ok((None, 0));
    }

    let ordered = ordered_moves(board, &moves);
    let mut best_move = None;
    let mut best_score = -INFINITY;
    let mut alpha = -INFINITY;
    for &mv in ordered.iter() {
        let undo = make_move_with_history(board, state, mv, history)?;
        let child = negamax(
            board,
            state,
            history,
            depth - 1,
            SearchWindow::new(-INFINITY, -alpha),
            1,
            context,
        );
        unmake_move_with_history(board, state, mv, undo, history);
        let score = -child?;
        if score > best_score {
            best_score = score;
            best_move = Some(mv);
        }
        alpha = alpha.max(score);
    }
    Ok((best_move, best_score))
}

fn negamax(
    board: &mut Board,
    state: &mut PositionState,
    history: &mut PositionHistory,
    depth: u8,
    window: SearchWindow,
    ply: u8,
    context: &mut SearchContext<'_>,
) -> Result<i32, SearchError> {
    context.poll()?;
    let mut alpha = window.alpha;
    let beta = window.beta;
    if depth == 0 || ply >= MAX_PLY {
        return quiescence(
            board,
            state,
            history,
            SearchWindow::new(alpha, beta),
            ply,
            0,
            context,
        );
    }
    let moves = generate_legal(board, state)?;
    if moves.is_empty() {
        return Ok(if is_in_check(board, state.side_to_move()) {
            -MATE_SCORE + i32::from(ply)
        } else {
            0
        });
    }
    if is_draw(board, state, history) {
        return Ok(0);
    }

    let ordered = ordered_moves(board, &moves);
    let mut best_score = -INFINITY;
    for &mv in ordered.iter() {
        let undo = make_move_with_history(board, state, mv, history)?;
        let child = negamax(
            board,
            state,
            history,
            depth - 1,
            SearchWindow::new(-beta, -alpha),
            ply + 1,
            context,
        );
        unmake_move_with_history(board, state, mv, undo, history);
        let score = -child?;
        best_score = best_score.max(score);
        alpha = alpha.max(score);
        if context.alpha_beta && alpha >= beta {
            break;
        }
    }
    Ok(best_score)
}

fn quiescence(
    board: &mut Board,
    state: &mut PositionState,
    history: &mut PositionHistory,
    window: SearchWindow,
    ply: u8,
    qply: u8,
    context: &mut SearchContext<'_>,
) -> Result<i32, SearchError> {
    context.poll()?;
    let mut alpha = window.alpha;
    let beta = window.beta;
    if ply >= MAX_PLY {
        return Ok(evaluate(board, state));
    }
    let in_check = is_in_check(board, state.side_to_move());
    let legal = generate_legal(board, state)?;
    if legal.is_empty() {
        return Ok(if in_check {
            -MATE_SCORE + i32::from(ply)
        } else {
            0
        });
    }
    if is_draw(board, state, history) {
        return Ok(0);
    }

    let stand_pat = evaluate(board, state);
    if !in_check {
        if context.alpha_beta && stand_pat >= beta {
            return Ok(stand_pat);
        }
        alpha = alpha.max(stand_pat);
    }
    if qply >= MAX_QUIESCENCE_PLY && !in_check {
        return Ok(stand_pat);
    }

    let tactical = if in_check {
        legal
    } else {
        legal.captures().map_err(MoveGenError::MoveListOverflow)?
    };
    let ordered = ordered_moves(board, &tactical);
    let mut best_score = if in_check { -INFINITY } else { stand_pat };
    for &mv in ordered.iter() {
        let undo = make_move_with_history(board, state, mv, history)?;
        let child = quiescence(
            board,
            state,
            history,
            SearchWindow::new(-beta, -alpha),
            ply + 1,
            qply + 1,
            context,
        );
        unmake_move_with_history(board, state, mv, undo, history);
        let score = -child?;
        best_score = best_score.max(score);
        alpha = alpha.max(score);
        if context.alpha_beta && alpha >= beta {
            break;
        }
    }
    Ok(best_score)
}

fn is_draw(board: &Board, state: &PositionState, history: &PositionHistory) -> bool {
    state.halfmove_clock() >= 100
        || is_insufficient_material(board)
        || history.has_one_repetition(state.hash(), state.halfmove_clock())
}

#[derive(Clone, Copy)]
struct SearchWindow {
    alpha: i32,
    beta: i32,
}

impl SearchWindow {
    const fn new(alpha: i32, beta: i32) -> Self {
        Self { alpha, beta }
    }
}

struct SearchContext<'a> {
    deadline: Option<Instant>,
    stop: &'a AtomicBool,
    nodes: u64,
    alpha_beta: bool,
    check_deadline: bool,
}

impl SearchContext<'_> {
    fn poll(&mut self) -> Result<(), SearchError> {
        self.nodes = self.nodes.saturating_add(1);
        if self.stop.load(Ordering::Relaxed) {
            return Err(SearchError::Interrupted);
        }
        if self.check_deadline
            && self.nodes.is_multiple_of(TIME_CHECK_INTERVAL)
            && self
                .deadline
                .is_some_and(|deadline| Instant::now() >= deadline)
        {
            return Err(SearchError::Interrupted);
        }
        Ok(())
    }
}

struct OrderedMoves {
    moves: [Move; MOVE_LIST_CAPACITY],
    scores: [i32; MOVE_LIST_CAPACITY],
    len: usize,
}

impl OrderedMoves {
    fn new() -> Self {
        Self {
            moves: [EMPTY_MOVE; MOVE_LIST_CAPACITY],
            scores: [0; MOVE_LIST_CAPACITY],
            len: 0,
        }
    }

    fn push(&mut self, mv: Move, score: i32) {
        let mut index = self.len;
        while index > 0 && self.scores[index - 1] < score {
            self.moves[index] = self.moves[index - 1];
            self.scores[index] = self.scores[index - 1];
            index -= 1;
        }
        self.moves[index] = mv;
        self.scores[index] = score;
        self.len += 1;
    }

    fn iter(&self) -> impl Iterator<Item = &Move> {
        self.moves[..self.len].iter()
    }

    fn get(&self, index: usize) -> Option<Move> {
        self.moves.get(index).copied().filter(|_| index < self.len)
    }
}

fn ordered_moves(board: &Board, moves: &MoveList) -> OrderedMoves {
    let mut ordered = OrderedMoves::new();
    for &mv in moves {
        let moving_piece = board
            .piece_at(mv.from())
            .expect("generated legal move has an occupied source")
            .1;
        let (victim, attacker) = if mv.flag() == MoveFlag::EnPassant {
            (PieceType::Pawn, PieceType::Pawn)
        } else if mv.is_capture() {
            (
                board
                    .piece_at(mv.to())
                    .map(|(_, piece)| piece)
                    .expect("generated capture has a victim"),
                moving_piece,
            )
        } else {
            (PieceType::Pawn, moving_piece)
        };
        let score = if mv.is_capture() {
            CAPTURE_MOVE_ORDER + piece_value(victim) * 16 - piece_value(attacker)
        } else if let Some(promoted) = mv.promotion_piece() {
            PROMOTION_MOVE_ORDER + piece_value(promoted)
        } else {
            QUIET_MOVE_ORDER
        };
        ordered.push(mv, score);
    }
    ordered
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::MATE_SCORE;
    use crate::fen::{parse_fen, starting_position};
    use crate::history::PositionHistory;
    use crate::movegen::generate_legal;

    fn search_position(fen: &str, depth: u8) -> SearchResult {
        let (board, state) = parse_fen(fen).unwrap();
        let history = PositionHistory::new(state.hash());
        search_fixed_depth(&board, &state, &history, depth).unwrap()
    }

    #[test]
    fn alpha_beta_matches_unpruned_minimax_at_fixed_depth() {
        for fen in [
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            "4k3/8/8/3q4/8/8/8/3RK3 w - - 0 1",
        ] {
            let (board, state) = parse_fen(fen).unwrap();
            let history = PositionHistory::new(state.hash());
            let pruned = search_fixed_depth(&board, &state, &history, 2).unwrap();
            let unpruned = minimax_score(board, state, history, 2).unwrap();
            assert_eq!(pruned.score, unpruned, "{fen}");
        }
    }

    #[test]
    fn mvv_lva_orders_pawn_takes_queen_before_queen_takes_pawn() {
        let (board, state) = parse_fen("7k/8/8/3q1p2/2P1Q3/8/8/7K w - - 0 1").unwrap();
        let legal = generate_legal(&board, &state).unwrap();
        let ordered = ordered_moves(&board, &legal);
        assert_eq!(ordered.get(0).unwrap().to_string(), "c4d5");
        let first_quiet = ordered.iter().position(|mv| !mv.is_capture()).unwrap();
        assert!(ordered.iter().take(first_quiet).all(|mv| mv.is_capture()));
    }

    #[test]
    fn annex_mate_in_one_and_hanging_queen_are_found() {
        let mate = search_position("6k1/5ppp/8/8/8/8/5PPP/R5K1 w - - 0 1", 3);
        assert_eq!(mate.best_move.unwrap().to_string(), "a1a8");
        assert_eq!(mate.score, MATE_SCORE - 1);

        let queen = search_position("4k3/8/8/3q4/8/8/8/3RK3 w - - 0 1", 2);
        assert_eq!(queen.best_move.unwrap().to_string(), "d1d5");
    }

    #[test]
    fn quiescence_avoids_the_poisoned_pawn() {
        let result = search_position("4k3/8/4p3/3p4/8/8/3Q4/4K3 w - - 0 1", 1);
        assert_ne!(result.best_move.unwrap().to_string(), "d2d5");
    }

    #[test]
    fn checkmate_stalemate_and_draw_positions_have_expected_scores() {
        let mate = search_position("7k/6Q1/6K1/8/8/8/8/8 b - - 0 1", 3);
        assert_eq!(mate.best_move, None);
        assert_eq!(mate.score, -MATE_SCORE);

        let stalemate = search_position("7k/5Q2/6K1/8/8/8/8/8 b - - 0 1", 3);
        assert_eq!(stalemate.best_move, None);
        assert_eq!(stalemate.score, 0);

        let fifty_move = search_position("4k3/8/8/8/8/8/8/4K2R w - - 100 80", 3);
        assert_eq!(fifty_move.best_move, None);
        assert_eq!(fifty_move.score, 0);
    }

    #[test]
    fn iterative_search_returns_a_legal_move_under_tiny_time_and_stop_signal() {
        let (board, state) = starting_position();
        let history = PositionHistory::new(state.hash());
        let stopped = AtomicBool::new(false);
        let timed = search(
            &board,
            &state,
            &history,
            SearchLimits::new(8).with_time_limit(Duration::ZERO),
            &stopped,
        )
        .unwrap();
        assert_eq!(timed.depth, 1);
        assert_legal_result(&board, &state, timed);

        stopped.store(true, Ordering::Relaxed);
        let stopped_result =
            search(&board, &state, &history, SearchLimits::new(8), &stopped).unwrap();
        assert_eq!(stopped_result.depth, 0);
        assert_legal_result(&board, &state, stopped_result);
    }

    #[test]
    fn external_stop_interrupts_an_active_iterative_search() {
        let (board, state) = starting_position();
        let history = PositionHistory::new(state.hash());
        let stop = std::sync::Arc::new(AtomicBool::new(false));
        let signal = std::sync::Arc::clone(&stop);
        let worker = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(1));
            signal.store(true, Ordering::Relaxed);
        });
        let result = search(&board, &state, &history, SearchLimits::new(MAX_PLY), &stop).unwrap();
        worker.join().unwrap();
        assert!(result.depth < MAX_PLY);
        assert_legal_result(&board, &state, result);
    }

    #[test]
    fn fixed_depth_search_is_deterministic() {
        let (board, state) = starting_position();
        let history = PositionHistory::new(state.hash());
        let first = search_fixed_depth(&board, &state, &history, 3).unwrap();
        let second = search_fixed_depth(&board, &state, &history, 3).unwrap();
        assert_eq!(first, second);
    }

    fn assert_legal_result(board: &Board, state: &PositionState, result: SearchResult) {
        let move_list = generate_legal(board, state).unwrap();
        assert!(
            result
                .best_move
                .is_some_and(|mv| move_list.iter().any(|legal| *legal == mv))
        );
    }

    fn minimax_score(
        mut board: Board,
        mut state: PositionState,
        mut history: PositionHistory,
        depth: u8,
    ) -> Result<i32, SearchError> {
        let stop = AtomicBool::new(false);
        let mut context = SearchContext {
            deadline: None,
            stop: &stop,
            nodes: 0,
            alpha_beta: false,
            check_deadline: false,
        };
        let moves = generate_legal(&board, &state)?;
        if moves.is_empty() {
            return Ok(if is_in_check(&board, state.side_to_move()) {
                -MATE_SCORE
            } else {
                0
            });
        }
        if is_draw(&board, &state, &history) {
            return Ok(0);
        }
        let ordered = ordered_moves(&board, &moves);
        let mut best_score = -INFINITY;
        for &mv in ordered.iter() {
            let undo = make_move_with_history(&mut board, &mut state, mv, &mut history)?;
            let child = negamax(
                &mut board,
                &mut state,
                &mut history,
                depth.saturating_sub(1),
                SearchWindow::new(-INFINITY, INFINITY),
                1,
                &mut context,
            );
            unmake_move_with_history(&mut board, &mut state, mv, undo, &mut history);
            best_score = best_score.max(-child?);
        }
        Ok(best_score)
    }
}

use std::fmt;

use crate::board::Board;
use crate::makemove::{MoveError, UndoMove, make_move, unmake_move};
use crate::moves::Move;
use crate::state::PositionState;
use crate::types::PieceType;

pub const HISTORY_CAPACITY: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PositionHistory {
    hashes: [u64; HISTORY_CAPACITY],
    len: usize,
}

impl PositionHistory {
    pub const fn new(initial_hash: u64) -> Self {
        let mut hashes = [0; HISTORY_CAPACITY];
        hashes[0] = initial_hash;
        Self { hashes, len: 1 }
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn as_slice(&self) -> &[u64] {
        &self.hashes[..self.len]
    }

    pub fn repetition_count(&self, current_hash: u64, reversible_plies: u8) -> usize {
        if self.len == 0 {
            return 0;
        }
        let max_distance = usize::from(reversible_plies).min(self.len - 1);
        let mut count = 0;
        let mut distance = 0;
        while distance <= max_distance {
            if self.hashes[self.len - 1 - distance] == current_hash {
                count += 1;
            }
            distance += 2;
        }
        count
    }

    pub fn has_one_repetition(&self, current_hash: u64, reversible_plies: u8) -> bool {
        self.repetition_count(current_hash, reversible_plies) >= 2
    }

    pub fn is_threefold_repetition(&self, current_hash: u64, reversible_plies: u8) -> bool {
        self.repetition_count(current_hash, reversible_plies) >= 3
    }

    fn push(&mut self, hash: u64, irreversible: bool) -> Result<HistoryUndo, HistoryOverflow> {
        let previous_len = self.len;
        let index = if irreversible { 0 } else { self.len };
        if index == HISTORY_CAPACITY {
            return Err(HistoryOverflow);
        }
        let undo = HistoryUndo {
            index,
            previous_hash: self.hashes[index],
            previous_len,
        };
        self.hashes[index] = hash;
        self.len = index + 1;
        Ok(undo)
    }

    fn pop(&mut self, undo: HistoryUndo) {
        self.hashes[undo.index] = undo.previous_hash;
        self.len = undo.previous_len;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HistoryUndo {
    index: usize,
    previous_hash: u64,
    previous_len: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HistoryMoveUndo {
    move_undo: UndoMove,
    history_undo: HistoryUndo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HistoryOverflow;

impl fmt::Display for HistoryOverflow {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "position history exceeds capacity of {HISTORY_CAPACITY}"
        )
    }
}

impl std::error::Error for HistoryOverflow {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryMoveError {
    Move(MoveError),
    HistoryOverflow,
}

impl fmt::Display for HistoryMoveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Move(error) => error.fmt(formatter),
            Self::HistoryOverflow => HistoryOverflow.fmt(formatter),
        }
    }
}

impl std::error::Error for HistoryMoveError {}

impl From<MoveError> for HistoryMoveError {
    fn from(error: MoveError) -> Self {
        Self::Move(error)
    }
}

pub fn make_move_with_history(
    board: &mut Board,
    state: &mut PositionState,
    mv: Move,
    history: &mut PositionHistory,
) -> Result<HistoryMoveUndo, HistoryMoveError> {
    let moving_piece = board.piece_at(mv.from()).map(|(_, piece)| piece);
    let move_undo = make_move(board, state, mv)?;
    let irreversible = moving_piece == Some(PieceType::Pawn) || move_undo.captured.is_some();
    let history_undo = match history.push(state.hash(), irreversible) {
        Ok(undo) => undo,
        Err(_) => {
            unmake_move(board, state, mv, move_undo);
            return Err(HistoryMoveError::HistoryOverflow);
        }
    };
    Ok(HistoryMoveUndo {
        move_undo,
        history_undo,
    })
}

pub fn unmake_move_with_history(
    board: &mut Board,
    state: &mut PositionState,
    mv: Move,
    undo: HistoryMoveUndo,
    history: &mut PositionHistory,
) {
    history.pop(undo.history_undo);
    unmake_move(board, state, mv, undo.move_undo);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fen::{parse_fen, starting_position};
    use crate::movegen::generate_legal;
    use crate::moves::MoveFlag;
    use crate::rules::{GameStatus, game_status};
    use crate::types::Square;

    fn apply_uci(
        board: &mut Board,
        state: &mut PositionState,
        history: &mut PositionHistory,
        uci: &str,
    ) -> HistoryMoveUndo {
        let legal = generate_legal(board, state).unwrap();
        let mv = legal
            .iter()
            .find(|mv| mv.to_string() == uci)
            .copied()
            .expect("test move is legal");
        make_move_with_history(board, state, mv, history).unwrap()
    }

    #[test]
    fn knight_shuffle_reports_one_and_threefold_repetition() {
        let (mut board, mut state) = starting_position();
        let mut history = PositionHistory::new(state.hash());
        let start_hash = state.hash();
        let mut undos = Vec::new();
        for uci in [
            "g1f3", "g8f6", "f3g1", "f6g8", "g1f3", "g8f6", "f3g1", "f6g8",
        ] {
            undos.push(apply_uci(&mut board, &mut state, &mut history, uci));
        }
        assert_eq!(state.hash(), start_hash);
        assert!(history.has_one_repetition(state.hash(), state.halfmove_clock()));
        assert!(history.is_threefold_repetition(state.hash(), state.halfmove_clock()));
        assert_eq!(
            game_status(&board, &state, &history).unwrap(),
            GameStatus::DrawByRepetition
        );

        for (mv, undo) in [
            Move::new(Square::F6, Square::G8, MoveFlag::Quiet),
            Move::new(Square::F3, Square::G1, MoveFlag::Quiet),
            Move::new(Square::G8, Square::F6, MoveFlag::Quiet),
            Move::new(Square::G1, Square::F3, MoveFlag::Quiet),
            Move::new(Square::F6, Square::G8, MoveFlag::Quiet),
            Move::new(Square::F3, Square::G1, MoveFlag::Quiet),
            Move::new(Square::G8, Square::F6, MoveFlag::Quiet),
            Move::new(Square::G1, Square::F3, MoveFlag::Quiet),
        ]
        .into_iter()
        .zip(undos.into_iter().rev())
        {
            unmake_move_with_history(&mut board, &mut state, mv, undo, &mut history);
        }
        assert_eq!(history.len(), 1);
        assert_eq!(history.as_slice(), &[start_hash]);
    }

    #[test]
    fn a_different_position_does_not_count_as_repetition() {
        let (mut board, mut state) = starting_position();
        let mut history = PositionHistory::new(state.hash());
        for uci in ["g1f3", "g8f6", "f3g1", "f6h5"] {
            apply_uci(&mut board, &mut state, &mut history, uci);
        }
        assert_ne!(state.hash(), history.as_slice()[0]);
        assert!(!history.is_threefold_repetition(state.hash(), state.halfmove_clock()));
    }

    #[test]
    fn irreversible_moves_start_a_new_repetition_window_and_unmake_restores_it() {
        let (mut board, mut state) =
            parse_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        let root_hash = state.hash();
        let mut history = PositionHistory::new(root_hash);
        for uci in ["g1f3", "g8f6", "f3g1", "f6g8"] {
            apply_uci(&mut board, &mut state, &mut history, uci);
        }
        let before_irreversible = history;
        let undo = apply_uci(&mut board, &mut state, &mut history, "e2e4");
        assert_eq!(history.len(), 1);
        assert_eq!(history.as_slice(), &[state.hash()]);
        assert!(!history.is_threefold_repetition(state.hash(), state.halfmove_clock()));
        unmake_move_with_history(
            &mut board,
            &mut state,
            Move::new(Square::E2, Square::E4, MoveFlag::DoublePawnPush),
            undo,
            &mut history,
        );
        assert_eq!(history, before_irreversible);
    }

    #[test]
    fn full_history_overflow_is_reported_without_changing_position_or_history() {
        let (mut board, mut state) = starting_position();
        let mut history = PositionHistory {
            hashes: [state.hash(); HISTORY_CAPACITY],
            len: HISTORY_CAPACITY,
        };
        let original_board = board;
        let original_state = state;
        let original_history = history;
        let mv = Move::new(Square::G1, Square::F3, MoveFlag::Quiet);

        assert_eq!(
            make_move_with_history(&mut board, &mut state, mv, &mut history),
            Err(HistoryMoveError::HistoryOverflow)
        );
        assert_eq!(board, original_board);
        assert_eq!(state, original_state);
        assert_eq!(history, original_history);
    }
}

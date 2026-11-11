use crate::board::Board;
use crate::history::PositionHistory;
use crate::movegen::{MoveGenError, generate_legal, is_in_check};
use crate::state::PositionState;
use crate::types::{Color, PieceType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameStatus {
    Ongoing,
    Checkmate { winner: Color },
    Stalemate,
    DrawByFiftyMoveRule,
    DrawByInsufficientMaterial,
    DrawByRepetition,
}

pub fn is_checkmate(board: &Board, state: &PositionState) -> Result<bool, MoveGenError> {
    Ok(generate_legal(board, state)?.is_empty() && is_in_check(board, state.side_to_move()))
}

pub fn is_stalemate(board: &Board, state: &PositionState) -> Result<bool, MoveGenError> {
    Ok(generate_legal(board, state)?.is_empty() && !is_in_check(board, state.side_to_move()))
}

pub const fn is_fifty_move_draw(state: &PositionState) -> bool {
    state.halfmove_clock() >= 100
}

pub fn is_insufficient_material(board: &Board) -> bool {
    if board.bitboard(PieceType::Pawn, Color::White) != 0
        || board.bitboard(PieceType::Pawn, Color::Black) != 0
        || board.bitboard(PieceType::Rook, Color::White) != 0
        || board.bitboard(PieceType::Rook, Color::Black) != 0
        || board.bitboard(PieceType::Queen, Color::White) != 0
        || board.bitboard(PieceType::Queen, Color::Black) != 0
    {
        return false;
    }

    let minors = board.bitboard(PieceType::Bishop, Color::White)
        | board.bitboard(PieceType::Bishop, Color::Black)
        | board.bitboard(PieceType::Knight, Color::White)
        | board.bitboard(PieceType::Knight, Color::Black);
    minors.count_ones() <= 1
}

pub fn is_repetition(state: &PositionState, history: &PositionHistory) -> bool {
    history.has_one_repetition(state.hash(), state.halfmove_clock())
}

pub fn is_threefold_repetition(state: &PositionState, history: &PositionHistory) -> bool {
    history.is_threefold_repetition(state.hash(), state.halfmove_clock())
}

pub fn game_status(
    board: &Board,
    state: &PositionState,
    history: &PositionHistory,
) -> Result<GameStatus, MoveGenError> {
    let moves = generate_legal(board, state)?;
    if moves.is_empty() {
        return Ok(if is_in_check(board, state.side_to_move()) {
            GameStatus::Checkmate {
                winner: state.side_to_move().opposite(),
            }
        } else {
            GameStatus::Stalemate
        });
    }
    if is_fifty_move_draw(state) {
        return Ok(GameStatus::DrawByFiftyMoveRule);
    }
    if is_insufficient_material(board) {
        return Ok(GameStatus::DrawByInsufficientMaterial);
    }
    if is_threefold_repetition(state, history) {
        return Ok(GameStatus::DrawByRepetition);
    }
    Ok(GameStatus::Ongoing)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::bitboard;
    use crate::fen::parse_fen;

    #[test]
    fn annex_mate_and_stalemate_positions_are_classified() {
        let (board, state) = parse_fen("7k/6Q1/6K1/8/8/8/8/8 b - - 0 1").unwrap();
        let history = PositionHistory::new(state.hash());
        assert_eq!(
            game_status(&board, &state, &history).unwrap(),
            GameStatus::Checkmate {
                winner: Color::White
            }
        );
        assert!(is_checkmate(&board, &state).unwrap());

        let (board, state) = parse_fen("7k/5Q2/6K1/8/8/8/8/8 b - - 0 1").unwrap();
        let history = PositionHistory::new(state.hash());
        assert_eq!(
            game_status(&board, &state, &history).unwrap(),
            GameStatus::Stalemate
        );
        assert!(is_stalemate(&board, &state).unwrap());
    }

    #[test]
    fn fifty_move_draw_does_not_override_checkmate() {
        let (board, state) = parse_fen("7k/6Q1/6K1/8/8/8/8/8 b - - 100 80").unwrap();
        let history = PositionHistory::new(state.hash());
        assert_eq!(
            game_status(&board, &state, &history).unwrap(),
            GameStatus::Checkmate {
                winner: Color::White
            }
        );

        let (board, state) = parse_fen("4k3/8/8/8/8/8/8/4K2R w - - 100 80").unwrap();
        let history = PositionHistory::new(state.hash());
        assert_eq!(
            game_status(&board, &state, &history).unwrap(),
            GameStatus::DrawByFiftyMoveRule
        );
    }

    #[test]
    fn only_kings_or_one_minor_are_insufficient_material() {
        for fen in [
            "4k3/8/8/8/8/8/8/4K3 w - - 0 1",
            "4k3/8/8/8/8/8/8/4KN2 w - - 0 1",
            "4k3/8/8/8/8/8/8/2B1K3 w - - 0 1",
        ] {
            let (board, _) = parse_fen(fen).unwrap();
            assert!(is_insufficient_material(&board), "{fen}");
        }
        for fen in [
            "4k3/8/8/8/8/8/8/4K2R w - - 0 1",
            "4k3/8/8/8/8/8/4P3/4K3 w - - 0 1",
            "4k3/8/8/8/8/8/8/2B1KN2 w - - 0 1",
        ] {
            let (board, _) = parse_fen(fen).unwrap();
            assert!(!is_insufficient_material(&board), "{fen}");
        }
    }

    #[test]
    fn material_test_counts_non_king_pieces_without_allocating() {
        let (board, _) = parse_fen("4k3/8/8/8/8/8/8/4KN2 w - - 0 1").unwrap();
        let minors = board.bitboard(PieceType::Knight, Color::White);
        assert_eq!(bitboard::count(minors), 1);
    }
}

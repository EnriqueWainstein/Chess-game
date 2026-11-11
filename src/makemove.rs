use std::fmt;

use crate::board::{Board, BoardError};
use crate::moves::{Move, MoveFlag};
use crate::state::{
    BLACK_KING_SIDE, BLACK_QUEEN_SIDE, PositionState, WHITE_KING_SIDE, WHITE_QUEEN_SIDE,
};
use crate::types::{Color, PieceType, Square};
use crate::zobrist::{
    capturable_en_passant_file, castling_key, en_passant_file_key, piece_key, side_key,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapturedPiece {
    pub piece: PieceType,
    pub square: Square,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UndoMove {
    pub captured: Option<CapturedPiece>,
    previous_state: PositionState,
}

impl UndoMove {
    pub const fn previous_state(self) -> PositionState {
        self.previous_state
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveError {
    InconsistentBoard,
    EmptySource(Square),
    WrongSideToMove(Square),
    OccupiedDestination(Square),
    MissingCapture(Square),
    InvalidMove(&'static str),
    Board(BoardError),
}

impl fmt::Display for MoveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InconsistentBoard => write!(formatter, "board bitboards are inconsistent"),
            Self::EmptySource(square) => write!(formatter, "no piece on source square {square}"),
            Self::WrongSideToMove(square) => {
                write!(formatter, "piece on {square} is not the side to move")
            }
            Self::OccupiedDestination(square) => {
                write!(formatter, "destination square {square} is occupied")
            }
            Self::MissingCapture(square) => {
                write!(formatter, "no opponent piece to capture on {square}")
            }
            Self::InvalidMove(reason) => write!(formatter, "invalid move: {reason}"),
            Self::Board(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for MoveError {}

impl From<BoardError> for MoveError {
    fn from(error: BoardError) -> Self {
        Self::Board(error)
    }
}

/// Applies a pseudo-legal move and returns the fixed-size data needed to undo it.
///
/// The caller is responsible for ensuring that the move obeys the piece's movement
/// rules and does not leave its king in check.
pub fn make_move(
    board: &mut Board,
    state: &mut PositionState,
    mv: Move,
) -> Result<UndoMove, MoveError> {
    debug_assert!(board.is_consistent());
    if mv.from() == mv.to() {
        return Err(MoveError::InvalidMove("source and destination are equal"));
    }

    let mover = state.side_to_move();
    let (piece_color, moving_piece) = board
        .piece_at(mv.from())
        .ok_or(MoveError::EmptySource(mv.from()))?;
    if piece_color != mover {
        return Err(MoveError::WrongSideToMove(mv.from()));
    }

    let captured = validate_move(board, *state, mv, moving_piece)?;
    let previous_state = *state;
    let old_castling_rights = state.castling_rights();
    let old_en_passant_file = capturable_en_passant_file(board, state);
    let mut hash = state.hash() ^ piece_key(mover, moving_piece, mv.from());
    if let Some(captured) = captured {
        hash ^= piece_key(mover.opposite(), captured.piece, captured.square);
    }
    let placed_piece = mv.flag().promotion_piece().unwrap_or(moving_piece);
    hash ^= piece_key(mover, placed_piece, mv.to());
    if matches!(
        mv.flag(),
        MoveFlag::KingSideCastle | MoveFlag::QueenSideCastle
    ) {
        let (rook_from, rook_to) = castle_rook_squares(mover, mv.flag());
        hash ^= piece_key(mover, PieceType::Rook, rook_from);
        hash ^= piece_key(mover, PieceType::Rook, rook_to);
    }
    let undo = UndoMove {
        captured,
        previous_state,
    };

    board.remove_piece(mv.from());
    if let Some(captured) = captured {
        board.remove_piece(captured.square);
    }

    match mv.flag() {
        MoveFlag::KingSideCastle | MoveFlag::QueenSideCastle => {
            let (rook_from, rook_to) = castle_rook_squares(mover, mv.flag());
            board.remove_piece(rook_from);
            board
                .place_piece(mover, PieceType::Rook, rook_to)
                .map_err(MoveError::from)?;
            board
                .place_piece(mover, PieceType::King, mv.to())
                .map_err(MoveError::from)?;
        }
        flag if flag.is_promotion() => {
            board
                .place_piece(
                    mover,
                    flag.promotion_piece().expect("promotion flag"),
                    mv.to(),
                )
                .map_err(MoveError::from)?;
        }
        _ => {
            board
                .place_piece(mover, moving_piece, mv.to())
                .map_err(MoveError::from)?;
        }
    }

    update_castling_rights(state, mover, moving_piece, mv.from(), captured);
    state.set_en_passant(match mv.flag() {
        MoveFlag::DoublePawnPush => Some(midpoint_square(mv.from(), mv.to())),
        _ => None,
    });
    state.set_halfmove_clock(if moving_piece == PieceType::Pawn || captured.is_some() {
        0
    } else {
        state.halfmove_clock().saturating_add(1)
    });
    if mover == Color::Black {
        state.set_fullmove_number(state.fullmove_number().saturating_add(1));
    }
    state.set_side_to_move(mover.opposite());
    hash ^= side_key();
    hash ^= castling_key(old_castling_rights) ^ castling_key(state.castling_rights());
    if let Some(file) = old_en_passant_file {
        hash ^= en_passant_file_key(file);
    }
    if let Some(file) = capturable_en_passant_file(board, state) {
        hash ^= en_passant_file_key(file);
    }
    state.set_hash(hash);

    debug_assert!(board.is_consistent());
    Ok(undo)
}

/// Restores a position using the move and the undo data returned by [`make_move`].
pub fn unmake_move(board: &mut Board, state: &mut PositionState, mv: Move, undo: UndoMove) {
    let mover = undo.previous_state.side_to_move();
    let moving_piece = match mv.flag().promotion_piece() {
        Some(_) => PieceType::Pawn,
        None => match mv.flag() {
            MoveFlag::KingSideCastle | MoveFlag::QueenSideCastle => PieceType::King,
            _ => {
                board
                    .remove_piece(mv.to())
                    .expect("destination contains the moved piece")
                    .1
            }
        },
    };

    if mv.flag().promotion_piece().is_some()
        || matches!(
            mv.flag(),
            MoveFlag::KingSideCastle | MoveFlag::QueenSideCastle
        )
    {
        board.remove_piece(mv.to());
    }
    board
        .place_piece(mover, moving_piece, mv.from())
        .expect("source square is clear after undoing the move");

    if matches!(
        mv.flag(),
        MoveFlag::KingSideCastle | MoveFlag::QueenSideCastle
    ) {
        let (rook_from, rook_to) = castle_rook_squares(mover, mv.flag());
        board.remove_piece(rook_to);
        board
            .place_piece(mover, PieceType::Rook, rook_from)
            .expect("rook source square is clear after undoing the castle");
    }
    if let Some(captured) = undo.captured {
        board
            .place_piece(mover.opposite(), captured.piece, captured.square)
            .expect("captured square is clear after undoing the move");
    }
    *state = undo.previous_state;
    debug_assert!(board.is_consistent());
}

fn validate_move(
    board: &Board,
    state: PositionState,
    mv: Move,
    moving_piece: PieceType,
) -> Result<Option<CapturedPiece>, MoveError> {
    let mover = state.side_to_move();
    let destination_piece = board.piece_at(mv.to());
    let captured = match mv.flag() {
        MoveFlag::Capture
        | MoveFlag::PromoteKnightCapture
        | MoveFlag::PromoteBishopCapture
        | MoveFlag::PromoteRookCapture
        | MoveFlag::PromoteQueenCapture => {
            let (color, piece) = destination_piece.ok_or(MoveError::MissingCapture(mv.to()))?;
            if color == mover || piece == PieceType::King {
                return Err(MoveError::InvalidMove(
                    "capture target must be an opponent non-king piece",
                ));
            }
            Some(CapturedPiece {
                piece,
                square: mv.to(),
            })
        }
        MoveFlag::EnPassant => {
            if destination_piece.is_some() {
                return Err(MoveError::OccupiedDestination(mv.to()));
            }
            let expected_rank = if mover == Color::White { 4 } else { 3 };
            let target_rank = if mover == Color::White { 5 } else { 2 };
            if moving_piece != PieceType::Pawn
                || mv.from().rank() != expected_rank
                || mv.to().rank() != target_rank
                || state.en_passant() != Some(mv.to())
                || mv.from().file().abs_diff(mv.to().file()) != 1
            {
                return Err(MoveError::InvalidMove("invalid en-passant capture"));
            }
            let captured_square = Square::from_file_rank(
                mv.to().file(),
                if mover == Color::White {
                    mv.to().rank() - 1
                } else {
                    mv.to().rank() + 1
                },
            )
            .expect("en-passant capture rank is in range");
            if board.piece_at(captured_square) != Some((mover.opposite(), PieceType::Pawn)) {
                return Err(MoveError::MissingCapture(captured_square));
            }
            Some(CapturedPiece {
                piece: PieceType::Pawn,
                square: captured_square,
            })
        }
        MoveFlag::Quiet
        | MoveFlag::DoublePawnPush
        | MoveFlag::KingSideCastle
        | MoveFlag::QueenSideCastle
        | MoveFlag::PromoteKnight
        | MoveFlag::PromoteBishop
        | MoveFlag::PromoteRook
        | MoveFlag::PromoteQueen => {
            if destination_piece.is_some() {
                return Err(MoveError::OccupiedDestination(mv.to()));
            }
            None
        }
    };

    match mv.flag() {
        MoveFlag::Quiet => {
            if moving_piece == PieceType::Pawn {
                let expected_rank = if mover == Color::White {
                    mv.from().rank().checked_add(1)
                } else {
                    mv.from().rank().checked_sub(1)
                };
                if matches!(mv.to().rank(), 0 | 7) {
                    return Err(MoveError::InvalidMove(
                        "pawn reaching last rank must promote",
                    ));
                }
                if mv.from().file() != mv.to().file() || expected_rank != Some(mv.to().rank()) {
                    return Err(MoveError::InvalidMove("invalid quiet pawn move"));
                }
            }
        }
        MoveFlag::DoublePawnPush => {
            let (start_rank, destination_rank) = if mover == Color::White {
                (1, 3)
            } else {
                (6, 4)
            };
            let middle_rank = (start_rank + destination_rank) / 2;
            if moving_piece != PieceType::Pawn
                || mv.from().rank() != start_rank
                || mv.to().rank() != destination_rank
                || mv.from().file() != mv.to().file()
                || board.is_occupied(
                    Square::from_file_rank(mv.from().file(), middle_rank)
                        .expect("double-push middle square is in range"),
                )
            {
                return Err(MoveError::InvalidMove("invalid double pawn push"));
            }
        }
        MoveFlag::KingSideCastle | MoveFlag::QueenSideCastle => {
            let home_rank = if mover == Color::White { 0 } else { 7 };
            let expected_to_file = if mv.flag() == MoveFlag::KingSideCastle {
                6
            } else {
                2
            };
            if moving_piece != PieceType::King
                || mv.from() != Square::from_file_rank(4, home_rank).expect("home square is valid")
                || mv.to()
                    != Square::from_file_rank(expected_to_file, home_rank).expect("target is valid")
            {
                return Err(MoveError::InvalidMove("invalid castling king move"));
            }
            let required_right = match (mover, mv.flag()) {
                (Color::White, MoveFlag::KingSideCastle) => WHITE_KING_SIDE,
                (Color::White, MoveFlag::QueenSideCastle) => WHITE_QUEEN_SIDE,
                (Color::Black, MoveFlag::KingSideCastle) => BLACK_KING_SIDE,
                (Color::Black, MoveFlag::QueenSideCastle) => BLACK_QUEEN_SIDE,
                _ => unreachable!("castling flags are matched"),
            };
            if !state.has_castling_right(required_right) {
                return Err(MoveError::InvalidMove("castling right is not available"));
            }
            let (rook_from, _) = castle_rook_squares(mover, mv.flag());
            if board.piece_at(rook_from) != Some((mover, PieceType::Rook)) {
                return Err(MoveError::InvalidMove("castling rook is missing"));
            }
            let files: &[u8] = if mv.flag() == MoveFlag::KingSideCastle {
                &[5, 6]
            } else {
                &[1, 2, 3]
            };
            if files.iter().any(|&file| {
                board.is_occupied(Square::from_file_rank(file, home_rank).expect("path is valid"))
            }) {
                return Err(MoveError::InvalidMove("castling path is occupied"));
            }
        }
        MoveFlag::Capture => {
            if moving_piece == PieceType::Pawn {
                let expected_rank = if mover == Color::White {
                    mv.from().rank().checked_add(1)
                } else {
                    mv.from().rank().checked_sub(1)
                };
                if matches!(mv.to().rank(), 0 | 7) {
                    return Err(MoveError::InvalidMove(
                        "pawn capture on last rank must promote",
                    ));
                }
                if expected_rank != Some(mv.to().rank())
                    || mv.from().file().abs_diff(mv.to().file()) != 1
                {
                    return Err(MoveError::InvalidMove("invalid pawn capture"));
                }
            }
        }
        MoveFlag::EnPassant => {}
        flag if flag.is_promotion() => {
            let source_rank = if mover == Color::White { 6 } else { 1 };
            let target_rank = if mover == Color::White { 7 } else { 0 };
            if moving_piece != PieceType::Pawn
                || mv.from().rank() != source_rank
                || mv.to().rank() != target_rank
                || (flag.is_capture() && mv.from().file().abs_diff(mv.to().file()) != 1)
                || (!flag.is_capture() && mv.from().file() != mv.to().file())
            {
                return Err(MoveError::InvalidMove("invalid pawn promotion"));
            }
        }
        _ => unreachable!("all move flags are covered"),
    }
    Ok(captured)
}

fn update_castling_rights(
    state: &mut PositionState,
    mover: Color,
    moving_piece: PieceType,
    from: Square,
    captured: Option<CapturedPiece>,
) {
    let mut rights = state.castling_rights();
    if moving_piece == PieceType::King {
        rights &= match mover {
            Color::White => !(WHITE_KING_SIDE | WHITE_QUEEN_SIDE),
            Color::Black => !(BLACK_KING_SIDE | BLACK_QUEEN_SIDE),
        };
    }
    if moving_piece == PieceType::Rook {
        rights &= !castling_right_for_rook_square(from);
    }
    if let Some(captured) = captured
        && captured.piece == PieceType::Rook
    {
        rights &= !castling_right_for_rook_square(captured.square);
    }
    state.set_castling_rights(rights);
}

fn castling_right_for_rook_square(square: Square) -> u8 {
    match square {
        Square::A1 => WHITE_QUEEN_SIDE,
        Square::H1 => WHITE_KING_SIDE,
        Square::A8 => BLACK_QUEEN_SIDE,
        Square::H8 => BLACK_KING_SIDE,
        _ => 0,
    }
}

fn castle_rook_squares(color: Color, flag: MoveFlag) -> (Square, Square) {
    let rank = if color == Color::White { 0 } else { 7 };
    match flag {
        MoveFlag::KingSideCastle => (
            Square::from_file_rank(7, rank).expect("rook home square is valid"),
            Square::from_file_rank(5, rank).expect("rook destination is valid"),
        ),
        MoveFlag::QueenSideCastle => (
            Square::from_file_rank(0, rank).expect("rook home square is valid"),
            Square::from_file_rank(3, rank).expect("rook destination is valid"),
        ),
        _ => unreachable!("only castling flags have rook squares"),
    }
}

fn midpoint_square(from: Square, to: Square) -> Square {
    Square::from_file_rank(
        from.file(),
        ((from.rank() as u16 + to.rank() as u16) / 2) as u8,
    )
    .expect("pawn double-push midpoint is in range")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fen::{parse_fen, to_fen};
    use crate::zobrist::calculate_hash;

    fn apply_and_undo(fen: &str, from: Square, to: Square, flag: MoveFlag) {
        let (mut board, mut state) = parse_fen(fen).unwrap();
        let original_board = board;
        let original_state = state;
        let mv = Move::new(from, to, flag);
        let undo = make_move(&mut board, &mut state, mv).unwrap();
        assert_eq!(state.hash(), calculate_hash(&board, &state));
        assert!(board.is_consistent());
        unmake_move(&mut board, &mut state, mv, undo);
        assert_eq!(board, original_board);
        assert_eq!(state, original_state);
    }

    #[test]
    fn quiet_move_updates_turn_clock_and_round_trips() {
        apply_and_undo(START, Square::G1, Square::F3, MoveFlag::Quiet);
        let (mut board, mut state) = parse_fen(START).unwrap();
        make_move(
            &mut board,
            &mut state,
            Move::new(Square::G1, Square::F3, MoveFlag::Quiet),
        )
        .unwrap();
        assert_eq!(state.side_to_move(), Color::Black);
        assert_eq!(state.halfmove_clock(), 1);
        assert_eq!(state.fullmove_number(), 1);
    }

    #[test]
    fn capture_and_double_pawn_push_update_irreversible_state() {
        apply_and_undo(
            "4k3/8/8/3p4/4P3/8/8/4K3 w - - 7 12",
            Square::E4,
            Square::D5,
            MoveFlag::Capture,
        );
        let (mut board, mut state) = parse_fen("4k3/8/8/8/8/8/4P3/4K3 w - - 9 2").unwrap();
        make_move(
            &mut board,
            &mut state,
            Move::new(Square::E2, Square::E4, MoveFlag::DoublePawnPush),
        )
        .unwrap();
        assert_eq!(state.en_passant(), Some(Square::E3));
        assert_eq!(state.halfmove_clock(), 0);
        assert_eq!(state.side_to_move(), Color::Black);
        assert_eq!(to_fen(&board, &state), "4k3/8/8/8/4P3/8/8/4K3 b - e3 0 2");
    }

    #[test]
    fn en_passant_capture_removes_and_restores_the_bypassed_pawn() {
        apply_and_undo(
            "4k3/8/8/3pP3/8/8/8/4K3 w - d6 3 8",
            Square::E5,
            Square::D6,
            MoveFlag::EnPassant,
        );
        let (mut board, mut state) = parse_fen("4k3/8/8/3pP3/8/8/8/4K3 w - d6 3 8").unwrap();
        let undo = make_move(
            &mut board,
            &mut state,
            Move::new(Square::E5, Square::D6, MoveFlag::EnPassant),
        )
        .unwrap();
        assert_eq!(
            undo.captured,
            Some(CapturedPiece {
                piece: PieceType::Pawn,
                square: Square::D5
            })
        );
        assert_eq!(board.piece_at(Square::D5), None);
        assert_eq!(
            board.piece_at(Square::D6),
            Some((Color::White, PieceType::Pawn))
        );
    }

    #[test]
    fn both_castles_move_rook_and_clear_kings_castling_rights() {
        let all_rights = "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1";
        apply_and_undo(all_rights, Square::E1, Square::G1, MoveFlag::KingSideCastle);
        apply_and_undo(
            all_rights,
            Square::E1,
            Square::C1,
            MoveFlag::QueenSideCastle,
        );
        let (mut board, mut state) = parse_fen(all_rights).unwrap();
        make_move(
            &mut board,
            &mut state,
            Move::new(Square::E1, Square::G1, MoveFlag::KingSideCastle),
        )
        .unwrap();
        assert_eq!(
            board.piece_at(Square::F1),
            Some((Color::White, PieceType::Rook))
        );
        assert_eq!(state.castling_rights(), BLACK_KING_SIDE | BLACK_QUEEN_SIDE);
    }

    #[test]
    fn moving_or_capturing_home_rooks_clears_their_castling_rights() {
        apply_and_undo(
            "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1",
            Square::H1,
            Square::H2,
            MoveFlag::Quiet,
        );
        let (mut board, mut state) = parse_fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1").unwrap();
        make_move(
            &mut board,
            &mut state,
            Move::new(Square::A1, Square::A8, MoveFlag::Capture),
        )
        .unwrap();
        assert_eq!(state.castling_rights(), WHITE_KING_SIDE | BLACK_KING_SIDE);
    }

    #[test]
    fn promotions_with_and_without_capture_round_trip_for_each_piece() {
        let quiet_flags = [
            MoveFlag::PromoteKnight,
            MoveFlag::PromoteBishop,
            MoveFlag::PromoteRook,
            MoveFlag::PromoteQueen,
        ];
        let capture_flags = [
            MoveFlag::PromoteKnightCapture,
            MoveFlag::PromoteBishopCapture,
            MoveFlag::PromoteRookCapture,
            MoveFlag::PromoteQueenCapture,
        ];
        for (quiet_flag, capture_flag) in quiet_flags.into_iter().zip(capture_flags) {
            apply_and_undo(
                "4k3/P7/8/8/8/8/8/4K3 w - - 0 1",
                Square::A7,
                Square::A8,
                quiet_flag,
            );
            apply_and_undo(
                "1r2k3/P7/8/8/8/8/8/4K3 w - - 0 1",
                Square::A7,
                Square::B8,
                capture_flag,
            );
            let (mut board, mut state) = parse_fen("4k3/P7/8/8/8/8/8/4K3 w - - 0 1").unwrap();
            make_move(
                &mut board,
                &mut state,
                Move::new(Square::A7, Square::A8, quiet_flag),
            )
            .unwrap();
            assert_eq!(
                board.piece_at(Square::A8),
                Some((Color::White, quiet_flag.promotion_piece().unwrap()))
            );
        }
    }

    #[test]
    fn black_move_increments_fullmove_number_and_captures_reset_halfmove_clock() {
        let (mut board, mut state) = parse_fen("4k3/8/8/8/8/8/8/4K2R b - - 12 7").unwrap();
        make_move(
            &mut board,
            &mut state,
            Move::new(Square::E8, Square::E7, MoveFlag::Quiet),
        )
        .unwrap();
        assert_eq!(state.fullmove_number(), 8);
        assert_eq!(state.halfmove_clock(), 13);

        let (mut board, mut state) = parse_fen("4k3/8/8/8/8/8/8/4K2R b - - 12 7").unwrap();
        make_move(
            &mut board,
            &mut state,
            Move::new(Square::E8, Square::H1, MoveFlag::Capture),
        )
        .unwrap();
        assert_eq!(state.halfmove_clock(), 0);
        assert_eq!(state.fullmove_number(), 8);
    }

    #[test]
    fn invalid_moves_are_rejected_without_changing_position() {
        let (mut board, mut state) = parse_fen(START).unwrap();
        let original_board = board;
        let original_state = state;
        let result = make_move(
            &mut board,
            &mut state,
            Move::new(Square::E2, Square::E4, MoveFlag::Quiet),
        );
        assert!(result.is_err());
        assert_eq!(board, original_board);
        assert_eq!(state, original_state);
    }

    #[test]
    fn invalid_capture_and_castle_are_rejected_without_mutation() {
        let (mut board, mut state) = parse_fen(START).unwrap();
        let original_board = board;
        let original_state = state;
        assert!(
            make_move(
                &mut board,
                &mut state,
                Move::new(Square::G1, Square::F3, MoveFlag::Capture),
            )
            .is_err()
        );
        assert_eq!(board, original_board);
        assert_eq!(state, original_state);

        assert!(
            make_move(
                &mut board,
                &mut state,
                Move::new(Square::E1, Square::G1, MoveFlag::KingSideCastle),
            )
            .is_err()
        );
        assert_eq!(board, original_board);
        assert_eq!(state, original_state);
    }

    const START: &str = crate::fen::STARTING_FEN;
}

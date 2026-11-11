use crate::board::{Board, bitboard};
use crate::movegen::is_square_attacked;
use crate::state::PositionState;
use crate::types::{Color, PieceType, Square};

include!(concat!(env!("OUT_DIR"), "/zobrist_tables.rs"));

pub const fn piece_key(color: Color, piece: PieceType, square: Square) -> u64 {
    ZOBRIST_PIECES[piece.index() * 2 + color.index()][square.index() as usize]
}

pub const fn side_key() -> u64 {
    ZOBRIST_TURN
}

pub const fn castling_key(rights: u8) -> u64 {
    ZOBRIST_CASTLING[(rights & 0x0f) as usize]
}

pub const fn en_passant_file_key(file: usize) -> u64 {
    ZOBRIST_EN_PASSANT_FILE[file]
}

pub fn calculate_hash(board: &Board, state: &PositionState) -> u64 {
    let mut hash = castling_key(state.castling_rights());
    if state.side_to_move() == Color::Black {
        hash ^= side_key();
    }

    for piece in PieceType::ALL {
        for color in [Color::White, Color::Black] {
            for square in bitboard::iter(board.bitboard(piece, color)) {
                hash ^= piece_key(color, piece, square);
            }
        }
    }

    if let Some(file) = capturable_en_passant_file(board, state) {
        hash ^= en_passant_file_key(file);
    }
    hash
}

pub fn capturable_en_passant_file(board: &Board, state: &PositionState) -> Option<usize> {
    let target = state.en_passant()?;
    let side = state.side_to_move();
    if board.is_occupied(target) {
        return None;
    }
    let captured_rank = match side {
        Color::White => target.rank().checked_sub(1)?,
        Color::Black => target.rank().checked_add(1).filter(|&rank| rank < 8)?,
    };
    let captured_square = Square::from_file_rank(target.file(), captured_rank)?;
    if board.piece_at(captured_square) != Some((side.opposite(), PieceType::Pawn)) {
        return None;
    }

    for file in [target.file().checked_sub(1), target.file().checked_add(1)] {
        let Some(file) = file.filter(|&file| file < 8) else {
            continue;
        };
        let Some(from) = Square::from_file_rank(file, captured_rank) else {
            continue;
        };
        if board.piece_at(from) != Some((side, PieceType::Pawn)) {
            continue;
        }

        let mut after = *board;
        after.remove_piece(from);
        after.remove_piece(captured_square);
        if after.place_piece(side, PieceType::Pawn, target).is_err() {
            continue;
        }
        let king = board.bitboard(PieceType::King, side);
        if king == 0 {
            continue;
        }
        let king_square = Square::try_from(king.trailing_zeros() as u8)
            .expect("king bit index is always below 64");
        if !is_square_attacked(&after, king_square, side.opposite()) {
            return Some(target.file() as usize);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::fen::parse_fen;

    #[test]
    fn generated_keys_are_nonzero_unique_and_have_required_dimensions() {
        assert_eq!(ZOBRIST_PIECES.len(), 12);
        assert!(ZOBRIST_PIECES.iter().all(|row| row.len() == 64));
        assert_eq!(ZOBRIST_CASTLING.len(), 16);
        assert_eq!(ZOBRIST_EN_PASSANT_FILE.len(), 8);

        let keys = ZOBRIST_PIECES
            .into_iter()
            .flatten()
            .chain([ZOBRIST_TURN])
            .chain(ZOBRIST_CASTLING)
            .chain(ZOBRIST_EN_PASSANT_FILE);
        let mut unique = HashSet::new();
        for key in keys {
            assert_ne!(key, 0);
            assert!(unique.insert(key), "duplicate Zobrist key {key:#018x}");
        }
        assert_eq!(unique.len(), 12 * 64 + 1 + 16 + 8);
    }

    #[test]
    fn en_passant_file_is_hashed_only_when_a_legal_capture_exists() {
        let (board, state) = parse_fen("4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1").unwrap();
        assert_eq!(capturable_en_passant_file(&board, &state), Some(3));
        let (_, without_en_passant) = parse_fen("4k3/8/8/3pP3/8/8/8/4K3 w - - 0 1").unwrap();
        assert_ne!(state.hash(), without_en_passant.hash());

        let (board, state) = parse_fen("4k3/8/8/3p4/8/8/8/4K3 w - d6 0 1").unwrap();
        assert_eq!(capturable_en_passant_file(&board, &state), None);
        let (_, without_en_passant) = parse_fen("4k3/8/8/3p4/8/8/8/4K3 w - - 0 1").unwrap();
        assert_eq!(state.hash(), without_en_passant.hash());

        let (board, state) = parse_fen("8/8/8/8/k2Pp2Q/8/8/3K4 b - d3 0 1").unwrap();
        assert_eq!(capturable_en_passant_file(&board, &state), None);
        let (_, without_en_passant) = parse_fen("8/8/8/8/k2Pp2Q/8/8/3K4 b - - 0 1").unwrap();
        assert_eq!(state.hash(), without_en_passant.hash());
    }

    #[test]
    fn hash_uses_position_identity_not_move_counters_and_round_trips_fen() {
        let (board, state) =
            parse_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        let (same_board, same_position) =
            parse_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 42 17").unwrap();
        assert_eq!(state.hash(), calculate_hash(&board, &state));
        assert_eq!(state.hash(), same_position.hash());
        assert_eq!(same_board, board);
    }
}

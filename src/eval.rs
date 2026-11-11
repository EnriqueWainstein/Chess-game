use crate::board::{Board, bitboard};
use crate::state::PositionState;
use crate::types::{Color, PieceType, Square};

pub const PIECE_VALUES: [i32; 6] = [100, 300, 300, 500, 900, 10_000];
pub const MATE_SCORE: i32 = 1_000_000;
pub const INFINITY: i32 = 1_100_000;
pub const MAX_PLY: u8 = 128;
pub const MAX_QUIESCENCE_PLY: u8 = 16;

const fn make_piece_square_table(piece: PieceType) -> [i32; 64] {
    let mut table = [0; 64];
    let mut index = 0;
    while index < 64 {
        let file = (index % 8) as i32;
        let rank = (index / 8) as i32;
        let center_distance = (file * 2 - 7).abs() + (rank * 2 - 7).abs();
        let center = 14 - center_distance;
        table[index] = match piece {
            PieceType::Pawn => rank * 8 + center * 2,
            PieceType::Knight => center * 6,
            PieceType::Bishop => center * 4,
            PieceType::Rook => rank * 2 + center,
            PieceType::Queen => center * 2,
            PieceType::King => -center * 4,
        };
        index += 1;
    }
    table
}

pub const PAWN_PST: [i32; 64] = make_piece_square_table(PieceType::Pawn);
pub const KNIGHT_PST: [i32; 64] = make_piece_square_table(PieceType::Knight);
pub const BISHOP_PST: [i32; 64] = make_piece_square_table(PieceType::Bishop);
pub const ROOK_PST: [i32; 64] = make_piece_square_table(PieceType::Rook);
pub const QUEEN_PST: [i32; 64] = make_piece_square_table(PieceType::Queen);
pub const KING_PST: [i32; 64] = make_piece_square_table(PieceType::King);

const PIECE_SQUARE_TABLES: [[i32; 64]; 6] = [
    PAWN_PST, KNIGHT_PST, BISHOP_PST, ROOK_PST, QUEEN_PST, KING_PST,
];

pub fn evaluate(board: &Board, state: &PositionState) -> i32 {
    let mut score = 0;
    for piece in PieceType::ALL {
        for color in [Color::White, Color::Black] {
            let sign = if color == Color::White { 1 } else { -1 };
            if piece != PieceType::King {
                score += sign
                    * PIECE_VALUES[piece.index()]
                    * bitboard::count(board.bitboard(piece, color)) as i32;
            }
            for square in bitboard::iter(board.bitboard(piece, color)) {
                let square = if color == Color::White {
                    square
                } else {
                    reflect_vertical(square)
                };
                score += sign * PIECE_SQUARE_TABLES[piece.index()][square.index() as usize];
            }
        }
    }
    if state.side_to_move() == Color::White {
        score
    } else {
        -score
    }
}

pub const fn piece_value(piece: PieceType) -> i32 {
    PIECE_VALUES[piece.index()]
}

pub fn reflect_vertical(square: Square) -> Square {
    Square::try_from(square.index() ^ 56).expect("reflected square remains on the board")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fen::parse_fen;

    #[test]
    fn initial_position_is_balanced_and_material_is_side_to_move_relative() {
        let (board, state) =
            parse_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        assert_eq!(evaluate(&board, &state), 0);

        let (board, state) =
            parse_fen("rnb1kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        let white_to_move = evaluate(&board, &state);
        assert!(white_to_move > 0);
        let mut black_to_move = state;
        black_to_move.set_side_to_move(Color::Black);
        assert_eq!(evaluate(&board, &black_to_move), -white_to_move);
    }

    #[test]
    fn annex_mirror_positions_have_equal_evaluation_from_side_to_move() {
        let (board_a, state_a) =
            parse_fen("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1").unwrap();
        let (board_b, state_b) =
            parse_fen("rnbqkbnr/pppp1ppp/8/4p3/8/8/PPPPPPPP/RNBQKBNR w KQkq e6 0 1").unwrap();
        assert_eq!(evaluate(&board_a, &state_a), evaluate(&board_b, &state_b));
    }

    #[test]
    fn mate_and_search_bounds_are_well_separated_and_safe_to_negate() {
        let mate_score = std::hint::black_box(MATE_SCORE);
        let infinity = std::hint::black_box(INFINITY);
        assert!(mate_score > 64 * PIECE_VALUES[PieceType::Queen.index()]);
        assert!(infinity < i32::MAX);
        assert_eq!(infinity.checked_neg(), Some(-1_100_000));
        assert!(mate_score - 1 > mate_score - 3);
    }
}

use std::fmt;

use crate::attacks::{bishop_attacks, rook_attacks};
use crate::board::{Board, bitboard};
use crate::makemove::{MoveError, make_move, unmake_move};
use crate::moves::{Move, MoveFlag};
use crate::state::{
    BLACK_KING_SIDE, BLACK_QUEEN_SIDE, PositionState, WHITE_KING_SIDE, WHITE_QUEEN_SIDE,
};
use crate::types::{Color, FILE_A, FILE_H, PieceType, Square};
use crate::zobrist::calculate_hash;

pub const MOVE_LIST_CAPACITY: usize = 256;

const EMPTY_MOVE: Move = Move::new(Square::A1, Square::A1, MoveFlag::Quiet);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoveListOverflow;

impl fmt::Display for MoveListOverflow {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "move list exceeds capacity of {MOVE_LIST_CAPACITY}"
        )
    }
}

impl std::error::Error for MoveListOverflow {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoveList {
    moves: [Move; MOVE_LIST_CAPACITY],
    len: usize,
}

impl MoveList {
    pub const fn new() -> Self {
        Self {
            moves: [EMPTY_MOVE; MOVE_LIST_CAPACITY],
            len: 0,
        }
    }

    pub fn push(&mut self, mv: Move) -> Result<(), MoveListOverflow> {
        if self.len == MOVE_LIST_CAPACITY {
            return Err(MoveListOverflow);
        }
        self.moves[self.len] = mv;
        self.len += 1;
        Ok(())
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn get(&self, index: usize) -> Option<Move> {
        self.as_slice().get(index).copied()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Move> {
        self.as_slice().iter()
    }

    pub fn as_slice(&self) -> &[Move] {
        &self.moves[..self.len]
    }

    pub fn captures(&self) -> Result<Self, MoveListOverflow> {
        self.filtered(|mv| mv.is_capture() || mv.promotion_piece() == Some(PieceType::Queen))
    }

    pub fn quiets(&self) -> Result<Self, MoveListOverflow> {
        self.filtered(|mv| !mv.is_capture() && mv.promotion_piece() != Some(PieceType::Queen))
    }

    fn filtered(&self, predicate: impl Fn(Move) -> bool) -> Result<Self, MoveListOverflow> {
        let mut result = Self::new();
        for &mv in self.iter() {
            if predicate(mv) {
                result.push(mv)?;
            }
        }
        Ok(result)
    }
}

impl Default for MoveList {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> IntoIterator for &'a MoveList {
    type Item = &'a Move;
    type IntoIter = std::slice::Iter<'a, Move>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[derive(Debug)]
pub enum MoveGenError {
    MoveListOverflow(MoveListOverflow),
    InvalidPseudoLegalMove(MoveError),
}

impl fmt::Display for MoveGenError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MoveListOverflow(error) => error.fmt(formatter),
            Self::InvalidPseudoLegalMove(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for MoveGenError {}

impl From<MoveListOverflow> for MoveGenError {
    fn from(error: MoveListOverflow) -> Self {
        Self::MoveListOverflow(error)
    }
}

impl From<MoveError> for MoveGenError {
    fn from(error: MoveError) -> Self {
        Self::InvalidPseudoLegalMove(error)
    }
}

const fn make_leaper_table(knight: bool) -> [u64; 64] {
    let mut table = [0; 64];
    let mut square_index = 0;
    while square_index < 64 {
        let file = (square_index % 8) as i8;
        let rank = (square_index / 8) as i8;
        let mut attacks = 0;
        let mut offset_index = 0;
        while offset_index < 8 {
            let (file_delta, rank_delta) = if knight {
                match offset_index {
                    0 => (1, 2),
                    1 => (2, 1),
                    2 => (2, -1),
                    3 => (1, -2),
                    4 => (-1, -2),
                    5 => (-2, -1),
                    6 => (-2, 1),
                    _ => (-1, 2),
                }
            } else {
                match offset_index {
                    0 => (1, 0),
                    1 => (1, 1),
                    2 => (0, 1),
                    3 => (-1, 1),
                    4 => (-1, 0),
                    5 => (-1, -1),
                    6 => (0, -1),
                    _ => (1, -1),
                }
            };
            let target_file = file + file_delta;
            let target_rank = rank + rank_delta;
            if target_file >= 0 && target_file < 8 && target_rank >= 0 && target_rank < 8 {
                attacks |= 1_u64 << (target_rank as u32 * 8 + target_file as u32);
            }
            offset_index += 1;
        }
        table[square_index] = attacks;
        square_index += 1;
    }
    table
}

pub const KNIGHT_ATTACKS: [u64; 64] = make_leaper_table(true);
pub const KING_ATTACKS: [u64; 64] = make_leaper_table(false);

pub const fn knight_attacks(square: Square) -> u64 {
    KNIGHT_ATTACKS[square.index() as usize]
}

pub const fn king_attacks(square: Square) -> u64 {
    KING_ATTACKS[square.index() as usize]
}

pub fn is_square_attacked(board: &Board, square: Square, by_color: Color) -> bool {
    is_square_attacked_with_occupancy(board, square, by_color, board.occupancy())
}

fn is_square_attacked_with_occupancy(
    board: &Board,
    square: Square,
    by_color: Color,
    occupancy: u64,
) -> bool {
    let pawn_attackers = pawn_attacks(square, by_color.opposite());
    if pawn_attackers & board.bitboard(PieceType::Pawn, by_color) != 0 {
        return true;
    }
    if knight_attacks(square) & board.bitboard(PieceType::Knight, by_color) != 0 {
        return true;
    }
    if king_attacks(square) & board.bitboard(PieceType::King, by_color) != 0 {
        return true;
    }
    if bishop_attacks(square, occupancy)
        & (board.bitboard(PieceType::Bishop, by_color) | board.bitboard(PieceType::Queen, by_color))
        != 0
    {
        return true;
    }
    rook_attacks(square, occupancy)
        & (board.bitboard(PieceType::Rook, by_color) | board.bitboard(PieceType::Queen, by_color))
        != 0
}

pub fn is_in_check(board: &Board, color: Color) -> bool {
    let king = board.bitboard(PieceType::King, color);
    if king == 0 {
        return false;
    }
    let square = Square::try_from(king.trailing_zeros() as u8).expect("king bit is on board");
    is_square_attacked(board, square, color.opposite())
}

pub fn generate_pseudo_legal(
    board: &Board,
    state: &PositionState,
) -> Result<MoveList, MoveListOverflow> {
    let mut moves = MoveList::new();
    let side = state.side_to_move();
    generate_pawn_moves(board, state, side, &mut moves)?;

    for (piece, attacks_for_piece) in [
        (PieceType::Knight, knight_attacks as fn(Square) -> u64),
        (PieceType::King, king_attacks as fn(Square) -> u64),
    ] {
        for from in bitboard::iter(board.bitboard(piece, side)) {
            let targets = attacks_for_piece(from) & !board.color_occupancy(side);
            add_piece_targets(board, from, targets, side, &mut moves)?;
        }
    }

    for (piece, sliding_attacks) in [
        (PieceType::Bishop, bishop_attacks as fn(Square, u64) -> u64),
        (PieceType::Rook, rook_attacks as fn(Square, u64) -> u64),
        (
            PieceType::Queen,
            crate::attacks::queen_attacks as fn(Square, u64) -> u64,
        ),
    ] {
        for from in bitboard::iter(board.bitboard(piece, side)) {
            let targets = sliding_attacks(from, board.occupancy()) & !board.color_occupancy(side);
            add_piece_targets(board, from, targets, side, &mut moves)?;
        }
    }

    generate_castling(board, state, side, &mut moves)?;
    Ok(moves)
}

pub fn generate_legal(board: &Board, state: &PositionState) -> Result<MoveList, MoveGenError> {
    let mover = state.side_to_move();
    let king_in_check = is_in_check(board, mover);
    let pinned = pinned_pieces(board, mover);
    let pseudo_legal = generate_pseudo_legal(board, state)?;
    let mut legal = MoveList::new();
    for &mv in &pseudo_legal {
        let moving_piece = board
            .piece_at(mv.from())
            .expect("generated move has an occupied source")
            .1;
        if !king_in_check
            && moving_piece != PieceType::King
            && mv.flag() != MoveFlag::EnPassant
            && pinned & mv.from().bit() == 0
        {
            legal.push(mv)?;
            continue;
        }
        let mut board_after = *board;
        let mut state_after = *state;
        make_move(&mut board_after, &mut state_after, mv)?;
        if !is_in_check(&board_after, mover) {
            legal.push(mv)?;
        }
    }
    Ok(legal)
}

fn pinned_pieces(board: &Board, color: Color) -> u64 {
    let king_board = board.bitboard(PieceType::King, color);
    if king_board == 0 {
        return 0;
    }
    let king = Square::try_from(king_board.trailing_zeros() as u8).expect("king bit is on board");
    let directions = [
        (1_i8, 0_i8, false),
        (-1, 0, false),
        (0, 1, false),
        (0, -1, false),
        (1, 1, true),
        (1, -1, true),
        (-1, 1, true),
        (-1, -1, true),
    ];
    let mut pinned = 0;
    for (file_delta, rank_delta, diagonal) in directions {
        let mut file = king.file() as i8 + file_delta;
        let mut rank = king.rank() as i8 + rank_delta;
        let mut candidate = None;
        while (0..8).contains(&file) && (0..8).contains(&rank) {
            let square = Square::from_file_rank(file as u8, rank as u8)
                .expect("ray coordinates are in range");
            if let Some((piece_color, piece)) = board.piece_at(square) {
                if piece_color == color {
                    if candidate.is_some() {
                        break;
                    }
                    candidate = Some(square);
                } else {
                    if let Some(candidate_square) = candidate
                        && (piece == PieceType::Queen
                            || (diagonal && piece == PieceType::Bishop)
                            || (!diagonal && piece == PieceType::Rook))
                    {
                        pinned |= candidate_square.bit();
                    }
                    break;
                }
            }
            file += file_delta;
            rank += rank_delta;
        }
    }
    pinned
}

pub fn perft(board: &mut Board, state: &mut PositionState, depth: u8) -> Result<u64, MoveGenError> {
    if depth == 0 {
        return Ok(1);
    }
    let moves = generate_legal(board, state)?;
    if depth == 1 {
        return Ok(moves.len() as u64);
    }

    let mut nodes = 0_u64;
    for &mv in &moves {
        let undo = make_move(board, state, mv)?;
        debug_assert_eq!(state.hash(), calculate_hash(board, state));
        let child_nodes = perft(board, state, depth - 1);
        unmake_move(board, state, mv, undo);
        nodes += child_nodes?;
    }
    Ok(nodes)
}

fn generate_pawn_moves(
    board: &Board,
    state: &PositionState,
    side: Color,
    moves: &mut MoveList,
) -> Result<(), MoveListOverflow> {
    let pawns = board.bitboard(PieceType::Pawn, side);
    let occupied = board.occupancy();
    let enemies = board.color_occupancy(side.opposite());
    let (forward, start_rank, promotion_rank) = match side {
        Color::White => (8_i16, 1, 7),
        Color::Black => (-8_i16, 6, 0),
    };

    for from in bitboard::iter(pawns) {
        let one_step_index = from.index() as i16 + forward;
        if (0..64).contains(&one_step_index) {
            let one_step = Square::try_from(one_step_index as u8).expect("pawn step is on board");
            if occupied & one_step.bit() == 0 {
                if one_step.rank() == promotion_rank {
                    add_promotions(from, one_step, false, moves)?;
                } else {
                    moves.push(Move::new(from, one_step, MoveFlag::Quiet))?;
                    if from.rank() == start_rank {
                        let two_step = Square::try_from((from.index() as i16 + forward * 2) as u8)
                            .expect("double pawn step is on board");
                        if occupied & two_step.bit() == 0 {
                            moves.push(Move::new(from, two_step, MoveFlag::DoublePawnPush))?;
                        }
                    }
                }
            }
        }

        let capture_targets = pawn_attacks(from, side) & enemies;
        for to in
            bitboard::iter(capture_targets & !(board.bitboard(PieceType::King, side.opposite())))
        {
            if to.rank() == promotion_rank {
                add_promotions(from, to, true, moves)?;
            } else {
                moves.push(Move::new(from, to, MoveFlag::Capture))?;
            }
        }

        if let Some(to) = state.en_passant()
            && pawn_attacks(from, side) & to.bit() != 0
            && board.piece_at(to).is_none()
        {
            let captured_square = Square::from_file_rank(
                to.file(),
                if side == Color::White {
                    to.rank()
                        .checked_sub(1)
                        .expect("valid white en-passant rank")
                } else {
                    to.rank()
                        .checked_add(1)
                        .expect("valid black en-passant rank")
                },
            )
            .expect("en-passant capture square is in range");
            if board.piece_at(captured_square) == Some((side.opposite(), PieceType::Pawn)) {
                moves.push(Move::new(from, to, MoveFlag::EnPassant))?;
            }
        }
    }
    Ok(())
}

fn pawn_attacks(square: Square, color: Color) -> u64 {
    let bit = square.bit();
    match color {
        Color::White => ((bit & !FILE_A) << 7) | ((bit & !FILE_H) << 9),
        Color::Black => ((bit & !FILE_A) >> 9) | ((bit & !FILE_H) >> 7),
    }
}

fn add_promotions(
    from: Square,
    to: Square,
    capture: bool,
    moves: &mut MoveList,
) -> Result<(), MoveListOverflow> {
    let flags = if capture {
        [
            MoveFlag::PromoteKnightCapture,
            MoveFlag::PromoteBishopCapture,
            MoveFlag::PromoteRookCapture,
            MoveFlag::PromoteQueenCapture,
        ]
    } else {
        [
            MoveFlag::PromoteKnight,
            MoveFlag::PromoteBishop,
            MoveFlag::PromoteRook,
            MoveFlag::PromoteQueen,
        ]
    };
    for flag in flags {
        moves.push(Move::new(from, to, flag))?;
    }
    Ok(())
}

fn add_piece_targets(
    board: &Board,
    from: Square,
    targets: u64,
    side: Color,
    moves: &mut MoveList,
) -> Result<(), MoveListOverflow> {
    for to in bitboard::iter(targets & !board.bitboard(PieceType::King, side.opposite())) {
        let flag = if board.color_occupancy(side.opposite()) & to.bit() != 0 {
            MoveFlag::Capture
        } else {
            MoveFlag::Quiet
        };
        moves.push(Move::new(from, to, flag))?;
    }
    Ok(())
}

fn generate_castling(
    board: &Board,
    state: &PositionState,
    side: Color,
    moves: &mut MoveList,
) -> Result<(), MoveListOverflow> {
    let rank = if side == Color::White { 0 } else { 7 };
    let (king_right, queen_right) = match side {
        Color::White => (WHITE_KING_SIDE, WHITE_QUEEN_SIDE),
        Color::Black => (BLACK_KING_SIDE, BLACK_QUEEN_SIDE),
    };
    let king_from = Square::from_file_rank(4, rank).expect("king home square is valid");
    if board.piece_at(king_from) != Some((side, PieceType::King))
        || is_square_attacked(board, king_from, side.opposite())
    {
        return Ok(());
    }

    for (right, flag, transit_file, destination_file, empty_files) in [
        (king_right, MoveFlag::KingSideCastle, 5, 6, [5, 6, 0]),
        (queen_right, MoveFlag::QueenSideCastle, 3, 2, [1, 2, 3]),
    ] {
        if !state.has_castling_right(right) {
            continue;
        }
        let rook_file = if flag == MoveFlag::KingSideCastle {
            7
        } else {
            0
        };
        let rook_from = Square::from_file_rank(rook_file, rank).expect("rook home square is valid");
        if board.piece_at(rook_from) != Some((side, PieceType::Rook)) {
            continue;
        }
        let path_clear = empty_files
            .into_iter()
            .filter(|&file| file != 0 || flag == MoveFlag::QueenSideCastle)
            .all(|file| {
                !board
                    .is_occupied(Square::from_file_rank(file, rank).expect("castle path is valid"))
            });
        if !path_clear {
            continue;
        }
        let mut transit_board = *board;
        transit_board.remove_piece(king_from);
        let transit =
            Square::from_file_rank(transit_file, rank).expect("castle transit square is valid");
        transit_board
            .place_piece(side, PieceType::King, transit)
            .expect("castle transit square is empty");
        if is_square_attacked(&transit_board, transit, side.opposite()) {
            continue;
        }
        transit_board.remove_piece(transit);
        let destination = Square::from_file_rank(destination_file, rank)
            .expect("castle destination square is valid");
        transit_board
            .place_piece(side, PieceType::King, destination)
            .expect("castle destination is empty");
        if is_square_attacked(&transit_board, destination, side.opposite()) {
            continue;
        }
        moves.push(Move::new(king_from, destination, flag))?;
    }
    Ok(())
}

pub fn perft_starting_position(depth: u8) -> Result<u64, MoveGenError> {
    let (mut board, mut state) = crate::fen::starting_position();
    perft(&mut board, &mut state, depth)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fen::parse_fen;

    #[test]
    fn move_list_is_fixed_capacity_and_reports_overflow() {
        let mut moves = MoveList::new();
        for _ in 0..MOVE_LIST_CAPACITY {
            moves
                .push(Move::new(Square::A1, Square::A2, MoveFlag::Quiet))
                .unwrap();
        }
        assert_eq!(moves.len(), 256);
        assert_eq!(moves.iter().count(), 256);
        assert!(
            moves
                .push(Move::new(Square::A1, Square::A2, MoveFlag::Quiet))
                .is_err()
        );
    }

    #[test]
    fn leaper_tables_match_edges_and_center() {
        assert_eq!(knight_attacks(Square::A1).count_ones(), 2);
        assert_eq!(knight_attacks(Square::D4).count_ones(), 8);
        assert_eq!(king_attacks(Square::A1).count_ones(), 3);
        for square in Square::ALL {
            assert_eq!(
                KNIGHT_ATTACKS[square.index() as usize],
                reference_knight(square)
            );
            assert_eq!(
                KING_ATTACKS[square.index() as usize],
                reference_king(square)
            );
        }
    }

    #[test]
    fn attack_detection_handles_all_piece_types_and_blockers() {
        for (fen, king, color) in [
            ("4k3/8/8/8/8/8/4P3/4K3 w - - 0 1", Square::D3, Color::White),
            ("4k3/8/8/8/8/5n2/8/4K3 w - - 0 1", Square::D4, Color::Black),
            ("4k3/8/8/8/8/8/8/3bK3 w - - 0 1", Square::E2, Color::Black),
            ("4k3/8/8/8/8/8/8/4K2r w - - 0 1", Square::E1, Color::Black),
            ("4k3/8/8/8/8/8/8/3qK3 w - - 0 1", Square::E1, Color::Black),
        ] {
            let (board, _) = parse_fen(fen).unwrap();
            assert!(is_square_attacked(&board, king, color), "{fen}");
        }
        let (blocked, _) = parse_fen("4r3/8/8/8/8/4P3/8/4K3 w - - 0 1").unwrap();
        assert!(!is_square_attacked(&blocked, Square::E1, Color::Black));
    }

    #[test]
    fn starting_position_has_twenty_pseudo_legal_and_legal_moves() {
        let (board, state) = crate::fen::starting_position();
        assert_eq!(generate_pseudo_legal(&board, &state).unwrap().len(), 20);
        assert_eq!(generate_legal(&board, &state).unwrap().len(), 20);
    }

    #[test]
    fn pawn_edges_en_passant_and_promotion_are_generated_correctly() {
        let (board, state) = parse_fen("4k3/8/8/8/8/8/P6p/4K3 w - - 0 1").unwrap();
        let moves = generate_pseudo_legal(&board, &state).unwrap();
        assert!(
            !moves
                .iter()
                .any(|mv| mv.from() == Square::A2 && mv.to().file() == 7)
        );

        let (board, state) = parse_fen("4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1").unwrap();
        let moves = generate_pseudo_legal(&board, &state).unwrap();
        assert!(moves.iter().any(|mv| mv.flag() == MoveFlag::EnPassant));

        let (board, state) = parse_fen("1r2k3/P7/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        let moves = generate_pseudo_legal(&board, &state).unwrap();
        let promotions: Vec<_> = moves
            .iter()
            .filter(|mv| mv.from() == Square::A7 && mv.to() == Square::B8)
            .collect();
        assert_eq!(promotions.len(), 4);
    }

    #[test]
    fn castling_requires_right_clear_path_and_safe_king_path() {
        let (board, state) = parse_fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1").unwrap();
        let castles = generate_pseudo_legal(&board, &state).unwrap();
        assert!(
            castles
                .iter()
                .any(|mv| mv.flag() == MoveFlag::KingSideCastle)
        );
        assert!(
            castles
                .iter()
                .any(|mv| mv.flag() == MoveFlag::QueenSideCastle)
        );

        let (no_right, state) = parse_fen("r3k2r/8/8/8/8/8/8/R3K2R w - - 0 1").unwrap();
        assert!(
            !generate_pseudo_legal(&no_right, &state)
                .unwrap()
                .iter()
                .any(|mv| matches!(
                    mv.flag(),
                    MoveFlag::KingSideCastle | MoveFlag::QueenSideCastle
                ))
        );

        let (attacked_transit, state) = parse_fen("4kr2/8/8/8/8/8/8/4K2R w K - 0 1").unwrap();
        assert!(
            !generate_pseudo_legal(&attacked_transit, &state)
                .unwrap()
                .iter()
                .any(|mv| mv.flag() == MoveFlag::KingSideCastle)
        );

        let (attacked_destination, state) = parse_fen("4k1r1/8/8/8/8/8/8/4K2R w K - 0 1").unwrap();
        assert!(
            !generate_pseudo_legal(&attacked_destination, &state)
                .unwrap()
                .iter()
                .any(|mv| mv.flag() == MoveFlag::KingSideCastle)
        );

        let (occupied_path, state) = parse_fen("4k3/8/8/8/8/8/8/4KN1R w K - 0 1").unwrap();
        assert!(
            !generate_pseudo_legal(&occupied_path, &state)
                .unwrap()
                .iter()
                .any(|mv| mv.flag() == MoveFlag::KingSideCastle)
        );

        let (king_in_check, state) = parse_fen("4r1k1/8/8/8/8/8/8/4K2R w K - 0 1").unwrap();
        assert!(
            !generate_pseudo_legal(&king_in_check, &state)
                .unwrap()
                .iter()
                .any(|mv| mv.flag() == MoveFlag::KingSideCastle)
        );
    }

    #[test]
    fn legal_filter_removes_pinned_and_horizontal_en_passant_moves() {
        let (board, state) = parse_fen("4r1k1/8/8/8/8/8/4R3/4K3 w - - 0 1").unwrap();
        let pseudo = generate_pseudo_legal(&board, &state).unwrap();
        let legal = generate_legal(&board, &state).unwrap();
        assert!(
            pseudo
                .iter()
                .any(|mv| mv.from() == Square::E2 && mv.to() == Square::D2)
        );
        assert!(
            !legal
                .iter()
                .any(|mv| mv.from() == Square::E2 && mv.to() == Square::D2)
        );

        let (board, state) = parse_fen("8/8/8/8/k2Pp2Q/8/8/3K4 b - d3 0 1").unwrap();
        assert!(
            !generate_legal(&board, &state)
                .unwrap()
                .iter()
                .any(|mv| mv.flag() == MoveFlag::EnPassant)
        );
    }

    #[test]
    fn capture_and_quiet_segments_cover_legal_moves_without_overlap() {
        let (board, state) = parse_fen("4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1").unwrap();
        let all = generate_legal(&board, &state).unwrap();
        let captures = all.captures().unwrap();
        let quiets = all.quiets().unwrap();
        assert_eq!(captures.len() + quiets.len(), all.len());
        assert!(
            captures
                .iter()
                .all(|mv| mv.is_capture() || mv.is_promotion())
        );
        assert!(
            quiets
                .iter()
                .all(|mv| !mv.is_capture() && mv.promotion_piece() != Some(PieceType::Queen))
        );
    }

    fn reference_knight(square: Square) -> u64 {
        let mut attacks = 0;
        for (df, dr) in [
            (1, 2),
            (2, 1),
            (2, -1),
            (1, -2),
            (-1, -2),
            (-2, -1),
            (-2, 1),
            (-1, 2),
        ] {
            if let Some(target) = Square::from_file_rank(
                (square.file() as i8 + df) as u8,
                (square.rank() as i8 + dr) as u8,
            ) {
                attacks |= target.bit();
            }
        }
        attacks
    }

    fn reference_king(square: Square) -> u64 {
        let mut attacks = 0;
        for df in -1..=1 {
            for dr in -1..=1 {
                if df == 0 && dr == 0 {
                    continue;
                }
                let file = square.file() as i8 + df;
                let rank = square.rank() as i8 + dr;
                if (0..8).contains(&file)
                    && (0..8).contains(&rank)
                    && let Some(target) = Square::from_file_rank(file as u8, rank as u8)
                {
                    attacks |= target.bit();
                }
            }
        }
        attacks
    }
}

#[cfg(test)]
mod perft_tests {
    use super::*;
    use crate::fen::parse_fen;

    const PERFT_CASES: [(&str, [u64; 4]); 5] = [
        (
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            [20, 400, 8902, 197281],
        ),
        (
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
            [48, 2039, 97862, 4085603],
        ),
        (
            "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
            [14, 191, 2812, 43238],
        ),
        (
            "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
            [6, 264, 9467, 422333],
        ),
        (
            "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
            [44, 1486, 62379, 2103487],
        ),
    ];

    #[test]
    fn perft_matches_all_annex_positions_through_depth_four() {
        for (fen, expected) in PERFT_CASES {
            let (mut board, mut state) = parse_fen(fen).unwrap();
            for (depth, expected_nodes) in (1..=4).zip(expected) {
                assert_eq!(
                    perft(&mut board, &mut state, depth).unwrap(),
                    expected_nodes,
                    "FEN {fen}, depth {depth}"
                );
            }
        }
    }

    #[test]
    fn maximum_legal_move_position_fits_the_fixed_move_list() {
        let (board, state) =
            parse_fen("R6R/3Q4/1Q4Q1/4Q3/2Q4Q/Q4Q2/pp1Q4/kBNN1KB1 w - - 0 1").unwrap();
        assert_eq!(generate_legal(&board, &state).unwrap().len(), 218);
    }
}

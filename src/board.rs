use std::fmt;

use crate::types::{Color, PieceType, Square};

const WHITE_OCCUPANCY: usize = 12;
const BLACK_OCCUPANCY: usize = 13;
const TOTAL_OCCUPANCY: usize = 14;

pub mod bitboard {
    use crate::types::Square;

    pub const fn contains(bitboard: u64, square: Square) -> bool {
        bitboard & square.bit() != 0
    }

    pub const fn set(bitboard: &mut u64, square: Square) {
        *bitboard |= square.bit();
    }

    pub const fn clear(bitboard: &mut u64, square: Square) {
        *bitboard &= !square.bit();
    }

    pub const fn count(bitboard: u64) -> u32 {
        bitboard.count_ones()
    }

    pub const fn iter(bitboard: u64) -> BitIter {
        BitIter(bitboard)
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct BitIter(u64);

    impl Iterator for BitIter {
        type Item = Square;

        fn next(&mut self) -> Option<Self::Item> {
            if self.0 == 0 {
                return None;
            }
            let index = self.0.trailing_zeros() as u8;
            self.0 &= self.0 - 1;
            Some(Square::try_from(index).expect("bit index is always below 64"))
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            let count = self.0.count_ones() as usize;
            (count, Some(count))
        }
    }

    impl ExactSizeIterator for BitIter {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardError {
    SquareOccupied(Square),
}

impl fmt::Display for BoardError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SquareOccupied(square) => write!(formatter, "square {square} is occupied"),
        }
    }
}

impl std::error::Error for BoardError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Board {
    bitboards: [u64; 15],
}

impl Board {
    pub const fn empty() -> Self {
        Self { bitboards: [0; 15] }
    }

    pub const fn bitboard(&self, piece: PieceType, color: Color) -> u64 {
        self.bitboards[piece.index() * 2 + color.index()]
    }

    pub const fn color_occupancy(&self, color: Color) -> u64 {
        self.bitboards[match color {
            Color::White => WHITE_OCCUPANCY,
            Color::Black => BLACK_OCCUPANCY,
        }]
    }

    pub const fn occupancy(&self) -> u64 {
        self.bitboards[TOTAL_OCCUPANCY]
    }

    pub const fn is_occupied(&self, square: Square) -> bool {
        self.occupancy() & square.bit() != 0
    }

    pub fn piece_at(&self, square: Square) -> Option<(Color, PieceType)> {
        let bit = square.bit();
        for piece in PieceType::ALL {
            for color in [Color::White, Color::Black] {
                if self.bitboard(piece, color) & bit != 0 {
                    return Some((color, piece));
                }
            }
        }
        None
    }

    pub fn place_piece(
        &mut self,
        color: Color,
        piece: PieceType,
        square: Square,
    ) -> Result<(), BoardError> {
        if self.is_occupied(square) {
            return Err(BoardError::SquareOccupied(square));
        }
        let bit = square.bit();
        self.bitboards[piece.index() * 2 + color.index()] |= bit;
        self.bitboards[match color {
            Color::White => WHITE_OCCUPANCY,
            Color::Black => BLACK_OCCUPANCY,
        }] |= bit;
        self.bitboards[TOTAL_OCCUPANCY] |= bit;
        Ok(())
    }

    pub fn remove_piece(&mut self, square: Square) -> Option<(Color, PieceType)> {
        let (color, piece) = self.piece_at(square)?;
        let bit = square.bit();
        self.bitboards[piece.index() * 2 + color.index()] &= !bit;
        self.bitboards[match color {
            Color::White => WHITE_OCCUPANCY,
            Color::Black => BLACK_OCCUPANCY,
        }] &= !bit;
        self.bitboards[TOTAL_OCCUPANCY] &= !bit;
        Some((color, piece))
    }

    pub fn is_consistent(&self) -> bool {
        let mut white = 0;
        let mut black = 0;
        for piece in PieceType::ALL {
            white |= self.bitboard(piece, Color::White);
            black |= self.bitboard(piece, Color::Black);
        }
        white == self.color_occupancy(Color::White)
            && black == self.color_occupancy(Color::Black)
            && white & black == 0
            && white | black == self.occupancy()
    }

    pub fn starting_position() -> Self {
        let mut board = Self::empty();
        for file in 0..8 {
            let white_pawn = Square::from_file_rank(file, 1).expect("file is in range");
            let black_pawn = Square::from_file_rank(file, 6).expect("file is in range");
            board
                .place_piece(Color::White, PieceType::Pawn, white_pawn)
                .expect("starting squares are empty");
            board
                .place_piece(Color::Black, PieceType::Pawn, black_pawn)
                .expect("starting squares are empty");
        }

        for (file, piece) in [
            PieceType::Rook,
            PieceType::Knight,
            PieceType::Bishop,
            PieceType::Queen,
            PieceType::King,
            PieceType::Bishop,
            PieceType::Knight,
            PieceType::Rook,
        ]
        .into_iter()
        .enumerate()
        {
            let white_square = Square::from_file_rank(file as u8, 0).expect("file is in range");
            let black_square = Square::from_file_rank(file as u8, 7).expect("file is in range");
            board
                .place_piece(Color::White, piece, white_square)
                .expect("starting squares are empty");
            board
                .place_piece(Color::Black, piece, black_square)
                .expect("starting squares are empty");
        }
        board
    }
}

impl Default for Board {
    fn default() -> Self {
        Self::empty()
    }
}

impl fmt::Display for Board {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for rank in (0..8).rev() {
            write!(formatter, "{} |", rank + 1)?;
            for file in 0..8 {
                let square =
                    Square::from_file_rank(file, rank).expect("rank and file are in range");
                match self.piece_at(square) {
                    Some((color, piece)) => write!(formatter, " {}", piece.fen_char(color))?,
                    None => formatter.write_str(" .")?,
                }
            }
            if rank != 0 {
                writeln!(formatter)?;
            }
        }
        write!(formatter, "\n  +----------------\n    a b c d e f g h")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::bitboard;

    #[test]
    fn bitboard_operations_query_count_and_iter_without_allocations() {
        let mut bits = 0;
        bitboard::set(&mut bits, Square::A1);
        bitboard::set(&mut bits, Square::H8);
        assert!(bitboard::contains(bits, Square::A1));
        assert_eq!(bitboard::count(bits), 2);
        assert_eq!(
            bitboard::iter(bits).collect::<Vec<_>>(),
            [Square::A1, Square::H8]
        );
        bitboard::clear(&mut bits, Square::A1);
        assert_eq!(bits, Square::H8.bit());
    }

    #[test]
    fn board_keeps_piece_and_aggregate_bitboards_in_sync() {
        let mut board = Board::empty();
        assert_eq!(std::mem::size_of::<Board>(), 120);
        board
            .place_piece(Color::White, PieceType::Queen, Square::D4)
            .unwrap();
        board
            .place_piece(Color::Black, PieceType::Pawn, Square::E5)
            .unwrap();
        assert!(board.is_consistent());
        assert_eq!(
            board.piece_at(Square::D4),
            Some((Color::White, PieceType::Queen))
        );
        assert_eq!(
            board.remove_piece(Square::D4),
            Some((Color::White, PieceType::Queen))
        );
        assert!(board.is_consistent());
        assert!(
            board
                .place_piece(Color::Black, PieceType::King, Square::E5)
                .is_err()
        );
        assert_eq!(
            board.piece_at(Square::E5),
            Some((Color::Black, PieceType::Pawn))
        );
    }

    #[test]
    fn aggregate_bitboards_remain_consistent_through_repeated_updates() {
        let mut board = Board::empty();
        for index in (0..64).step_by(2) {
            let square = Square::try_from(index).unwrap();
            let color = if index % 4 == 0 {
                Color::White
            } else {
                Color::Black
            };
            let piece = PieceType::ALL[(index as usize / 2) % PieceType::ALL.len()];
            board.place_piece(color, piece, square).unwrap();
            assert!(board.is_consistent());
        }
        for index in (0..64).step_by(4) {
            board.remove_piece(Square::try_from(index).unwrap());
            assert!(board.is_consistent());
        }
    }

    #[test]
    fn initial_board_has_all_twelve_piece_bitboards() {
        let board = Board::starting_position();
        assert_eq!(board.occupancy().count_ones(), 32);
        assert_eq!(
            board.bitboard(PieceType::Pawn, Color::White).count_ones(),
            8
        );
        assert_eq!(
            board.bitboard(PieceType::Pawn, Color::Black).count_ones(),
            8
        );
        assert_eq!(
            board.bitboard(PieceType::King, Color::White),
            Square::E1.bit()
        );
        assert_eq!(
            board.bitboard(PieceType::King, Color::Black),
            Square::E8.bit()
        );
        assert!(board.is_consistent());
    }

    #[test]
    fn display_has_rank_eight_at_top_and_file_a_at_left() {
        let board = Board::starting_position();
        let display = board.to_string();
        let rows: Vec<_> = display.lines().collect();
        assert!(rows[0].starts_with("8 | r n b q k b n r"));
        assert!(rows[7].starts_with("1 | R N B Q K B N R"));
        assert!(rows[9].ends_with("a b c d e f g h"));
    }
}

use std::fmt;

use crate::types::{PieceType, Square};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum MoveFlag {
    Quiet = 0,
    DoublePawnPush = 1,
    KingSideCastle = 2,
    QueenSideCastle = 3,
    Capture = 4,
    EnPassant = 5,
    PromoteKnight = 8,
    PromoteBishop = 9,
    PromoteRook = 10,
    PromoteQueen = 11,
    PromoteKnightCapture = 12,
    PromoteBishopCapture = 13,
    PromoteRookCapture = 14,
    PromoteQueenCapture = 15,
}

impl MoveFlag {
    pub const ALL: [Self; 14] = [
        Self::Quiet,
        Self::DoublePawnPush,
        Self::KingSideCastle,
        Self::QueenSideCastle,
        Self::Capture,
        Self::EnPassant,
        Self::PromoteKnight,
        Self::PromoteBishop,
        Self::PromoteRook,
        Self::PromoteQueen,
        Self::PromoteKnightCapture,
        Self::PromoteBishopCapture,
        Self::PromoteRookCapture,
        Self::PromoteQueenCapture,
    ];

    pub const fn value(self) -> u8 {
        self as u8
    }

    pub const fn is_capture(self) -> bool {
        matches!(
            self,
            Self::Capture
                | Self::EnPassant
                | Self::PromoteKnightCapture
                | Self::PromoteBishopCapture
                | Self::PromoteRookCapture
                | Self::PromoteQueenCapture
        )
    }

    pub const fn is_promotion(self) -> bool {
        matches!(
            self,
            Self::PromoteKnight
                | Self::PromoteBishop
                | Self::PromoteRook
                | Self::PromoteQueen
                | Self::PromoteKnightCapture
                | Self::PromoteBishopCapture
                | Self::PromoteRookCapture
                | Self::PromoteQueenCapture
        )
    }

    pub const fn promotion_piece(self) -> Option<PieceType> {
        match self {
            Self::PromoteKnight | Self::PromoteKnightCapture => Some(PieceType::Knight),
            Self::PromoteBishop | Self::PromoteBishopCapture => Some(PieceType::Bishop),
            Self::PromoteRook | Self::PromoteRookCapture => Some(PieceType::Rook),
            Self::PromoteQueen | Self::PromoteQueenCapture => Some(PieceType::Queen),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Move {
    from: Square,
    to: Square,
    flag: MoveFlag,
}

impl Move {
    pub const fn new(from: Square, to: Square, flag: MoveFlag) -> Self {
        Self { from, to, flag }
    }

    pub const fn from(self) -> Square {
        self.from
    }

    pub const fn to(self) -> Square {
        self.to
    }

    pub const fn flag(self) -> MoveFlag {
        self.flag
    }

    pub const fn is_capture(self) -> bool {
        self.flag.is_capture()
    }

    pub const fn is_promotion(self) -> bool {
        self.flag.is_promotion()
    }

    pub const fn promotion_piece(self) -> Option<PieceType> {
        self.flag.promotion_piece()
    }
}

impl fmt::Display for Move {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}{}", self.from, self.to)?;
        if let Some(piece) = self.promotion_piece() {
            let symbol = match piece {
                PieceType::Knight => 'n',
                PieceType::Bishop => 'b',
                PieceType::Rook => 'r',
                PieceType::Queen => 'q',
                PieceType::Pawn | PieceType::King => unreachable!("promotion flag is restricted"),
            };
            write!(formatter, "{symbol}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_have_four_bit_reference_values_without_collisions() {
        for (index, flag) in MoveFlag::ALL.into_iter().enumerate() {
            assert!(flag.value() < 16);
            assert!(!MoveFlag::ALL[..index].contains(&flag));
        }
        assert_eq!(
            MoveFlag::ALL.map(MoveFlag::value),
            [0, 1, 2, 3, 4, 5, 8, 9, 10, 11, 12, 13, 14, 15]
        );
    }

    #[test]
    fn move_accessors_and_promotion_metadata_are_typed() {
        let mv = Move::new(Square::E7, Square::E8, MoveFlag::PromoteQueenCapture);
        assert_eq!(mv.from(), Square::E7);
        assert_eq!(mv.to(), Square::E8);
        assert_eq!(mv.flag(), MoveFlag::PromoteQueenCapture);
        assert!(mv.is_capture());
        assert!(mv.is_promotion());
        assert_eq!(mv.promotion_piece(), Some(PieceType::Queen));
        assert_eq!(
            Move::new(Square::E2, Square::E4, MoveFlag::Quiet).promotion_piece(),
            None
        );
        assert_eq!(std::mem::size_of::<Move>(), 3);
    }

    #[test]
    fn move_display_uses_uci_coordinate_notation() {
        assert_eq!(
            Move::new(Square::E2, Square::E4, MoveFlag::DoublePawnPush).to_string(),
            "e2e4"
        );
        assert_eq!(
            Move::new(Square::E7, Square::E8, MoveFlag::PromoteQueen).to_string(),
            "e7e8q"
        );
        assert_eq!(
            Move::new(Square::E1, Square::G1, MoveFlag::KingSideCastle).to_string(),
            "e1g1"
        );
        assert_eq!(
            Move::new(Square::E5, Square::D6, MoveFlag::EnPassant).to_string(),
            "e5d6"
        );
    }
}

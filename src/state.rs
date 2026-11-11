use crate::types::{Color, Square};

pub const WHITE_KING_SIDE: u8 = 1 << 0;
pub const WHITE_QUEEN_SIDE: u8 = 1 << 1;
pub const BLACK_KING_SIDE: u8 = 1 << 2;
pub const BLACK_QUEEN_SIDE: u8 = 1 << 3;
pub const ALL_CASTLING_RIGHTS: u8 =
    WHITE_KING_SIDE | WHITE_QUEEN_SIDE | BLACK_KING_SIDE | BLACK_QUEEN_SIDE;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PositionState {
    hash: u64,
    fullmove_number: u16,
    side_to_move: Color,
    castling_rights: u8,
    en_passant: Option<Square>,
    halfmove_clock: u8,
}

impl PositionState {
    pub const fn new(
        side_to_move: Color,
        castling_rights: u8,
        en_passant: Option<Square>,
        halfmove_clock: u8,
        fullmove_number: u16,
    ) -> Self {
        Self {
            hash: 0,
            fullmove_number,
            side_to_move,
            castling_rights: castling_rights & ALL_CASTLING_RIGHTS,
            en_passant,
            halfmove_clock,
        }
    }

    pub const fn hash(self) -> u64 {
        self.hash
    }

    pub const fn set_hash(&mut self, hash: u64) {
        self.hash = hash;
    }

    pub const fn side_to_move(self) -> Color {
        self.side_to_move
    }

    pub const fn set_side_to_move(&mut self, color: Color) {
        self.side_to_move = color;
    }

    pub const fn castling_rights(self) -> u8 {
        self.castling_rights
    }

    pub const fn has_castling_right(self, right: u8) -> bool {
        self.castling_rights & right != 0
    }

    pub const fn set_castling_rights(&mut self, rights: u8) {
        self.castling_rights = rights & ALL_CASTLING_RIGHTS;
    }

    pub const fn en_passant(self) -> Option<Square> {
        self.en_passant
    }

    pub const fn set_en_passant(&mut self, square: Option<Square>) {
        self.en_passant = square;
    }

    pub const fn halfmove_clock(self) -> u8 {
        self.halfmove_clock
    }

    pub const fn set_halfmove_clock(&mut self, halfmove_clock: u8) {
        self.halfmove_clock = halfmove_clock;
    }

    pub const fn fullmove_number(self) -> u16 {
        self.fullmove_number
    }

    pub const fn set_fullmove_number(&mut self, fullmove_number: u16) {
        self.fullmove_number = fullmove_number;
    }
}

impl Default for PositionState {
    fn default() -> Self {
        Self::new(Color::White, 0, None, 0, 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_keeps_position_metadata_separate_from_board() {
        let state = PositionState::default();
        assert_eq!(std::mem::size_of::<PositionState>(), 16);
        assert_eq!(state.hash(), 0);
        assert_eq!(state.side_to_move(), Color::White);
        assert_eq!(state.castling_rights(), 0);
        assert_eq!(state.en_passant(), None);
        assert_eq!(state.halfmove_clock(), 0);
        assert_eq!(state.fullmove_number(), 1);
        assert_eq!(
            PositionState::new(Color::Black, u8::MAX, Some(Square::E3), 99, 42).castling_rights(),
            ALL_CASTLING_RIGHTS
        );
    }
}

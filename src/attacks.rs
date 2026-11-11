use crate::magic_tables::{
    BISHOP_ATTACKS, BISHOP_MAGICS, BISHOP_MASKS, BISHOP_OFFSETS, BISHOP_SHIFTS, ROOK_ATTACKS,
    ROOK_MAGICS, ROOK_MASKS, ROOK_OFFSETS, ROOK_SHIFTS,
};
use crate::types::Square;

const ROOK_DIRECTIONS: [(i8, i8); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
const BISHOP_DIRECTIONS: [(i8, i8); 4] = [(1, 1), (1, -1), (-1, 1), (-1, -1)];

pub fn rook_attacks(square: Square, occupancy: u64) -> u64 {
    let index = square.index() as usize;
    let table_index =
        (occupancy & ROOK_MASKS[index]).wrapping_mul(ROOK_MAGICS[index]) >> ROOK_SHIFTS[index];
    ROOK_ATTACKS[ROOK_OFFSETS[index] + table_index as usize]
}

pub fn bishop_attacks(square: Square, occupancy: u64) -> u64 {
    let index = square.index() as usize;
    let table_index = (occupancy & BISHOP_MASKS[index]).wrapping_mul(BISHOP_MAGICS[index])
        >> BISHOP_SHIFTS[index];
    BISHOP_ATTACKS[BISHOP_OFFSETS[index] + table_index as usize]
}

pub fn queen_attacks(square: Square, occupancy: u64) -> u64 {
    rook_attacks(square, occupancy) | bishop_attacks(square, occupancy)
}

pub fn rook_attacks_ray(square: Square, occupancy: u64) -> u64 {
    attacks_in_directions(square, occupancy, &ROOK_DIRECTIONS)
}

pub fn bishop_attacks_ray(square: Square, occupancy: u64) -> u64 {
    attacks_in_directions(square, occupancy, &BISHOP_DIRECTIONS)
}

fn attacks_in_directions(square: Square, occupancy: u64, directions: &[(i8, i8); 4]) -> u64 {
    let mut attacks = 0;
    for &(file_delta, rank_delta) in directions {
        let mut file = square.file() as i8 + file_delta;
        let mut rank = square.rank() as i8 + rank_delta;
        while (0..8).contains(&file) && (0..8).contains(&rank) {
            let target = Square::from_file_rank(file as u8, rank as u8)
                .expect("ray coordinates are in range");
            let bit = target.bit();
            attacks |= bit;
            if occupancy & bit != 0 {
                break;
            }
            file += file_delta;
            rank += rank_delta;
        }
    }
    attacks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sliding_attacks_on_empty_board_match_expected_rays() {
        assert_eq!(rook_attacks(Square::D4, 0).count_ones(), 14);
        assert_eq!(bishop_attacks(Square::D4, 0).count_ones(), 13);
        assert_eq!(queen_attacks(Square::D4, 0).count_ones(), 27);
        assert_eq!(rook_attacks(Square::A1, 0).count_ones(), 14);
        assert_eq!(bishop_attacks(Square::A1, 0).count_ones(), 7);
    }

    #[test]
    fn sliding_attacks_include_the_first_blocker_but_not_squares_behind_it() {
        let blockers = Square::D6.bit() | Square::F4.bit() | Square::B2.bit();
        let rook = rook_attacks(Square::D4, blockers);
        assert_ne!(rook & Square::D6.bit(), 0);
        assert_eq!(rook & Square::D7.bit(), 0);
        assert_ne!(rook & Square::F4.bit(), 0);
        assert_eq!(rook & Square::G4.bit(), 0);

        let bishop = bishop_attacks(Square::D4, blockers);
        assert_ne!(bishop & Square::B2.bit(), 0);
        assert_eq!(bishop & Square::A1.bit(), 0);
    }

    #[test]
    fn magic_attacks_match_ray_reference_for_deterministic_occupancies() {
        let mut random = 0xD1B5_4A32_D192_ED03_u64;
        for square in Square::ALL {
            for occupancy in [0, u64::MAX] {
                assert_eq!(
                    rook_attacks(square, occupancy),
                    rook_attacks_ray(square, occupancy)
                );
                assert_eq!(
                    bishop_attacks(square, occupancy),
                    bishop_attacks_ray(square, occupancy)
                );
            }
            for _ in 0..128 {
                random ^= random << 13;
                random ^= random >> 7;
                random ^= random << 17;
                assert_eq!(
                    rook_attacks(square, random),
                    rook_attacks_ray(square, random),
                    "rook square {square}"
                );
                assert_eq!(
                    bishop_attacks(square, random),
                    bishop_attacks_ray(square, random),
                    "bishop square {square}"
                );
            }
        }
    }
}

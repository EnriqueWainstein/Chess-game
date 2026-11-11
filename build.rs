use std::collections::HashSet;
use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

const ROOK_DIRECTIONS: [(i8, i8); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
const BISHOP_DIRECTIONS: [(i8, i8); 4] = [(1, 1), (1, -1), (-1, 1), (-1, -1)];

struct MagicSet {
    masks: [u64; 64],
    magics: [u64; 64],
    shifts: [u8; 64],
    offsets: [usize; 64],
    attacks: Vec<u64>,
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let mut random = 0x9E37_79B9_7F4A_7C15;
    let rooks = build_magics(&ROOK_DIRECTIONS, &mut random);
    let bishops = build_magics(&BISHOP_DIRECTIONS, &mut random);
    let mut generated = String::new();
    emit_set(&mut generated, "ROOK", &rooks);
    emit_set(&mut generated, "BISHOP", &bishops);

    let output =
        PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR is set")).join("magic_tables.rs");
    fs::write(output, generated).expect("write generated magic tables");
    let mut zobrist = String::new();
    emit_zobrist(&mut zobrist);
    let output = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR is set"))
        .join("zobrist_tables.rs");
    fs::write(output, zobrist).expect("write generated Zobrist table");
}

fn emit_zobrist(output: &mut String) {
    let mut random = 0xD1B5_4A32_D192_ED03;
    let mut seen = HashSet::with_capacity(12 * 64 + 1 + 16 + 8);
    let mut keys = Vec::with_capacity(12 * 64 + 1 + 16 + 8);
    while keys.len() < 12 * 64 + 1 + 16 + 8 {
        let key = next_random(&mut random);
        if key != 0 && seen.insert(key) {
            keys.push(key);
        }
    }

    writeln!(output, "pub const ZOBRIST_PIECES: [[u64; 64]; 12] = [").unwrap();
    for row_index in 0..12 {
        let row = &keys[row_index * 64..(row_index + 1) * 64];
        output.push_str("    [\n");
        for chunk in row.chunks(4) {
            output.push_str("        ");
            for key in chunk {
                write!(output, "{key:#018x}, ").unwrap();
            }
            output.push('\n');
        }
        output.push_str("    ],\n");
    }
    output.push_str("];\n");
    let mut next = 12 * 64;
    writeln!(
        output,
        "pub const ZOBRIST_TURN: u64 = {:#018x};",
        keys[next]
    )
    .unwrap();
    next += 1;
    emit_const_u64_array(output, "ZOBRIST_CASTLING", &keys[next..next + 16]);
    next += 16;
    emit_const_u64_array(output, "ZOBRIST_EN_PASSANT_FILE", &keys[next..next + 8]);
}

fn build_magics(directions: &[(i8, i8); 4], random: &mut u64) -> MagicSet {
    let mut masks = [0; 64];
    let mut magics = [0; 64];
    let mut shifts = [0; 64];
    let mut offsets = [0; 64];
    let mut attacks = Vec::new();

    for square_index in 0..64 {
        let square = square_index as u8;
        let mask = relevant_mask(square, directions);
        let bit_count = mask.count_ones() as u8;
        let shift = 64 - bit_count;
        let table_size = 1_usize << bit_count;
        let mut occupancies = Vec::with_capacity(table_size);
        let mut reference_attacks = Vec::with_capacity(table_size);
        let mut subset = 0_u64;
        loop {
            occupancies.push(subset);
            reference_attacks.push(ray_attacks(square, subset, directions));
            subset = subset.wrapping_sub(mask) & mask;
            if subset == 0 {
                break;
            }
        }

        let (magic, table) = find_magic(mask, shift, &occupancies, &reference_attacks, random);
        masks[square_index] = mask;
        magics[square_index] = magic;
        shifts[square_index] = shift;
        offsets[square_index] = attacks.len();
        attacks.extend(table);
    }

    MagicSet {
        masks,
        magics,
        shifts,
        offsets,
        attacks,
    }
}

fn find_magic(
    mask: u64,
    shift: u8,
    occupancies: &[u64],
    reference_attacks: &[u64],
    random: &mut u64,
) -> (u64, Vec<u64>) {
    let table_size = occupancies.len();
    let mut used = vec![false; table_size];
    let mut table = vec![0; table_size];
    let mut attempts = 0_u64;
    loop {
        attempts += 1;
        if attempts > 200_000_000 {
            panic!("failed to find magic multiplier for mask {mask:#018x}");
        }
        let magic = next_random(random) & next_random(random) & next_random(random);
        if (mask.wrapping_mul(magic) & 0xFF00_0000_0000_0000).count_ones() < 6 {
            continue;
        }
        used.fill(false);
        let mut valid = true;
        for (&occupancy, &attack) in occupancies.iter().zip(reference_attacks) {
            let index = (occupancy.wrapping_mul(magic) >> shift) as usize;
            if used[index] && table[index] != attack {
                valid = false;
                break;
            }
            used[index] = true;
            table[index] = attack;
        }
        if valid {
            return (magic, table);
        }
    }
}

fn next_random(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

fn relevant_mask(square: u8, directions: &[(i8, i8); 4]) -> u64 {
    let mut mask = 0;
    let file = (square % 8) as i8;
    let rank = (square / 8) as i8;
    for &(file_delta, rank_delta) in directions {
        let mut target_file = file + file_delta;
        let mut target_rank = rank + rank_delta;
        while on_board(target_file, target_rank) {
            let next_file = target_file + file_delta;
            let next_rank = target_rank + rank_delta;
            if !on_board(next_file, next_rank) {
                break;
            }
            mask |= 1_u64 << (target_rank as u32 * 8 + target_file as u32);
            target_file = next_file;
            target_rank = next_rank;
        }
    }
    mask
}

fn ray_attacks(square: u8, occupancy: u64, directions: &[(i8, i8); 4]) -> u64 {
    let mut attacks = 0;
    let file = (square % 8) as i8;
    let rank = (square / 8) as i8;
    for &(file_delta, rank_delta) in directions {
        let mut target_file = file + file_delta;
        let mut target_rank = rank + rank_delta;
        while on_board(target_file, target_rank) {
            let bit = 1_u64 << (target_rank as u32 * 8 + target_file as u32);
            attacks |= bit;
            if occupancy & bit != 0 {
                break;
            }
            target_file += file_delta;
            target_rank += rank_delta;
        }
    }
    attacks
}

fn on_board(file: i8, rank: i8) -> bool {
    (0..8).contains(&file) && (0..8).contains(&rank)
}

fn emit_set(output: &mut String, prefix: &str, set: &MagicSet) {
    emit_u64_array(output, &format!("{prefix}_MASKS"), &set.masks);
    emit_u64_array(output, &format!("{prefix}_MAGICS"), &set.magics);
    emit_u8_array(output, &format!("{prefix}_SHIFTS"), &set.shifts);
    emit_usize_array(output, &format!("{prefix}_OFFSETS"), &set.offsets);
    emit_u64_array(output, &format!("{prefix}_ATTACKS"), &set.attacks);
}

fn emit_u64_array(output: &mut String, name: &str, values: &[u64]) {
    writeln!(output, "pub static {name}: [u64; {}] = [", values.len()).unwrap();
    for chunk in values.chunks(4) {
        output.push_str("    ");
        for value in chunk {
            write!(output, "{value:#018x}, ").unwrap();
        }
        output.push('\n');
    }
    output.push_str("];\n");
}

fn emit_const_u64_array(output: &mut String, name: &str, values: &[u64]) {
    writeln!(output, "pub const {name}: [u64; {}] = [", values.len()).unwrap();
    for chunk in values.chunks(4) {
        output.push_str("    ");
        for value in chunk {
            write!(output, "{value:#018x}, ").unwrap();
        }
        output.push('\n');
    }
    output.push_str("];\n");
}

fn emit_u8_array(output: &mut String, name: &str, values: &[u8]) {
    writeln!(output, "pub const {name}: [u8; {}] = [", values.len()).unwrap();
    for chunk in values.chunks(16) {
        output.push_str("    ");
        for value in chunk {
            write!(output, "{value}, ").unwrap();
        }
        output.push('\n');
    }
    output.push_str("];\n");
}

fn emit_usize_array(output: &mut String, name: &str, values: &[usize]) {
    writeln!(output, "pub const {name}: [usize; {}] = [", values.len()).unwrap();
    for chunk in values.chunks(8) {
        output.push_str("    ");
        for value in chunk {
            write!(output, "{value}, ").unwrap();
        }
        output.push('\n');
    }
    output.push_str("];\n");
}

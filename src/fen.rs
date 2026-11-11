use std::fmt;

use crate::board::{Board, BoardError};
use crate::state::{
    BLACK_KING_SIDE, BLACK_QUEEN_SIDE, PositionState, WHITE_KING_SIDE, WHITE_QUEEN_SIDE,
};
use crate::types::{Color, PieceType, Square};

pub const STARTING_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FenError {
    InvalidFieldCount { found: usize },
    InvalidPlacement,
    InvalidPiece(char),
    InvalidRankWidth { rank: u8, width: u8 },
    InvalidActiveColor,
    InvalidCastlingRights(char),
    InvalidEnPassant(String),
    InvalidHalfmoveClock,
    InvalidFullmoveNumber,
}

impl fmt::Display for FenError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFieldCount { found } => {
                write!(
                    formatter,
                    "FEN must contain exactly 6 fields; found {found}"
                )
            }
            Self::InvalidPlacement => write!(formatter, "invalid FEN piece placement"),
            Self::InvalidPiece(piece) => write!(formatter, "invalid FEN piece character: {piece}"),
            Self::InvalidRankWidth { rank, width } => write!(
                formatter,
                "FEN rank {rank} describes {width} squares instead of 8"
            ),
            Self::InvalidActiveColor => {
                write!(formatter, "FEN active color must be 'w' or 'b'")
            }
            Self::InvalidCastlingRights(right) => {
                write!(formatter, "invalid or repeated FEN castling right: {right}")
            }
            Self::InvalidEnPassant(square) => {
                write!(formatter, "invalid FEN en-passant square: {square}")
            }
            Self::InvalidHalfmoveClock => {
                write!(
                    formatter,
                    "FEN halfmove clock must be an integer from 0 to 255"
                )
            }
            Self::InvalidFullmoveNumber => {
                write!(formatter, "FEN fullmove number must be a positive integer")
            }
        }
    }
}

impl std::error::Error for FenError {}

impl From<BoardError> for FenError {
    fn from(_: BoardError) -> Self {
        Self::InvalidPlacement
    }
}

pub fn parse_fen(fen: &str) -> Result<(Board, PositionState), FenError> {
    let mut fields = fen.split_whitespace();
    let placement = fields.next();
    let active_color = fields.next();
    let castling = fields.next();
    let en_passant = fields.next();
    let halfmove_clock = fields.next();
    let fullmove_number = fields.next();
    let found = [
        placement,
        active_color,
        castling,
        en_passant,
        halfmove_clock,
        fullmove_number,
    ]
    .into_iter()
    .filter(Option::is_some)
    .count()
        + fields.count();
    if found != 6 {
        return Err(FenError::InvalidFieldCount { found });
    }

    let mut board = Board::empty();
    parse_placement(placement.expect("field count was checked"), &mut board)?;
    let side_to_move = match active_color.expect("field count was checked") {
        "w" => Color::White,
        "b" => Color::Black,
        _ => return Err(FenError::InvalidActiveColor),
    };
    let castling_rights = parse_castling(castling.expect("field count was checked"))?;
    let en_passant = parse_en_passant(en_passant.expect("field count was checked"), side_to_move)?;
    let halfmove_clock = halfmove_clock
        .expect("field count was checked")
        .parse::<u8>()
        .map_err(|_| FenError::InvalidHalfmoveClock)?;
    let fullmove_number = fullmove_number
        .expect("field count was checked")
        .parse::<u16>()
        .ok()
        .filter(|&number| number > 0)
        .ok_or(FenError::InvalidFullmoveNumber)?;

    let mut state = PositionState::new(
        side_to_move,
        castling_rights,
        en_passant,
        halfmove_clock,
        fullmove_number,
    );
    state.set_hash(crate::zobrist::calculate_hash(&board, &state));
    Ok((board, state))
}

pub fn to_fen(board: &Board, state: &PositionState) -> String {
    let mut fen = String::with_capacity(96);
    for rank in (0..8).rev() {
        if rank != 7 {
            fen.push('/');
        }
        let mut empty_squares = 0;
        for file in 0..8 {
            let square = Square::from_file_rank(file, rank).expect("rank and file are in range");
            if let Some((color, piece)) = board.piece_at(square) {
                if empty_squares > 0 {
                    fen.push(char::from(b'0' + empty_squares));
                    empty_squares = 0;
                }
                fen.push(piece.fen_char(color));
            } else {
                empty_squares += 1;
            }
        }
        if empty_squares > 0 {
            fen.push(char::from(b'0' + empty_squares));
        }
    }

    fen.push(' ');
    fen.push(match state.side_to_move() {
        Color::White => 'w',
        Color::Black => 'b',
    });
    fen.push(' ');
    let rights = state.castling_rights();
    if rights == 0 {
        fen.push('-');
    } else {
        for (right, symbol) in [
            (WHITE_KING_SIDE, 'K'),
            (WHITE_QUEEN_SIDE, 'Q'),
            (BLACK_KING_SIDE, 'k'),
            (BLACK_QUEEN_SIDE, 'q'),
        ] {
            if rights & right != 0 {
                fen.push(symbol);
            }
        }
    }
    fen.push(' ');
    if let Some(square) = state.en_passant() {
        fen.push_str(&square.to_string());
    } else {
        fen.push('-');
    }
    use std::fmt::Write as _;
    write!(
        &mut fen,
        " {} {}",
        state.halfmove_clock(),
        state.fullmove_number()
    )
    .expect("writing to a String cannot fail");
    fen
}

pub fn starting_position() -> (Board, PositionState) {
    parse_fen(STARTING_FEN).expect("the built-in starting FEN is valid")
}

fn parse_placement(placement: &str, board: &mut Board) -> Result<(), FenError> {
    let mut rank_count = 0;
    for (rank_index, row) in placement.split('/').enumerate() {
        if rank_index >= 8 {
            return Err(FenError::InvalidPlacement);
        }
        rank_count += 1;
        let rank = 7 - rank_index as u8;
        let mut file = 0_u8;
        let mut previous_was_digit = false;
        for character in row.chars() {
            if ('1'..='8').contains(&character) {
                if previous_was_digit {
                    return Err(FenError::InvalidPlacement);
                }
                let empty_count = character as u8 - b'0';
                file = file
                    .checked_add(empty_count)
                    .ok_or(FenError::InvalidPlacement)?;
                previous_was_digit = true;
            } else if let Some((color, piece)) = PieceType::from_fen_char(character) {
                if file >= 8 {
                    return Err(FenError::InvalidRankWidth {
                        rank: rank + 1,
                        width: file,
                    });
                }
                let square =
                    Square::from_file_rank(file, rank).expect("rank and file are in range");
                board.place_piece(color, piece, square)?;
                file += 1;
                previous_was_digit = false;
            } else if character.is_ascii_digit() {
                return Err(FenError::InvalidPlacement);
            } else {
                return Err(FenError::InvalidPiece(character));
            }
            if file > 8 {
                return Err(FenError::InvalidRankWidth {
                    rank: rank + 1,
                    width: file,
                });
            }
        }
        if file != 8 {
            return Err(FenError::InvalidRankWidth {
                rank: rank + 1,
                width: file,
            });
        }
    }
    if rank_count != 8 {
        return Err(FenError::InvalidPlacement);
    }
    Ok(())
}

fn parse_castling(castling: &str) -> Result<u8, FenError> {
    if castling == "-" {
        return Ok(0);
    }
    let mut rights = 0;
    for character in castling.chars() {
        let right = match character {
            'K' => WHITE_KING_SIDE,
            'Q' => WHITE_QUEEN_SIDE,
            'k' => BLACK_KING_SIDE,
            'q' => BLACK_QUEEN_SIDE,
            _ => return Err(FenError::InvalidCastlingRights(character)),
        };
        if rights & right != 0 {
            return Err(FenError::InvalidCastlingRights(character));
        }
        rights |= right;
    }
    if rights == 0 {
        return Err(FenError::InvalidCastlingRights('-'));
    }
    Ok(rights)
}

fn parse_en_passant(square: &str, side_to_move: Color) -> Result<Option<Square>, FenError> {
    if square == "-" {
        return Ok(None);
    }
    let bytes = square.as_bytes();
    if bytes.len() != 2 || !(b'a'..=b'h').contains(&bytes[0]) {
        return Err(FenError::InvalidEnPassant(square.to_owned()));
    }
    let rank = match (side_to_move, bytes[1]) {
        (Color::Black, b'3') => 2,
        (Color::White, b'6') => 5,
        _ => return Err(FenError::InvalidEnPassant(square.to_owned())),
    };
    let file = bytes[0] - b'a';
    Ok(Some(
        Square::from_file_rank(file, rank).expect("validated en-passant coordinates"),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PERFT_FENS: [&str; 5] = [
        STARTING_FEN,
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
        "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
    ];
    const OTHER_ANNEX_FENS: [&str; 13] = [
        "R6R/3Q4/1Q4Q1/4Q3/2Q4Q/Q4Q2/pp1Q4/kBNN1KB1 w - - 0 1",
        "8/8/8/8/k2Pp2Q/8/8/3K4 b - d3 0 1",
        "7k/6Q1/6K1/8/8/8/8/8 b - - 0 1",
        "7k/5Q2/6K1/8/8/8/8/8 b - - 0 1",
        "4k3/8/8/8/8/8/8/4K3 w - - 0 1",
        "4k3/8/8/8/8/8/8/4KN2 w - - 0 1",
        "4k3/8/8/8/8/8/8/4K2R w - - 100 80",
        "6k1/5ppp/8/8/8/8/5PPP/R5K1 w - - 0 1",
        "4k3/8/8/3q4/8/8/8/3RK3 w - - 0 1",
        "4k3/8/4p3/3p4/8/8/3Q4/4K3 w - - 0 1",
        "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1",
        "rnbqkbnr/pppp1ppp/8/4p3/8/8/PPPPPPPP/RNBQKBNR w KQkq e6 0 1",
        STARTING_FEN,
    ];

    #[test]
    fn valid_annex_perft_fens_round_trip() {
        for fen in PERFT_FENS {
            let (board, state) = parse_fen(fen).unwrap();
            assert_eq!(to_fen(&board, &state), fen);
            assert!(board.is_consistent());
        }
    }

    #[test]
    fn other_annex_positions_parse_and_round_trip() {
        for fen in OTHER_ANNEX_FENS {
            let (board, state) = parse_fen(fen).unwrap();
            assert_eq!(to_fen(&board, &state), fen);
            assert!(board.is_consistent());
        }
    }

    #[test]
    fn starting_position_matches_standard_fen_and_board() {
        let (board, state) = starting_position();
        assert_eq!(to_fen(&board, &state), STARTING_FEN);
        assert_eq!(board, Board::starting_position());
    }

    #[test]
    fn invalid_fens_return_descriptive_errors() {
        for fen in [
            "8/8/8/8/8/8/8/8 w - - 0",
            "8/8/8/8/8/8/8/7x w - - 0 1",
            "8/8/8/8/8/8/8/7 w - - 0 1",
            "8/8/8/8/8/8/8/44 w - - 0 1",
            "8/8/8/8/8/8/8/٠8 w - - 0 1",
            "8/8/8/8/8/8/8/8 x - - 0 1",
            "8/8/8/8/8/8/8/8 w - - 0 1 extra",
            "8/8/8/8/8/8/8/8 w KK - 0 1",
            "8/8/8/8/8/8/8/8 w - e3 0 1",
            "8/8/8/8/8/8/8/8 w - - 256 1",
            "8/8/8/8/8/8/8/8 w - - 0 0",
        ] {
            assert!(parse_fen(fen).is_err(), "{fen}");
        }
    }

    #[test]
    fn en_passant_and_castling_fields_round_trip() {
        let fen = "8/8/8/8/k2Pp2Q/8/8/3K4 b Kq d3 17 42";
        let (board, state) = parse_fen(fen).unwrap();
        assert_eq!(state.castling_rights(), WHITE_KING_SIDE | BLACK_QUEEN_SIDE);
        assert_eq!(state.en_passant(), Some(Square::D3));
        assert_eq!(state.halfmove_clock(), 17);
        assert_eq!(state.fullmove_number(), 42);
        assert_eq!(to_fen(&board, &state), fen);
    }
}

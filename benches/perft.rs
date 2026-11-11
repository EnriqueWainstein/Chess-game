use chess_game::attacks::{bishop_attacks, bishop_attacks_ray, rook_attacks, rook_attacks_ray};
use chess_game::fen::starting_position;
use chess_game::movegen::perft;
use chess_game::types::Square;
use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};

fn bench_perft(c: &mut Criterion) {
    let mut group = c.benchmark_group("perft");
    let (mut board, mut state) = starting_position();
    group.throughput(Throughput::Elements(4_865_609));
    group.bench_function("startpos-depth-5-magics", |b| {
        b.iter(|| {
            let nodes = perft(black_box(&mut board), black_box(&mut state), black_box(5))
                .expect("starting-position perft succeeds");
            assert_eq!(nodes, 4_865_609);
            nodes
        })
    });
    group.finish();
}

fn bench_sliding_attacks(c: &mut Criterion) {
    let occupancies = [
        0x8100_0000_0000_0081,
        0x00FF_0000_0000_FF00,
        0x0042_0000_0000_4200,
        0x0000_1800_0018_0000,
    ];
    let mut group = c.benchmark_group("sliding-attacks-64-squares");
    group.throughput(Throughput::Elements(128));
    for (name, magic, ray) in [
        (
            "rook",
            rook_attacks as fn(Square, u64) -> u64,
            rook_attacks_ray as fn(Square, u64) -> u64,
        ),
        (
            "bishop",
            bishop_attacks as fn(Square, u64) -> u64,
            bishop_attacks_ray as fn(Square, u64) -> u64,
        ),
    ] {
        for (implementation, attacks) in [("magic", magic), ("ray", ray)] {
            group.bench_with_input(
                BenchmarkId::new(name, implementation),
                &occupancies,
                |b, occupancies| {
                    b.iter(|| {
                        let mut result = 0;
                        for square in Square::ALL {
                            for occupancy in occupancies {
                                result ^= attacks(square, black_box(*occupancy));
                            }
                        }
                        black_box(result)
                    })
                },
            );
        }
    }
    group.finish();
}

criterion_group!(benches, bench_perft, bench_sliding_attacks);
criterion_main!(benches);

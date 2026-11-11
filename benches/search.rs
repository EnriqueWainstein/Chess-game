use chess_game::fen::{parse_fen, starting_position};
use chess_game::history::PositionHistory;
use chess_game::search::{SearchLimits, search};
use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use std::sync::atomic::AtomicBool;

fn bench_start_position_search(c: &mut Criterion) {
    let (board, state) = starting_position();
    let history = PositionHistory::new(state.hash());
    let limits = SearchLimits::new(4);
    let stop = AtomicBool::new(false);
    let baseline =
        search(&board, &state, &history, limits, &stop).expect("starting-position search succeeds");
    eprintln!("startpos depth 4: {} nodes per search", baseline.nodes);
    let mut group = c.benchmark_group("search");
    group.throughput(Throughput::Elements(baseline.nodes));
    group.bench_function("startpos-depth-4", |b| {
        b.iter(|| {
            let result = search(
                black_box(&board),
                black_box(&state),
                black_box(&history),
                limits,
                black_box(&stop),
            )
            .expect("starting-position search succeeds");
            assert_eq!(result, baseline);
            result.nodes
        })
    });
    group.finish();
}

fn bench_tactical_search(c: &mut Criterion) {
    let (board, state) =
        parse_fen("4k3/8/8/3q4/8/8/8/3RK3 w - - 0 1").expect("tactical benchmark FEN is valid");
    let history = PositionHistory::new(state.hash());
    let limits = SearchLimits::new(3);
    let stop = AtomicBool::new(false);
    let baseline =
        search(&board, &state, &history, limits, &stop).expect("tactical search succeeds");
    let mut group = c.benchmark_group("search");
    group.throughput(Throughput::Elements(baseline.nodes));
    group.bench_function("queen-capture-depth-3", |b| {
        b.iter(|| {
            let result = search(
                black_box(&board),
                black_box(&state),
                black_box(&history),
                limits,
                black_box(&stop),
            )
            .expect("tactical search succeeds");
            assert_eq!(result, baseline);
            result.nodes
        })
    });
    group.finish();
}

criterion_group!(benches, bench_start_position_search, bench_tactical_search);
criterion_main!(benches);

use std::io::Write;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

fn engine() -> std::process::Child {
    Command::new(env!("CARGO_BIN_EXE_chess-game"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("engine process starts")
}

fn send(child: &mut std::process::Child, command: &str) {
    writeln!(child.stdin.as_mut().unwrap(), "{command}").unwrap();
    child.stdin.as_mut().unwrap().flush().unwrap();
}

#[test]
fn protocol_handshake_position_errors_diagnostics_and_perft_work() {
    let mut child = engine();
    for command in [
        "uci",
        "isready",
        "unknown-command",
        "position startpos moves e2e4 e7e5",
        "position startpos moves e2e5",
        "d",
        "position startpos",
        "go perft 4",
        "quit",
    ] {
        send(&mut child, command);
    }
    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stdout.contains("id name Chess Game"));
    assert!(stdout.contains("id author Chess-game contributors"));
    assert!(stdout.contains("uciok"));
    assert!(stdout.contains("readyok"));
    assert!(
        stdout.contains(
            "info string fen rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq e6 0 2"
        )
    );
    assert!(stdout.contains("info string perft nodes 197281"));
    assert!(stderr.contains("illegal move: e2e5"));
}

#[test]
fn go_depth_emits_info_and_mate_in_one_uses_uci_mate_score() {
    let mut child = engine();
    send(
        &mut child,
        "position fen 6k1/5ppp/8/8/8/8/5PPP/R5K1 w - - 0 1",
    );
    send(&mut child, "go depth 3");
    thread::sleep(Duration::from_millis(100));
    send(&mut child, "quit");
    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("info depth 1 score mate 1"));
    assert_eq!(
        stdout
            .lines()
            .filter(|line| line.starts_with("bestmove "))
            .count(),
        1
    );
    assert!(stdout.contains("bestmove a1a8"));
}

#[test]
fn infinite_search_remains_responsive_and_stop_emits_one_legal_bestmove() {
    let mut child = engine();
    send(&mut child, "go infinite");
    send(&mut child, "isready");
    thread::sleep(Duration::from_millis(20));
    send(&mut child, "stop");
    send(&mut child, "quit");
    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("readyok"));
    assert_eq!(
        stdout
            .lines()
            .filter(|line| line.starts_with("bestmove "))
            .count(),
        1
    );
    assert!(
        stdout
            .lines()
            .any(|line| line.starts_with("bestmove ") && !line.ends_with("0000"))
    );
}

#[test]
fn terminal_position_emits_null_move_and_stop_latency_is_bounded() {
    let mut child = engine();
    send(&mut child, "position fen 7k/6Q1/6K1/8/8/8/8/8 b - - 0 1");
    send(&mut child, "go depth 1");
    thread::sleep(Duration::from_millis(20));
    send(&mut child, "quit");
    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("bestmove 0000"));

    let mut child = engine();
    send(&mut child, "go infinite");
    thread::sleep(Duration::from_millis(20));
    let started = std::time::Instant::now();
    send(&mut child, "stop");
    send(&mut child, "quit");
    let output = child.wait_with_output().unwrap();
    let elapsed = started.elapsed();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        elapsed < Duration::from_millis(500),
        "stop took {elapsed:?}"
    );
    assert_eq!(
        stdout
            .lines()
            .filter(|line| line.starts_with("bestmove "))
            .count(),
        1
    );
}

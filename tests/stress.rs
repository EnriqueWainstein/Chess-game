use std::io::Write;
use std::process::{Command, Stdio};

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
fn repeated_go_stop_and_position_cycles_emit_one_bestmove_each() {
    const CYCLES: usize = 200;
    let mut child = engine();

    for cycle in 0..CYCLES {
        send(&mut child, "go depth 8");
        send(&mut child, "stop");
        if cycle % 2 == 0 {
            send(&mut child, "position startpos moves e2e4 e7e5");
        } else {
            send(&mut child, "ucinewgame");
        }
        send(&mut child, "isready");
    }
    send(&mut child, "quit");

    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        stdout
            .lines()
            .filter(|line| line.starts_with("bestmove "))
            .count(),
        CYCLES
    );
    assert_eq!(
        stdout.lines().filter(|line| *line == "readyok").count(),
        CYCLES
    );
    assert!(
        stdout
            .lines()
            .filter(|line| line.starts_with("bestmove "))
            .all(|line| !line.ends_with("0000"))
    );
}

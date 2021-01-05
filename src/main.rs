mod search;
mod board;

use std::time::Instant;

use board::{Action, Board};

fn path_to_string(path: &Vec<Action>) -> String {
    path.into_iter()
        .map(|a| match a {
            Action::Up => 'u',
            Action::Down => 'd',
            Action::Left => 'l',
            Action::Right => 'r',
        })
        .collect()
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 2 {
        println!("Usage: {} <sokoban level file>", args[0]);
        std::process::exit(1);
    }

    let level_string = std::fs::read_to_string(&args[1])?;
    let (board, start) = Board::parse_level_string(&level_string).unwrap();

    let start_time = Instant::now();

    match search::find_path(&board, &start) {
        Some(path) => println!("Found solution: {}", path_to_string(&path)),
        None => println!("Exhausted search, level is not solvable."),
    }

    println!(
        "Search finished after {} seconds.",
        start_time.elapsed().as_secs_f64()
    );

    Ok(())
}

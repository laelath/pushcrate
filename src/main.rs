use std::collections::{HashSet, VecDeque};
use std::env;

#[derive(PartialEq, Eq, Clone, Copy)]
enum Action {
    Up,
    Down,
    Left,
    Right,
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
enum Tile {
    Empty,
    Wall,
    Box,
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
struct Board {
    width: usize,
    height: usize,
    player: (usize, usize),
    goals: Vec<(usize, usize)>,
    tiles: Vec<Tile>,
}

impl Board {
    fn get_tile(&self, x: usize, y: usize) -> Tile {
        self.tiles[y * self.width + x]
    }

    fn set_tile(&mut self, x: usize, y: usize, tile: Tile) {
        self.tiles[y * self.width + x] = tile
    }

    fn is_satisfied(&self) -> bool {
        for (x, y) in &self.goals {
            if self.get_tile(*x, *y) != Tile::Box {
                return false;
            }
        }
        true
    }

    fn do_move(&mut self, action: Action) {
        let px = self.player.0;
        let py = self.player.1;

        match action {
            Action::Up => match self.get_tile(px, py - 1) {
                Tile::Empty => self.player.1 = py - 1,
                Tile::Wall => (),
                Tile::Box => {
                    if self.get_tile(px, py - 2) == Tile::Empty {
                        self.set_tile(px, py - 2, Tile::Box);
                        self.set_tile(px, py - 1, Tile::Empty);
                        self.player.1 = py - 1;
                    }
                }
            },
            Action::Down => match self.get_tile(px, py + 1) {
                Tile::Empty => self.player.1 = py + 1,
                Tile::Wall => (),
                Tile::Box => {
                    if self.get_tile(px, py + 2) == Tile::Empty {
                        self.set_tile(px, py + 2, Tile::Box);
                        self.set_tile(px, py + 1, Tile::Empty);
                        self.player.1 = py + 1;
                    }
                }
            },
            Action::Left => match self.get_tile(px - 1, py) {
                Tile::Empty => self.player.0 = px - 1,
                Tile::Wall => (),
                Tile::Box => {
                    if self.get_tile(px - 2, py) == Tile::Empty {
                        self.set_tile(px - 2, py, Tile::Box);
                        self.set_tile(px - 1, py, Tile::Empty);
                        self.player.0 = px - 1;
                    }
                }
            },
            Action::Right => match self.get_tile(px + 1, py) {
                Tile::Empty => self.player.0 = px + 1,
                Tile::Wall => (),
                Tile::Box => {
                    if self.get_tile(px + 2, py) == Tile::Empty {
                        self.set_tile(px + 2, py, Tile::Box);
                        self.set_tile(px + 1, py, Tile::Empty);
                        self.player.0 = px + 1;
                    }
                }
            },
        }
    }
}

fn parse_level_string(level: &String) -> Result<Board, &'static str> {
    // ensure that the level only contains valid characters
    for c in level.chars() {
        if !"#pPbB@+$*. -_\n".contains(c) {
            return Err("Level contains invalid character");
        }
    }

    let lines: Vec<&str> = level
        .split('\n')
        .map(|s| s.trim_end()) // trim trailing whitespace on all lines
        .skip_while(|s| s == &"") // skip empty preceding lines
        .take_while(|s| s != &"") // take until the empty trailing lines
        .collect();

    if lines.is_empty() {
        return Err("Level is empty");
    }

    let height = lines.len();
    let width = lines.iter().map(|s| s.len()).max().unwrap();

    let mut players = Vec::new();

    let mut goals = Vec::new();
    let mut tiles = vec![Tile::Empty; width * height];

    let mut num_boxes = 0;

    for (i, line) in lines.into_iter().enumerate() {
        for (j, c) in line.chars().enumerate() {
            match c {
                '#' => tiles[width * i + j] = Tile::Wall,
                'p' | '@' => players.push((j, i)),
                'P' | '+' => {
                    players.push((j, i));
                    goals.push((j, i));
                }
                'b' | '$' => {
                    tiles[width * i + j] = Tile::Box;
                    num_boxes += 1;
                }
                'B' | '*' => {
                    tiles[width * i + j] = Tile::Box;
                    goals.push((j, i));
                    num_boxes += 1;
                }
                '.' => goals.push((j, i)),
                ' ' | '-' | '_' => (),
                _ => panic!(),
            }
        }
    }

    if players.len() == 0 {
        return Err("Level has no player");
    } else if players.len() > 1 {
        return Err("Level has more than one player");
    }

    if num_boxes != goals.len() {
        return Err("Number of boxes and number of goals are not the same");
    }

    // TODO: verify the level is enclosed in walls

    Ok(Board {
        width: width,
        height: height,
        player: players[0],
        goals: goals,
        tiles: tiles,
    })
}

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
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        println!("Usage: {} <sokoban level file>", args[0]);
        std::process::exit(1);
    }

    let level_string = std::fs::read_to_string(&args[1])?;
    let start = parse_level_string(&level_string).unwrap();

    let mut seen: HashSet<Board> = HashSet::new();
    let mut queue: VecDeque<(Board, Vec<Action>)> = VecDeque::new();

    seen.insert(start.clone());
    queue.push_back((start, vec![]));

    loop {
        match queue.pop_front() {
            None => {
                println!("Exhausted search, level is not solvable.");
                break;
            }
            Some((board, path)) => {
                if board.is_satisfied() {
                    println!("Found solution: {}", path_to_string(&path));
                    break;
                }

                for action in &[Action::Up, Action::Down, Action::Left, Action::Right] {
                    let mut child = board.clone();
                    child.do_move(*action);

                    if !seen.contains(&child) {
                        seen.insert(child.clone());

                        let mut child_path = path.clone();
                        child_path.push(*action);
                        queue.push_back((child, child_path));
                    }
                }
            }
        }
    }

    Ok(())
}

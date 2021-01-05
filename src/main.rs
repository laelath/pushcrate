use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::collections::hash_map::Entry;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::rc::Rc;
use std::time::Instant;

#[derive(PartialEq, Eq, Clone, Copy)]
enum Action {
    Up,
    Down,
    Left,
    Right,
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum Tile {
    Empty,
    Wall,
    Box,
}

#[derive(PartialEq, Eq, Clone)]
struct Board {
    width: u8,
    player: (u8, u8),
    goals: Vec<(u8, u8)>,
    tiles: Vec<Tile>,
}

// somewhat hacky, we only hash the parts of the board that can change
impl Hash for Board {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.player.hash(state);
        self.iter_boxes().for_each(|xy| xy.hash(state));
    }
}

#[derive(PartialEq, Eq)]
enum Path {
    None,
    Prev(Rc<Path>, Action),
}

#[derive(Eq)]
struct Node {
    board: Rc<Board>,
    path: Rc<Path>,
    h: u32,
    g: u32,
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        // swapped for min heap
        (other.h + other.g).cmp(&(self.h + self.g))
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.h + self.g == other.h + other.g
    }
}

fn difference(x: u32, y: u32) -> u32 {
    if x < y {
        y - x
    } else {
        x - y
    }
}

fn manhattan_distance(u: (u32, u32), v: (u32, u32)) -> u32 {
    difference(u.0, v.0) + difference(u.1, v.1)
}

impl Board {
    fn get_tile(&self, x: u8, y: u8) -> Tile {
        self.tiles[y as usize * self.width as usize + x as usize]
    }

    fn set_tile(&mut self, x: u8, y: u8, tile: Tile) {
        self.tiles[y as usize * self.width as usize + x as usize] = tile
    }

    fn iter_boxes<'a>(&'a self) -> impl Iterator<Item = (u8, u8)> + 'a {
        self.tiles
            .iter()
            .enumerate()
            .filter(|(_, tile)| tile == &&Tile::Box)
            .map(move |(i, _)| {
                (
                    (i % self.width as usize) as u8,
                    (i / self.width as usize) as u8,
                )
            })
    }

    fn is_empty(&self, x: u8, y: u8) -> bool {
        self.get_tile(x, y) == Tile::Empty
    }

    fn is_wall(&self, x: u8, y: u8) -> bool {
        self.get_tile(x, y) == Tile::Wall
    }

    fn is_box(&self, x: u8, y: u8) -> bool {
        self.get_tile(x, y) == Tile::Box
    }

    fn is_goal(&self, x: u8, y: u8) -> bool {
        for (gx, gy) in &self.goals {
            if x == *gx && y == *gy {
                return true;
            }
        }
        false
    }

    fn is_satisfied(&self) -> bool {
        for (x, y) in &self.goals {
            if self.get_tile(*x, *y) != Tile::Box {
                return false;
            }
        }
        true
    }

    fn is_unsolvable(&self) -> bool {
        for (x, y) in self.iter_boxes() {
            // board is unsolvable if there is a box in a corner not on a goal
            if (self.is_wall(x - 1, y) && self.is_wall(x, y - 1))
                || (self.is_wall(x, y - 1) && self.is_wall(x + 1, y))
                || (self.is_wall(x + 1, y) && self.is_wall(x, y + 1))
                || (self.is_wall(x, y + 1) && self.is_wall(x - 1, y))
            {
                if !self.is_goal(x, y) {
                    return true;
                }
            }

            // board is unsolvable if there are two boxes next to each other next to walls
            if self.is_box(x + 1, y)
                && (self.is_wall(x, y - 1) || self.is_wall(x, y + 1))
                && (self.is_wall(x + 1, y - 1) || self.is_wall(x + 1, y + 1))
            {
                if !(self.is_goal(x, y) && self.is_goal(x + 1, y)) {
                    return true;
                }
            }

            if self.is_box(x, y + 1)
                && (self.is_wall(x - 1, y) || self.is_wall(x + 1, y))
                && (self.is_wall(x - 1, y + 1) || self.is_wall(x + 1, y + 1))
            {
                if !(self.is_goal(x, y) && self.is_goal(x, y + 1)) {
                    return true;
                }
            }
        }

        false
    }

    fn heuristic(&self) -> u32 {
        let boxes: Vec<_> = self.iter_boxes().collect();

        let mut h = 0;

        // requires each box to be moved to a goal
        // therefore it takes at least as many moves as it takes to move each
        // box to the goal closest to it
        h += boxes
            .iter()
            .filter(|(bx, by)| !self.is_goal(*bx, *by))
            .map(|(bx, by)| {
                self.goals
                    .iter()
                    .map(|(gx, gy)| {
                        manhattan_distance((*bx as u32, *by as u32), (*gx as u32, *gy as u32))
                    })
                    .min()
                    .unwrap()
            })
            .sum::<u32>();

        // requires the player to move next to a box to start pushing it
        // therefore we add the the minimum moves to the closest box not on a goal
        h += boxes
            .iter()
            .filter(|(bx, by)| !self.is_goal(*bx, *by))
            .map(|(bx, by)| manhattan_distance((self.player.0 as u32, self.player.1 as u32), (*bx as u32, *by as u32)))
            .min()
            .unwrap_or(1)
            // subtract one since we only need to move next to the box
            - 1;

        h
    }

    fn create_children(&self) -> Vec<(Board, Action)> {
        let px = self.player.0;
        let py = self.player.1;

        let mut children = Vec::new();

        match self.get_tile(px, py - 1) {
            Tile::Empty => {
                let mut child = self.clone();
                child.player.1 = py - 1;
                children.push((child, Action::Up));
            }
            Tile::Wall => (),
            Tile::Box => {
                if self.is_empty(px, py - 2) {
                    let mut child = self.clone();
                    child.set_tile(px, py - 2, Tile::Box);
                    child.set_tile(px, py - 1, Tile::Empty);
                    child.player.1 = py - 1;
                    children.push((child, Action::Up));
                }
            }
        }

        match self.get_tile(px, py + 1) {
            Tile::Empty => {
                let mut child = self.clone();
                child.player.1 = py + 1;
                children.push((child, Action::Down));
            }
            Tile::Wall => (),
            Tile::Box => {
                if self.is_empty(px, py + 2) {
                    let mut child = self.clone();
                    child.set_tile(px, py + 2, Tile::Box);
                    child.set_tile(px, py + 1, Tile::Empty);
                    child.player.1 = py + 1;
                    children.push((child, Action::Down));
                }
            }
        }

        match self.get_tile(px - 1, py) {
            Tile::Empty => {
                let mut child = self.clone();
                child.player.0 = px - 1;
                children.push((child, Action::Left));
            }
            Tile::Wall => (),
            Tile::Box => {
                if self.is_empty(px - 2, py) {
                    let mut child = self.clone();
                    child.set_tile(px - 2, py, Tile::Box);
                    child.set_tile(px - 1, py, Tile::Empty);
                    child.player.0 = px - 1;
                    children.push((child, Action::Left));
                }
            }
        }

        match self.get_tile(px + 1, py) {
            Tile::Empty => {
                let mut child = self.clone();
                child.player.0 = px + 1;
                children.push((child, Action::Right));
            }
            Tile::Wall => (),
            Tile::Box => {
                if self.is_empty(px + 2, py) {
                    let mut child = self.clone();
                    child.set_tile(px + 2, py, Tile::Box);
                    child.set_tile(px + 1, py, Tile::Empty);
                    child.player.0 = px + 1;
                    children.push((child, Action::Right));
                }
            }
        }

        children
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

    if width > u8::MAX.into() {
        return Err("Level width is greater than 255.");
    } else if height > u8::MAX.into() {
        return Err("Level height is greater than 255.");
    }

    let mut players = Vec::new();

    let mut goals = Vec::new();
    let mut tiles = vec![Tile::Empty; width * height];

    let mut num_boxes = 0;

    for (i, line) in lines.into_iter().enumerate() {
        for (j, c) in line.chars().enumerate() {
            match c {
                '#' => tiles[width * i + j] = Tile::Wall,
                'p' | '@' => players.push((j as u8, i as u8)),
                'P' | '+' => {
                    players.push((j as u8, i as u8));
                    goals.push((j as u8, i as u8));
                }
                'b' | '$' => {
                    tiles[width * i + j] = Tile::Box;
                    num_boxes += 1;
                }
                'B' | '*' => {
                    tiles[width * i + j] = Tile::Box;
                    goals.push((j as u8, i as u8));
                    num_boxes += 1;
                }
                '.' => goals.push((j as u8, i as u8)),
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
        width: width as u8,
        player: players[0],
        goals: goals,
        tiles: tiles,
    })
}

fn read_path(end_state: &Rc<Path>) -> Vec<Action> {
    let mut path = vec![];
    let mut state = end_state.as_ref();

    while let Path::Prev(prev, action) = state {
        path.push(*action);
        state = prev.as_ref();
    }

    path.reverse();
    path
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

struct ProgressTracker {
    frequency: u32,
    max_seen_depth: u32,
    max_seen_f: u32,
    counter: u32,
}

impl ProgressTracker {
    fn update(&mut self, depth: u32, h: u32) {
        self.counter += 1;

        self.max_seen_depth = std::cmp::max(self.max_seen_depth, depth);
        self.max_seen_f = std::cmp::max(self.max_seen_f, depth + h);

        if self.counter % self.frequency == 0 {
            self.print_progress();
            std::io::stdout().flush().unwrap();
        }
    }

    fn print_progress(&self) {
        print!("\x1B[0G\x1B[K");
        print!(
            "Searched {} states, to a max depth of {}, solution is at least {} steps.",
            self.counter, self.max_seen_depth, self.max_seen_f
        );
    }

    fn finish(&self) {
        self.print_progress();
        println!();
    }

    fn create(frequency: u32) -> Self {
        let pt = ProgressTracker {
            frequency: frequency,
            max_seen_depth: 0,
            max_seen_f: 0,
            counter: 0,
        };

        pt.print_progress();

        pt
    }
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 2 {
        println!("Usage: {} <sokoban level file>", args[0]);
        std::process::exit(1);
    }

    let level_string = std::fs::read_to_string(&args[1])?;
    let start = Rc::new(parse_level_string(&level_string).unwrap());

    // Use a HashMap so we can use the Entry API - hopefully won't need to in a future version of Rust
    let mut seen: HashMap<Rc<Board>, ()> = HashMap::new();
    let mut heap: BinaryHeap<Node> = BinaryHeap::new();

    {
        seen.insert(start.clone(), ());

        heap.push(Node {
            board: start,
            path: Rc::new(Path::None),
            h: 0, // don't really need heuristic for start node
            g: 0,
        });
    }

    // frequency is visually appealing - not obvious it's skipping numbers
    let mut tracker = ProgressTracker::create(7919);

    let start_time = Instant::now();

    loop {
        match heap.pop() {
            None => {
                tracker.finish();
                println!(
                    "Exhausted search after {} states, level is not solvable.",
                    seen.len()
                );
                break;
            }
            Some(node) => {
                let board = &node.board;

                tracker.update(node.g, node.h);

                if board.is_satisfied() {
                    tracker.finish();
                    println!(
                        "Found solution of length {}: {}",
                        node.g,
                        path_to_string(&read_path(&node.path))
                    );
                    break;
                }

                for (child, action) in board.create_children() {
                    // check if the level is known unsolvable and drop
                    // do it here to avoid the cost of inserting it into the heap
                    if child.is_unsolvable() {
                        continue;
                    }

                    match seen.entry(Rc::new(child)) {
                        Entry::Occupied(_) => (),
                        Entry::Vacant(entry) => {
                            heap.push(Node {
                                board: entry.key().clone(),
                                path: Rc::new(Path::Prev(node.path.clone(), action)),
                                h: entry.key().heuristic(),
                                g: node.g + 1,
                            });

                            entry.insert(());
                        },
                    }
                }
            }
        }
    }

    println!(
        "Search finished after {} seconds.",
        start_time.elapsed().as_secs_f64()
    );

    Ok(())
}

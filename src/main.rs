use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};
use std::hash::{Hash, Hasher};
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
    height: u8,
    player: (u8, u8),
    goals: Vec<(u8, u8)>,
    tiles: Vec<Tile>,
}

struct ConstState {
    width: u8,
    height: u8,
    goals: Vec<(u8, u8)>,
    walls: Vec<bool>,
}

#[derive(Eq, Clone)]
struct SolveState {
    player: (u8, u8),
    boxes: Vec<(u8, u8)>,
    prev: Option<Rc<SolveState>>,
    action: Option<Action>,
}

impl PartialEq for SolveState {
    fn eq(&self, other: &Self) -> bool {
        self.player == other.player && self.boxes == other.boxes
    }
}

impl Hash for SolveState {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.player.hash(state);
        self.boxes.hash(state);
    }
}

#[derive(Eq)]
struct HeapState {
    state: Rc<SolveState>,
    h: u32,
    g: u32,
}

impl Ord for HeapState {
    fn cmp(&self, other: &Self) -> Ordering {
        // swapped for min heap
        (other.h + other.g).cmp(&(self.h + self.g))
    }
}

impl PartialOrd for HeapState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for HeapState {
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

    #[allow(dead_code)]
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
        for (x, y) in self
            .tiles
            .iter()
            .enumerate()
            .filter(|(_, tile)| tile == &&Tile::Box)
            .map(|(i, _)| {
                (
                    (i % self.width as usize) as u8,
                    (i / self.width as usize) as u8,
                )
            })
        {
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
        let boxes: Vec<_> = self
            .tiles
            .iter()
            .enumerate()
            .filter(|(_, tile)| tile == &&Tile::Box)
            .map(|(i, _)| {
                (
                    (i % self.width as usize) as u32,
                    (i / self.width as usize) as u32,
                )
            })
            .collect();

        let mut h = 0;

        // requires each box to be moved to a goal
        // therefore it takes at least as many moves as it takes to move each
        // box to the goal closest to it
        h += boxes
            .iter()
            .filter(|(bx, by)| !self.is_goal(*bx as u8, *by as u8))
            .map(|(bx, by)| {
                self.goals
                    .iter()
                    .map(|(gx, gy)| manhattan_distance((*bx, *by), (*gx as u32, *gy as u32)))
                    .min()
                    .unwrap()
            })
            .sum::<u32>();

        // requires the player to move next to a box to start pushing it
        // therefore we add the the minimum moves to the closest box not on a goal
        h += boxes
            .iter()
            .filter(|(bx, by)| !self.is_goal(*bx as u8, *by as u8))
            .map(|(bx, by)| manhattan_distance((self.player.0 as u32, self.player.1 as u32), (*bx, *by)))
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
                if self.get_tile(px, py - 2) == Tile::Empty {
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
                if self.get_tile(px, py + 2) == Tile::Empty {
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
                if self.get_tile(px - 2, py) == Tile::Empty {
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
                if self.get_tile(px + 2, py) == Tile::Empty {
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

    fn extract_const_state(&self) -> ConstState {
        let mut walls = vec![false; self.tiles.len()];
        for (i, tile) in self.tiles.iter().enumerate() {
            if tile == &Tile::Wall {
                walls[i] = true;
            }
        }

        ConstState {
            width: self.width,
            height: self.height,
            goals: self.goals.clone(),
            walls: walls,
        }
    }

    fn extract_solve_state(&self) -> SolveState {
        SolveState {
            player: self.player,
            boxes: self
                .tiles
                .iter()
                .enumerate()
                .filter(|(_, tile)| tile == &&Tile::Box)
                .map(|(i, _)| {
                    (
                        (i % self.width as usize) as u8,
                        (i / self.width as usize) as u8,
                    )
                })
                .collect::<Vec<_>>(),
            prev: None,
            action: None,
        }
    }

    fn from_const_and_solve(const_state: &ConstState, solve_state: &SolveState) -> Self {
        let mut tiles = vec![Tile::Empty; const_state.walls.len()];
        for (i, wall) in const_state.walls.iter().enumerate() {
            if *wall {
                tiles[i] = Tile::Wall;
            }
        }

        for (bx, by) in &solve_state.boxes {
            tiles[*by as usize * const_state.width as usize + *bx as usize] = Tile::Box;
        }

        Board {
            width: const_state.width,
            height: const_state.height,
            player: solve_state.player,
            goals: const_state.goals.clone(),
            tiles: tiles,
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
        height: height as u8,
        player: players[0],
        goals: goals,
        tiles: tiles,
    })
}

fn read_path(end_state: &SolveState) -> Vec<Action> {
    let mut path = vec![];
    let mut state = end_state;

    while state.prev.is_some() {
        path.push(state.action.unwrap());
        state = &state.prev.as_ref().unwrap();
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

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 2 {
        println!("Usage: {} <sokoban level file>", args[0]);
        std::process::exit(1);
    }

    let level_string = std::fs::read_to_string(&args[1])?;
    let start = parse_level_string(&level_string).unwrap();

    let const_state = start.extract_const_state();

    let mut seen: HashSet<Rc<SolveState>> = HashSet::new();
    let mut heap: BinaryHeap<HeapState> = BinaryHeap::new();

    {
        let start_state = Rc::new(start.extract_solve_state());

        seen.insert(start_state.clone());
        heap.push(HeapState {
            state: start_state,
            h: start.heuristic(),
            g: 0,
        });
    }

    let start_time = Instant::now();

    loop {
        match heap.pop() {
            None => {
                println!(
                    "Exhausted search after {} states, level is not solvable.",
                    seen.len()
                );
                break;
            }
            Some(heap_state) => {
                let board = Board::from_const_and_solve(&const_state, &heap_state.state);

                if board.is_satisfied() {
                    println!(
                        "Found solution in {} move after searching {} states: {}",
                        heap_state.g,
                        seen.len() - heap.len(),
                        path_to_string(&read_path(&heap_state.state))
                    );
                    break;
                }

                for (child, action) in board.create_children() {
                    // check if the level is known unsolvable and drop
                    // do it here to avoid the cost of inserting it into the heap
                    if child.is_unsolvable() {
                        continue;
                    }

                    let mut child_state = child.extract_solve_state();

                    if !seen.contains(&child_state) {
                        child_state.action = Some(action);
                        child_state.prev = Some(heap_state.state.clone());

                        let child_state = Rc::new(child_state);

                        seen.insert(child_state.clone());
                        heap.push(HeapState {
                            state: child_state,
                            h: child.heuristic(),
                            g: heap_state.g + 1,
                        });
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

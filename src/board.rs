use std::collections::VecDeque;
use std::hash::{Hash, Hasher};

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Action {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Clone)]
pub struct Board {
    goals: Box<[((u32, u32), Box<[u32]>)]>,
    goal_tiles: Box<[bool]>,
    walls: Box<[bool]>,
    dead_tiles: Box<[bool]>,
    width: usize,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct BoardState {
    player: (u32, u32),
    crates: Box<[bool]>,
}

impl Hash for BoardState {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.player.hash(state);
        self.crates
            .iter()
            .enumerate()
            .filter(|(_, b)| **b)
            .for_each(|(i, _)| i.hash(state));
    }
}

impl Board {
    #[inline]
    fn is_goal(&self, x: u32, y: u32) -> bool {
        self.goal_tiles[y as usize * self.width + x as usize]
    }

    #[inline]
    fn is_empty(&self, state: &BoardState, x: u32, y: u32) -> bool {
        !self.is_wall(x, y) && !self.is_crate(state, x, y)
    }

    #[inline]
    fn is_wall(&self, x: u32, y: u32) -> bool {
        self.walls[y as usize * self.width + x as usize]
    }

    #[inline]
    fn is_crate(&self, state: &BoardState, x: u32, y: u32) -> bool {
        state.crates[y as usize * self.width + x as usize]
    }

    #[inline]
    fn set_crate(&self, state: &mut BoardState, x: u32, y: u32, crate_bit: bool) {
        state.crates[y as usize * self.width + x as usize] = crate_bit;
    }

    #[inline]
    fn is_dead_tile(&self, x: u32, y: u32) -> bool {
        self.dead_tiles[y as usize * self.width + x as usize]
    }

    pub fn is_goal_state(&self, state: &BoardState) -> bool {
        for ((x, y), _) in self.goals.iter() {
            if !self.is_crate(state, *x, *y) {
                return false;
            }
        }

        true
    }

    fn iter_crates<'a>(&'a self, state: &'a BoardState) -> impl Iterator<Item = (u32, u32)> + 'a {
        state
            .crates
            .iter()
            .enumerate()
            .filter(|(_, tile)| **tile)
            .map(move |(i, _)| ((i % self.width) as u32, (i / self.width) as u32))
    }

    fn is_unsolvable(&self, state: &BoardState) -> bool {
        for (x, y) in self.iter_crates(state) {
            // we now check this as we move the crates
            // board is unsolvable if there is a crate on a dead tile
            // if self.is_dead_tile(x, y) {
            //     return true;
            // }

            // board is unsolvable if there are two crates next to each other next to walls
            if self.is_crate(state, x + 1, y)
                && (self.is_wall(x, y - 1) || self.is_wall(x, y + 1))
                && (self.is_wall(x + 1, y - 1) || self.is_wall(x + 1, y + 1))
            {
                if !(self.is_goal(x, y) && self.is_goal(x + 1, y)) {
                    return true;
                }
            }

            if self.is_crate(state, x, y + 1)
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

    pub fn heuristic(&self, state: &BoardState) -> u32 {
        let mut h = 0;

        // TODO: Use Hungarian Algorithm to find optimal matching

        let unsat_goal_dists: Vec<_> = self
            .goals
            .iter()
            .filter(|((x, y), _)| !self.is_crate(state, *x, *y))
            .map(|(_, dists)| dists)
            .collect();

        // requires each crate to be moved to a goal
        // therefore it takes at least as many moves as it takes to move each
        // crate to the goal closest to it
        h += self
            .iter_crates(state)
            .filter(|(x, y)| !self.is_goal(*x, *y))
            .map(|(bx, by)| {
                unsat_goal_dists
                    .iter()
                    .map(|dists| dists[by as usize * self.width + bx as usize])
                    .min()
                    .unwrap()
            })
            .sum::<u32>();

        h
    }

    pub fn create_children(&self, state: &BoardState) -> Vec<(BoardState, Box<[Action]>)> {
        let mut children = Vec::new();

        let mut paths = vec![None; self.walls.len()];
        let mut seen = vec![false; self.walls.len()];
        let mut queue = VecDeque::new();

        let read_path = |paths: &Vec<_>, index, action| -> Box<[Action]> {
            let mut path = vec![action];
            let mut index = index;

            while let Some(action) = paths[index] {
                path.push(action);

                match action {
                    Action::Up => index += self.width,
                    Action::Down => index -= self.width,
                    Action::Left => index += 1,
                    Action::Right => index -= 1,
                }
            }

            path.into_boxed_slice()
        };

        queue.push_back((state.player.0, state.player.1, None));

        while let Some((x, y, action)) = queue.pop_front() {
            let index = y as usize * self.width + x as usize;

            if !seen[index] && self.is_empty(state, x, y) {
                seen[index] = true;
                paths[index] = action;

                if self.is_crate(state, x, y - 1)
                    && self.is_empty(state, x, y - 2)
                    && !self.is_dead_tile(x, y - 2)
                {
                    let mut child = state.clone();
                    self.set_crate(&mut child, x, y - 2, true);
                    self.set_crate(&mut child, x, y - 1, false);
                    child.player = (x, y - 1);
                    if !self.is_unsolvable(&child) {
                        children.push((child, read_path(&paths, index, Action::Up)));
                    }
                }

                if self.is_crate(state, x, y + 1)
                    && self.is_empty(state, x, y + 2)
                    && !self.is_dead_tile(x, y + 2)
                {
                    let mut child = state.clone();
                    self.set_crate(&mut child, x, y + 2, true);
                    self.set_crate(&mut child, x, y + 1, false);
                    child.player = (x, y + 1);
                    if !self.is_unsolvable(&child) {
                        children.push((child, read_path(&paths, index, Action::Down)));
                    }
                }

                if self.is_crate(state, x - 1, y)
                    && self.is_empty(state, x - 2, y)
                    && !self.is_dead_tile(x - 2, y)
                {
                    let mut child = state.clone();
                    self.set_crate(&mut child, x - 2, y, true);
                    self.set_crate(&mut child, x - 1, y, false);
                    child.player = (x - 1, y);
                    if !self.is_unsolvable(&child) {
                        children.push((child, read_path(&paths, index, Action::Left)));
                    }
                }

                if self.is_crate(state, x + 1, y)
                    && self.is_empty(state, x + 2, y)
                    && !self.is_dead_tile(x + 2, y)
                {
                    let mut child = state.clone();
                    self.set_crate(&mut child, x + 2, y, true);
                    self.set_crate(&mut child, x + 1, y, false);
                    child.player = (x + 1, y);
                    if !self.is_unsolvable(&child) {
                        children.push((child, read_path(&paths, index, Action::Right)));
                    }
                }

                queue.push_back((x, y - 1, Some(Action::Up)));
                queue.push_back((x, y + 1, Some(Action::Down)));
                queue.push_back((x - 1, y, Some(Action::Left)));
                queue.push_back((x + 1, y, Some(Action::Right)));
            }
        }

        children
    }

    pub fn parse_level_string(level: &String) -> Result<(Self, BoardState), &'static str> {
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
        let mut walls = vec![false; width * height];
        let mut crates = vec![false; width * height];

        let mut num_crates = 0;

        for (i, line) in lines.into_iter().enumerate() {
            for (j, c) in line.chars().enumerate() {
                match c {
                    '#' => walls[width * i + j] = true,
                    'p' | '@' => players.push((j as u32, i as u32)),
                    'P' | '+' => {
                        players.push((j as u32, i as u32));
                        goals.push((j as u32, i as u32));
                    }
                    'b' | '$' => {
                        crates[width * i + j] = true;
                        num_crates += 1;
                    }
                    'B' | '*' => {
                        goals.push((j as u32, i as u32));
                        crates[width * i + j] = true;
                        num_crates += 1;
                    }
                    '.' => goals.push((j as u32, i as u32)),
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

        if num_crates != goals.len() {
            return Err("Number of crates and number of goals are not the same");
        }

        // verify the level is enclosed in walls
        let mut interior = vec![false; walls.len()];
        let mut queue = VecDeque::new();

        queue.push_back((players[0].0 as usize, players[0].1 as usize));

        while let Some((x, y)) = queue.pop_front() {
            if !interior[y * width + x] && !walls[y * width + x] {
                if x == 0 || x == width - 1 || y == 0 || y == height - 1 {
                    return Err("Player is not enclosed in walls");
                } else {
                    interior[y * width + x] = true;
                    queue.push_back((x + 1, y));
                    queue.push_back((x - 1, y));
                    queue.push_back((x, y + 1));
                    queue.push_back((x, y - 1));
                }
            }
        }

        let mut goal_tiles = vec![false; walls.len()];
        for (x, y) in goals.iter() {
            goal_tiles[*y as usize * width + *x as usize] = true;
        }

        let dead_tiles = Board::find_dead_tiles(width, &walls, &interior, &goal_tiles);

        // print out board info for debug purposes
        // TODO: separate this out into a function
        for i in 0..walls.len() {
            let is_player = i == players[0].1 as usize * width + players[0].0 as usize;
            match (walls[i], dead_tiles[i], goal_tiles[i], crates[i], is_player) {
                (true, _, _, _, _) => print!("#"),
                (false, true, _, false, false) => print!("-"),
                (false, true, _, false, true) => print!("%"),
                (false, true, _, true, _) => print!("!"),
                (false, false, true, false, false) => print!("."),
                (false, false, true, false, true) => print!("+"),
                (false, false, false, true, _) => print!("$"),
                (false, false, true, true, _) => print!("*"),
                (false, false, false, false, true) => print!("@"),
                (false, false, false, false, false) => print!(" "),
            }

            if i % width == width - 1 {
                println!();
            }
        }

        let goal_distances = Board::calculate_goal_distances(&goals, width, &walls, &dead_tiles);

        Ok((
            Board {
                goals: goals.into_iter().zip(goal_distances).collect(),
                goal_tiles: goal_tiles.into_boxed_slice(),
                walls: walls.into_boxed_slice(),
                dead_tiles: dead_tiles.into_boxed_slice(),
                width: width,
            },
            BoardState {
                player: players[0],
                crates: crates.into_boxed_slice(),
            },
        ))
    }

    fn calculate_goal_distances<'a>(
        goals: &'a [(u32, u32)],
        width: usize,
        walls: &'a [bool],
        dead_tiles: &'a [bool],
    ) -> Vec<Box<[u32]>> {
        goals
            .iter()
            .map(|goal| {
                Board::calculate_goal_distance(*goal, width, walls, dead_tiles).into_boxed_slice()
            })
            .collect()
    }

    fn calculate_goal_distance(
        goal: (u32, u32),
        width: usize,
        walls: &[bool],
        dead_tiles: &[bool],
    ) -> Vec<u32> {
        let mut dists = vec![0; walls.len()];

        let mut seen = vec![false; walls.len()];
        let mut queue = VecDeque::new();

        queue.push_back((goal.0 as usize, goal.1 as usize, 0));

        while let Some((x, y, d)) = queue.pop_front() {
            if !seen[y * width + x] && !walls[y * width + x] && !dead_tiles[y * width + x] {
                seen[y * width + x] = true;
                dists[y * width + x] = d;
                queue.push_back((x + 1, y, d + 1));
                queue.push_back((x - 1, y, d + 1));
                queue.push_back((x, y + 1, d + 1));
                queue.push_back((x, y - 1, d + 1));
            }
        }

        dists
    }

    fn find_dead_tiles(
        width: usize,
        walls: &[bool],
        interior: &[bool],
        goal_tiles: &[bool],
    ) -> Vec<bool> {
        let mut corners = vec![false; walls.len()];
        let mut next_to_walls = vec![false; walls.len()];

        // find corners and open tiles next to walls
        for (i, inside) in interior.iter().enumerate() {
            if *inside {
                if walls[i - width] && walls[i + 1]
                    || walls[i + 1] && walls[i + width]
                    || walls[i + width] && walls[i - 1]
                    || walls[i - 1] && walls[i - width]
                {
                    corners[i] = true;
                }

                if walls[i - 1] || walls[i + 1] || walls[i - width] || walls[i + width] {
                    next_to_walls[i] = true;
                }
            }
        }

        let corners = corners;
        let next_to_walls = next_to_walls;

        let is_dead_across = |start: usize| -> bool {
            for i in start.. {
                if !next_to_walls[i] || goal_tiles[i] {
                    return false;
                } else if corners[i] && walls[i + 1] {
                    return true;
                }
            }
            unreachable!()
        };

        let is_dead_down = |start: usize| -> bool {
            for i in (start..).step_by(width) {
                if !next_to_walls[i] || goal_tiles[i] {
                    return false;
                } else if corners[i] && walls[i + width] {
                    return true;
                }
            }
            unreachable!()
        };

        let mut dead_tiles = vec![false; walls.len()];

        // use corners and next to walls to find dead tiles
        for (i, corner) in corners.iter().enumerate() {
            if *corner && !goal_tiles[i] {
                // all corners not on goals are dead tiles
                dead_tiles[i] = true;

                // search across
                if is_dead_across(i) {
                    let mut j = i;
                    while !walls[j] {
                        dead_tiles[j] = true;
                        j += 1;
                    }
                }

                // search down
                if is_dead_down(i) {
                    let mut j = i;
                    while !walls[j] {
                        dead_tiles[j] = true;
                        j += width;
                    }
                }
            }
        }

        dead_tiles
    }
}

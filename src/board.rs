use std::collections::VecDeque;

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Action {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Clone)]
pub struct Board {
    goals: Vec<((u8, u8), Vec<u32>)>,
    width: u8,
    walls: Vec<bool>,
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct BoardState {
    player: (u8, u8),
    boxes: Vec<bool>,
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
    fn is_goal(&self, x: u8, y: u8) -> bool {
        for ((gx, gy), _) in &self.goals {
            if x == *gx && y == *gy {
                return true;
            }
        }

        false
    }

    fn is_empty(&self, state: &BoardState, x: u8, y: u8) -> bool {
        !self.is_wall(x, y) && !self.is_box(state, x, y)
    }

    fn is_wall(&self, x: u8, y: u8) -> bool {
        self.walls[y as usize * self.width as usize + x as usize]
    }

    fn is_box(&self, state: &BoardState, x: u8, y: u8) -> bool {
        state.boxes[y as usize * self.width as usize + x as usize]
    }

    fn set_box(&self, state: &mut BoardState, x: u8, y: u8, box_bit: bool) {
        state.boxes[y as usize * self.width as usize + x as usize] = box_bit;
    }

    pub fn is_goal_state(&self, state: &BoardState) -> bool {
        for ((x, y), _) in &self.goals {
            if !self.is_box(state, *x, *y) {
                return false;
            }
        }

        true
    }

    fn iter_boxes<'a>(&'a self, state: &'a BoardState) -> impl Iterator<Item = (u8, u8)> + 'a {
        state
            .boxes
            .iter()
            .enumerate()
            .filter(|(_, tile)| **tile)
            .map(move |(i, _)| {
                (
                    (i % self.width as usize) as u8,
                    (i / self.width as usize) as u8,
                )
            })
    }

    fn is_unsolvable(&self, state: &BoardState) -> bool {
        for (x, y) in self.iter_boxes(state) {
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
            if self.is_box(state, x + 1, y)
                && (self.is_wall(x, y - 1) || self.is_wall(x, y + 1))
                && (self.is_wall(x + 1, y - 1) || self.is_wall(x + 1, y + 1))
            {
                if !(self.is_goal(x, y) && self.is_goal(x + 1, y)) {
                    return true;
                }
            }

            if self.is_box(state, x, y + 1)
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

        let unsat_goal_dists: Vec<_> = self
            .goals
            .iter()
            .filter(|((x, y), _)| !self.is_box(state, *x, *y))
            .map(|(_, dists)| dists)
            .collect();

        // requires each box to be moved to a goal
        // therefore it takes at least as many moves as it takes to move each
        // box to the goal closest to it
        h += self
            .iter_boxes(state)
            .filter(|(x, y)| !self.is_goal(*x, *y))
            .map(|(bx, by)| {
                unsat_goal_dists
                    .iter()
                    .map(|dists| dists[by as usize * self.width as usize + bx as usize])
                    .min()
                    .unwrap()
            })
            .sum::<u32>();

        // requires the player to move next to a box to start pushing it
        // therefore we add the the minimum moves to the closest box not on a goal
        h += self
            .iter_boxes(state)
            .filter(|(bx, by)| !self.is_goal(*bx, *by))
            .map(|(bx, by)| manhattan_distance((state.player.0 as u32, state.player.1 as u32), (bx as u32, by as u32)))
            .min()
            .unwrap_or(1)
            // subtract one since we only need to move next to the box
            - 1;

        h
    }

    pub fn create_children(&self, state: &BoardState) -> Vec<(BoardState, Action)> {
        let px = state.player.0;
        let py = state.player.1;

        let mut children: Vec<(BoardState, Action)> = Vec::new();

        match (self.is_wall(px, py - 1), self.is_box(state, px, py - 1)) {
            (false, false) => {
                let mut child = state.clone();
                child.player.1 = py - 1;
                children.push((child, Action::Up));
            }
            (false, true) => {
                if self.is_empty(state, px, py - 2) {
                    let mut child = state.clone();
                    self.set_box(&mut child, px, py - 2, true);
                    self.set_box(&mut child, px, py - 1, false);
                    child.player.1 = py - 1;
                    children.push((child, Action::Up));
                }
            }
            (true, _) => (),
        }

        match (self.is_wall(px, py + 1), self.is_box(state, px, py + 1)) {
            (false, false) => {
                let mut child = state.clone();
                child.player.1 = py + 1;
                children.push((child, Action::Down));
            }
            (false, true) => {
                if self.is_empty(state, px, py + 2) {
                    let mut child = state.clone();
                    self.set_box(&mut child, px, py + 2, true);
                    self.set_box(&mut child, px, py + 1, false);
                    child.player.1 = py + 1;
                    children.push((child, Action::Down));
                }
            }
            (true, _) => (),
        }

        match (self.is_wall(px - 1, py), self.is_box(state, px - 1, py)) {
            (false, false) => {
                let mut child = state.clone();
                child.player.0 = px - 1;
                children.push((child, Action::Left));
            }
            (false, true) => {
                if self.is_empty(state, px - 2, py) {
                    let mut child = state.clone();
                    self.set_box(&mut child, px - 2, py, true);
                    self.set_box(&mut child, px - 1, py, false);
                    child.player.0 = px - 1;
                    children.push((child, Action::Left));
                }
            }
            (true, _) => (),
        }

        match (self.is_wall(px + 1, py), self.is_box(state, px + 1, py)) {
            (false, false) => {
                let mut child = state.clone();
                child.player.0 = px + 1;
                children.push((child, Action::Right));
            }
            (false, true) => {
                if self.is_empty(state, px + 2, py) {
                    let mut child = state.clone();
                    self.set_box(&mut child, px + 2, py, true);
                    self.set_box(&mut child, px + 1, py, false);
                    child.player.0 = px + 1;
                    children.push((child, Action::Right));
                }
            }
            (true, _) => (),
        }

        // Filter out children that are known unsolvable
        children
            .into_iter()
            .filter(|(state, _)| !self.is_unsolvable(state))
            .collect()
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

        if width > u8::MAX.into() {
            return Err("Level width is greater than 255.");
        } else if height > u8::MAX.into() {
            return Err("Level height is greater than 255.");
        }

        let mut players = Vec::new();

        let mut goals = Vec::new();
        let mut walls = vec![false; width * height];
        let mut boxes = vec![false; width * height];

        let mut num_boxes = 0;

        for (i, line) in lines.into_iter().enumerate() {
            for (j, c) in line.chars().enumerate() {
                match c {
                    '#' => walls[width * i + j] = true,
                    'p' | '@' => players.push((j as u8, i as u8)),
                    'P' | '+' => {
                        players.push((j as u8, i as u8));
                        goals.push((j as u8, i as u8));
                    }
                    'b' | '$' => {
                        boxes[width * i + j] = true;
                        num_boxes += 1;
                    }
                    'B' | '*' => {
                        goals.push((j as u8, i as u8));
                        boxes[width * i + j] = true;
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

        let goal_distances = Board::calculate_goal_distances(&goals, width, &walls);

        Ok((
            Board {
                goals: goals.into_iter().zip(goal_distances).collect(),
                width: width as u8,
                walls: walls,
            },
            BoardState {
                player: players[0],
                boxes: boxes,
            },
        ))
    }

    fn calculate_goal_distances(
        goals: &Vec<(u8, u8)>,
        width: usize,
        walls: &Vec<bool>,
    ) -> Vec<Vec<u32>> {
        goals
            .iter()
            .map(|goal| Board::calculate_goal_distance(*goal, width, walls))
            .collect()
    }

    fn calculate_goal_distance(goal: (u8, u8), width: usize, walls: &Vec<bool>) -> Vec<u32> {
        let mut dists = vec![0; walls.len()];

        let mut seen = vec![false; walls.len()];
        let mut queue = VecDeque::new();

        queue.push_back((goal.0 as usize, goal.1 as usize, 0));

        while let Some((x, y, d)) = queue.pop_front() {
            if !seen[y * width + x] && !walls[y * width + x] {
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
}

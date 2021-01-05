use crate::board::{Action, Board, BoardState};

use std::collections::{BinaryHeap, HashMap};
use std::collections::hash_map::Entry;
use std::rc::Rc;
use std::cmp::Ordering;
use std::io::Write;

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
        print!(
            "\rSearched {} states, to a max depth of {}, solution is at least {} steps.\x1B[0K",
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

#[derive(PartialEq, Eq)]
enum Path {
    None,
    Prev(Rc<Path>, Action),
}

#[derive(Eq)]
struct Node {
    state: Rc<BoardState>,
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

pub fn find_path(board: &Board, start: &BoardState) -> Option<Vec<Action>> {
    // Use a HashMap so we can use the Entry API - hopefully won't need to in a future version of Rust
    let mut seen: HashMap<Rc<BoardState>, ()> = HashMap::new();
    let mut heap: BinaryHeap<Node> = BinaryHeap::new();

    {
        let start = Rc::new(start.clone());

        seen.insert(start.clone(), ());

        heap.push(Node {
            state: start,
            path: Rc::new(Path::None),
            h: 0, // don't really need heuristic for start node
            g: 0,
        });
    }

    // frequency is visually appealing - not obvious it's skipping numbers
    let mut tracker = ProgressTracker::create(7919);

    loop {
        match heap.pop() {
            None => {
                tracker.finish();
                return None;
            }
            Some(node) => {
                let state = &node.state;

                tracker.update(node.g, node.h);

                if board.is_goal_state(&state) {
                    tracker.finish();
                    return Some(read_path(&node.path));
                }

                for (child, action) in board.create_children(&state) {
                    match seen.entry(Rc::new(child)) {
                        Entry::Occupied(_) => (),
                        Entry::Vacant(entry) => {
                            heap.push(Node {
                                state: entry.key().clone(),
                                path: Rc::new(Path::Prev(node.path.clone(), action)),
                                h: board.heuristic(&entry.key()),
                                g: node.g + 1,
                            });

                            entry.insert(());
                        },
                    }
                }
            }
        }
    }
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


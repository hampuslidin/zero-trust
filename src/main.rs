use std::fmt::{self, Display, Formatter};

use rand::prelude::*;
use sha2::{Digest, Sha256, digest::Output};

macro_rules! sudoku {
    (@impl [$($cells:tt)*] [$($rows:tt)*] [$($given:tt)*] ($x:expr, $y:expr) _ $($rest:tt)+ ) => {
        sudoku!(@impl [$($cells)* 0,] [$($rows)*] [$($given)*] ($x + 1, $y) $($rest)+ )
    };

    (@impl [$($cells:tt)*] [$($rows:tt)*] [$($given:tt)*] ($x:expr, $y:expr) $number:literal $($rest:tt)+) => {
        sudoku!(@impl [$($cells)* $number,] [$($rows)*] [$($given)* ($x, $y),] ($x + 1, $y) $($rest)+)
    };

    (@impl [$($cells:tt)*] [$($rows:tt)*] [$($given:tt)*] ($x:expr, $y:expr) ; $($rest:tt)+) => {
        sudoku!(@impl [] [$($rows)* [$($cells)*],] [$($given)*] (0, $y + 1) $($rest)+)
    };

    (@impl [$($cells:tt)+] [$($rows:tt)+] [$($given:tt)*] ($x:expr, $y:expr) ;) => {
        Sudoku {
            grid: [
                $($rows)+
                [$($cells)+],
            ],
            given: Box::new([$($given)*]),
        }
    };

    (@impl $($unknown:tt)*) => {
        compile_error!(concat!("Unknown tokens: ", stringify!($($unknown)*)))
    };

    ($($input:tt)+) => {
        sudoku!(@impl [] [] [] (0, 0) $($input)+)
    };
}

fn main() {
    let puzzle = sudoku! {
        4 _ _ _ 9 6 2 _ 8;
        3 _ 8 1 _ _ _ 9 _;
        9 6 1 _ _ _ 7 _ _;
        _ _ 3 4 _ 5 9 6 _;
        6 _ _ 9 2 8 _ 7 4;
        _ _ 4 7 _ _ 1 _ _;
        _ _ 9 _ _ 2 _ _ 1;
        _ _ _ 8 3 1 6 4 _;
        _ _ _ _ 4 _ _ 2 7;
    };

    println!("{puzzle}");

    let graph = Graph::from(&puzzle);
    let (encrypted_graph, keys) = graph.clone().encrypted();

    let mut rng = rand::rng();
    for _ in 0..1_000_000 {
        let random_edge = encrypted_graph
            .edges
            .choose(&mut rng)
            .expect("slice is non-empty");

        let (first_value, second_value) = (graph.nodes[random_edge.0], graph.nodes[random_edge.1]);
        let (first_key, second_key) = (keys[random_edge.0], keys[random_edge.1]);

        assert_eq!(
            encrypted_graph.nodes[random_edge.0],
            hash(first_value, first_key),
        );
        assert_eq!(
            encrypted_graph.nodes[random_edge.1],
            hash(second_value, second_key),
        );
    }
}

fn hash(value: u8, key: u64) -> Output<Sha256> {
    let mut hasher = Sha256::new();
    hasher.update((value as u64 ^ key).to_le_bytes());
    hasher.finalize()
}

struct Sudoku {
    grid: [[u8; 9]; 9],
    given: Box<[(usize, usize)]>,
}

impl Display for Sudoku {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        writeln!(f, "╔═══╤═══╤═══╦═══╤═══╤═══╦═══╤═══╤═══╗")?;
        for (y, row) in self.grid.into_iter().enumerate() {
            if y == 3 || y == 6 {
                writeln!(f, "╠═══╪═══╪═══╬═══╪═══╪═══╬═══╪═══╪═══╣")?;
            } else if y > 0 {
                writeln!(f, "╟───┼───┼───╫───┼───┼───╫───┼───┼───╢")?;
            }

            write!(f, "║")?;
            for (x, cell) in row.into_iter().enumerate() {
                if x == 3 || x == 6 {
                    write!(f, "║")?;
                } else if x > 0 {
                    write!(f, "│")?
                }

                match cell {
                    0 => write!(f, "   ")?,
                    value => {
                        if self.given.contains(&(x, y)) {
                            write!(f, "\x1b[1;7m {value} \x1b[0m")?
                        } else {
                            write!(f, " {value} ")?
                        }
                    }
                }
            }
            writeln!(f, "║")?
        }
        write!(f, "╚═══╧═══╧═══╩═══╧═══╧═══╩═══╧═══╧═══╝")
    }
}

#[derive(Clone)]
struct Graph<T> {
    nodes: [T; 90],
    edges: Box<[(usize, usize)]>,
}

impl Graph<u8> {
    fn encrypted(self) -> (Graph<Output<Sha256>>, [u64; 90]) {
        let mut rng = rand::rng();
        let keys = rng.random::<[u64; 90]>();
        let mut encrypted_nodes = [Output::<Sha256>::default(); 90];
        for (node, (encrypted_node, key)) in self
            .nodes
            .into_iter()
            .zip(encrypted_nodes.iter_mut().zip(keys.iter().copied()))
        {
            *encrypted_node = hash(node, key);
        }

        (
            Graph {
                nodes: encrypted_nodes,
                edges: self.edges,
            },
            keys,
        )
    }
}

impl From<&Sudoku> for Graph<u8> {
    fn from(sudoku: &Sudoku) -> Self {
        // One node for each cell, as well as 9 constraint nodes for the given cells.
        let mut nodes = [0; 90];

        // Each row has 9 * 8 / 2 edges and there are 9 rows, making a total of 324 edges.
        // By symmetry, each column has the same number of edges as the rows.
        // Each 3-by-3 grid has 18 non-coaxial edges and there are 9 of them, making a total of 162.
        // Combined, the rows, columns, and 3-by-3 grids form 2 * 324 + 162 = 810 edges.
        // Each given number has 8 edges, one for each constraint node that is not equal to the
        // given number.
        let expected_edges = 810 + 8 * sudoku.given.len();
        let mut edges = Vec::with_capacity(expected_edges);

        for (y, row) in sudoku.grid.into_iter().enumerate() {
            for (x, cell) in row.into_iter().enumerate() {
                nodes[9 * y + x] = cell;

                for i in x + 1..9 {
                    edges.push((9 * y + x, 9 * y + i))
                }

                for j in y + 1..9 {
                    edges.push((9 * y + x, 9 * j + x))
                }

                for (i, j) in (y + 1..(y + 3) / 3 * 3).flat_map(|j| {
                    (x / 3 * 3..)
                        .take(3)
                        .filter(|i| *i != x)
                        .map(move |i| (i, j))
                }) {
                    edges.push((9 * y + x, 9 * j + i))
                }
            }
        }

        for (i, j) in sudoku.given.iter().copied() {
            let value = sudoku.grid[j][i];
            for v in (1..=9).filter(|v| *v != value) {
                edges.push((9 * j + i, 80 + v as usize));
            }
        }

        debug_assert_eq!(edges.len(), expected_edges);

        Self {
            nodes,
            edges: edges.into(),
        }
    }
}

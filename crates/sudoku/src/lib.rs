use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    mem::{self, MaybeUninit},
    ops::Index,
};

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
            given: [$($given)*],
        }
    };

    (@impl $($unknown:tt)*) => {
        compile_error!(concat!("Unknown tokens: ", stringify!($($unknown)*)))
    };

    ($($input:tt)+) => {
        sudoku!(@impl [] [] [] (0, 0) $($input)+)
    };
}

pub const PUZZLE: Sudoku<38> = sudoku! {
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

pub fn hash(value: u8, key: u64) -> Output<Sha256> {
    let mut hasher = Sha256::new();
    hasher.update((value as u64 ^ key).to_le_bytes());
    hasher.finalize()
}

pub struct Sudoku<const GIVEN: usize>  {
    pub grid: [[u8; 9]; 9],
    pub given: [(usize, usize); GIVEN],
}

impl<const GIVEN: usize> Display for Sudoku<GIVEN> {
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

// One node for each cell, as well as 9 constraint nodes for the given cells.
const NUM_GRAPH_NODES: usize = 90;

#[derive(Clone)]
pub struct Graph<T> {
    nodes: [T; NUM_GRAPH_NODES],
    edges: Box<[Edge]>,
}

impl<T> Graph<T> {
    pub fn random_edge(&self) -> Edge {
        let mut rng = rand::rng();
        *self
            .edges
            .choose(&mut rng)
            .expect("slice is non-empty")
    }

    pub fn get(&self, edge: Edge) -> (&T, &T) {
        (&self[edge.0], &self[edge.1])
    }
}

impl Graph<u8> {
    pub fn encrypted(self) -> (Graph<Output<Sha256>>, Keys) {
        let mut rng = rand::rng();
        let keys = rng.random::<[u64; NUM_GRAPH_NODES]>();
        let mut encrypted_nodes = [Output::<Sha256>::default(); NUM_GRAPH_NODES];
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
            Keys(keys),
        )
    }
}

impl Graph<Output<Sha256>> {
    pub fn to_bytes(&self) -> Box<[u8]> {
        let mut bytes = Vec::with_capacity(mem::size_of::<Self>());

        for node in &self.nodes {
            bytes.extend(node);
        }

        for edge in &self.edges {
            bytes.extend(edge.0.to_le_bytes());
            bytes.extend(edge.1.to_le_bytes());
        }

        bytes.into()
    }

    pub fn from_bytes(mut bytes: &[u8]) -> Result<Self, GraphError> {
        let node_size = mem::size_of::<Output<Sha256>>();

        if bytes.len() < node_size * NUM_GRAPH_NODES {
            return Err(GraphError::InvalidBytes);
        }

        let mut nodes: [MaybeUninit<Output<Sha256>>; NUM_GRAPH_NODES] = [const { MaybeUninit::uninit() }; NUM_GRAPH_NODES];
        for node in &mut nodes {
            node.write(Output::<Sha256>::clone_from_slice(&bytes[..node_size]));
            bytes = &bytes[node_size..];
        }
        
        // SAFETY: All elements are initialized, since we iterated over each element and unconditionally wrote a value to it.
        let nodes = unsafe { mem::transmute(nodes) };

        let edge_size = mem::size_of::<Edge>();

        if bytes.len() % edge_size != 0 {
            return Err(GraphError::InvalidBytes);
        }

        let num_edges = bytes.len() / edge_size;
        let mut edges = Vec::with_capacity(num_edges);
        for _ in 0..num_edges {
            let edge_bytes = [
                bytes[0], bytes[1], bytes[2],  bytes[3],  bytes[4],  bytes[5],  bytes[6],  bytes[7],
                bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
            ];
            edges.push(Edge::from_bytes(edge_bytes));
            bytes = &bytes[..16];
        }

        debug_assert!(bytes.is_empty());

        Ok(Self {
            nodes,
            edges: edges.into(),
        })
    }
}

#[derive(Debug)]
pub enum GraphError {
    InvalidBytes,
}

impl Error for GraphError {}

impl Display for GraphError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::InvalidBytes => write!(f, "invalid bytes"),
        }
    }
}

impl<T> Index<usize> for Graph<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.nodes[index]
    }
}

impl<const GIVEN: usize> From<&Sudoku<GIVEN>> for Graph<u8> {
    fn from(sudoku: &Sudoku<GIVEN>) -> Self {
        let mut nodes = [0; NUM_GRAPH_NODES];
        
        // Each row has 9 * 8 / 2 edges and there are 9 rows, making a total of 324 edges.
        // By symmetry, each column has the same number of edges as the rows.
        // Each 3-by-3 grid has 18 non-coaxial edges and there are 9 of them, making a total of 162.
        // Combined, the rows, columns, and 3-by-3 grids form 2 * 324 + 162 = 810 edges.
        // Each given number has 8 edges, one for each constraint node that is not equal to the
        // given number.
        let expected_edges = 810 + 8 * GIVEN;
        let mut edges = Vec::with_capacity(expected_edges);

        for (y, row) in sudoku.grid.into_iter().enumerate() {
            for (x, cell) in row.into_iter().enumerate() {
                nodes[9 * y + x] = cell;

                for i in x + 1..9 {
                    edges.push(Edge(9 * y + x, 9 * y + i));
                }

                for j in y + 1..9 {
                    edges.push(Edge(9 * y + x, 9 * j + x));
                }

                for (i, j) in (y + 1..(y + 3) / 3 * 3).flat_map(|j| {
                    (x / 3 * 3..)
                        .take(3)
                        .filter(|i| *i != x)
                        .map(move |i| (i, j))
                }) {
                    edges.push(Edge(9 * y + x, 9 * j + i));
                }
            }
        }

        for (i, j) in sudoku.given.iter().copied() {
            let value = sudoku.grid[j][i];
            for v in (1..=9).filter(|v| *v != value) {
                edges.push(Edge(9 * j + i, 80 + v as usize));
            }
        }

        debug_assert_eq!(edges.len(), expected_edges);

        Self {
            nodes,
            edges: edges.into(),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Edge(usize, usize);

impl Edge {
    pub fn to_bytes(&self) -> [u8; 16] {
        let mut bytes: [MaybeUninit<u8>; 16] = [const { MaybeUninit::uninit() }; 16];
        let (left, right) = bytes.split_at_mut(8);
        left.copy_from_slice(&self.0.to_le_bytes().map(|b| MaybeUninit::new(b)));
        right.copy_from_slice(&self.1.to_le_bytes().map(|b| MaybeUninit::new(b)));

        // SAFETY: Each member in Edge is 8 bytes large, so every element will have been
        // initialized if we initialize the halves 0..=7 and 8..=15 separately.
        unsafe { mem::transmute(bytes) }
    }

    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(
            usize::from_le_bytes([bytes[0], bytes[1], bytes[2],  bytes[3],  bytes[4],  bytes[5],  bytes[6],  bytes[7]]),
            usize::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]]),
        )
    }
}

pub struct Keys([u64; NUM_GRAPH_NODES]);

impl Keys {
    pub fn get(&self, edge: Edge) -> (u64, u64) {
        (self[edge.0], self[edge.1])
    }
}

impl Index<usize> for Keys {
    type Output = u64;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

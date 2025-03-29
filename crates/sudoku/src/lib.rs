use graph::{Edge, Graph};
use std::cell::LazyCell;
use std::fmt::{self, Display, Formatter};

#[macro_export]
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

pub const PUZZLE: LazyCell<Sudoku> = LazyCell::new(|| {
    sudoku! {
        4 _ _ _ 9 6 2 _ 8;
        3 _ 8 1 _ _ _ 9 _;
        9 6 1 _ _ _ 7 _ _;
        _ _ 3 4 _ 5 9 6 _;
        6 _ _ 9 2 8 _ 7 4;
        _ _ 4 7 _ _ 1 _ _;
        _ _ 9 _ _ 2 _ _ 1;
        _ _ _ 8 3 1 6 4 _;
        _ _ _ _ 4 _ _ 2 7;
    }
});

#[derive(Clone)]
pub struct Sudoku {
    pub grid: [[u8; 9]; 9],
    pub given: Box<[(usize, usize)]>,
}

impl From<&Sudoku> for Graph<u8> {
    fn from(sudoku: &Sudoku) -> Self {
        // One node for each cell, as well as nine nodes for each constraint for the given cells.
        let mut nodes = Vec::with_capacity(90);

        // Each row has 9 * 8 / 2 edges and there are 9 rows, making a total of 324 edges.
        // By symmetry, each column has the same number of edges as the rows.
        // Each 3-by-3 grid has 18 non-coaxial edges and there are 9 of them, making a total of 162.
        // Combined, the rows, columns, and 3-by-3 grids form 2 * 324 + 162 = 810 edges.
        // Each given number has 8 edges, one for each constraint node that is not equal to the
        // given number.
        let expected_num_edges = 810 + 8 * sudoku.given.len();
        let mut edges = Vec::with_capacity(expected_num_edges);

        for (y, row) in sudoku.grid.into_iter().enumerate() {
            for (x, cell) in row.into_iter().enumerate() {
                nodes.push(cell);

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

        // The constraint nodes for the given numbers.
        nodes.extend(1..=9);

        for (i, j) in sudoku.given.iter().copied() {
            let value = sudoku.grid[j][i];
            for v in (1..=9).filter(|v| *v != value) {
                edges.push(Edge(9 * j + i, 80 + v as usize));
            }
        }

        debug_assert_eq!(edges.len(), expected_num_edges);

        Self {
            nodes: nodes.into(),
            edges: edges.into(),
        }
    }
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

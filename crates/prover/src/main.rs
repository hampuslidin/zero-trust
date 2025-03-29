use std::{array, cell::LazyCell, io, sync::Arc, sync::RwLock, thread};

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    execute, queue, style,
    style::{ContentStyle, StyledContent, Stylize},
    terminal::{self, ClearType},
};
use rand::prelude::*;
use tiny_http::{Method, Response, Server};

use bytes::Bytes;
use graph::{Edge, Graph};
use sudoku::{PUZZLE, Sudoku, sudoku};

const SOLUTION: LazyCell<Sudoku> = LazyCell::new(|| {
    sudoku! {
        4 5 7 3 9 6 2 1 8;
        3 2 8 1 5 7 4 9 6;
        9 6 1 2 8 4 7 5 3;
        7 8 3 4 1 5 9 6 2;
        6 1 5 9 2 8 3 7 4;
        2 9 4 7 6 3 1 8 5;
        8 4 9 6 7 2 5 3 1;
        5 7 2 8 3 1 6 4 9;
        1 3 6 5 4 9 8 2 7;
    }
});
const FAKE_SOLUTION: LazyCell<Sudoku> = LazyCell::new(|| {
    sudoku! {
        1 2 3 4 5 6 7 8 9;
        4 5 6 7 8 9 1 2 3;
        7 8 9 1 2 3 4 5 6;
        2 3 4 5 6 7 8 9 1;
        5 6 7 8 9 1 2 3 4;
        8 9 1 2 3 4 5 6 7;
        3 4 5 6 7 8 9 1 2;
        6 7 8 9 1 2 3 4 5;
        9 1 2 3 4 5 6 7 8;
    }
});

fn main() -> io::Result<()> {
    let progress = Arc::new(RwLock::new(PUZZLE.clone()));
    run_verification_server(Arc::clone(&progress));
    run_sudoku_game(progress, &mut io::stdout())
}

fn run_verification_server(progress: Arc<RwLock<Sudoku>>) {
    let mut verification_keys = Vec::new();
    let mut mappers = Vec::new();

    let server = Server::http("0.0.0.0:8000").expect("valid connection");
    thread::spawn(move || {
        for mut request in server.incoming_requests() {
            let mut graph = Graph::from(&*progress.read().expect("poisoned"));
            let num_edges = graph.edges.len();

            let url = request.url();
            let (path, query) = url
                .split_once('?')
                .map_or((url, None), |(path, query)| (path, Some(query)));

            match (request.method(), path) {
                (Method::Get, "/nodes") => {
                    verification_keys.clear();
                    mappers.clear();

                    let count = query
                        .and_then(|q| q.split_once('='))
                        .and_then(|(key, value)| (key == "count").then_some(value))
                        .and_then(|value| value.parse::<usize>().ok())
                        .unwrap_or(num_edges)
                        .max(1)
                        .min(num_edges);

                    let mut rng = rand::rng();

                    let mut encrypted_nodes = Vec::with_capacity(count);
                    for _ in 0..count {
                        let mut mapper: [u8; 10] = array::from_fn(|i| i as u8);
                        mapper[1..].shuffle(&mut rng);

                        let (encrypted_nodes_elem, keys) = graph.map(&mapper).encrypt();

                        encrypted_nodes.push(encrypted_nodes_elem);
                        verification_keys.push(keys);
                        mappers.push(mapper);
                    }

                    let bytes = encrypted_nodes.to_bytes();
                    let _ = request.respond(Response::from_data(bytes));
                }

                (Method::Post, "/verify") => 'post_verify: {
                    if verification_keys.is_empty() {
                        let _ = request.respond(Response::empty(400));
                        break 'post_verify;
                    }

                    let mut edge_bytes = Vec::new();
                    let Ok(_) = request.as_reader().read_to_end(&mut edge_bytes) else {
                        let _ = request.respond(Response::empty(400));
                        break 'post_verify;
                    };

                    let Ok(edges) = <Vec<Edge>>::from_bytes(&edge_bytes) else {
                        let _ = request.respond(Response::empty(400));
                        break 'post_verify;
                    };

                    if edges.len() != verification_keys.len() {
                        let _ = request.respond(Response::empty(400));
                        break 'post_verify;
                    }

                    let mut combined_mapper: [u8; 10] = array::from_fn(|i| i as u8);

                    let verification_data: Vec<_> = edges
                        .into_iter()
                        .zip(verification_keys.iter().zip(&mappers))
                        .map(|(edge, (key, mapper))| {
                            let (val_0, val_1) = graph.get_copied(edge);
                            combined_mapper[1..]
                                .iter_mut()
                                .for_each(|v| *v = mapper[*v as usize]);

                            (
                                combined_mapper[val_0 as usize],
                                combined_mapper[val_1 as usize],
                                key.get(edge),
                            )
                        })
                        .collect();

                    let verification_data_bytes = verification_data.to_bytes();
                    let _ = request.respond(Response::from_data(verification_data_bytes));
                }

                _ => {
                    let _ = request.respond(Response::empty(404));
                }
            }
        }
    });
}

fn run_sudoku_game<W>(progress: Arc<RwLock<Sudoku>>, w: &mut W) -> io::Result<()>
where
    W: io::Write,
{
    execute!(w, terminal::EnterAlternateScreen)?;

    terminal::enable_raw_mode()?;

    let mut position = (0usize, 0usize);

    let mut exit_app = false;
    while !exit_app {
        queue!(
            w,
            style::ResetColor,
            terminal::Clear(ClearType::All),
            cursor::Show,
            cursor::MoveTo(0, 0)
        )?;

        let puzzle_str = progress.read().expect("poisoned").to_string();
        for line in puzzle_str.lines() {
            queue!(w, style::Print(line), cursor::MoveToNextLine(1))?;
        }

        let instr_offs = 2 + 4 * 9;
        queue!(
            w,
            cursor::MoveTo(instr_offs, 0),
            style::PrintStyledContent(StyledContent::new(
                ContentStyle::default().underlined(),
                "Instructions"
            )),
        )?;

        [
            "Solve the puzzle by entering digits such that all rows,",
            "columns and 3-by-3 squares contain the digits 1 to 9 exactly",
            "once. Digits with a filled background are the given hints and",
            "can't be changed (except when entering a fake solution with ",
            "'f' and 'g').",
            "",
            "Up, Down, Left, Right - Move cursor",
            "1 through 9           - Enter digit",
            "Space or 0            - Clear digit",
            "s                     - Solve the puzzle",
            "c                     - Clear all digits",
            "f                     - Enter a fake solution",
            "g                     - Enter a fake solution with only givens",
        ]
        .into_iter()
        .enumerate()
        .try_for_each(|(i, line)| {
            queue!(
                w,
                cursor::MoveTo(instr_offs, 2 + i as u16),
                style::Print(line)
            )
        })?;

        queue!(
            w,
            cursor::MoveTo(2 + 4 * position.0 as u16, 1 + 2 * position.1 as u16)
        )?;

        w.flush()?;

        loop {
            if let Ok(Event::Key(KeyEvent {
                code,
                kind: KeyEventKind::Press,
                ..
            })) = event::read()
            {
                let mut progress = progress.write().expect("poisoned");
                let can_write = !progress
                    .given
                    .contains(&(position.0 as usize, position.1 as usize));

                match code {
                    KeyCode::Esc => exit_app = true,
                    KeyCode::Left => {
                        position.0 = (position.0 + 8) % 9;
                    }
                    KeyCode::Right => {
                        position.0 = (position.0 + 1) % 9;
                    }
                    KeyCode::Up => {
                        position.1 = (position.1 + 8) % 9;
                    }
                    KeyCode::Down => {
                        position.1 = (position.1 + 1) % 9;
                    }
                    KeyCode::Char(c @ '0'..='9') if can_write => {
                        progress.grid[position.1][position.0] = c as u8 - '0' as u8;
                    }
                    KeyCode::Char(' ') if can_write => {
                        progress.grid[position.1][position.0] = 0;
                    }
                    KeyCode::Char('s') => {
                        progress.grid = SOLUTION.grid;
                    }
                    KeyCode::Char('c') => {
                        progress.grid = PUZZLE.grid;
                    }
                    KeyCode::Char('f') => {
                        progress.given = PUZZLE.given.clone();
                        progress.grid = FAKE_SOLUTION.grid;
                    }
                    KeyCode::Char('g') => {
                        *progress = FAKE_SOLUTION.clone();
                    }
                    _ => continue,
                }

                break;
            }
        }
    }

    let disable_raw_mode_res = terminal::disable_raw_mode();

    let execute_res = execute!(
        w,
        style::ResetColor,
        cursor::Show,
        terminal::LeaveAlternateScreen
    );

    disable_raw_mode_res?;
    execute_res?;

    Ok(())
}

use std::{io, sync::RwLock, thread};

use crossterm::{
    cursor, event,
    event::{Event, KeyCode, KeyEvent, KeyEventKind},
    execute, queue, style, terminal,
    terminal::ClearType,
};
use tiny_http::{Method, Response, Server};

use bytes::Bytes;
use graph::{Edge, Graph};
use sudoku::{PUZZLE, Sudoku};

static SOLUTION: RwLock<Sudoku<38>> = RwLock::new(PUZZLE);

fn main() -> io::Result<()> {
    run_verification_server();
    run_sudoku_game(&mut io::stdout())
}

fn run_verification_server() {
    let mut verification_keys = Vec::new();

    let server = Server::http("0.0.0.0:8000").expect("valid connection");
    thread::spawn(move || {
        for mut request in server.incoming_requests() {
            let graph = Graph::from(&*SOLUTION.read().expect("poisoned"));
            let num_edges = graph.num_edges();

            let url = request.url();
            let (path, query) = url
                .split_once('?')
                .map_or((url, None), |(path, query)| (path, Some(query)));

            match (request.method(), path) {
                (Method::Get, "/graph") => {
                    let count = query
                        .and_then(|q| q.split_once('='))
                        .and_then(|(key, value)| (key == "count").then_some(value))
                        .and_then(|value| value.parse::<usize>().ok())
                        .unwrap_or(num_edges)
                        .max(1)
                        .min(num_edges);

                    verification_keys.clear();
                    let mut encrypted_graphs = Vec::with_capacity(count);
                    for _ in 0..count {
                        let (encrypted_graph, keys) = graph.encrypt();
                        encrypted_graphs.push(encrypted_graph);
                        verification_keys.push(keys);
                    }

                    let bytes = encrypted_graphs.to_bytes();
                    let _ = request.respond(Response::from_data(bytes));

                    // Skip resetting the verification keys.
                    continue;
                }

                (Method::Post, "/edge") => 'post_edge: {
                    if verification_keys.is_empty() {
                        let _ = request.respond(Response::empty(400));
                        break 'post_edge;
                    }

                    let mut edge_bytes = Vec::new();
                    let Ok(_) = request.as_reader().read_to_end(&mut edge_bytes) else {
                        let _ = request.respond(Response::empty(400));
                        break 'post_edge;
                    };

                    if edge_bytes.len() != 16 {
                        let _ = request.respond(Response::empty(400));
                        break 'post_edge;
                    }

                    let edge = Edge::from_bytes([
                        edge_bytes[0],
                        edge_bytes[1],
                        edge_bytes[2],
                        edge_bytes[3],
                        edge_bytes[4],
                        edge_bytes[5],
                        edge_bytes[6],
                        edge_bytes[7],
                        edge_bytes[8],
                        edge_bytes[9],
                        edge_bytes[10],
                        edge_bytes[11],
                        edge_bytes[12],
                        edge_bytes[13],
                        edge_bytes[14],
                        edge_bytes[15],
                    ]);

                    let values = graph.get(edge);
                    let keys = verification_keys.get(edge);

                    let mut verification_data = Vec::with_capacity(18);
                    verification_data.push(*values.0);
                    verification_data.push(*values.1);
                    verification_data.extend(keys.0.to_le_bytes());
                    verification_data.extend(keys.1.to_le_bytes());

                    debug_assert_eq!(verification_data.len(), 18);

                    let _ = request.respond(Response::from_data(verification_data.clone()));
                }

                _ => {
                    let _ = request.respond(Response::empty(404));
                }
            }

            verification_keys.take();
        }
    });
}

fn run_sudoku_game<W>(w: &mut W) -> io::Result<()>
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

        let puzzle_str = SOLUTION.read().expect("poisoned").to_string();
        for line in puzzle_str.lines() {
            queue!(w, style::Print(line), cursor::MoveToNextLine(1))?;
        }

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
                let mut solution = SOLUTION.write().expect("poisoned");
                let can_write = !solution
                    .given
                    .contains(&(position.0 as usize, position.1 as usize));

                match code {
                    KeyCode::Esc => {
                        exit_app = true;
                        break;
                    }
                    KeyCode::Left => {
                        position.0 = (position.0 + 8) % 9;
                        break;
                    }
                    KeyCode::Right => {
                        position.0 = (position.0 + 1) % 9;
                        break;
                    }
                    KeyCode::Up => {
                        position.1 = (position.1 + 8) % 9;
                        break;
                    }
                    KeyCode::Down => {
                        position.1 = (position.1 + 1) % 9;
                        break;
                    }
                    KeyCode::Char(c @ '0'..='9') if can_write => {
                        solution.grid[position.1][position.0] = c as u8 - '0' as u8;
                        break;
                    }
                    KeyCode::Char(' ') if can_write => {
                        solution.grid[position.1][position.0] = 0;
                        break;
                    }
                    KeyCode::Char('s') => {
                        solution.grid[0][1] = 5;
                        solution.grid[0][2] = 7;
                        solution.grid[0][3] = 3;
                        solution.grid[0][7] = 1;
                        solution.grid[1][1] = 2;
                        solution.grid[1][4] = 5;
                        solution.grid[1][5] = 7;
                        solution.grid[1][6] = 4;
                        solution.grid[1][8] = 6;
                        solution.grid[2][3] = 2;
                        solution.grid[2][4] = 8;
                        solution.grid[2][5] = 4;
                        solution.grid[2][7] = 5;
                        solution.grid[2][8] = 3;
                        solution.grid[3][0] = 7;
                        solution.grid[3][1] = 8;
                        solution.grid[3][4] = 1;
                        solution.grid[3][8] = 2;
                        solution.grid[4][1] = 1;
                        solution.grid[4][2] = 5;
                        solution.grid[4][6] = 3;
                        solution.grid[5][0] = 2;
                        solution.grid[5][1] = 9;
                        solution.grid[5][4] = 6;
                        solution.grid[5][5] = 3;
                        solution.grid[5][7] = 8;
                        solution.grid[5][8] = 5;
                        solution.grid[6][0] = 8;
                        solution.grid[6][1] = 4;
                        solution.grid[6][3] = 6;
                        solution.grid[6][4] = 7;
                        solution.grid[6][6] = 5;
                        solution.grid[6][7] = 3;
                        solution.grid[7][0] = 5;
                        solution.grid[7][1] = 7;
                        solution.grid[7][2] = 2;
                        solution.grid[7][8] = 9;
                        solution.grid[8][0] = 1;
                        solution.grid[8][1] = 3;
                        solution.grid[8][2] = 6;
                        solution.grid[8][3] = 5;
                        solution.grid[8][5] = 9;
                        solution.grid[8][6] = 8;
                        break;
                    }
                    KeyCode::Char('c') => {
                        *solution = PUZZLE;
                        break;
                    }
                    _ => (),
                }
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

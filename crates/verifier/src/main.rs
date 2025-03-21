use std::{
    error::Error,
    io::{self, Write},
    thread, 
    time::Duration,
};

use sudoku::{Graph, PUZZLE};

fn main() {
    loop {
        print!("Verifying");
        io::stdout().flush().expect("flush should succeed");

        match verify() {
            Ok(()) => println!(" - Success"),
            Err(err) => println!(" - Failure: {err}"),
        }

        thread::sleep(Duration::from_millis(1000));
    }

}

fn verify() -> Result<(), Box<dyn Error>> {
    let encrypted_graph_bytes: Vec<u8> = ureq::get("http://127.0.0.1:8000/graph")
        .call()?
        .body_mut()
        .read_to_vec()?;

    let encrypted_graph = Graph::from_bytes(&encrypted_graph_bytes)?;
    let edge = encrypted_graph.random_edge();

    let response: Vec<u8> = ureq::post("http://127.0.0.1:8000/edge")
        .send(&edge.to_bytes())?
        .body_mut()
        .read_to_vec()?;

    let graph = Graph::from(&PUZZLE);
    let values = graph.get(edge);

    Ok(())
}

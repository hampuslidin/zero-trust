use std::{
    error::Error,
    fmt,
    fmt::{Display, Formatter},
    io::{self, Write},
    thread,
    time::Duration,
};

use sudoku::Graph;

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
    for _ in 0..10 {
        let encrypted_graph_bytes: Vec<u8> = ureq::get("http://127.0.0.1:8000/graph?count=10")
            .call()?
            .body_mut()
            .read_to_vec()?;

        let encrypted_graph = Graph::from_bytes(&encrypted_graph_bytes)?;
        let edge = encrypted_graph.random_edge();

        let verification_data: Vec<u8> = ureq::post("http://127.0.0.1:8000/edge")
            .send(&edge.to_bytes())?
            .body_mut()
            .read_to_vec()?;

        if verification_data.len() != 18 {
            return Err(VerificationError::InvalidVerificationData.into());
        }

        let values = (verification_data[0], verification_data[1]);

        if values.0 == values.1 {
            return Err(VerificationError::AdjacentIdenticalNodes.into());
        }

        let keys = (
            u64::from_le_bytes([
                verification_data[2],
                verification_data[3],
                verification_data[4],
                verification_data[5],
                verification_data[6],
                verification_data[7],
                verification_data[8],
                verification_data[9],
            ]),
            u64::from_le_bytes([
                verification_data[10],
                verification_data[11],
                verification_data[12],
                verification_data[13],
                verification_data[14],
                verification_data[15],
                verification_data[16],
                verification_data[17],
            ]),
        );

        if encrypted_graph[edge.0] != sudoku::hash(values.0, keys.0) {
            return Err(VerificationError::HashMismatch.into());
        }

        if encrypted_graph[edge.1] != sudoku::hash(values.1, keys.1) {
            return Err(VerificationError::HashMismatch.into());
        }
    }

    Ok(())
}

#[derive(Debug)]
enum VerificationError {
    InvalidVerificationData,
    AdjacentIdenticalNodes,
    HashMismatch,
}

impl Error for VerificationError {}

impl Display for VerificationError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::InvalidVerificationData => write!(f, "invalid verification data"),
            Self::AdjacentIdenticalNodes => write!(f, "adjacent identical nodes"),
            Self::HashMismatch => write!(f, "hash mismatch"),
        }
    }
}

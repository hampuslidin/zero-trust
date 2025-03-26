use std::{
    error::Error,
    fmt,
    fmt::{Display, Formatter},
    io::{self, Write},
    thread,
    time::Duration,
};

use bytes::Bytes;
use graph::{Edge, EncryptedGraph};
use sudoku::PUZZLE;

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

        println!("{:0x} {:0x} {:0x} {:0x}", encrypted_graph_bytes[0], encrypted_graph_bytes[1], encrypted_graph_bytes[2], encrypted_graph_bytes[3]);
        let encrypted_graphs: Vec<EncryptedGraph<{ PUZZLE.given.len() }>> = Bytes::from_bytes(&encrypted_graph_bytes)?;

        let edges: Vec<Edge> = encrypted_graphs.iter().map(|graph| graph.random_edge()).collect();

        let verification_data_bytes: Vec<u8> = ureq::post("http://127.0.0.1:8000/edge")
            .send(&*edges.to_bytes())?
            .body_mut()
            .read_to_vec()?;

        let Ok(verification_data) = <Vec<((u8, u8), (u64, u64))>>::from_bytes(&verification_data_bytes) else {
            return Err(VerificationError::InvalidVerificationData.into());
        };


        for (i, (values, keys)) in verification_data.into_iter().enumerate() {
            if values.0 == values.1 {
                return Err(VerificationError::AdjacentIdenticalNodes.into());
            }

            if encrypted_graphs[i][edges[i].0] != graph::hash(values.0, keys.0) {
                return Err(VerificationError::HashMismatch.into());
            }

            if encrypted_graphs[i][edges[i].1] != graph::hash(values.1, keys.1) {
                return Err(VerificationError::HashMismatch.into());
            }
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

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    io::{self, Write},
    thread,
    time::Duration,
};

use bytes::Bytes;
use graph::{Edge, EncryptedNode, Graph};
use rand::prelude::*;

fn main() {
    let graph = Graph::from(&*sudoku::PUZZLE);
    let mut edges = graph.edges.clone();

    loop {
        print!("Verifying");
        io::stdout().flush().expect("flush should succeed");

        match verify(&mut edges) {
            Ok(()) => println!(" - Solved"),
            Err(err) => println!(" - {err}"),
        }

        thread::sleep(Duration::from_millis(100));
    }
}

fn verify(edges: &mut Box<[Edge]>) -> Result<(), Box<dyn Error>> {
    let encrypted_node_bytes: Vec<u8> = ureq::get("http://127.0.0.1:8000/nodes")
        .call()?
        .body_mut()
        .read_to_vec()?;
    let encrypted_nodes: Vec<Box<[EncryptedNode]>> = Bytes::from_bytes(&encrypted_node_bytes)?;

    let mut rng = rand::rng();
    edges.shuffle(&mut rng);

    let verification_data_bytes: Vec<u8> = ureq::post("http://127.0.0.1:8000/verify")
        .send(&*edges.to_bytes())?
        .body_mut()
        .read_to_vec()?;

    let Ok(verification_data) = <Vec<((u8, u8), (u64, u64))>>::from_bytes(&verification_data_bytes)
    else {
        return Err(VerificationError::InvalidVerificationData.into());
    };

    for (i, (values, keys)) in verification_data.into_iter().enumerate() {
        let edge = edges[i];

        if values.0 == 0 || values.1 == 0 {
            return Err(VerificationError::Unsolved.into());
        }

        if values.0 == values.1 {
            return Err(VerificationError::UnsatisfiedConstraint.into());
        }

        if encrypted_nodes[i][edge.0] != graph::hash(values.0, keys.0) {
            return Err(VerificationError::IncorrectHash.into());
        }

        if encrypted_nodes[i][edge.1] != graph::hash(values.1, keys.1) {
            return Err(VerificationError::IncorrectHash.into());
        }
    }

    Ok(())
}

#[derive(Debug)]
enum VerificationError {
    IncorrectHash,
    InvalidVerificationData,
    UnsatisfiedConstraint,
    Unsolved,
}

impl Error for VerificationError {}

impl Display for VerificationError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::IncorrectHash => write!(f, "Incorrect hash"),
            Self::InvalidVerificationData => write!(f, "Invalid verification data"),
            Self::UnsatisfiedConstraint => write!(f, "Unsatisfied constraint"),
            Self::Unsolved => write!(f, "Unsolved"),
        }
    }
}

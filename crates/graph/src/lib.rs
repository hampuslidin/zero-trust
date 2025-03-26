use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    mem::{self, MaybeUninit},
    ops::Index,
};

use derive_deftly::Deftly;
use rand::prelude::*;
use sha2::{Digest, Sha256};

use bytes::derive_deftly_template_Bytes;

pub fn hash(value: u8, key: u64) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update((value as u64 ^ key).to_le_bytes());
    let output = hasher.finalize();
    output.as_slice().try_into().expect("size is not 32 bytes")
}

#[derive(Clone, Debug, Deftly)]
#[derive_deftly(Bytes)]
pub struct Graph<const NUM_NODES: usize, T> {
    pub nodes: [T; NUM_NODES],
    pub edges: Box<[Edge]>,
}

pub type EncryptedGraph<const NUM_NODES: usize> = Graph<NUM_NODES, [u8; 32]>;

impl<const NUM_NODES: usize, T> Graph<NUM_NODES, T> {
    pub fn num_edges(&self) -> usize {
        self.edges.len()
    }

    pub fn random_edge(&self) -> Edge {
        let mut rng = rand::rng();
        *self.edges.choose(&mut rng).expect("slice is non-empty")
    }

    pub fn get(&self, edge: Edge) -> (&T, &T) {
        (&self[edge.0], &self[edge.1])
    }

    pub fn get_copied(&self, edge: Edge) -> (T, T) where T: Copy {
        (self[edge.0], self[edge.1])
    }
}

impl<const NUM_NODES: usize> Graph<NUM_NODES, u8> {
    pub fn encrypt(&self) -> (EncryptedGraph<NUM_NODES>, Keys<NUM_NODES>) {
        let mut rng = rand::rng();
        let keys = rng.random::<[u64; NUM_NODES]>();
        let mut encrypted_nodes = [MaybeUninit::uninit(); NUM_NODES];
        for (node, (encrypted_node, key)) in self
            .nodes
            .into_iter()
            .zip(encrypted_nodes.iter_mut().zip(keys.iter().copied()))
        {
            encrypted_node.write(hash(node, key));
        }

        // SAFETY: `keys`, `encrypted_nodes`, and `self.nodes` all have the same number of elements,
        // so the zipped elements above will also have the same number of elements. Thus, all
        // elements have been initialized.
        let nodes = unsafe { mem::transmute_copy(&encrypted_nodes) };

        (
            Graph {
                nodes,
                edges: self.edges.clone(),
            },
            Keys(keys),
        )
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

impl<const NUM_NODES: usize, T> Index<usize> for Graph<NUM_NODES, T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.nodes[index]
    }
}

#[derive(Clone, Copy, Debug, Deftly)]
#[derive_deftly(Bytes)]
pub struct Edge(pub usize, pub usize);

pub struct Keys<const N: usize>([u64; N]);

impl<const N: usize> Keys<N> {
    pub fn get(&self, edge: Edge) -> (u64, u64) {
        (self[edge.0], self[edge.1])
    }
}

impl<const N: usize> Index<usize> for Keys<N> {
    type Output = u64;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

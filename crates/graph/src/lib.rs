use std::ops::Index;

use derive_deftly::Deftly;
use rand::Rng;
use sha2::{Digest, Sha256};

use bytes::derive_deftly_template_Bytes;

pub fn hash(value: u8, key: u64) -> EncryptedNode {
    let mut hasher = Sha256::new();
    hasher.update((value as u64 ^ key).to_le_bytes());
    let output = hasher.finalize();
    output.as_slice().try_into().expect("size is not 32 bytes")
}

#[derive(Clone, Debug, Deftly)]
#[derive_deftly(Bytes)]
pub struct Graph<T> {
    pub nodes: Box<[T]>,
    pub edges: Box<[Edge]>,
}

pub type EncryptedNode = [u8; 32];

impl<T> Graph<T> {
    pub fn get(&self, edge: Edge) -> (&T, &T) {
        (&self[edge.0], &self[edge.1])
    }

    pub fn get_copied(&self, edge: Edge) -> (T, T)
    where
        T: Copy,
    {
        (self[edge.0], self[edge.1])
    }
}

impl Graph<u8> {
    pub fn map(&mut self, mapper: &[u8; 10]) -> &mut Self {
        self.nodes
            .iter_mut()
            .for_each(|node| *node = mapper[*node as usize]);
        self
    }

    pub fn encrypt(&self) -> (Box<[EncryptedNode]>, Keys) {
        let mut rng = rand::rng();
        let mut encrypted_nodes = Vec::with_capacity(self.nodes.len());
        let mut keys = Vec::with_capacity(self.nodes.len());
        for (node, (encrypted_node, key)) in self
            .nodes
            .iter()
            .zip(encrypted_nodes.iter_mut().zip(keys.iter_mut()))
        {
            let rand_key = rng.random();
            *encrypted_node = hash(*node, rand_key);
            *key = rand_key;
        }

        (encrypted_nodes.into(), Keys(keys.into()))
    }
}

impl<T> Index<usize> for Graph<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.nodes[index]
    }
}

#[derive(Clone, Copy, Debug, Deftly)]
#[derive_deftly(Bytes)]
pub struct Edge(pub usize, pub usize);

pub struct Keys(Box<[u64]>);

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

use std::{
    collections::HashSet,
    hash::{DefaultHasher, Hash, Hasher},
};
use rand::Rng;

pub struct DedupHasher {
    used: HashSet<u64>,
}
impl DedupHasher {
    pub fn new() -> Self {
        DedupHasher {
            used: HashSet::new(),
        }
    }
    pub fn hash(&mut self, value: &str) -> u64 {
        let mut rng = rand::rng();
        let mut feed = format!("{:0>11}", base62::encode(rng.random::<u64>()));
        for _retry in 0.. {
            let base = format!("{value}{feed}");
            let mut hasher = DefaultHasher::new();
            base.hash(&mut hasher);
            let hash = hasher.finish();
            if self.used.insert(hash) {
                return hash;
            } else {
                feed = format!("{:0>11}", base62::encode(hash));
            }
        }
        unreachable!()
    }
}

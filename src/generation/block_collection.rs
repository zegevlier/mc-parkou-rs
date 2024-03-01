use std::collections::HashMap;

use crate::weighted_vec::WeightedVec;
use valence::prelude::*;

/// The `BlockChoice` struct represents a choice between blocks of type `T`, with
/// the option to choose only one block for a specific generation or to choose
/// multiple blocks with a weighted probability.
///
/// Properties:
///
/// * `blocks`: The `blocks` property is a `WeightedVec<T>`, which is a vector of
/// elements of type `T` with associated weights. Each element in the vector is
/// assigned a weight, which determines the probability of that element being
/// chosen.
/// * `uniform`: The `uniform` property is a boolean value that determines whether
/// the `BlockChoice` will choose only one block or multiple blocks. If `uniform`
/// is `true`, then only one block will be chosen. If `uniform` is `false`, then
/// it will choose a random block each time.
#[derive(Clone, Debug)]
pub struct BlockChoice<T> {
    pub blocks: WeightedVec<T>,
    pub uniform: bool, // TODO: I don't like this. I sometimes even ignore it. There has to be a better way.
}

#[derive(Clone, Debug)]
pub struct BlockCollection(pub BlockChoice<BlockState>);

/// The `BlockCollectionMap` struct represents a collection of an arbitrary number
/// of `BlockCollection`s with a name associated with each one. This is used to
/// store the different types of blocks used in a generation.
///
/// If you require different shapes of the same type of block (e.g. full blocks,
/// slabs, and stairs), then the keys should be of the form
/// `"<name>_<shape>"`, where `<name>` is the name of the block and `<shape>` is
/// the shape of the block. For example, if you have a block called `stone` and
/// you want to use full blocks, slabs, and stairs, then you should use the keys
/// `"stone_full"`, `"stone_slab"`, and `"stone_stair"`.
///
/// If only one shape of a block is required, then the key should just be the name
/// of the block. For example, if you have a block called `grass`, then you should
/// use the key `"grass"`.
///
/// Properties:
///
/// * `collections`: The `collections` property is a `HashMap<String, BlockCollection>`.
/// It maps a name to a `BlockCollection`.
#[derive(Clone, Debug)]
pub struct BlockCollectionMap {
    pub collections: HashMap<String, BlockCollection>,
}

impl BlockCollectionMap {
    pub fn new() -> Self {
        Self {
            collections: HashMap::new(),
        }
    }

    pub fn add(&mut self, name: String, collection: BlockCollection) {
        self.collections.insert(name, collection);
    }

    pub fn build(self) -> BuiltBlockCollectionMap {
        let mut collections = HashMap::new();
        for (name, collection) in self.collections {
            let index = if collection.0.uniform {
                collection
                    .0
                    .blocks
                    .get_random_index()
                    .expect("No blocks in collection")
            } else {
                0
            };
            collections.insert(name, (collection, index));
        }
        BuiltBlockCollectionMap { collections }
    }
}

impl From<Vec<(String, BlockCollection)>> for BlockCollectionMap {
    fn from(collections: Vec<(String, BlockCollection)>) -> Self {
        let mut map = Self::new();
        for (name, collection) in collections {
            map.add(name, collection);
        }
        map
    }
}

impl<const N: usize> From<[(String, BlockCollection); N]> for BlockCollectionMap {
    fn from(arr: [(String, BlockCollection); N]) -> Self {
        let mut map = Self::new();
        for (name, collection) in arr {
            map.add(name, collection);
        }
        map
    }
}

impl<const N: usize> From<[(&str, BlockCollection); N]> for BlockCollectionMap {
    fn from(arr: [(&str, BlockCollection); N]) -> Self {
        let mut map = Self::new();
        for (name, collection) in arr {
            map.add(name.to_owned(), collection);
        }
        map
    }
}

#[derive(Clone, Debug)]
pub struct BuiltBlockCollectionMap {
    pub collections: HashMap<String, (BlockCollection, usize)>,
}

#[allow(dead_code)]
impl BuiltBlockCollectionMap {

    /// Gets a block from the `BlockCollectionMap` with the given name. If the
    /// `BlockCollection` is uniform, then it will always return the same block.
    pub fn get_block_opt(&self, name: &str) -> Option<BlockState> {
        let (collection, index) = self.collections.get(name)?;
        if collection.0.uniform {
            Some(collection.0.blocks[*index])
        } else {
            Some(*collection.0.blocks.get_random().unwrap())
        }
    }

    /// Gets a block from the `BlockCollectionMap` with the given name. If the
    /// `BlockCollection` is uniform, then it will always return the same block.
    ///
    /// Panics if the block does not exist.
    pub fn get_block(&self, name: &str) -> BlockState {
        self.get_block_opt(name)
            .unwrap_or_else(|| panic!("No block `{}`", name))
    }
}

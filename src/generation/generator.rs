use std::collections::HashMap;

use crate::{prediction::prediction_state::PredictionState, utils::*};

use super::{block_collection::*, generation::*, theme::GenerationTheme};
use valence::prelude::*;

/// The `GenerationType` enum represents the different types of parkour generations
/// that can be used.
///
/// Variants:
/// * `Single`: The `Single` variant represents a single block.
/// * `Slime`: The `Slime` variant represents a slime block. // TODO
/// * `Ramp`: The `Ramp` variant represents blocks and slabs that are used to create
/// a ramp.
/// * `Island`: The `Island` variant represents blocks that are used to create an
/// island.
/// * `Indoor`: The `Indoor` variant represents blocks that are used to create an
/// indoor area.
/// * `Cave`: The `Cave` variant represents blocks that are used to create a cave.
/// * `Snake`: The `Snake` variant represents blocks that are used to create a
/// snake.
/// * `BlinkBlocks`: The `BlinkBlocks` variant represents blocks that are used to
/// create a blinking platform.
/// * `SingleCustom`: The `SingleCustom` variant represents a custom parkour
/// generation. It has preset blocks, a start position, and an end position.
/// * `MultiCustom`: The `MultiCustom` variant represents a custom parkour
/// generation. It has a start custom generation, a number of middle custom
/// generations, and an end custom generation.
/// * `ComplexCustom`: The `ComplexCustom` variant represents a custom parkour
/// generation that is generated using a DFS algorithm. It produces a tile-based
/// generation.
#[derive(Clone, Debug)]
pub enum GenerationType {
    Single(String),
}

/// The `Generator` struct represents a parkour generator.
///
/// Properties:
///
/// * `theme`: The `theme` property is of type `GenerationTheme`. It represents the
/// theme of the parkour generator.
/// * `type`: The `type` property is of type `GenerationType`. It represents the
/// type of parkour generation that is used.
/// * `start`: The `start` property is of type `BlockPos`. It represents the start
/// position of the parkour generation.
#[derive(Clone, Debug)]
pub struct Generator {
    pub theme: GenerationTheme,
    pub generation_type: GenerationType,
    pub start: BlockPos,
}

impl Generator {
    pub fn first_in_generation(start: BlockPos, theme: &GenerationTheme) -> Generation {
        let theme = theme.clone();
        let s = Self {
            generation_type: theme.generation_types[0].clone(),
            theme,
            start: BlockPos::new(0, 0, 0),
        };

        let yaw = random_yaw();

        let mut g = s.generate(JumpDirection::DoesntMatter); // no lines for first generation

        g.offset = start;
        g.end_state = PredictionState::running_jump_block(start, yaw);

        g
    }

    pub fn next_in_generation(
        direction: JumpDirection,
        theme: &GenerationTheme,
        generation: &Generation,
    ) -> Generation {
        let theme = theme.clone();
        let mut state = generation.end_state;

        let target_y = (state.pos.y as i32 + direction.get_y_offset()) as f64;

        let g = loop {
            let mut new_state = state;
            new_state.tick();

            if new_state.vel.y > 0. || new_state.pos.y > target_y {
                state = new_state;
            } else {
                break Self {
                    generation_type: theme.get_random_generation_type(),
                    theme,
                    start: state.get_block_pos(),
                };
            }
        };

        g.generate(direction)
    }

    pub fn generate(&self, direction: JumpDirection) -> Generation {
        let mut blocks = HashMap::new();
        let offset: BlockPos = self.start;
        let children = Vec::new();
        let ordered = true;

        let params = BlockGenParams {
            direction,
            block_map: self.theme.block_map.clone().build(),
        };

        let end_state = match &self.generation_type {
            GenerationType::Single(key) => {
                blocks.insert(BlockPos::new(0, 0, 0), params.block_map.get_block(key));
                // 3x3 area centered at 0,0,0
                // blocks.insert(BlockPos::new(1, 0, 0), params.block_map.get_block(key));
                // blocks.insert(BlockPos::new(-1, 0, 0), params.block_map.get_block(key));
                // blocks.insert(BlockPos::new(0, 0, 1), params.block_map.get_block(key));
                // blocks.insert(BlockPos::new(0, 0, -1), params.block_map.get_block(key));
                // blocks.insert(BlockPos::new(1, 0, 1), params.block_map.get_block(key));
                // blocks.insert(BlockPos::new(-1, 0, 1), params.block_map.get_block(key));
                // blocks.insert(BlockPos::new(1, 0, -1), params.block_map.get_block(key));
                // blocks.insert(BlockPos::new(-1, 0, -1), params.block_map.get_block(key));
                
                PredictionState::running_jump_block(self.start, random_yaw())
            }
        };

        Generation {
            blocks,
            children,
            ordered,
            offset,
            end_state,
        }
    }
}

/// The `BlockGenParams` struct represents parameters for a block generator.
#[derive(Clone, Debug)]
pub struct BlockGenParams {
    pub direction: JumpDirection,
    pub block_map: BuiltBlockCollectionMap,
}

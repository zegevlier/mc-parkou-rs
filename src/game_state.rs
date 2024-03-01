use std::collections::VecDeque;

use valence::prelude::*;

use crate::{
    generation::{generation::Generation, theme::GenerationTheme},
    prediction::prediction_state::PredictionState,
    utils::*,
};

#[derive(Component)]
pub struct GameState {
    pub generations: VecDeque<Generation>,
    pub target_y: i32,
    pub direction: JumpDirection,
    pub theme: GenerationTheme,
    pub score: u32,
    pub combo: u32,
    pub stopped_running: bool,
    pub tick: usize,
    pub prev_pos: DVec3,
    pub test_state: PredictionState,
}

#![allow(clippy::type_complexity)]

use std::collections::VecDeque;

use game_state::GameState;
use generation::block_collection::*;

use generation::generator::{GenerationType, Generator};

use generation::theme::GenerationTheme;
use prediction::prediction_state::PredictionState;
use utils::JumpDirection;
use valence::prelude::*;
use valence::protocol::sound::{Sound, SoundCategory};
use valence::spawn::IsFlat;

mod game_state;
mod generation;
mod prediction;
mod utils;
mod weighted_vec;

const START_POS: BlockPos = BlockPos::new(0, 100, 0);
const DIFF: i32 = 10;
const MIN_Y: i32 = START_POS.y - DIFF;
const MAX_Y: i32 = START_POS.y + DIFF;
const VIEW_DIST: u8 = 32;

pub fn main() {
    App::new()
        .insert_resource(NetworkSettings {
            connection_mode: ConnectionMode::Offline,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                init_clients,
                reset_clients.after(init_clients),
                manage_chunks.after(reset_clients).before(manage_blocks),
                manage_blocks,
                despawn_disconnected_clients,
                cleanup_clients,
            ),
        )
        .add_systems(EventLoopUpdate, (detect_stop_running,))
        .run();
}

fn setup(
    mut commands: Commands,
    server: Res<Server>,
    dimensions: Res<DimensionTypeRegistry>,
    biomes: Res<BiomeRegistry>,
) {
    let layer = LayerBundle::new(ident!("overworld"), &dimensions, &biomes, &server);

    commands.spawn(layer);
}

fn init_clients(
    mut clients: Query<
        (
            Entity,
            &mut Client,
            &mut EntityLayerId,
            &mut VisibleChunkLayer,
            &mut VisibleEntityLayers,
            &mut IsFlat,
            &mut GameMode,
        ),
        Added<Client>,
    >,
    layers: Query<Entity, (With<ChunkLayer>, With<EntityLayer>)>,
    server: Res<Server>,
    dimensions: Res<DimensionTypeRegistry>,
    biomes: Res<BiomeRegistry>,
    mut commands: Commands,
) {
    for (
        entity,
        mut client,
        mut layer_id,
        mut visible_chunk_layer,
        mut visible_entity_layers,
        mut is_flat,
        mut game_mode,
    ) in clients.iter_mut()
    {
        let layer = layers.single();

        layer_id.0 = layer;
        visible_chunk_layer.0 = layer;
        visible_entity_layers.0.insert(layer);

        visible_chunk_layer.0 = entity;
        is_flat.0 = true;
        *game_mode = GameMode::Adventure; // TODO: Change to adventure

        client.send_chat_message("Welcome to epic infinite parkour game!".italic());

        let state = GameState {
            generations: VecDeque::new(),
            direction: JumpDirection::DoesntMatter,
            theme: GenerationTheme::new(
                "name".to_owned(),
                BlockCollectionMap::from([(
                    "concrete",
                    BlockCollection(BlockChoice {
                        blocks: weighted_vec![
                            BlockState::WHITE_CONCRETE,
                            BlockState::ORANGE_CONCRETE,
                            BlockState::MAGENTA_CONCRETE,
                            BlockState::LIGHT_BLUE_CONCRETE,
                            BlockState::YELLOW_CONCRETE,
                            BlockState::LIME_CONCRETE,
                            BlockState::PINK_CONCRETE,
                            BlockState::GRAY_CONCRETE,
                            BlockState::LIGHT_GRAY_CONCRETE,
                            BlockState::CYAN_CONCRETE,
                            BlockState::PURPLE_CONCRETE,
                            BlockState::BLUE_CONCRETE,
                            BlockState::BROWN_CONCRETE,
                            BlockState::GREEN_CONCRETE,
                            BlockState::RED_CONCRETE,
                            // BlockState::BLACK_CONCRETE, // black has no contrast and is completely invisible at night
                        ],
                        uniform: true,
                    }),
                )]),
                weighted_vec![(GenerationType::Single("concrete".to_string()), 100.0),],
            ),
            score: 0,
            combo: 0,
            target_y: 0,
            stopped_running: false,
            tick: 0,
            prev_pos: DVec3::new(
                START_POS.x as f64 + 0.5,
                START_POS.y as f64 + 1.0,
                START_POS.z as f64 + 0.5,
            ),
            test_state: PredictionState::new(
                DVec3::new(
                    START_POS.x as f64 + 0.5,
                    START_POS.y as f64 + 1.0,
                    START_POS.z as f64 + 0.5,
                ),
                DVec3::ZERO,
                0.0,
            ),
        };

        let layer = ChunkLayer::new(ident!("overworld"), &dimensions, &biomes, &server);

        commands.entity(entity).insert((state, layer));
    }
}

fn reset_clients(
    mut clients: Query<(
        &mut Client,
        &mut Position,
        &mut Look,
        &mut GameState,
        &mut ChunkLayer,
    )>,
) {
    for (mut client, mut pos, mut look, mut state, mut layer) in clients.iter_mut() {
        state.test_state.yaw = look.yaw / 180.0 * std::f32::consts::PI;
        state.test_state.vel = pos.0 - state.prev_pos;

        state.test_state.pos = pos.0;
        state.prev_pos = pos.0;

        let out_of_bounds = (pos.0.y as i32) < START_POS.y - 40;

        if out_of_bounds || state.is_added() {
            if out_of_bounds && !state.is_added() {
                client.send_chat_message(
                    "Your score was ".italic()
                        + state
                            .score
                            .to_string()
                            .color(Color::GOLD)
                            .bold()
                            .not_italic(),
                );
            }

            // Init chunks.
            for pos in ChunkView::new(ChunkPos::from_block_pos(START_POS), VIEW_DIST).iter() {
                layer.insert_chunk(pos, UnloadedChunk::new());
            }

            state.score = 0;
            state.combo = 0;
            {
                let state = &mut *state;
                for block in &state.generations {
                    block.remove(&mut layer);
                }
            }

            state.generations.clear();
            let gen = Generator::first_in_generation(START_POS, &state.theme);
            gen.place(&mut layer);
            state.generations.push_back(gen);

            for _ in 0..10 {
                generate_next_block(&mut state, &mut layer);
            }

            pos.set([
                START_POS.x as f64 + 0.5,
                START_POS.y as f64 + 1.0,
                START_POS.z as f64 + 0.5,
            ]);
            look.yaw = 0.0;
            look.pitch = 0.0;
        }
    }
}

fn cleanup_clients(
    mut disconnected_clients: RemovedComponents<Client>,
    mut query: Query<&mut GameState>,
) {
    for entity in disconnected_clients.iter() {
        if let Ok(mut state) = query.get_mut(entity) {
            state.generations.clear();
        }
    }
}

fn detect_stop_running(mut event: EventReader<SprintEvent>, mut clients: Query<&mut GameState>) {
    for mut state in clients.iter_mut() {
        if let Some(event) = event.iter().next() {
            if matches!(event.state, SprintState::Stop) {
                state.stopped_running = true;
            }
        }
    }
}

fn manage_blocks(mut clients: Query<(&mut Client, &Position, &mut GameState, &mut ChunkLayer)>) {
    for (client, pos, mut state, mut layer) in clients.iter_mut() {
        if let Some(index) = state
            .generations
            .iter()
            .position(|block| block.has_reached(*pos))
        {
            if index > 0 {
                let mut score = index as u32;

                if !state.generations[index].ordered {
                    score -= 1;
                }

                for i in 0..index {
                    let s = state.generations[i].get_unreached_child_count();
                    score += s;
                }
                {
                    let state = &mut *state;

                    for _ in 0..index {
                        remove_block(state, &mut layer);
                        generate_next_block(state, &mut layer);
                    }
                }
                reached_thing(state, score, client, pos);
            } else {
                let s = state.generations[0].has_reached_child(*pos);
                if s > 0 {
                    reached_thing(state, s, client, pos);
                }
            }
        }
    }
}

fn reached_thing(
    mut state: Mut<'_, GameState>,
    score: u32,
    mut client: Mut<'_, Client>,
    pos: &Position,
) {
    if state.stopped_running {
        state.combo = 0;
    } else {
        state.combo += score;
    }

    state.score += score;

    let pitch = 0.9 + ((state.combo as f32) - 1.0) * 0.05;
    client.play_sound(
        Sound::BlockNoteBlockBass,
        SoundCategory::Master,
        pos.0,
        1.0,
        pitch,
    );

    client.set_action_bar(state.score.to_string().color(Color::LIGHT_PURPLE).bold());
}

fn manage_chunks(mut clients: Query<(&Position, &OldPosition, &mut ChunkLayer), With<Client>>) {
    for (pos, old_pos, mut layer) in &mut clients {
        let old_view = ChunkView::new(old_pos.chunk_pos(), VIEW_DIST);
        let view = ChunkView::new(pos.to_chunk_pos(), VIEW_DIST);

        if old_view != view {
            for pos in old_view.diff(view) {
                layer.remove_chunk(pos);
            }

            for pos in view.diff(old_view) {
                layer.chunk_entry(pos).or_default();
            }
        }
    }
}

fn remove_block(state: &mut GameState, world: &mut ChunkLayer) {
    let removed_block = state.generations.pop_front().unwrap();
    removed_block.remove(world);
}

fn generate_next_block(state: &mut GameState, layer: &mut ChunkLayer) {
    let prev_gen = state.generations.back().unwrap();

    if prev_gen.end_state.get_block_pos().y < MIN_Y {
        state.target_y = START_POS.y;
        state.direction = JumpDirection::Up;
    } else if prev_gen.end_state.get_block_pos().y > MAX_Y {
        state.target_y = START_POS.y;
        state.direction = JumpDirection::Down;
    } else {
        match state.direction {
            JumpDirection::Up => {
                if prev_gen.end_state.get_block_pos().y >= state.target_y {
                    state.direction = JumpDirection::DoesntMatter;
                }
            }
            JumpDirection::Down => {
                if prev_gen.end_state.get_block_pos().y <= state.target_y {
                    state.direction = JumpDirection::DoesntMatter;
                }
            }
            _ => {}
        }
    }

    let next_gen = Generator::next_in_generation(state.direction, &state.theme, prev_gen);

    next_gen.place(layer);
    state.generations.push_back(next_gen);

    // Combo System
    state.stopped_running = false;
}

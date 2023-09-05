#![allow(clippy::type_complexity)]

use std::collections::VecDeque;

use bunch_of_blocks::{BunchOfBlocks, BunchType};
use game_state::GameState;
use prediction::player_state::PlayerState;
use utils::particle_outline_block;
use valence::entity::block_display::{BlockDisplayEntityBundle, self};
use valence::entity::display::Scale;
use valence::entity::entity::NameVisible;
use valence::entity::sheep::SheepEntityBundle;
use valence::prelude::*;
use valence::protocol::sound::{Sound, SoundCategory};
use valence::spawn::IsFlat;

mod block_types;
mod bunch_of_blocks;
mod game_state;
mod parkour_gen_params;
mod prediction;
mod utils;
mod weighted_vec;
mod line;

const START_POS: BlockPos = BlockPos::new(0, 100, 0);
const VIEW_DIST: u8 = 10;

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
                // spawn_lines,
                despawn_disconnected_clients,
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
    for (entity, mut client, mut layer_id, mut visible_chunk_layer, mut visible_entity_layers, mut is_flat, mut game_mode) in
        clients.iter_mut()
    {
        let layer = layers.single();

        layer_id.0 = layer;
        visible_chunk_layer.0 = layer;
        visible_entity_layers.0.insert(layer);

        visible_chunk_layer.0 = entity;
        is_flat.0 = true;
        *game_mode = GameMode::Adventure;

        client.send_chat_message("Welcome to epic infinite parkour game!".italic());

        let state = GameState {
            blocks: VecDeque::new(),
            prev_type: None,
            score: 0,
            combo: 0,
            target_y: 0,
            stopped_running: false,
            prev_pos: DVec3::new(
                START_POS.x as f64 + 0.5,
                START_POS.y as f64 + 1.0,
                START_POS.z as f64 + 0.5,
            ),
            test_state: PlayerState::new(
                DVec3::new(
                    START_POS.x as f64 + 0.5,
                    START_POS.y as f64 + 1.0,
                    START_POS.z as f64 + 0.5,
                ),
                DVec3::ZERO,
                0.0,
            ),
            entities: Vec::new(),
            lines: Vec::new(),
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
    for (mut client, mut pos, mut look, mut state, mut layer) in clients.iter_mut()
    {
        state.test_state.yaw = look.yaw / 180.0 * std::f32::consts::PI;
        state.test_state.vel = pos.0 - state.prev_pos;
        // if state.test_state.vel.y == 0. {
        //     if state.test_state.vel.x == 0. && state.test_state.vel.z == 0. {
        //         state.test_state.vel.x = -0.215 * state.test_state.yaw.sin() as f64;
        //         state.test_state.vel.z = 0.215 * state.test_state.yaw.cos() as f64;
        //     }
        //     state.test_state.vel.y = 0.42f32 as f64;
        // }
        state.test_state.pos = pos.0;
        state.prev_pos = pos.0;

        // state.test_state.draw_particles(32, &mut client);

        // for entity in state.entities.iter() {
        //     commands.entity(*entity).despawn();
        // }

        // state.entities.clear();

        // let command = commands.spawn(SheepEntityBundle {
        //         position: *pos,
        //         layer: *entity_layer_id,
        //         entity_name_visible: NameVisible(true),
        //         ..Default::default()
        //     }).id();
        
        // state.entities.push(command);

        // let mut lines = Vec::new();

        // for bbb in state.blocks.iter() {
        //     // lines.append(&mut bbb.next_params.initial_state.get_lines_for_number_of_ticks(bbb.next_params.ticks as usize));
        //     bbb.next_params
        //         .initial_state
        //         .draw_particles(bbb.next_params.ticks as usize, &mut client);

        //     particle_outline_block(bbb.next_params.end_pos, Vec3::new(1., 0., 0.), &mut client);
        //     particle_outline_block(bbb.next_params.next_pos, Vec3::new(0., 1., 0.), &mut client);
        // }

        // state.lines = lines;

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

            for block in &state.blocks {
                block.remove(&mut layer)
            }
            state.blocks.clear();
            let blocc = BunchOfBlocks::single(START_POS, BlockState::STONE, &*state);
            blocc.place(&mut layer);
            state.blocks.push_back(blocc);
            state.prev_type = Some(BunchType::Single);

            for _ in 0..10 {
                generate_next_block(&mut state, &mut layer, false);
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

fn detect_stop_running(mut event: EventReader<SprintEvent>, mut clients: Query<&mut GameState>) {
    for mut state in clients.iter_mut() {
        if let Some(event) = event.iter().next() {
            if matches!(event.state, SprintState::Stop) {
                state.stopped_running = true;
            }
        }
    }
}

fn spawn_lines(
    mut commands: Commands,
    mut clients: Query<(
        &mut GameState,
        &EntityLayerId
    )>,
) {
    for (mut state, layer) in clients.iter_mut() {
        for entity in state.entities.iter() {
            commands.entity(*entity).insert(Despawned);
        }

        let mut entities = Vec::new();

        for line in state.lines.iter() {
            let mut display = BlockDisplayEntityBundle {
                block_display_block_state: block_display::BlockState(BlockState::STONE),
                position: Position(line.start.as_dvec3()),
                display_scale: Scale(line.end - line.start),
                ..Default::default()
            };//line.to_block_display();
            display.layer = *layer;
            let cmd = commands.spawn(display);

            entities.push(cmd.id());
        }

        state.entities = entities;
    }
}

fn manage_blocks(mut clients: Query<(&mut Client, &Position, &mut GameState, &mut ChunkLayer)>) {
    for (mut client, pos, mut state, mut layer) in clients.iter_mut() {
        if let Some(index) = state
            .blocks
            .iter()
            .position(|block| block.has_reached(*pos))
        {
            if index > 0 {
                if state.stopped_running {
                    state.combo = 0
                } else {
                    state.combo += index as u32
                }

                for _ in 0..index {
                    generate_next_block(&mut state, &mut layer, true)
                }

                let pitch = 0.9 + ((state.combo as f32) - 1.0) * 0.05;
                client.play_sound(
                    Sound::BlockNoteBlockBass,
                    SoundCategory::Master,
                    pos.0,
                    1.0,
                    pitch,
                );

                if state.score < 50 && state.score % 10 == 0
                    || state.score == 75
                    || state.score >= 100 && state.score % 50 == 0
                {
                    client.set_title("");
                    client.set_subtitle(state.score.to_string().color(Color::LIGHT_PURPLE).bold());
                }
            }
        }
    }
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

fn generate_next_block(state: &mut GameState, layer: &mut ChunkLayer, in_game: bool) {
    if in_game {
        let removed_block = state.blocks.pop_front().unwrap();
        removed_block.remove(layer);

        state.score += 1
    }

    let next_params = &state.blocks.back().unwrap().next_params;
    let last_pos = next_params.end_pos;

    if last_pos.y == START_POS.y {
        state.target_y = 0
    } else if last_pos.y < START_POS.y - 20 || last_pos.y > START_POS.y + 20 {
        state.target_y = START_POS.y;
    }

    let bunch = next_params.generate(&*state);

    state.prev_type = Some(bunch.bunch_type);

    bunch.place(layer);
    state.blocks.push_back(bunch);

    // Combo System
    state.stopped_running = false;
}

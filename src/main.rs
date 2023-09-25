use bevy::prelude::*;

use game_over_screen::GameOverScreenPlugin;
use grid::{GridPlugin, GridLocation, ApplyGridMovement, AnimateTranslation, MovementComplete};
use inventory::{Inventory, Item, ItemGet, PickUpItems, InventoryPlugin};
use spawn_level::SpawnLevelPlugin;
use ui::{UiPlugin, UpdateUi};

mod game_over_screen;
mod grid;
mod inventory;
mod ui;
mod spawn_level;

// Current gameplay:
// - move down and right on a grid, optimize your path to get the most candy
// - grab fuel to be able to go up or left, adds a nice bit of complexity to the optimization problem.

// Immediate next steps:
// - Add sound effects !! (done)
// - Fix the double-despawn bug (kinda fixed, still latent)
// - Fix fuel and candy spawning on first/last cell (done)
// - Generalize to n time loops (go until all candy is collected)
// - UI / appstate changes for time loops
// - Iterate further on plugin structure

// More gameplay:
// - Add a between-levels upgrade system of some kind; spend candy, get upgrades.

// Tech debt:
// - No hierarchical entity relationships; everything is just flat right now. That is fine for now but not forever.
// - Pull out more plugins. main.rs is a dumping ground right now lol. Next: looping? movement? application state?

// Bugs:
// - Fuel can spawn on the last cell, and if it does you won't get to use it as the game will end first.
// - Some kind of double-despawn bug seems to be happening.
// Figured out some of why this was happening - pickups on the last spot were despawning twice, once when they were
// picked up and once when the level despawned. I fixed this by not spawning pickups on the last spot.
// However this is a broader issue with system ordering when transitioning between states.

// Polish:
// - Show the actual candies/fuel collected in the score display instead of a number
// - Animate candy/fuel collection - have it fly up to the score/fuel display?
// - Add a background to the level
// - Style the score/fuel display
// - Control the window dimensions ?
// - Use a sprite for the grid cells instead of a solid color
// - Sound effects for picking up candies
// - Transparency for the candy sprite
// - Queue inputs so they aren't skipped if the player is moving
// - Animate a wiggle when the player tries to move off the grid
// - Show the recorded moves on the grid (maybe a path in a different color and offset for each soot?)

// Time loop todo:
// - Make score and fuel into components on the player (done)
// - Review those changes, they felt a little awkward (done)
// - Record the player's moves (done)
// - Add state to track whether we're in the first or second loop (done)
// - Add a system to replay the player's moves from the first loop (done)
// - Add a second player entity to represent the past self, only present during the second loop (done)
// - Have the past self replay the moves from the first loop instead of the player (done)
// - Add state to track whose turn it is (player or past self should alternate) (done)
// - Make player/past self only move when it's their turn (done)
// - When one player has no more moves, skip their turn (done)
// - Keep the same map for both loops (done)
// - One recording per loop, not one recording for the whole game
// - Differentiate between next loop and next game (next loop - quick transition, no UI; next game - slow transition, show UI?)
// - Any number of loops - keep going until all candy is collected
// - Show the total collected candy across all soots in UI and at end of game

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(GridPlugin)
        .add_plugins(InventoryPlugin)
        .add_plugins(UiPlugin)
        .add_plugins(SpawnLevelPlugin)
        .add_plugins(GameOverScreenPlugin)
        .add_state::<AppState>()
        .add_systems(Startup, spawn_cam)
        .add_systems(OnEnter(AppState::Playing), reset_move_buffer)
        .configure_sets(Update, (ApplyGridMovement, PickUpItems, UpdateUi).chain())
        .add_systems(Update,
            (
                (
                    process_movement_input,
                    (debuffer_move_inputs, replay_move_attempts),
                    validate_move,
                    (move_soot_on_grid, record_moves),
                ).chain().before(ApplyGridMovement),
                (
                    play_item_pickup_sound,
                ).chain().after(PickUpItems),
                next_turn,
                detect_game_over,
            ).chain().run_if(in_state(AppState::Playing)))
        .insert_resource(MoveBuffer::default())
        .add_event::<MoveAttempt>()
        .add_event::<Move>()
        .insert_resource(TimeLoopRecording::default())
        .insert_resource(LoopCounter(0))
        .insert_resource(TurnCounter(0))
        .add_systems(OnExit(AppState::Playing), despawn_after_playing)
        .add_systems(OnExit(AppState::GameOver), (despawn_after_game_over, swap_loop))
        .run();
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum AppState {
    #[default]
    Playing,
    GameOver,
}

#[derive(Component, Clone, Copy)]
struct DespawnOnExitPlaying;

#[derive(Component, Clone, Copy)]
struct DespawnOnExitGameOver;

fn despawn_after_playing(mut commands: Commands, query: Query<Entity, With<DespawnOnExitPlaying>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

fn despawn_after_game_over(mut commands: Commands, query: Query<Entity, With<DespawnOnExitGameOver>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

fn spawn_cam(mut commands: Commands) {
    let max_grid_location = Vec2 {x: MAX_X as f32 - 1., y: MAX_Y as f32 - 1.};
    let max_grid_pixel = max_grid_location * GRID_SPACING as f32;
    let center = (max_grid_pixel/2.).extend(0.);
    commands.spawn(Camera2dBundle{
        transform: Transform { translation: center, ..default() },
        ..default()
    });
}

const MAX_X: i32 = 5;
const MAX_Y: i32 = 5;
const GRID_SPACING: i32 = 130;
const START_SPACE: IVec2 = IVec2 {x: 0, y: MAX_Y - 1};
const END_SPACE: IVec2 = IVec2 {x: MAX_X - 1, y: 0};

#[derive(Component)]
struct Player;

#[derive(Component)]
struct SootSprite {
    loop_number: i32,
}

#[derive(Resource, Default)]
struct MoveBuffer {
    next_move: IVec2
}

fn reset_move_buffer(mut move_buffer: ResMut<MoveBuffer>) {
    move_buffer.next_move = IVec2::ZERO;
}

fn process_movement_input(
    keyboard_input: Res<Input<KeyCode>>,
    mut move_buffer: ResMut<MoveBuffer>,
) {
    let mut offset = IVec2 {x:0, y:0};
    if keyboard_input.any_just_pressed([KeyCode::Right, KeyCode::D]) {
        offset.x += 1;
    }
    if keyboard_input.any_just_pressed([KeyCode::Left, KeyCode::A]) {
        offset.x -= 1;
    }
    if keyboard_input.any_just_pressed([KeyCode::Down, KeyCode::S]) {
        offset.y -= 1;
    }
    if keyboard_input.any_just_pressed([KeyCode::Up, KeyCode::W]) {
        offset.y += 1;
    }

    if offset.length_squared() == 1 {
        move_buffer.next_move = offset;
    }
}

#[derive(Event)]
struct MoveAttempt {
    mover: Entity,
    offset: IVec2,
}

fn debuffer_move_inputs(
    player: Query<(Entity, &AnimateTranslation), With<Player>>,
    mut move_buffer: ResMut<MoveBuffer>,
    mut event_writer: EventWriter<MoveAttempt>,
    turn_counter: Res<TurnCounter>,
) {
    if turn_counter.0 != 0 {
        return;
    }

    let (player, animation) = player.single();
    if !animation.timer.finished() {
        return;
    }

    let offset = move_buffer.next_move;
    if offset.length_squared() == 0 {
        return;
    }

    move_buffer.next_move = IVec2::ZERO;
    event_writer.send(MoveAttempt{mover: player, offset});
}

fn replay_move_attempts(
    soot_sprite: Query<(Entity, &AnimateTranslation), (With<SootSprite>, Without<Player>)>,
    mut recording: ResMut<TimeLoopRecording>,
    mut event_writer: EventWriter<MoveAttempt>,
    turn_counter: Res<TurnCounter>,
) {
    if turn_counter.0 != 1 {
        return;
    }

    let (soot_entity, animation) = soot_sprite.single();
    if !animation.timer.finished() {
        return;
    }

    if recording.moves.is_empty() {
        return;
    }

    let offset = recording.moves.remove(0);
    event_writer.send(MoveAttempt{mover:soot_entity, offset});
}

#[derive(Event)]
struct Move {
    mover: Entity,
    offset: IVec2,
    fuel_cost: i32,
}

fn validate_move(
    soot_sprites: Query<(&GridLocation, &Inventory), With<SootSprite>>,
    mut attempts: EventReader<MoveAttempt>,
    mut moves: EventWriter<Move>,
) {
    if attempts.is_empty() {
        return;
    }

    if attempts.len() > 1 {
        panic!("Multiple move attempts in one frame!");
    }

    let &MoveAttempt{mover: soot_entity, offset} = attempts.iter().next().unwrap();
    let (grid_location, inventory) = soot_sprites.get(soot_entity).unwrap();

    let mut fuel_cost = 0;
    if offset.x < 0 {
        fuel_cost += 1;
    }
    if offset.y > 0 {
        fuel_cost += 1;
    }

    if fuel_cost > inventory.fuel {
        return;
    }

    let next_pos = grid_location.0 + offset;
    if next_pos.x < 0 || next_pos.x >= MAX_X || next_pos.y < 0 || next_pos.y >= MAX_Y {
        return;
    }

    moves.send(Move{mover: soot_entity, offset, fuel_cost});
}

fn move_soot_on_grid(
    mut soot_sprites: Query<(&mut GridLocation, &mut Inventory), With<SootSprite>>,
    mut events: EventReader<Move>,
) {
    if events.is_empty() {
        return;
    }

    if events.len() > 1 {
        panic!("Multiple moves in one frame!");
    }

    let &Move{mover: soot_entity, offset, fuel_cost} = events.iter().next().unwrap();
    let (mut grid_location, mut inventory) = soot_sprites.get_mut(soot_entity).unwrap();
    grid_location.0 += offset;

    if fuel_cost > 0 {
        inventory.fuel -= fuel_cost;
    }
}

#[derive(Resource, Default)]
struct TimeLoopRecording {
    moves: Vec<IVec2>,
}

fn record_moves(
    mut recording: ResMut<TimeLoopRecording>,
    mut events: EventReader<Move>,
    loop_counter: Res<LoopCounter>,
) {
    if loop_counter.0 != 0 {
        return;
    }

    for event in events.iter() {
        recording.moves.push(event.offset);
    }
}

fn detect_game_over(
    soots: Query<(&GridLocation, &AnimateTranslation), With<SootSprite>>,
    mut app_state: ResMut<NextState<AppState>>,
) {
    for (soot_location, animation) in soots.iter() {
        if !animation.timer.finished() {
            return;
        }

        if soot_location != (&GridLocation(END_SPACE)) {
            return;
        }
    }

    app_state.set(AppState::GameOver);
}

#[derive(Resource)]
struct LoopCounter(i32);

#[derive(Resource)]
struct TurnCounter(i32);

fn swap_loop(mut loop_counter: ResMut<LoopCounter>, mut recording: ResMut<TimeLoopRecording>) {
    println!("Moves recorded: {:?}", recording.moves);
    if loop_counter.0 == 0 {
        loop_counter.0 += 1;
        return;
    }
    loop_counter.0 = 0;
    recording.moves.clear();
}

fn next_turn(
    mut turn_counter: ResMut<TurnCounter>,
    loop_counter: Res<LoopCounter>,
    soots: Query<(&SootSprite, &GridLocation)>,
    mut movement_events: EventReader<MovementComplete>,
) {
    if movement_events.is_empty() {
        return;
    }

    if movement_events.len() > 1 {
        panic!("Multiple movement events in one frame!");
    }

    // Validate that the correct entity just moved.
    let &MovementComplete{entity} = movement_events.iter().next().unwrap();
    let (soot_sprite, _) = soots.get(entity).unwrap();
    if soot_sprite.loop_number != turn_counter.0 {
        panic!("Wrong entity moved! Expected loop {}, got loop {}.", loop_counter.0, soot_sprite.loop_number);
    }

    let can_move = |loop_number: i32| {
        for (soot_sprite, grid_location) in soots.iter() {
            if soot_sprite.loop_number == loop_number && grid_location.0 == (END_SPACE) {
                return false;
            }
        }
        true
    };

    let num_loops = loop_counter.0 + 1;
    for turn_increment in 1..=num_loops {
        let next_turn = (turn_counter.0 + turn_increment) % num_loops;
        if !can_move(next_turn) {
            continue;
        }
        turn_counter.0 = next_turn;
        return;
    }

    // This case will happen if nobody can move; prepares us for next loop.
    turn_counter.0 = 0;
}

fn play_item_pickup_sound(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut event_reader: EventReader<ItemGet>)
{
    for event in event_reader.iter() {
        let sound = match event.item {
            Item::Candy => "candy-pickup.wav",
            Item::Fuel => "fuel-pickup.wav",
        };
        commands.spawn(AudioBundle{
            source: asset_server.load(sound),
            ..default()
        });
    }
}

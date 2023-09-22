use bevy::prelude::*;

use game_over_screen::GameOverScreenPlugin;
use grid::{GridPlugin, GridLocation, ApplyGridMovement, AnimateTranslation};
use inventory::{Inventory, Item, ItemGet, PickUpItems, InventoryPlugin};
use spawn_level::{SpawnLevel, SpawnLevelPlugin};
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
// - Basic player animation (done)
// - Figure out system ordering - too many things are nondeterministic right now. (done)
// - Sound on picking up candy and fuel (done-ish)
// - Add a basic time loop - play twice, with your past self going through the level alongside you the second time.
// - Iterate further on plugin structure (better)

// More gameplay:
// - Add a between-levels upgrade system of some kind; spend candy, get upgrades.

// Tech debt:
// - No hierarchical entity relationships; everything is just flat right now. That is fine for now but not forever.
// - Pull out more plugins. main.rs is a dumping ground right now lol. Next: looping? movement? application state?

// Bugs:
// - Fuel can spawn on the last cell, and if it does you won't get to use it as the game will end first.

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

// Time loop todo:
// - Make score and fuel into components on the player (done)
// - Review those changes, they felt a little awkward (done)
// - Record the player's moves (done)
// - Add state to track whether we're in the first or second loop (done)
// - Add a system to replay the player's moves from the first loop (done)
// - Add a second player entity to represent the past self, only present during the second loop
// - Have the past self replay the moves from the first loop instead of the player
// - Add state to track whose turn it is (player or past self should alternate)

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
        .configure_sets(OnEnter(AppState::Playing), (SpawnLevel, ApplyGridMovement).chain())
        .configure_sets(Update, (ApplyGridMovement, PickUpItems, UpdateUi).chain())
        .add_systems(Update,
            (
                (
                    process_movement_input,
                    debuffer_move_inputs.run_if(in_state(LoopState::FirstLoop)),
                    replay_move_attempts.run_if(in_state(LoopState::SecondLoop)),
                    move_player_on_grid,
                    record_move_attempts.run_if(in_state(LoopState::FirstLoop)),
                ).chain().before(ApplyGridMovement),
                (
                    play_item_pickup_sound,
                ).chain().after(PickUpItems),
                detect_game_over,
            ).chain().run_if(in_state(AppState::Playing)))
        .insert_resource(MoveBuffer::default())
        .add_event::<MoveAttempt>()
        .insert_resource(TimeLoopRecording::default())
        .add_state::<LoopState>()
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

#[derive(Component)]
struct DespawnOnExitPlaying;

#[derive(Component)]
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

#[derive(Component)]
struct Player;

#[derive(Resource, Default)]
struct MoveBuffer {
    next_move: IVec2
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

    if offset.length_squared() > 0 {
        move_buffer.next_move = offset;
    }
}

#[derive(Event)]
struct MoveAttempt {
    player: Entity,
    offset: IVec2,
}

fn debuffer_move_inputs(
    player: Query<(Entity, &AnimateTranslation), With<Player>>,
    mut move_buffer: ResMut<MoveBuffer>,
    mut event_writer: EventWriter<MoveAttempt>,
) {
    let (player, animation) = player.single();
    if !animation.timer.finished() {
        return;
    }

    let offset = move_buffer.next_move;
    if offset.length_squared() == 0 {
        return;
    }

    move_buffer.next_move = IVec2::ZERO;
    event_writer.send(MoveAttempt{player, offset});
}

fn move_player_on_grid(
    mut player: Query<(&mut GridLocation, &mut Inventory), With<Player>>,
    mut events: EventReader<MoveAttempt>,
) {
    if events.is_empty() {
        return;
    }

    if events.len() > 1 {
        panic!("Multiple move attempts in one frame!");
    }

    let &MoveAttempt{player: player_entity, offset} = events.iter().next().unwrap();
    let (grid_location, inventory) = player.get(player_entity).unwrap();

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

    let (mut grid_location, mut inventory) = player.single_mut();
    grid_location.0 = next_pos;

    if fuel_cost > 0 {
        inventory.fuel -= fuel_cost;
    }
}

#[derive(Resource, Default)]
struct TimeLoopRecording {
    moves: Vec<IVec2>,
}

fn record_move_attempts(
    mut recording: ResMut<TimeLoopRecording>,
    mut events: EventReader<MoveAttempt>,
) {
    for event in events.iter() {
        recording.moves.push(event.offset);
    }
}

fn replay_move_attempts(
    player: Query<(Entity, &AnimateTranslation), With<Player>>,
    mut recording: ResMut<TimeLoopRecording>,
    mut event_writer: EventWriter<MoveAttempt>,
) {
    let (player, animation) = player.single();
    if !animation.timer.finished() {
        return;
    }

    if recording.moves.is_empty() {
        return;
    }

    let offset = recording.moves.remove(0);
    event_writer.send(MoveAttempt{player, offset});
}

fn detect_game_over(
    player: Query<(&GridLocation, &AnimateTranslation), With<Player>>,
    mut app_state: ResMut<NextState<AppState>>,
) {
    let (player_location, animation) = player.single();
    if !animation.timer.finished() {
        return;
    }

    if player_location == (&GridLocation(IVec2{x: MAX_X - 1, y: 0})) {
        app_state.set(AppState::GameOver);
    }
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum LoopState {
    #[default]
    FirstLoop,
    SecondLoop,
}

fn swap_loop(loop_state: Res<State<LoopState>>, mut next_loop_state: ResMut<NextState<LoopState>>, mut recording: ResMut<TimeLoopRecording>) {
    if *loop_state.get() == LoopState::FirstLoop {
        next_loop_state.set(LoopState::SecondLoop);
        println!("Done with first loop. Recording: {:?}", recording.moves);
    } else {
        next_loop_state.set(LoopState::FirstLoop);
        recording.moves.clear();
        println!("Done with second loop.");
    }
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

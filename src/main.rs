use std::time::Duration;

use bevy::{prelude::*, sprite::{MaterialMesh2dBundle, Mesh2dHandle}};
use rand::Rng;

use game_over_screen::GameOverScreenPlugin;
use grid::{GridPlugin, GridLocation, SnapToGrid, DistributeOnGrid, center_of, ApplyGridMovement, AnimateTranslation};

mod game_over_screen;
mod grid;

// Current gameplay:
// - move down and right on a grid, optimize your path to get the most candy
// - grab fuel to be able to go up or left, adds a nice bit of complexity to the optimization problem.

// Immediate next steps:
// - Basic player animation (done)
// - Figure out system ordering - too many things are nondeterministic right now. (done)
// - Sound on picking up candy and fuel
// - Add a basic time loop - play twice, with your past self going through the level alongside you the second time.
// - Iterate further on plugin structure

// More gameplay:
// - Add a between-levels upgrade system of some kind; spend candy, get upgrades.

// Tech debt:
// - No hierarchical entity relationships; everything is just flat right now. That is fine for now but not forever.
// - Pull out more plugins. main.rs is a dumping ground right now lol. Maybe inventory stuff?

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
// - Record the player's moves
// - Add state to track whether we're in the first or second loop
// - Add a system to replay the player's moves from the first loop
// - Add state to track whose turn it is (player or past self should alternate)

// General things to think about:
// - Adding/removing components is actually a pain, because it requires apply_deferred. This makes ordering a bit more complicated.
// Should I be using exclusive systems instead when I basically want ~immediate application?
// - Ordering across plugin boundaries kinda sucks. System sets help.
// - Plugin abstraction is unclear in general. It seems like there are a lot of ways for plugins to interact with each other,
// besides simply adding ECS things, or events or resources, to the same world.
// - Not using events yet. Should I be? Would they be helpful for the movement animation in lieu of adding/removing components?
// - Plugins can be parameterized (they're structs). Can that be used for the grid parameters?

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_state::<AppState>()
        .add_systems(Startup, spawn_cam)
        .add_plugins(GridPlugin)
        .add_systems(OnEnter(AppState::Playing),
            (
                (
                    spawn_player,
                    spawn_grid,
                    spawn_candies,
                    spawn_fuel,
                    spawn_ui,
                ),
                apply_deferred
            ).chain().before(ApplyGridMovement)
        ).add_systems(Update,
            (
                move_player_on_grid.before(ApplyGridMovement),
                (
                    pick_up_item,
                    add_item_to_inventory,
                    update_score_display,
                    update_fuel_display,
                    play_item_pickup_sound,
                ).chain().after(ApplyGridMovement),
                detect_game_over,
            ).chain().run_if(in_state(AppState::Playing)))
        .add_event::<ItemGet>()
        .add_systems(OnExit(AppState::Playing), despawn_after_playing)
        .add_plugins(GameOverScreenPlugin)
        .add_systems(OnExit(AppState::GameOver), despawn_after_game_over)
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

fn spawn_grid(mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>
) {
    let mesh: Mesh2dHandle = meshes.add(Mesh::from(shape::Quad::default())).into();
    let material = materials.add(ColorMaterial::from(Color::PURPLE));
    let make_grid_item = |x: i32, y:i32| {
        let size: Vec3 = Vec3::splat(128.);
        (
            GridLocation(IVec2 {x, y}),
            MaterialMesh2dBundle {
                mesh: mesh.clone(),
                transform: Transform::default()
                    .with_scale(size)
                    .with_translation(Vec3::new((x * GRID_SPACING) as f32, (y * GRID_SPACING) as f32, 0.)),
                material: material.clone(),
                ..default()
            },
            DespawnOnExitGameOver,
        )
    };
    for x in 0..MAX_X {
        for y in 0..MAX_Y {
            commands.spawn(make_grid_item(x, y));
        }
    }
}

#[derive(Component)]
struct Player;

#[derive(Component, Clone, Copy)]
struct Inventory {
    candies: i32,
    fuel: i32,
}

impl Inventory {
    fn add(&mut self, item: Item) {
        match item {
            Item::Candy => self.candies += 1,
            Item::Fuel => self.fuel += 1,
        }
    }
}

#[derive(Component, Clone, Copy)]
enum Item {
    Candy,
    Fuel,
}

fn spawn_player(mut commands: Commands, asset_server: Res<AssetServer>) {
    let grid_location = GridLocation(IVec2 {x: 0, y: MAX_Y - 1});
    let make_finished_timer = |duration: Duration| {
        let mut timer = Timer::new(duration, TimerMode::Once);
        timer.tick(duration);
        timer
    };

    commands.spawn((
        Player,
        grid_location,
        Inventory{candies: 0, fuel: 0},
        SpriteBundle {
            texture: asset_server.load("soot-sprite.png"),
            transform: Transform::from_translation(center_of(&grid_location).extend(0.)),
            ..default()
        },
        SnapToGrid,
        AnimateTranslation{
            start: center_of(&grid_location),
            end: center_of(&grid_location),
            timer: make_finished_timer(Duration::from_millis(200)),
            ease: CubicSegment::new_bezier(Vec2::new(0., 0.), Vec2::new(0.4, 1.5)),
        },
        DespawnOnExitGameOver,
    ));
}

// TODO split input handling and movement into separate systems to facilitate input queuing
// Should those communicate with events?
fn move_player_on_grid(
    mut player: Query<(&mut GridLocation, &mut Inventory, &AnimateTranslation), With<Player>>,
    keyboard_input: Res<Input<KeyCode>>
) {
    let mut offset = IVec2 {x:0, y:0};
    let mut fuel_cost = 0;
    if keyboard_input.any_just_pressed([KeyCode::Right, KeyCode::D]) {
        offset.x += 1;
    }
    if keyboard_input.any_just_pressed([KeyCode::Left, KeyCode::A]) {
        offset.x -= 1;
        if offset.x < 0 {
            fuel_cost += 1;
        }
    }
    if keyboard_input.any_just_pressed([KeyCode::Down, KeyCode::S]) {
        offset.y -= 1;
    }
    if keyboard_input.any_just_pressed([KeyCode::Up, KeyCode::W]) {
        offset.y += 1;
        if offset.y > 0 {
            fuel_cost += 1;
        }
    }

    // TODO split would be here basically
    let (grid_location, inventory, animation) = player.single();
    if !animation.timer.finished() {
        return;
    }

    if offset.length_squared() == 0 || fuel_cost > inventory.fuel {
        return;
    }

    let next_pos = grid_location.0 + offset;
    if next_pos.x < 0 || next_pos.x >= MAX_X || next_pos.y < 0 || next_pos.y >= MAX_Y {
        return;
    }
    
    let (mut grid_location, mut inventory, _) = player.single_mut();
    grid_location.0 = next_pos;

    if fuel_cost > 0 {
        inventory.fuel -= fuel_cost;
    }
}

fn detect_game_over(mut next_state: ResMut<NextState<AppState>>, player: Query<(&GridLocation, &AnimateTranslation), With<Player>>) {
    let (player_location, animation) = player.single();
    if !animation.timer.finished() {
        return;
    }

    if player_location == (&GridLocation(IVec2{x: MAX_X - 1, y: 0})) {
        next_state.set(AppState::GameOver);
    }
}

#[derive(Clone)]
enum CandyColor {
   Red,
   Green,
   // Blue,
   Yellow,
}

impl CandyColor {
   fn asset_name(&self) -> &'static str {
       match self {
           CandyColor::Red => "red-candy.png",
           CandyColor::Green => "green-candy.png",
           // CandyColor::Blue => "blue-candy.png",
           CandyColor::Yellow => "yellow-candy.png"
       }
   }
}

const NUM_CANDIES: usize = 10;

fn spawn_candies(mut commands: Commands, asset_server: Res<AssetServer>) {
   let mut rng = rand::thread_rng();
   for _ in 0..NUM_CANDIES {
       let color =  match rng.gen_range(0..3) {
           0 => CandyColor::Red,
           1 => CandyColor::Green,
           2 => CandyColor::Yellow,
           _ => unreachable!(),
       };
       commands.spawn((
           Item::Candy,
           GridLocation (IVec2 {x: rng.gen_range(0..MAX_X), y: rng.gen_range(0..MAX_Y)}),
           SpriteBundle {
               texture: asset_server.load(color.asset_name()),
               sprite: Sprite {
                   custom_size: Some(Vec2::splat(64.)),
                   ..default()
               },
               ..default()
           },
           DistributeOnGrid,
           DespawnOnExitGameOver,
       ));
   }
}

#[derive(Event)]
struct ItemGet {
    player: Entity,
    item: Item,
}

fn pick_up_item(
    mut commands: Commands,
    player: Query<(Entity, &GridLocation, &AnimateTranslation), (With<Player>, With<Inventory>)>,
    items: Query<(Entity, &GridLocation, &Item)>,
    mut event_writer: EventWriter<ItemGet>)
{
    let (player, &player_location, animation) = player.single();
    if !animation.timer.finished() {
        return;
    }

    for (entity, item_location, item) in items.iter() {
        if player_location == *item_location {
            commands.entity(entity).despawn();
            event_writer.send(ItemGet{player, item: *item});
        }
    }
}

fn add_item_to_inventory(
    mut player: Query<&mut Inventory, With<Player>>,
    mut event_reader: EventReader<ItemGet>)
{
    for event in event_reader.iter() {
        let mut inventory = player.get_mut(event.player).unwrap();
        inventory.add(event.item);
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

const NUM_FUEL: usize = 2;

fn spawn_fuel(mut commands: Commands, asset_server: Res<AssetServer>) {
    let mut rng = rand::thread_rng();
    for _ in 0..NUM_FUEL {
        commands.spawn((
            Item::Fuel,
            GridLocation (IVec2 {x: rng.gen_range(0..MAX_X), y: rng.gen_range(0..MAX_Y)}),
            SpriteBundle {
                texture: asset_server.load("fuel.png"),
                sprite: Sprite {
                    custom_size: Some(Vec2::splat(64.)),
                    ..default()
                },
                ..default()
            },
            DistributeOnGrid,
            DespawnOnExitGameOver,
        ));
    }
}

#[derive(Component)]
struct FuelDisplay;

#[derive(Component)]
struct ScoreDisplay;

fn spawn_ui(mut commands: Commands) {
    commands.spawn((
        NodeBundle{
            style: Style {
                width:Val::Percent(100.),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::FlexStart,
                ..default()
            },
            ..default()
        },
        DespawnOnExitPlaying,
    )).with_children(|parent|{
        parent.spawn((
            ScoreDisplay,
            TextBundle::from_section("Score: 0", TextStyle {font_size: 50., ..default()}),
        ));
        parent.spawn((
            FuelDisplay,
            TextBundle::from_section("Fuel: 0", TextStyle {font_size: 50., ..default()}),
        ));
    });
}

fn update_score_display(
    player: Query<&Inventory, (With<Player>, Changed<Inventory>)>,
    mut display: Query<&mut Text, With<ScoreDisplay>>
) {
    if player.is_empty() {
        return;
    }
    
    let player = player.single();
    for mut text in display.iter_mut() {
        text.sections[0].value = format!("Score: {}", player.candies);
    }
}

fn update_fuel_display(
    player: Query<&Inventory, (With<Player>, Changed<Inventory>)>,
    mut display: Query<&mut Text, With<FuelDisplay>>
) {
    if player.is_empty() {
        return;
    }

    let player = player.single();
    for mut text in display.iter_mut() {
        text.sections[0].value = format!("Fuel: {}", player.fuel);
    }
}

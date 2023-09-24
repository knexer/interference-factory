use std::collections::HashMap;
use std::time::Duration;

use bevy::prelude::*;
use bevy::sprite::{MaterialMesh2dBundle, Mesh2dHandle};
use rand::Rng;

use crate::inventory::{Inventory, Item};
use crate::{AppState, DespawnOnExitGameOver, Player, MAX_X, MAX_Y, SootSprite, LoopCounter, GRID_SPACING};
use crate::grid::{GridLocation, AnimateTranslation, SnapToGrid};


#[derive(SystemSet, Hash, Debug, Clone, Eq, PartialEq)]
pub struct SpawnLevel;

pub struct SpawnLevelPlugin;

impl Plugin for SpawnLevelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Playing),
            (
                reset_level,
                (
                    spawn_player,
                    spawn_past_self,
                    spawn_grid,
                    add_candies_to_level,
                    add_fuel_to_level,
                ),
                spawn_level,
                apply_deferred,
                distribute_on_grid,
            ).in_set(SpawnLevel).chain())
            .insert_resource::<Level>(default());
    }
}

fn spawn_grid(mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>
) {
    let mesh: Mesh2dHandle = meshes.add(Mesh::from(shape::Quad::default())).into();
    let material = materials.add(ColorMaterial::from(Color::PURPLE));
    let make_grid_item = |x: i32, y:i32| {
        let grid_location = GridLocation(IVec2 {x, y});
        let size: Vec3 = Vec3::splat(128.);
        (
            grid_location.clone(),
            SnapToGrid,
            MaterialMesh2dBundle {
                mesh: mesh.clone(),
                transform: Transform::default().with_scale(size),
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

fn spawn_player(mut commands: Commands, asset_server: Res<AssetServer>) {
    let grid_location = GridLocation(IVec2 {x: 0, y: MAX_Y - 1});
    let make_finished_timer = |duration: Duration| {
        let mut timer = Timer::new(duration, TimerMode::Once);
        timer.tick(duration);
        timer
    };

    commands.spawn((
        Player,
        SootSprite{loop_number: 0},
        grid_location,
        Inventory{candies: 0, fuel: 0},
        SpriteBundle {
            texture: asset_server.load("soot-sprite.png"),
            ..default()
        },
        SnapToGrid,
        AnimateTranslation{
            start: default(),
            end: default(),
            timer: make_finished_timer(Duration::from_millis(200)),
            ease: CubicSegment::new_bezier(Vec2::new(0., 0.), Vec2::new(0.4, 1.5)),
        },
        DespawnOnExitGameOver,
    ));
}

fn spawn_past_self(mut commands: Commands, asset_server: Res<AssetServer>, loop_counter: Res<LoopCounter>) {
    if loop_counter.0 != 1 {
        return;
    }

    let grid_location = GridLocation(IVec2 {x: 0, y: MAX_Y - 1});
    let make_finished_timer = |duration: Duration| {
        let mut timer = Timer::new(duration, TimerMode::Once);
        timer.tick(duration);
        timer
    };

    commands.spawn((
        SootSprite{loop_number: 1},
        grid_location,
        Inventory{candies: 0, fuel: 0},
        SpriteBundle {
            texture: asset_server.load("soot-sprite.png"),
            sprite: Sprite {
                color: Color::rgba(0.6, 0.6, 0.6, 0.6),
                ..default()
            },
            ..default()
        },
        SnapToGrid,
        AnimateTranslation{
            start: default(),
            end: default(),
            timer: make_finished_timer(Duration::from_millis(200)),
            ease: CubicSegment::new_bezier(Vec2::new(0., 0.), Vec2::new(0.4, 1.5)),
        },
        DespawnOnExitGameOver,
    ));
}

trait BundleBox {
    fn apply_bundle(&self, commands: &mut Commands);
}
impl<T: Bundle + Clone> BundleBox for T {
    fn apply_bundle(&self, commands: &mut Commands) {
        commands.spawn(self.clone());
    }
}

#[derive(Resource, Default)]
struct Level {
    spawn: Vec<Box<dyn BundleBox + Send + Sync>>,
}

const NUM_CANDIES: usize = 10;

fn add_candies_to_level(mut level: ResMut<Level>, loop_counter: Res<LoopCounter>, asset_server: Res<AssetServer>) {
    if loop_counter.0 != 0 {
        return;
    }

    // TODO: Don't spawn candies on the start space.
    let mut rng = rand::thread_rng();
    for _ in 0..NUM_CANDIES {
        let color =  match rng.gen_range(0..3) {
            0 => "red-candy.png",
            1 => "green-candy.png",
            2 => "yellow-candy.png",
            _ => unreachable!(),
        };
        let bundle = (
            Item::Candy,
            GridLocation (IVec2 {x: rng.gen_range(0..MAX_X), y: rng.gen_range(0..MAX_Y)}),
            SpriteBundle {
                texture: asset_server.load(color),
                sprite: Sprite {
                    custom_size: Some(Vec2::splat(64.)),
                    ..default()
                },
                ..default()
            },
            DistributeOnGrid,
            DespawnOnExitGameOver,
        );

        level.spawn.push(Box::new(bundle));
    }
}

const NUM_FUEL: usize = 2;

fn add_fuel_to_level(mut level: ResMut<Level>, loop_counter: Res<LoopCounter>, asset_server: Res<AssetServer>) {
    if loop_counter.0 != 0 {
        return;
    }

    // TODO: Don't spawn fuel on the start space.
    // TODO: Don't spawn fuel on the end space.
    let mut rng = rand::thread_rng();
    for _ in 0..NUM_FUEL {
        let bundle = (
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
        );

        level.spawn.push(Box::new(bundle));
    }
}

fn reset_level(mut level: ResMut<Level>, loop_counter: Res<LoopCounter>) {
    if loop_counter.0 != 0 {
        return;
    }

    level.spawn.clear();
}

fn spawn_level(mut commands: Commands, level: Res<Level>) {
    for spawn in level.spawn.iter() {
        spawn.apply_bundle(&mut commands);
    }
}

#[derive(Component, Clone, Copy)]
pub struct DistributeOnGrid;

fn distribute_on_grid(mut query: Query<(&mut Transform, &GridLocation), With<DistributeOnGrid>>) {
    // Group by location.
    let mut transforms_per_location = query.iter_mut().fold(HashMap::new(),
        |mut map, (transform, grid_location)| {
            map.entry(grid_location).or_insert(vec![]).push(transform);
            map
        });

    for (grid_location, entities) in transforms_per_location.iter_mut() {
        let center: Vec2 = (grid_location.0 * GRID_SPACING).as_vec2();
        let count = entities.len() as i32;
        match count {
            1 => {
                let transform = entities.first_mut().unwrap();
                transform.translation = center.extend(0.);
            },
            _ => {
                // Arrange the entities radially around the center.
                let angle = 2. * std::f32::consts::PI / count as f32;
                let initial_angle = if count % 2 == 0 { angle / 2. } else { 0. };
                for (i, transform) in entities.iter_mut().enumerate() {
                    let radial_vector = Vec2 {
                        x: GRID_SPACING as f32 / 4. * (i as f32 * angle + initial_angle).cos(),
                        y: GRID_SPACING as f32 / 4. * (i as f32 * angle + initial_angle).sin()
                    };
                    transform.translation = (center + radial_vector).extend(0.);
                    transform.scale = Vec3::splat(0.7);
                }
            },
        }
    }
}

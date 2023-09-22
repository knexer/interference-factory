use std::time::Duration;

use bevy::prelude::*;
use bevy::sprite::{MaterialMesh2dBundle, Mesh2dHandle};
use rand::Rng;

use crate::inventory::{Inventory, Item};
use crate::{AppState, DespawnOnExitGameOver, Player, MAX_X, MAX_Y};
use crate::grid::{GridLocation, AnimateTranslation, SnapToGrid, center_of, DistributeOnGrid};


#[derive(SystemSet, Hash, Debug, Clone, Eq, PartialEq)]
pub struct SpawnLevel;

pub struct SpawnLevelPlugin;

impl Plugin for SpawnLevelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Playing),
            (
                (
                    spawn_player,
                    spawn_grid,
                    spawn_candies,
                    spawn_fuel,
                ),
                apply_deferred
            ).in_set(SpawnLevel).chain());
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
            MaterialMesh2dBundle {
                mesh: mesh.clone(),
                transform: Transform::default()
                    .with_scale(size)
                    .with_translation(center_of(&grid_location).extend(0.)),
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

const NUM_CANDIES: usize = 10;

fn spawn_candies(mut commands: Commands, asset_server: Res<AssetServer>) {
   let mut rng = rand::thread_rng();
   for _ in 0..NUM_CANDIES {
       let color =  match rng.gen_range(0..3) {
           0 => "red-candy.png",
           1 => "green-candy.png",
           2 => "yellow-candy.png",
           _ => unreachable!(),
       };
       commands.spawn((
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
       ));
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


use std::{collections::HashMap, time::Duration};
use bevy::prelude::*;

use crate::{AppState, GRID_SPACING};

#[derive(SystemSet, Hash, Debug, Clone, Eq, PartialEq)]
pub struct ApplyGridMovement;

pub struct GridPlugin;

impl Plugin for GridPlugin {
    fn build (&self, app: &mut App) {
        app
        .add_systems(OnEnter(AppState::Playing), distribute_on_grid.in_set(ApplyGridMovement))
        .add_systems(Update, (
            snap_to_grid,
            apply_deferred,
            animate_translation,
            apply_deferred,
        ).in_set(ApplyGridMovement).chain().run_if(in_state(AppState::Playing)));
    }
}

#[derive(Component, PartialEq, Eq, Hash, Copy, Clone, Debug, Deref, DerefMut)]
pub struct GridLocation(pub IVec2);

#[derive(Component)]
pub struct SnapToGrid {
    pub animate: Option<(Duration, CubicSegment<Vec2>)>,
}

pub fn center_of(grid_location: &GridLocation) -> Vec2 {
    Vec2::new((grid_location.x * GRID_SPACING) as f32, (grid_location.y * GRID_SPACING) as f32)
}

fn snap_to_grid(mut commands: Commands, mut query: Query<(Entity, &mut Transform, &GridLocation, &SnapToGrid), Changed<GridLocation>>) {
    for (entity, mut transform, grid_location, snap) in query.iter_mut() {
        let destination = Vec2::new((grid_location.x * GRID_SPACING) as f32, (grid_location.y * GRID_SPACING) as f32);
        match &snap.animate {
            Some((duration, ease)) => {
                // Add a component to animate.
                commands.entity(entity).insert(AnimateTranslation {
                    start: transform.translation.truncate(),
                    end: destination,
                    timer: Timer::new(*duration, TimerMode::Once),
                    ease: ease.clone()
                });
            },
            None => {
                transform.translation = destination.extend(0.);
            },
        }
    }
}

#[derive(Component)]
pub struct DistributeOnGrid;

// TODO: how to make this only run when the grid location changes?
// It's not as simple as naively using Changed<GridLocation>, because we also need all other entities with the same grid location to be updated.
// Ref<GridLocation> might be useful here, as it gives us the location plus whether it changed.

// I think this would work:
// We could have our query get RefMut<GridLocation>. 
// First collect a set of grid locations with changes,
// and then filter the query by that set.
// At that point we can proceed just as below.
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

#[derive(Component)]
pub struct AnimateTranslation {
    pub start: Vec2,
    pub end: Vec2,
    pub timer: Timer,
    pub ease: CubicSegment<Vec2>
}

fn animate_translation(mut commands: Commands, time: Res<Time>, mut query: Query<(Entity, &mut Transform, &mut AnimateTranslation)>) {
    for (entity, mut transform, mut animate_translation) in query.iter_mut() {
        if animate_translation.timer.tick(time.delta()).just_finished() {
            transform.translation = animate_translation.end.extend(0.);
            commands.entity(entity).remove::<AnimateTranslation>();
        } else {
            let progress = animate_translation.timer.percent();
            let lerp = animate_translation.ease.ease(progress);
            transform.translation = animate_translation.start.lerp(animate_translation.end, lerp).extend(0.);
        }
    }
}

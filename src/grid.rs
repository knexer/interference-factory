use bevy::prelude::*;

use crate::{AppState, GRID_SPACING};

#[derive(SystemSet, Hash, Debug, Clone, Eq, PartialEq)]
pub struct ApplyGridMovement;

pub struct GridPlugin;

impl Plugin for GridPlugin {
    fn build (&self, app: &mut App) {
        app
        .add_event::<MovementComplete>()
        .add_systems(Update, (
            snap_to_grid,
            animate_translation,
        ).in_set(ApplyGridMovement).chain().run_if(in_state(AppState::Playing)));
    }
}

#[derive(Component, PartialEq, Eq, Hash, Copy, Clone, Debug, Deref, DerefMut)]
pub struct GridLocation(pub IVec2);

#[derive(Component)]
pub struct SnapToGrid;

#[derive(Event)]
pub struct MovementComplete {
    pub entity: Entity,
}

fn center_of(grid_location: &GridLocation) -> Vec2 {
    Vec2::new((grid_location.x * GRID_SPACING) as f32, (grid_location.y * GRID_SPACING) as f32)
}

fn snap_to_grid(
    mut query: Query<(&mut Transform, Option<&mut AnimateTranslation>, Ref<GridLocation>),
    (With<SnapToGrid>, Changed<GridLocation>)>
) {
    for (mut transform, animate_transform, grid_location) in query.iter_mut() {
        let destination = center_of(&grid_location);
        // Insta-snap newly added components.
        if grid_location.is_added() {
            transform.translation = destination.extend(0.);
            continue;
        }

        match animate_transform {
            Some(mut animate_transform) => {
                animate_transform.start = transform.translation.truncate();
                animate_transform.end = destination;
                animate_transform.timer.reset();
            },
            None => {
                transform.translation = destination.extend(0.);
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

fn animate_translation(
    time: Res<Time>,
    mut event_writer: EventWriter<MovementComplete>,
    mut query: Query<(Entity, &mut Transform, &mut AnimateTranslation)>
) {
    for (entity, mut transform, mut animate_translation) in query.iter_mut() {
        if animate_translation.timer.finished() {
            continue;
        }

        if animate_translation.timer.tick(time.delta()).just_finished() {
            transform.translation = animate_translation.end.extend(0.);
            event_writer.send(MovementComplete{entity});
        } else {
            let progress = animate_translation.timer.percent();
            let lerp = animate_translation.ease.ease(progress);
            transform.translation = animate_translation.start.lerp(animate_translation.end, lerp).extend(0.);
        }
    }
}

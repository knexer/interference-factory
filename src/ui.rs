use bevy::prelude::*;

use crate::{AppState, DespawnOnExitPlaying, Player};
use crate::inventory::Inventory;


#[derive(SystemSet, Hash, Debug, Clone, Eq, PartialEq)]
pub struct UpdateUi;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(AppState::Playing), spawn_ui)
            .add_systems(Update, (
                update_score_display,
                update_fuel_display,
            ).in_set(UpdateUi).chain().run_if(in_state(AppState::Playing)));
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

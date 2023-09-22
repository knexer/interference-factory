use bevy::prelude::*;

use crate::{AppState, DespawnOnExitGameOver, Player};

use crate::inventory::Inventory;

pub struct GameOverScreenPlugin;

impl Plugin for GameOverScreenPlugin {
    fn build (&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::GameOver), spawn_game_over_screen)
           .add_systems(Update, update_game_over_screen.run_if(in_state(AppState::GameOver)));
    }
}

const NORMAL_BUTTON: Color = Color::rgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::rgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::rgb(0.35, 0.75, 0.35);

fn spawn_game_over_screen(mut commands: Commands, inventory: Query<&Inventory, With<Player>>) {
    let inventory = inventory.single();
    commands.spawn((
        NodeBundle {
            style: Style {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            background_color: Color::rgba(0., 0., 0., 0.7).into(),
            ..default()
        },
        DespawnOnExitGameOver,
    )).with_children(|parent| {
        parent.spawn(TextBundle::from_section(
            format!("Game over! Score: {}", inventory.candies),
            TextStyle {font_size: 50., ..default()}));
        parent.spawn(ButtonBundle{
            style: Style {
                width: Val::Px(150.),
                height: Val::Px(65.),
                // horizontally center child text
                justify_content: JustifyContent::Center,
                // vertically center child text
                align_items: AlignItems::Center,
                ..default()
            },
            background_color: NORMAL_BUTTON.into(),
            ..default()}).with_children(|parent| {
                parent.spawn(TextBundle::from_section("Restart", TextStyle::default()));
        });
    });
}

fn update_game_over_screen(
    mut next_state: ResMut<NextState<AppState>>,
    mut interaction_query: Query<(&Interaction, &mut BackgroundColor), With<Button>>
) {
    for (interaction, mut color) in interaction_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                next_state.set(AppState::Playing);
                *color = PRESSED_BUTTON.into();
            }
            Interaction::Hovered => {
                *color = HOVERED_BUTTON.into();
            },
            Interaction::None => {
                *color = NORMAL_BUTTON.into();
            },
        }
    }
}

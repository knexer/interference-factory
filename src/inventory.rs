use bevy::prelude::*;

use crate::{Player, AppState};
use crate::grid::{AnimateTranslation, GridLocation};

#[derive(SystemSet, Hash, Debug, Clone, Eq, PartialEq)]
pub struct PickUpItems;

pub struct InventoryPlugin;

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            pick_up_item,
            add_item_to_inventory,
        ).in_set(PickUpItems).chain().run_if(in_state(AppState::Playing)))
        .add_event::<ItemGet>();
    }
}

#[derive(Component, Clone, Copy)]
pub enum Item {
    Candy,
    Fuel,
}

#[derive(Component, Clone, Copy)]
pub struct Inventory {
    pub candies: i32,
    pub fuel: i32,
}

impl Inventory {
    fn add(&mut self, item: Item) {
        match item {
            Item::Candy => self.candies += 1,
            Item::Fuel => self.fuel += 1,
        }
    }
}

#[derive(Event)]
pub struct ItemGet {
    pub player: Entity,
    pub item: Item,
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

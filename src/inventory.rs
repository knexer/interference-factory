use bevy::prelude::*;

use crate::{AppState, SootSprite};
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
    pub soot: Entity,
    pub item: Item,
}

fn pick_up_item(
    mut commands: Commands,
    soot_sprites: Query<(Entity, &GridLocation, &AnimateTranslation), (With<SootSprite>, With<Inventory>)>,
    items: Query<(Entity, &GridLocation, &Item)>,
    mut event_writer: EventWriter<ItemGet>)
{
    for (soot, &soot_location, animation) in soot_sprites.iter() {
        if !animation.timer.finished() {
            continue;
        }
        for (entity, item_location, item) in items.iter() {
            if soot_location == *item_location {
                commands.entity(entity).despawn();
                event_writer.send(ItemGet{soot, item: *item});
            }
        }
    }
}

fn add_item_to_inventory(
    mut soot: Query<&mut Inventory, With<SootSprite>>,
    mut event_reader: EventReader<ItemGet>)
{
    for event in event_reader.iter() {
        let mut inventory = soot.get_mut(event.soot).unwrap();
        inventory.add(event.item);
    }
}

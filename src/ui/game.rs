mod client_state;
mod game_ui;
use crate::data::wizard::Wizard;
use crate::error::ChaosError;
use crate::gfx::color::Color::*;
use crate::net::ChaosClient;
use crate::net::Message;
use crate::window::Window;
use client_state::ClientState;
use game_ui::GameUI;

pub fn game(win: &mut Window, client: &mut ChaosClient, wizard: Wizard) -> Result<(), ChaosError> {
    let state = &mut ClientState::new(wizard);
    let ui = &mut GameUI::new(win, state);
    loop {
        if let Some((id, msg)) = client.recv()? {
            match msg {
                Message::Shutdown => return Ok(()),
                Message::AddWizard { wizard, x, y } => {
                    ui.panel.add_wizard(id, &wizard.name);
                    state.arena.get_mut(x, y).wizard = Some(wizard);
                    state.turns_left = state.arena.number_of_wizards() * 2 + 15;
                }
                Message::ChooseSpell => {
                    ui.set_status(win, "CHOOSE A SPELL", BrightYellow);
                    let spell_id = ui.choose_spell(win, state)?;
                    if let Some((id, _)) = spell_id {
                        if id != 0 {
                            state.wizard.spells.remove(id as usize);
                            ui.update_spells(win, state);
                        }
                    }
                    ui.clear_status(win);
                    client.send(Message::ChosenSpell(spell_id))?;
                }
                Message::WaitingForOtherPlayers(0) => {
                    ui.clear_status(win);
                }
                Message::WaitingForOtherPlayers(count) => {
                    let text = if count == 1 {
                        format!("WAITING FOR {} OTHER PLAYER", count)
                    } else {
                        format!("WAITING FOR {} OTHER PLAYERS", count)
                    };
                    ui.set_status(win, &text, BrightYellow);
                }
                Message::CastSpell { spell_name, range } => {
                    ui.spell_cast_info(win, state, id, spell_name, range)?;
                }
                Message::DeBuffWizard(stats) => {
                    state.arena.find_wizard_mut(id).update_stats(stats);
                }
                Message::BuffWizard(stats) => {
                    let (x, y) = state.arena.find_wizard_pos(id);
                    ui.twirl(win, state, x, y)?;
                    state.arena.find_wizard_mut(id).update_stats(stats);
                }
                Message::ChoosePiece(tiles) => {
                    let name = &state.arena.find_wizard(id).name;
                    let text = format!("{name}'S TURN");
                    ui.set_status(win, &text, BrightYellow);
                    let tile_id = ui.choose_tile(win, state, tiles, BrightYellow)?;
                    client.send(Message::ChosenTile(tile_id))?;
                    ui.clear_status(win);
                }
                Message::ChooseTarget(tiles) => {
                    ui.set_status(win, "CHOOSE A TARGET", BrightYellow);
                    let tile_id = ui.choose_tile(win, state, tiles, BrightCyan)?;
                    client.send(Message::ChosenTile(tile_id))?;
                    ui.clear_status(win);
                }
                Message::EngagedInCombat(tiles) => {
                    ui.set_status(win, "ENGAGED TO ENEMY", BrightYellow);
                    let tile_id = ui.choose_tile(win, state, tiles, BrightRed)?;
                    client.send(Message::ChosenTile(tile_id))?;
                    ui.clear_status(win);
                }
                Message::ChooseRangedCombat { range, tiles } => {
                    ui.border(win, BrightMagenta);
                    let content = [("RANGED COMBAT,RANGE=", BrightGreen), (&range.to_string(), BrightYellow)];
                    ui.multi_color_status(win, &content);
                    let tile_id = ui.choose_tile(win, state, tiles, BrightMagenta)?;
                    client.send(Message::ChosenTile(tile_id))?;
                    ui.border(win, BrightBlue);
                    ui.clear_status(win);
                }
                Message::MovementRange { range, flying, tiles } => {
                    let content = [
                        ("MOVEMENT RANGE=", BrightGreen),
                        (&range.to_string(), BrightYellow),
                        if flying {
                            ("  (FLYING)", BrightCyan)
                        } else {
                            ("", BrightCyan)
                        },
                    ];
                    ui.multi_color_status(win, &content);
                    let tile_id = ui.choose_tile(win, state, tiles, BrightCyan)?;
                    client.send(Message::ChosenTile(tile_id))?;
                    ui.clear_status(win);
                }
                Message::MovementPoints { points, tiles } => {
                    let content = [("MOVEMENT POINTS LEFT=", BrightGreen), (&points.to_string(), BrightYellow)];
                    ui.multi_color_status(win, &content);
                    let tile_id = ui.choose_tile(win, state, tiles, BrightCyan)?;
                    client.send(Message::ChosenTile(tile_id))?;
                    ui.clear_status(win);
                }
                Message::UndeadCannotBeAttacked => {
                    ui.set_status(win, "UNDEAD-CANNOT BE ATTACKED", BrightCyan);
                    ui.wait_for(win, state, 400)?;
                    ui.clear_status(win);
                }
                Message::SuccessfulAttack { x, y, corpse } => {
                    ui.attack(win, state, x, y)?;
                    let tile = state.arena.get(x, y).clone();
                    if tile.spawn.is_some() {
                        state.arena.remove_spawn(x, y);
                    } else if let Some(creation) = tile.creation {
                        if creation.stats.magic_wood && tile.wizard.is_some() {
                            let wizard_id = state.arena.get_wizard(x, y).id;
                            ui.wizard_death(win, state, wizard_id)?;
                        } else {
                            state.arena.kill_creation(x, y, corpse);
                        }
                    } else if let Some(wizard) = tile.wizard {
                        ui.wizard_death(win, state, wizard.id)?;
                    } else {
                        unreachable!();
                    }
                    ui.wait_for_frames(win, state, 4)?;
                }
                Message::FailedAttack { x, y } => {
                    ui.attack(win, state, x, y)?;
                    ui.wait_for_frames(win, state, 4)?;
                }
                Message::SuccessfulRangedAttack {
                    sx,
                    sy,
                    dx,
                    dy,
                    corpse,
                    color,
                } => {
                    ui.ranged_attack(win, state, sx, sy, dx, dy, color)?;
                    let tile = state.arena.get(dx, dy).clone();
                    if tile.spawn.is_some() {
                        state.arena.remove_spawn(dx, dy);
                    } else if let Some(creation) = tile.creation {
                        if creation.stats.magic_wood && tile.wizard.is_some() {
                            let wizard_id = state.arena.get_wizard(dx, dy).id;
                            ui.wizard_death(win, state, wizard_id)?;
                        } else {
                            state.arena.kill_creation(dx, dy, corpse);
                        }
                    } else if let Some(wizard) = tile.wizard {
                        ui.wizard_death(win, state, wizard.id)?;
                    } else {
                        unreachable!();
                    }
                    ui.wait_for_frames(win, state, 4)?;
                }
                Message::SuccessfulDragonRangedAttack { sx, sy, dx, dy } => {
                    ui.dragon_ranged_attack(win, state, sx, sy, dx, dy)?;
                    let tile = state.arena.get(dx, dy).clone();
                    if tile.spawn.is_some() {
                        state.arena.remove_spawn(dx, dy);
                    } else if let Some(creation) = tile.creation {
                        if creation.stats.magic_wood && tile.wizard.is_some() {
                            let wizard_id = state.arena.get_wizard(dx, dy).id;
                            ui.wizard_death(win, state, wizard_id)?;
                        } else {
                            state.arena.kill_creation(dx, dy, false);
                        }
                    } else if let Some(wizard) = tile.wizard {
                        ui.wizard_death(win, state, wizard.id)?;
                    } else {
                        unreachable!();
                    }
                    ui.wait_for_frames(win, state, 4)?;
                }
                Message::FailedDragonRangedAttack { sx, sy, dx, dy } => {
                    ui.dragon_ranged_attack(win, state, sx, sy, dx, dy)?;
                    ui.wait_for_frames(win, state, 4)?;
                }
                Message::FailedRangedAttack { sx, sy, dx, dy, color } => {
                    ui.ranged_attack(win, state, sx, sy, dx, dy, color)?;
                    ui.wait_for_frames(win, state, 4)?;
                }
                Message::SpellSucceeds(alignment) => {
                    state.arena.alignment = alignment;
                    ui.update_alignment(win, state);
                    ui.update_spells(win, state);
                    ui.set_status(win, "SPELL SUCCEEDS", BrightWhite);
                    ui.wait_for(win, state, 800)?;
                    ui.clear_status(win);
                }
                Message::SpellFails => {
                    ui.set_status(win, "SPELL FAILS", BrightMagenta);
                    ui.wait_for(win, state, 800)?;
                    ui.clear_status(win);
                }
                Message::CreationSpell { x, y, creation } => {
                    let (sx, sy) = state.arena.find_wizard_pos(id);
                    ui.spell_ray(win, state, sx, sy, x, y)?;
                    ui.twirl(win, state, x, y)?;
                    if let Some(creation) = creation {
                        state.arena.get_mut(x, y).creation = Some(creation);
                    }
                }
                Message::CastFire { x, y, fire } => {
                    let (sx, sy) = state.arena.find_wizard_pos(id);
                    ui.spell_ray(win, state, sx, sy, x, y)?;
                    ui.twirl(win, state, x, y)?;
                    if let Some(fire) = fire {
                        state.arena.spawn_fire(x, y, fire);
                    }
                }
                Message::CastBlob { x, y, blob } => {
                    let (sx, sy) = state.arena.find_wizard_pos(id);
                    ui.spell_ray(win, state, sx, sy, x, y)?;
                    ui.twirl(win, state, x, y)?;
                    if let Some(blob) = blob {
                        state.arena.spawn_blob(x, y, blob);
                    }
                }
                Message::SendSpell { spell } => {
                    state.wizard.spells.push(spell);
                    ui.update_spells(win, state);
                }
                Message::NewSpell { x, y } => {
                    state.arena.get_mut(x, y).creation = None;
                    ui.new_spell(win, state, id)?;
                }
                Message::ShadowWoodInfo => {
                    ui.set_status(win, "CANNOT BE PLACED TOGETHER", BrightCyan);
                    ui.wait_for(win, state, 800)?;
                    ui.clear_status(win);
                }
                Message::NoPossibleMoves => {
                    ui.set_status(win, "NO POSSIBLE MOVES", BrightCyan);
                    ui.wait_for(win, state, 800)?;
                    ui.clear_status(win);
                }
                Message::Disbelieve { x, y, success } => {
                    let (sx, sy) = state.arena.find_wizard_pos(id);
                    ui.spell_ray(win, state, sx, sy, x, y)?;
                    ui.twirl(win, state, x, y)?;
                    if success {
                        ui.explosion(win, state, x, y)?;
                        state.arena.get_mut(x, y).creation = None;
                    }
                }
                Message::Turn => {
                    let name = &state.arena.find_wizard(id).name;
                    let text = format!("{name}'S TURN");
                    ui.set_status(win, &text, BrightYellow);
                }
                Message::TurnEnd => {
                    ui.clear_status(win);
                    state.turns_left -= 1;
                }
                Message::MoveWizard { x, y } => {
                    state.arena.move_wizard(id, x, y);
                }
                Message::MoveCreation { sx, sy, dx, dy } => {
                    state.arena.move_creation(sx, sy, dx, dy);
                }
                Message::AskForDismount => {
                    ui.set_status(win, "DISMOUNT WIZARD? (Y OR N)", BrightWhite);
                    let dismount = ui.ask_for_dismount(win, state)?;
                    client.send(Message::Dismount(dismount))?;
                    ui.clear_status(win);
                }
                Message::NoLineOfSight => {
                    ui.set_status(win, "NO LINE OF SIGHT", BrightCyan);
                    ui.wait_for(win, state, 400)?;
                    ui.clear_status(win);
                }
                Message::Subversion { x, y, success } => {
                    let (sx, sy) = state.arena.find_wizard_pos(id);
                    ui.spell_ray(win, state, sx, sy, x, y)?;
                    ui.twirl(win, state, x, y)?;
                    if success {
                        state.arena.subvert(x, y, id);
                    }
                }
                Message::RaiseDead { x, y, success } => {
                    let (sx, sy) = state.arena.find_wizard_pos(id);
                    ui.spell_ray(win, state, sx, sy, x, y)?;
                    ui.twirl(win, state, x, y)?;
                    if success {
                        state.arena.raise_dead(x, y, id);
                    }
                }
                Message::MagicBolt { x, y, success } => {
                    let (sx, sy) = state.arena.find_wizard_pos(id);
                    ui.magic_bolt(win, state, sx, sy, x, y)?;
                    if success {
                        ui.explosion(win, state, x, y)?;
                        let tile = state.arena.get(x, y);
                        if tile.creation.is_some() {
                            state.arena.kill_creation(x, y, false);
                        } else if let Some(ref wizard) = tile.wizard {
                            ui.wizard_death(win, state, wizard.id)?;
                        } else {
                            unreachable!();
                        }
                    }
                }
                Message::Lightning { x, y, success } => {
                    let (sx, sy) = state.arena.find_wizard_pos(id);
                    ui.lightning(win, state, sx, sy, x, y)?;
                    if success {
                        ui.explosion(win, state, x, y)?;
                        let tile = state.arena.get(x, y);
                        if tile.creation.is_some() {
                            state.arena.kill_creation(x, y, false);
                        } else if let Some(ref wizard) = tile.wizard {
                            ui.wizard_death(win, state, wizard.id)?;
                        } else {
                            unreachable!();
                        }
                    }
                }
                Message::ShelterDisappears { x, y } => {
                    ui.explosion(win, state, x, y)?;
                    state.arena.kill_creation(x, y, false);
                }
                Message::Results(players) => {
                    ui.wait_for(win, state, 800)?;
                    ui.results(win, players)?;
                    return Ok(());
                }
                Message::MagicalAttack { x, y, success } => {
                    ui.flash_attack(win, state, x, y)?;
                    if success {
                        let tile = state.arena.get(x, y).clone();
                        if tile.creation.is_some() {
                            ui.explosion(win, state, x, y)?;
                            state.arena.kill_creation(x, y, false);
                        } else if let Some(wizard) = tile.wizard {
                            let coords = state.arena.get_topmost_creations_and_corpses_coords(wizard.id);
                            ui.explosions(win, state, coords)?;
                            state.arena.destroy_all_wizard_creations(wizard.id);
                        } else {
                            unreachable!();
                        }
                    }
                }
                Message::SpawnFire { x, y, fire } => {
                    if let Some(fire) = fire {
                        let tile = state.arena.get(x, y).clone();
                        if tile.creation.is_some() {
                            ui.attack(win, state, x, y)?;
                            ui.wait_for_frames(win, state, 4)?;
                            state.arena.kill_creation(x, y, false);
                            if tile.wizard.is_none() {
                                state.arena.spawn_fire(x, y, fire);
                            }
                        } else if let Some(wizard) = tile.wizard {
                            ui.attack(win, state, x, y)?;
                            ui.wait_for_frames(win, state, 4)?;
                            ui.wizard_death(win, state, wizard.id)?;
                            state.arena.spawn_fire(x, y, fire);
                        } else {
                            state.arena.spawn_fire(x, y, fire);
                        }
                    } else {
                        ui.attack(win, state, x, y)?;
                        ui.wait_for_frames(win, state, 4)?;
                    }
                }
                Message::SpawnBlob { x, y, blob } => {
                    if let Some(blob) = blob {
                        let tile = state.arena.get(x, y).clone();
                        if let Some(wizard) = tile.wizard {
                            ui.attack(win, state, x, y)?;
                            ui.wait_for_frames(win, state, 4)?;
                            ui.wizard_death(win, state, wizard.id)?;
                            state.arena.spawn_blob(x, y, blob);
                        } else {
                            state.arena.spawn_fire(x, y, blob);
                        }
                    } else {
                        ui.attack(win, state, x, y)?;
                        ui.wait_for_frames(win, state, 4)?;
                    }
                }
                Message::RemoveSpawn { x, y } => {
                    state.arena.remove_spawn(x, y);
                }
                _ => {}
            }
        }
        win.update()?;
        ui.render(win, state)?;
    }
}

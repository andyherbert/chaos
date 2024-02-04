use super::{sender::Sender, server_state::ServerState};
use crate::config::Player;
use crate::data::arena::{Arena, Spawn};
use crate::data::creation::GameCreation;
use crate::data::spells::{Spell, SpellKind};
use crate::data::stats::{AttackBuff, CreationStats, DefenceBuff};
use crate::data::wizard::{GameWizard, LobbyWizards};
use crate::error::ChaosError;
use crate::gfx::color::Color::*;
use crate::net::{Message, NetworkError, RecieveMsg, SendMsg};
use rand::SeedableRng;
use rand::{rngs::StdRng, seq::SliceRandom, Rng};
use std::collections::HashSet;
use tokio::select;
use tokio::sync::{mpsc, oneshot};

pub struct GameLogic {
    rx: mpsc::Receiver<RecieveMsg>,
    tx: Sender,
    quit_rx: oneshot::Receiver<()>,
}

impl GameLogic {
    pub fn new(rx: mpsc::Receiver<RecieveMsg>, tx: mpsc::Sender<SendMsg>, quit_rx: oneshot::Receiver<()>) -> Self {
        let tx = Sender::new(tx);
        Self { rx, tx, quit_rx }
    }

    pub async fn lobby_loop(&mut self) -> Result<Option<LobbyWizards>, NetworkError> {
        let mut wizards = LobbyWizards::new();
        loop {
            select! {
                _ = &mut self.quit_rx => {
                    self.tx.shutdown().await?;
                }
                Some(msg) = self.rx.recv() => {
                    match msg {
                        RecieveMsg::Connected { id } => {
                            self.tx.send_all_wizards_to(id, &wizards).await?;
                        }
                        RecieveMsg::Disconnected { id } => {
                            if wizards.leave(id).is_some() {
                                self.tx.leave(id).await?;
                            }
                        }
                        RecieveMsg::Message { id, msg } => {
                            match msg {
                                Message::Join(player) => {
                                    if wizards.join(id, player.clone()) {
                                        self.tx.join(id, &player).await?;
                                    }
                                }
                                Message::Ready(ready) => {
                                    if wizards.ready(id, ready) {
                                        self.tx.ready(id, ready).await?;
                                        if wizards.is_ready() {
                                            return Ok(Some(wizards));
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    async fn select_spells(&mut self, state: &mut ServerState) -> Result<Vec<(u32, Spell, bool)>, ChaosError> {
        let mut left_to_choose = HashSet::<u32>::from_iter(state.wizards.all_active_ids());
        self.tx.waiting_for_other_players(left_to_choose.len()).await?;
        let mut spells = Vec::with_capacity(left_to_choose.len());
        for id in left_to_choose.iter().copied() {
            self.tx.choose_spell(id).await?;
        }
        'spell_loop: loop {
            select! {
                _ = &mut self.quit_rx => {
                    self.tx.shutdown().await?;
                }
                Some(msg) = self.rx.recv() => {
                    match msg {
                        RecieveMsg::Disconnected { id } => {
                            if left_to_choose.remove(&id) {
                                self.tx.waiting_for_other_players(left_to_choose.len()).await?;
                                state.wizards.get_mut(id)?.disconnected = true;
                                if left_to_choose.is_empty() {
                                    break 'spell_loop;
                                }
                            }
                        }
                        RecieveMsg::Message { id, msg } => {
                            match msg {
                                Message::ChosenSpell(Some((0, _))) => {
                                    if left_to_choose.remove(&id) {
                                        self.tx.waiting_for_other_players(left_to_choose.len()).await?;
                                        let spell = state.wizards.get_mut(id)?.spells.first().expect("disbelieve");
                                        spells.push((id, spell.clone(), false));
                                        if left_to_choose.is_empty() {
                                            break 'spell_loop;
                                        }
                                    }
                                }
                                Message::ChosenSpell(Some((spell_id, illusion))) => {
                                    if left_to_choose.remove(&id) {
                                        self.tx.waiting_for_other_players(left_to_choose.len()).await?;
                                        let game_wizard = state.arena.find_wizard_mut(id);
                                        game_wizard.stats.number_of_spells -= 1;
                                        self.tx.debuff_wizard(id, &game_wizard.stats).await?;
                                        let spell = state.wizards.get_mut(id)?.spells.remove(spell_id as usize);
                                        spells.push((id, spell, illusion));
                                        if left_to_choose.is_empty() {
                                            break 'spell_loop;
                                        }
                                    }
                                }
                                Message::ChosenSpell(None) => {
                                    if left_to_choose.remove(&id) {
                                        self.tx.waiting_for_other_players(left_to_choose.len()).await?;
                                        if left_to_choose.is_empty() {
                                            break 'spell_loop;
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        spells.sort_by(|(id_a, _, _), (id_b, _, _)| id_a.cmp(id_b));
        Ok(spells)
    }

    async fn chosen_tile(
        &mut self,
        state: &mut ServerState,
        id: u32,
        tiles: Vec<(u8, u8)>,
    ) -> Result<Option<(u8, u8)>, NetworkError> {
        if state.wizards.has_disconnected(id)? {
            return Ok(None);
        }
        loop {
            select! {
                _ = &mut self.quit_rx => {
                    self.tx.shutdown().await?;
                }
                Some(msg) = self.rx.recv() => {
                    match msg {
                        RecieveMsg::Disconnected { id: disconnected_id } => {
                            state.wizards.get_mut(disconnected_id)?.disconnected = true;
                            if id == disconnected_id {
                                return Ok(None);
                            }
                        }
                        RecieveMsg::Message { id: msg_id, msg } => {
                            match msg {
                                Message::ChosenTile(tile_id) if msg_id == id => {
                                    match tile_id {
                                        Some(tile_id) => {
                                            if let Some((x, y)) = tiles.get(tile_id as usize).copied() {
                                                return Ok(Some((x, y)));
                                            }
                                        }
                                        None => {
                                            return Ok(None);
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    async fn do_spell(&mut self, state: &mut ServerState, id: u32, spell: Spell, illusion: bool) -> Result<(), ChaosError> {
        let alignment = state.arena.alignment;
        let wizard = state.arena.find_wizard_mut(id);
        let spell_ability = wizard.stats.spell_ability;
        self.tx.cast_spell(id, &spell).await?;
        match spell.kind {
            SpellKind::Disbelieve => {
                let (x, y) = state.arena.find_wizard_pos(id);
                let tiles = state.arena.cast_spell_on_attackable_tiles(x, y, spell.range, id);
                if tiles.is_empty() {
                    self.tx.no_possible_moves(id).await?;
                    return Ok(());
                }
                self.tx.choose_target(id, &tiles).await?;
                if let Some((x, y)) = self.chosen_tile(state, id, tiles).await? {
                    let tile = state.arena.get_mut(x, y);
                    if let Some(GameCreation { illusion: true, .. }) = tile.creation.as_mut() {
                        self.tx.disbelieve(id, x, y, true).await?;
                        tile.creation = None;
                        state.arena.adjust_alignment(spell.alignment);
                        self.tx.spell_succeeds(state.arena.alignment).await?;
                    } else {
                        self.tx.disbelieve(id, x, y, false).await?;
                        self.tx.spell_fails().await?;
                    }
                }
            }
            SpellKind::Creation(ref stats) => {
                let (sx, sy) = state.arena.find_wizard_pos(id);
                loop {
                    let tiles = state.arena.creation_spell_tiles(sx, sy, spell.range);
                    if tiles.is_empty() {
                        self.tx.no_possible_moves(id).await?;
                        return Ok(());
                    }
                    self.tx.choose_target(id, &tiles).await?;
                    if let Some((dx, dy)) = self.chosen_tile(state, id, tiles).await? {
                        if !state.arena.line_of_sight(sx, sy, dx, dy) {
                            self.tx.no_line_of_sight(id).await?;
                            continue;
                        }
                        if illusion || spell.cast(alignment, spell_ability) {
                            let mut creation = GameCreation::new(id, stats.clone());
                            self.tx.creation_spell(id, dx, dy, Some(&creation)).await?;
                            let tile = state.arena.get_mut(dx, dy);
                            creation.illusion = illusion;
                            tile.creation = Some(creation);
                            state.arena.adjust_alignment(spell.alignment);
                            self.tx.spell_succeeds(state.arena.alignment).await?;
                        } else {
                            self.tx.creation_spell(id, dx, dy, None).await?;
                            self.tx.spell_fails().await?;
                        }
                    }
                    return Ok(());
                }
            }
            SpellKind::MagicFire(ref stats) => {
                let (sx, sy) = state.arena.find_wizard_pos(id);
                loop {
                    let tiles = state.arena.creation_spell_tiles(sx, sy, spell.range);
                    if tiles.is_empty() {
                        self.tx.no_possible_moves(id).await?;
                        return Ok(());
                    }
                    self.tx.choose_target(id, &tiles).await?;
                    if let Some((dx, dy)) = self.chosen_tile(state, id, tiles).await? {
                        if !state.arena.line_of_sight(sx, sy, dx, dy) {
                            self.tx.no_line_of_sight(id).await?;
                            continue;
                        }
                        if spell.cast(alignment, spell_ability) {
                            let fire = GameCreation::new(id, stats.clone());
                            self.tx.cast_fire(id, dx, dy, Some(&fire)).await?;
                            state.arena.spawn_fire(dx, dy, fire);
                            state.arena.adjust_alignment(spell.alignment);
                            self.tx.spell_succeeds(state.arena.alignment).await?;
                        } else {
                            self.tx.cast_fire(id, dx, dy, None).await?;
                            self.tx.spell_fails().await?;
                        }
                    }
                    return Ok(());
                }
            }
            SpellKind::GooeyBlob(ref stats) => {
                let (sx, sy) = state.arena.find_wizard_pos(id);
                loop {
                    let tiles = state.arena.creation_spell_tiles(sx, sy, spell.range);
                    if tiles.is_empty() {
                        self.tx.no_possible_moves(id).await?;
                        return Ok(());
                    }
                    self.tx.choose_target(id, &tiles).await?;
                    if let Some((dx, dy)) = self.chosen_tile(state, id, tiles).await? {
                        if !state.arena.line_of_sight(sx, sy, dx, dy) {
                            self.tx.no_line_of_sight(id).await?;
                            continue;
                        }
                        if spell.cast(alignment, spell_ability) {
                            let blob = GameCreation::new(id, stats.clone());
                            self.tx.cast_blob(id, dx, dy, Some(&blob)).await?;
                            state.arena.spawn_blob(dx, dy, blob);
                            state.arena.adjust_alignment(spell.alignment);
                            self.tx.spell_succeeds(state.arena.alignment).await?;
                        } else {
                            self.tx.cast_blob(id, dx, dy, None).await?;
                            self.tx.spell_fails().await?;
                        }
                    }
                    return Ok(());
                }
            }
            SpellKind::MagicWood(ref stats) => {
                let (sx, sy) = state.arena.find_wizard_pos(id);
                let mut cast = false;
                let mut count = 0;
                let mut rng = StdRng::from_entropy();
                loop {
                    let mut tiles = state.arena.creation_spell_tiles(sx, sy, spell.range);
                    if tiles.is_empty() {
                        self.tx.no_possible_moves(id).await?;
                        return Ok(());
                    }
                    tiles.shuffle(&mut rng);
                    for (dx, dy) in tiles {
                        if state.arena.line_of_sight(sx, sy, dx, dy) {
                            if !cast && !spell.cast(alignment, spell_ability) {
                                self.tx.creation_spell(id, dx, dy, None).await?;
                                self.tx.spell_fails().await?;
                                return Ok(());
                            }
                            let wood = GameCreation::new(id, stats.clone());
                            self.tx.creation_spell(id, dx, dy, Some(&wood)).await?;
                            let tile = state.arena.get_mut(dx, dy);
                            tile.creation = Some(wood);
                            if !cast {
                                state.arena.adjust_alignment(spell.alignment);
                                self.tx.spell_succeeds(state.arena.alignment).await?;
                                cast = true;
                            }
                            count += 1;
                            if count == 8 {
                                return Ok(());
                            }
                        }
                    }
                }
            }
            SpellKind::ShadowWood(ref stats) => {
                let (sx, sy) = state.arena.find_wizard_pos(id);
                let mut count = 0;
                let mut cast = false;
                loop {
                    let tiles = state.arena.creation_spell_tiles(sx, sy, spell.range);
                    if tiles.is_empty() {
                        self.tx.no_possible_moves(id).await?;
                        return Ok(());
                    }
                    self.tx.choose_target(id, &tiles).await?;
                    if let Some((dx, dy)) = self.chosen_tile(state, id, tiles).await? {
                        if !state.arena.line_of_sight(sx, sy, dx, dy) {
                            self.tx.no_line_of_sight(id).await?;
                            continue;
                        }
                        if state.arena.is_next_to_shadow_wood(dx, dy) {
                            self.tx.shadow_wood_info(id).await?;
                            continue;
                        }
                        if !cast && !spell.cast(alignment, spell_ability) {
                            self.tx.creation_spell(id, dx, dy, None).await?;
                            self.tx.spell_fails().await?;
                            return Ok(());
                        }
                        let creation = GameCreation::new(id, stats.clone());
                        self.tx.creation_spell(id, dx, dy, Some(&creation)).await?;
                        let tile = state.arena.get_mut(dx, dy);
                        tile.creation = Some(creation);
                        if !cast {
                            state.arena.adjust_alignment(spell.alignment);
                            self.tx.spell_succeeds(state.arena.alignment).await?;
                            cast = true;
                        }
                        count += 1;
                        if count == 8 {
                            return Ok(());
                        }
                    } else {
                        return Ok(());
                    }
                }
            }
            SpellKind::Shelter(ref stats) => {
                let (sx, sy) = state.arena.find_wizard_pos(id);
                loop {
                    let tiles = state.arena.creation_spell_tiles(sx, sy, spell.range);
                    if tiles.is_empty() {
                        self.tx.no_possible_moves(id).await?;
                        return Ok(());
                    }
                    self.tx.choose_target(id, &tiles).await?;
                    if let Some((dx, dy)) = self.chosen_tile(state, id, tiles).await? {
                        if !state.arena.line_of_sight(sx, sy, dx, dy) {
                            self.tx.no_line_of_sight(id).await?;
                            continue;
                        }
                        if spell.cast(alignment, spell_ability) {
                            state.arena.adjust_alignment(spell.alignment);
                            self.tx.spell_succeeds(state.arena.alignment).await?;
                            let creation = GameCreation::new(id, stats.clone());
                            self.tx.creation_spell(id, dx, dy, Some(&creation)).await?;
                            state.arena.get_mut(dx, dy).creation = Some(creation);
                        } else {
                            self.tx.creation_spell(id, dx, dy, None).await?;
                            self.tx.spell_fails().await?;
                        }
                    }
                    return Ok(());
                }
            }
            SpellKind::Wall(ref stats) => {
                let (sx, sy) = state.arena.find_wizard_pos(id);
                let mut cast = false;
                let mut count = 0;
                loop {
                    let tiles = state.arena.creation_spell_tiles(sx, sy, spell.range);
                    if tiles.is_empty() {
                        self.tx.no_possible_moves(id).await?;
                        return Ok(());
                    }
                    self.tx.choose_target(id, &tiles).await?;
                    if let Some((dx, dy)) = self.chosen_tile(state, id, tiles).await? {
                        if !state.arena.line_of_sight(sx, sy, dx, dy) {
                            self.tx.no_line_of_sight(id).await?;
                            continue;
                        }
                        if !cast && !spell.cast(alignment, spell_ability) {
                            self.tx.creation_spell(id, dx, dy, None).await?;
                            self.tx.spell_fails().await?;
                            return Ok(());
                        }
                        let creation = GameCreation::new(id, stats.clone());
                        self.tx.creation_spell(id, dx, dy, Some(&creation)).await?;
                        state.arena.get_mut(dx, dy).creation = Some(creation);
                        count += 1;
                        if count == 4 {
                            return Ok(());
                        }
                        if !cast {
                            state.arena.adjust_alignment(spell.alignment);
                            self.tx.spell_succeeds(state.arena.alignment).await?;
                            cast = true;
                        }
                    } else {
                        return Ok(());
                    }
                }
            }
            SpellKind::MagicBolt => {
                let (sx, sy) = state.arena.find_wizard_pos(id);
                let spell_ability = state.arena.find_wizard(id).stats.spell_ability;
                if spell.cast(alignment, spell_ability) {
                    loop {
                        let tiles = state.arena.cast_spell_on_attackable_tiles(sx, sy, spell.range, id);
                        if tiles.is_empty() {
                            self.tx.no_possible_moves(id).await?;
                            return Ok(());
                        }
                        self.tx.choose_target(id, &tiles).await?;
                        if let Some((dx, dy)) = self.chosen_tile(state, id, tiles).await? {
                            if !state.arena.line_of_sight(sx, sy, dx, dy) {
                                self.tx.no_line_of_sight(id).await?;
                                continue;
                            }
                            let tile = state.arena.get_mut(dx, dy).clone();
                            if let Some(creation) = tile.creation {
                                if creation.defend_against_attack(3) {
                                    self.tx.magic_bolt(id, dx, dy, true).await?;
                                    state.arena.kill_creation(dx, dy, false);
                                } else {
                                    self.tx.magic_bolt(id, dx, dy, false).await?;
                                }
                            } else if let Some(wizard) = tile.wizard {
                                if wizard.defend_against_attack(3) {
                                    self.tx.magic_bolt(id, dx, dy, true).await?;
                                    state.arena.kill_wizard_and_creations(wizard.id);
                                    if state.wizards.check_for_winning_condition() {
                                        return Ok(());
                                    }
                                } else {
                                    self.tx.magic_bolt(id, dx, dy, false).await?;
                                }
                            }
                        }
                        return Ok(());
                    }
                } else {
                    self.tx.spell_fails().await?;
                }
            }
            SpellKind::Lightning => {
                let (sx, sy) = state.arena.find_wizard_pos(id);
                let spell_ability = state.arena.find_wizard(id).stats.spell_ability;
                if spell.cast(alignment, spell_ability) {
                    state.arena.adjust_alignment(spell.alignment);
                    self.tx.spell_succeeds(state.arena.alignment).await?;
                    loop {
                        let tiles = state.arena.cast_spell_on_attackable_tiles(sx, sy, spell.range, id);
                        if tiles.is_empty() {
                            self.tx.no_possible_moves(id).await?;
                            return Ok(());
                        }
                        self.tx.choose_target(id, &tiles).await?;
                        if let Some((dx, dy)) = self.chosen_tile(state, id, tiles).await? {
                            if !state.arena.line_of_sight(sx, sy, dx, dy) {
                                self.tx.no_line_of_sight(id).await?;
                                continue;
                            }
                            let tile = state.arena.get_mut(dx, dy).clone();
                            if let Some(creation) = tile.creation {
                                if creation.defend_against_attack(6) {
                                    self.tx.lightning(id, dx, dy, true).await?;
                                    state.arena.kill_creation(dx, dy, false);
                                } else {
                                    self.tx.lightning(id, dx, dy, false).await?;
                                }
                            } else if let Some(wizard) = tile.wizard {
                                if wizard.defend_against_attack(6) {
                                    self.tx.lightning(id, dx, dy, true).await?;
                                    state.arena.kill_wizard_and_creations(wizard.id);
                                    if state.wizards.check_for_winning_condition() {
                                        return Ok(());
                                    }
                                } else {
                                    self.tx.lightning(id, dx, dy, false).await?;
                                }
                            }
                        }
                        return Ok(());
                    }
                } else {
                    self.tx.spell_fails().await?;
                }
            }
            SpellKind::MagicalAttack(attempts) => {
                let (sx, sy) = state.arena.find_wizard_pos(id);
                let spell_ability = state.arena.find_wizard(id).stats.spell_ability;
                if spell.cast(alignment, spell_ability) {
                    state.arena.adjust_alignment(spell.alignment);
                    self.tx.spell_succeeds(state.arena.alignment).await?;
                    for _ in 0..attempts {
                        let tiles = state.arena.cast_spell_on_attackable_tiles(sx, sy, spell.range, id);
                        if tiles.is_empty() {
                            self.tx.no_possible_moves(id).await?;
                            return Ok(());
                        }
                        self.tx.choose_target(id, &tiles).await?;
                        if let Some((dx, dy)) = self.chosen_tile(state, id, tiles).await? {
                            let tile = state.arena.get_mut(dx, dy).clone();
                            if let Some(creation) = tile.creation {
                                if creation.defend_against_magical_attack(spell_ability) {
                                    self.tx.magical_attack(id, dx, dy, true).await?;
                                    state.arena.kill_creation(dx, dy, false);
                                } else {
                                    self.tx.magical_attack(id, dx, dy, false).await?;
                                }
                            } else if let Some(wizard) = tile.wizard {
                                if wizard.defend_against_magical_attack(spell_ability) {
                                    self.tx.magical_attack(id, dx, dy, true).await?;
                                    state.arena.destroy_all_wizard_creations(wizard.id);
                                } else {
                                    self.tx.magical_attack(id, dx, dy, false).await?;
                                }
                            }
                        } else {
                            return Ok(());
                        }
                    }
                } else {
                    self.tx.spell_fails().await?;
                }
            }
            SpellKind::WizardAttackBuff(ref buff) => {
                if spell.cast(alignment, spell_ability) {
                    wizard.stats.attack_buff = Some(buff.clone());
                    match buff {
                        AttackBuff::MagicKnife => wizard.stats.magic_knife(),
                        AttackBuff::MagicSword => wizard.stats.magic_sword(),
                    }
                    self.tx.buff_wizard(wizard.id, &wizard.stats).await?;
                    state.arena.adjust_alignment(spell.alignment);
                    self.tx.spell_succeeds(state.arena.alignment).await?;
                } else {
                    self.tx.spell_fails().await?;
                }
            }
            SpellKind::WizardDefenceBuff(ref buff) => {
                if spell.cast(alignment, spell_ability) {
                    wizard.stats.defence_buff = Some(buff.clone());
                    match buff {
                        DefenceBuff::MagicShield => wizard.stats.magic_shield(),
                        DefenceBuff::MagicArmour => wizard.stats.magic_armour(),
                    }
                    self.tx.buff_wizard(wizard.id, &wizard.stats).await?;
                    state.arena.adjust_alignment(spell.alignment);
                    self.tx.spell_succeeds(state.arena.alignment).await?;
                } else {
                    self.tx.spell_fails().await?;
                }
            }
            SpellKind::MagicBow => {
                if spell.cast(alignment, spell_ability) {
                    wizard.stats.magic_bow();
                    self.tx.buff_wizard(wizard.id, &wizard.stats).await?;
                    state.arena.adjust_alignment(spell.alignment);
                    self.tx.spell_succeeds(state.arena.alignment).await?;
                } else {
                    self.tx.spell_fails().await?;
                }
            }
            SpellKind::MagicWings => {
                if spell.cast(alignment, spell_ability) {
                    wizard.stats.magic_wings();
                    self.tx.buff_wizard(wizard.id, &wizard.stats).await?;
                    state.arena.adjust_alignment(spell.alignment);
                    self.tx.spell_succeeds(state.arena.alignment).await?;
                } else {
                    self.tx.spell_fails().await?;
                }
            }
            SpellKind::WorldAlignment => {
                if spell.cast(alignment, spell_ability) {
                    state.arena.adjust_alignment(spell.alignment);
                    self.tx.spell_succeeds(state.arena.alignment).await?;
                } else {
                    self.tx.spell_fails().await?;
                }
            }
            SpellKind::ShadowForm => {
                if spell.cast(alignment, spell_ability) {
                    wizard.stats.shadow_form = true;
                    self.tx.buff_wizard(id, &wizard.stats).await?;
                    state.arena.adjust_alignment(spell.alignment);
                    self.tx.spell_succeeds(state.arena.alignment).await?;
                } else {
                    self.tx.spell_fails().await?;
                }
            }
            SpellKind::Subversion => {
                let (sx, sy) = state.arena.find_wizard_pos(id);
                let spell_ability = state.arena.find_wizard(id).stats.spell_ability;
                loop {
                    let tiles = state.arena.all_subvertable_opposition_tiles(sx, sy, spell.range, id);
                    if tiles.is_empty() {
                        self.tx.no_possible_moves(id).await?;
                        return Ok(());
                    }
                    self.tx.choose_target(id, &tiles).await?;
                    if let Some((dx, dy)) = self.chosen_tile(state, id, tiles).await? {
                        if !state.arena.line_of_sight(sx, sy, dx, dy) {
                            self.tx.no_line_of_sight(id).await?;
                            continue;
                        }
                        let creation = state.arena.get_creation(dx, dy);
                        if spell.cast(alignment, spell_ability)
                            && !creation.illusion
                            && creation.defend_against_magical_attack(spell_ability)
                        {
                            state.arena.adjust_alignment(spell.alignment);
                            self.tx.spell_succeeds(state.arena.alignment).await?;
                            self.tx.subversion(id, dx, dy, true).await?;
                            state.arena.subvert(dx, dy, id);
                        } else {
                            self.tx.spell_fails().await?;
                        }
                    }
                    return Ok(());
                }
            }
            SpellKind::RaiseDead => {
                let (sx, sy) = state.arena.find_wizard_pos(id);
                let spell_ability = state.arena.find_wizard(id).stats.spell_ability;
                loop {
                    let tiles = state.arena.visible_corpse_tiles(sx, sy, spell.range);
                    if tiles.is_empty() {
                        self.tx.no_possible_moves(id).await?;
                        return Ok(());
                    }
                    self.tx.choose_target(id, &tiles).await?;
                    if let Some((dx, dy)) = self.chosen_tile(state, id, tiles).await? {
                        if !state.arena.line_of_sight(sx, sy, dx, dy) {
                            self.tx.no_line_of_sight(id).await?;
                            continue;
                        }
                        if spell.cast(alignment, spell_ability)
                            && state.arena.get_corpse(dx, dy).defend_against_magical_attack(spell_ability)
                        {
                            self.tx.raise_dead(id, dx, dy, true).await?;
                            state.arena.raise_dead(dx, dy, id);
                            state.arena.adjust_alignment(spell.alignment);
                            self.tx.spell_succeeds(state.arena.alignment).await?;
                        } else {
                            self.tx.raise_dead(id, dx, dy, false).await?;
                            self.tx.spell_fails().await?;
                        }
                    }
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    async fn creation_attack(
        &mut self,
        state: &mut ServerState,
        id: u32,
        sx: u8,
        sy: u8,
        dx: u8,
        dy: u8,
    ) -> Result<(), ChaosError> {
        state.arena.get_mut_creation(sx, sy).moves_left = 0;
        let shadow_wood = state.arena.get_creation(sx, sy).stats.shadow_wood;
        let tile = state.arena.get_mut(dx, dy).clone();
        if tile.spawn.is_some() {
            let combat = state.arena.get_creation(sx, sy).stats.base.combat;
            if state.arena.get_blob(dx, dy).defend_against_attack(combat) {
                self.tx.successful_attack(id, dx, dy, false).await?;
                state.arena.remove_spawn(dx, dy);
                if !shadow_wood && tile.creation.is_none() && tile.wizard.is_none() {
                    self.tx.move_creation(id, sx, sy, dx, dy).await?;
                    state.arena.move_creation(sx, sy, dx, dy);
                    self.check_for_creation_ranged_combat(state, id, dx, dy).await?;
                }
            } else {
                self.tx.failed_attack(id, dx, dy).await?;
                self.check_for_creation_ranged_combat(state, id, sx, sy).await?;
            }
        } else if let Some(other) = tile.creation {
            let creation = state.arena.get_creation(sx, sy);
            if other.defend_against_attack(creation.stats.base.combat) {
                if other.stats.magic_wood {
                    let wizard_id = state.arena.get_wizard(dx, dy).id;
                    self.tx.successful_attack(id, dx, dy, false).await?;
                    state.arena.kill_wizard_and_creations(wizard_id);
                    state.wizards.get_mut(wizard_id)?.alive = false;
                    if state.wizards.check_for_winning_condition() {
                        return Ok(());
                    }
                    if !shadow_wood {
                        self.tx.move_creation(id, sx, sy, dx, dy).await?;
                        state.arena.move_creation(sx, sy, dx, dy);
                        self.check_for_creation_ranged_combat(state, id, dx, dy).await?;
                    }
                } else {
                    let corpse = !(other.illusion || other.stats.undead);
                    self.tx.successful_attack(id, dx, dy, corpse).await?;
                    state.arena.kill_creation(dx, dy, corpse);
                    if !shadow_wood && tile.wizard.is_none() {
                        self.tx.move_creation(id, sx, sy, dx, dy).await?;
                        state.arena.move_creation(sx, sy, dx, dy);
                        self.check_for_creation_ranged_combat(state, id, dx, dy).await?;
                    }
                }
            } else {
                self.tx.failed_attack(id, dx, dy).await?;
                self.check_for_creation_ranged_combat(state, id, sx, sy).await?;
            }
        } else if let Some(ref wizard) = tile.wizard {
            let creation = state.arena.get_creation(sx, sy);
            if wizard.defend_against_attack(creation.stats.base.combat) {
                self.tx.successful_attack(id, dx, dy, false).await?;
                state.arena.kill_wizard_and_creations(wizard.id);
                state.wizards.get_mut(wizard.id)?.alive = false;
                if state.wizards.check_for_winning_condition() {
                    return Ok(());
                }
                if !shadow_wood {
                    self.tx.move_creation(id, sx, sy, dx, dy).await?;
                    state.arena.move_creation(sx, sy, dx, dy);
                }
                self.check_for_creation_ranged_combat(state, id, dx, dy).await?;
            } else {
                self.tx.failed_attack(id, dx, dy).await?;
                self.check_for_creation_ranged_combat(state, id, sx, sy).await?;
            }
        } else {
            unreachable!();
        }
        Ok(())
    }

    async fn wizard_attack(
        &mut self,
        state: &mut ServerState,
        id: u32,
        sx: u8,
        sy: u8,
        dx: u8,
        dy: u8,
    ) -> Result<(), ChaosError> {
        let tile = state.arena.get_mut(dx, dy).clone();
        let wizard = state.arena.get_mut_wizard(sx, sy);
        wizard.moves_left = 0;
        wizard.stats.shadow_form = false;
        if tile.spawn.is_some() {
            let combat = wizard.stats.get_combat();
            if state.arena.get_blob(dx, dy).defend_against_attack(combat) {
                self.tx.successful_attack(id, dx, dy, false).await?;
                state.arena.remove_spawn(dx, dy);
                if tile.creation.is_none() && tile.wizard.is_none() {
                    self.tx.move_wizard(id, dx, dy).await?;
                    state.arena.move_wizard(id, dx, dy);
                    self.check_for_wizard_ranged_combat(state, id, dx, dy).await?;
                }
            } else {
                self.tx.failed_attack(id, dx, dy).await?;
                self.check_for_wizard_ranged_combat(state, id, sx, sy).await?;
            }
        } else if let Some(other) = tile.creation {
            if other.defend_against_attack(wizard.stats.get_combat()) {
                if other.stats.magic_wood {
                    let wizard_id = state.arena.get_wizard(dx, dy).id;
                    self.tx.successful_attack(id, dx, dy, false).await?;
                    if state.wizards.check_for_winning_condition() {
                        return Ok(());
                    }
                    state.arena.kill_wizard_and_creations(wizard_id);
                    state.wizards.get_mut(wizard_id)?.alive = false;
                    if state.wizards.check_for_winning_condition() {
                        return Ok(());
                    }
                    state.arena.move_wizard(id, dx, dy);
                    self.check_for_wizard_ranged_combat(state, id, dx, dy).await?;
                } else {
                    let corpse = !(other.illusion || other.stats.undead);
                    self.tx.successful_attack(id, dx, dy, corpse).await?;
                    state.arena.kill_creation(dx, dy, corpse);
                    if tile.wizard.is_none() {
                        self.tx.move_wizard(id, dx, dy).await?;
                        state.arena.move_wizard(id, dx, dy);
                        self.check_for_wizard_ranged_combat(state, id, dx, dy).await?;
                    }
                }
            } else {
                self.tx.failed_attack(id, dx, dy).await?;
                self.check_for_wizard_ranged_combat(state, id, sx, sy).await?;
            }
        } else if let Some(ref other) = tile.wizard {
            if other.defend_against_attack(wizard.stats.get_combat()) {
                self.tx.successful_attack(id, dx, dy, false).await?;
                state.arena.kill_wizard_and_creations(other.id);
                state.wizards.get_mut(other.id)?.alive = false;
                if state.wizards.check_for_winning_condition() {
                    return Ok(());
                }
                self.tx.move_wizard(id, dx, dy).await?;
                state.arena.move_wizard(id, dx, dy);
                self.check_for_wizard_ranged_combat(state, id, dx, dy).await?;
            } else {
                self.tx.failed_attack(id, dx, dy).await?;
                self.check_for_wizard_ranged_combat(state, id, sx, sy).await?;
            }
        } else {
            unreachable!();
        }
        Ok(())
    }

    async fn creation_engaged_in_combat(&mut self, state: &mut ServerState, id: u32, sx: u8, sy: u8) -> Result<(), ChaosError> {
        state.arena.get_mut_creation(sx, sy).moves_left = 0;
        let undead = state.arena.get_creation(sx, sy).stats.undead;
        loop {
            let tiles = state.arena.creation_combat_tiles(sx, sy, id);
            self.tx.engaged_in_combat(id, &tiles).await?;
            if let Some((dx, dy)) = self.chosen_tile(state, id, tiles).await? {
                if let Some(ref creation) = state.arena.get(dx, dy).creation {
                    if !undead && creation.stats.undead {
                        self.tx.undead_cannot_be_attacked(id).await?;
                        continue;
                    }
                }
                return self.creation_attack(state, id, sx, sy, dx, dy).await;
            } else {
                return self.check_for_creation_ranged_combat(state, id, sx, sy).await;
            }
        }
    }

    pub async fn wizard_engaged_in_combat(&mut self, state: &mut ServerState, id: u32, x: u8, y: u8) -> Result<(), ChaosError> {
        state.arena.get_mut_wizard(x, y).moves_left = 0;
        loop {
            let tiles = state.arena.wizard_combat_tiles(x, y, id);
            self.tx.engaged_in_combat(id, &tiles).await?;
            if let Some((dx, dy)) = self.chosen_tile(state, id, tiles).await? {
                if let Some(ref creation) = state.arena.get(dx, dy).creation {
                    if creation.stats.undead && state.arena.get_wizard(x, y).stats.attack_buff.is_none() {
                        self.tx.undead_cannot_be_attacked(id).await?;
                        continue;
                    }
                }
                return self.wizard_attack(state, id, x, y, dx, dy).await;
            } else {
                return self.check_for_wizard_ranged_combat(state, id, x, y).await;
            }
        }
    }

    async fn move_wizard(&mut self, state: &mut ServerState, id: u32, mut sx: u8, mut sy: u8) -> Result<(), ChaosError> {
        loop {
            let tiles = state.arena.wizard_movement_tiles(sx, sy, id);
            if tiles.is_empty() {
                return Ok(());
            }
            let wizard = state.arena.get_wizard(sx, sy);
            let shadow_form = wizard.stats.shadow_form;
            if wizard.moves_left == wizard.stats.get_movement() {
                self.tx.movement_range(id, wizard.stats.base.movement, false, &tiles).await?;
            } else {
                self.tx.movement_points(id, wizard.moves_left, &tiles).await?;
            }
            match self.chosen_tile(state, id, tiles).await? {
                Some((dx, dy)) => {
                    let tile = state.arena.get(dx, dy);
                    if tile.spawn.is_some() {
                        return self.wizard_attack(state, id, sx, sy, dx, dy).await;
                    }
                    if let Some(ref creation) = tile.creation {
                        if creation.stats.undead && state.arena.get_wizard(sx, sy).stats.attack_buff.is_none() {
                            self.tx.undead_cannot_be_attacked(id).await?;
                            continue;
                        }
                    }
                    if tile.wizard.is_some()
                        || tile
                            .creation
                            .as_ref()
                            .is_some_and(|creation| creation.id != id && !creation.stats.magic_wood)
                    {
                        return self.wizard_attack(state, id, sx, sy, dx, dy).await;
                    }
                    self.tx.move_wizard(id, dx, dy).await?;
                    state.arena.move_wizard(id, dx, dy);
                    if !shadow_form && state.arena.has_neighbouring_foes(dx, dy, id) {
                        return self.wizard_engaged_in_combat(state, id, dx, dy).await;
                    }
                    if let Some(creation) = state.arena.get_mut(dx, dy).creation.as_mut() {
                        if creation.stats.mount {
                            creation.moves_left = 0;
                            state.arena.get_mut_wizard(dx, dy).moves_left = 0;
                            return Ok(());
                        }
                    }
                    let wizard = state.arena.get_mut_wizard(dx, dy);
                    wizard.moves_left -= 1;
                    if wizard.moves_left == 0 {
                        self.check_for_wizard_ranged_combat(state, id, dx, dy).await?;
                        return Ok(());
                    }
                    sx = dx;
                    sy = dy;
                }
                None => {
                    state.arena.get_mut_wizard(sx, sy).moves_left = 0;
                    self.check_for_wizard_ranged_combat(state, id, sx, sy).await?;
                    return Ok(());
                }
            }
        }
    }

    async fn fly_wizard(&mut self, state: &mut ServerState, id: u32, x: u8, y: u8) -> Result<(), ChaosError> {
        let wizard = state.arena.get_mut_wizard(x, y);
        let shadow_form = wizard.stats.shadow_form;
        wizard.moves_left = 0;
        let movement = 6;
        loop {
            let tiles = state.arena.wizard_flying_tiles(x, y, movement, id);
            if tiles.is_empty() {
                return Ok(());
            }
            self.tx.movement_range(id, movement, true, &tiles).await?;
            if let Some((dx, dy)) = self.chosen_tile(state, id, tiles).await? {
                let tile = state.arena.get(dx, dy);
                if let Some(ref creation) = tile.creation {
                    if creation.stats.undead && state.arena.get_wizard(x, y).stats.attack_buff.is_none() {
                        self.tx.undead_cannot_be_attacked(id).await?;
                        continue;
                    }
                }
                if tile.spawn.is_some()
                    || tile.wizard.is_some()
                    || tile.creation.as_ref().is_some_and(|creation| creation.id != id)
                {
                    return self.wizard_attack(state, id, x, y, dx, dy).await;
                }
                self.tx.move_wizard(id, dx, dy).await?;
                state.arena.move_wizard(id, dx, dy);
                if let Some(creation) = state.arena.get_mut(dx, dy).creation.as_mut() {
                    if creation.id == id {
                        creation.moves_left = 0;
                    }
                    return Ok(());
                }
                if !shadow_form && state.arena.has_neighbouring_foes(dx, dy, id) {
                    return self.wizard_engaged_in_combat(state, id, dx, dy).await;
                }
                return self.check_for_wizard_ranged_combat(state, id, dx, dy).await;
            } else {
                return self.check_for_wizard_ranged_combat(state, id, x, y).await;
            }
        }
    }

    async fn move_creation(&mut self, state: &mut ServerState, id: u32, mut sx: u8, mut sy: u8) -> Result<(), ChaosError> {
        loop {
            let tiles = state.arena.creation_movement_tiles(sx, sy, id);
            if tiles.is_empty() {
                return Ok(());
            }
            let creation = state.arena.get_creation(sx, sy);
            if creation.moves_left == creation.stats.base.movement {
                self.tx
                    .movement_range(id, creation.stats.base.movement, false, &tiles)
                    .await?;
            } else {
                self.tx.movement_points(id, creation.moves_left, &tiles).await?;
            }
            match self.chosen_tile(state, id, tiles).await? {
                Some((dx, dy)) => {
                    let tile = state.arena.get(dx, dy);
                    if tile.spawn.is_some() {
                        return self.creation_attack(state, id, sx, sy, dx, dy).await;
                    }
                    if let Some(ref creation) = tile.creation {
                        if creation.stats.undead && !state.arena.get_creation(sx, sy).stats.undead {
                            self.tx.undead_cannot_be_attacked(id).await?;
                            continue;
                        }
                    }
                    if tile.wizard.is_some() || tile.creation.is_some() {
                        return self.creation_attack(state, id, sx, sy, dx, dy).await;
                    }
                    self.tx.move_creation(id, sx, sy, dx, dy).await?;
                    state.arena.move_creation(sx, sy, dx, dy);
                    if state.arena.has_neighbouring_foes(dx, dy, id) {
                        return self.creation_engaged_in_combat(state, id, dx, dy).await;
                    }
                    let creation = state.arena.get_mut_creation(dx, dy);
                    creation.moves_left -= 1;
                    if creation.moves_left == 0 {
                        self.check_for_creation_ranged_combat(state, id, dx, dy).await?;
                        return Ok(());
                    }
                    sx = dx;
                    sy = dy;
                }
                None => {
                    state.arena.get_mut_creation(sx, sy).moves_left = 0;
                    self.check_for_creation_ranged_combat(state, id, sx, sy).await?;
                    return Ok(());
                }
            }
        }
    }

    pub async fn fly_creation(&mut self, state: &mut ServerState, id: u32, x: u8, y: u8) -> Result<(), ChaosError> {
        let creation = state.arena.get_mut_creation(x, y);
        creation.moves_left = 0;
        let movement = creation.stats.base.movement;
        loop {
            let tiles = state.arena.creation_flying_tiles(x, y, movement, id);
            if tiles.is_empty() {
                return Ok(());
            }
            self.tx.movement_range(id, movement, true, &tiles).await?;
            if let Some((dx, dy)) = self.chosen_tile(state, id, tiles).await? {
                let tile = state.arena.get(dx, dy);
                if let Some(ref creation) = tile.creation {
                    if creation.stats.undead && !state.arena.get_creation(x, y).stats.undead {
                        self.tx.undead_cannot_be_attacked(id).await?;
                        continue;
                    }
                }
                if tile.spawn.is_some() || tile.wizard.is_some() || tile.creation.is_some() {
                    return self.creation_attack(state, id, x, y, dx, dy).await;
                }
                self.tx.move_creation(id, x, y, dx, dy).await?;
                state.arena.move_creation(x, y, dx, dy);
                if state.arena.has_neighbouring_foes(dx, dy, id) {
                    return self.creation_engaged_in_combat(state, id, dx, dy).await;
                }
                return self.check_for_creation_ranged_combat(state, id, dx, dy).await;
            } else {
                return self.check_for_creation_ranged_combat(state, id, x, y).await;
            }
        }
    }

    async fn dismount_loop(&mut self, state: &mut ServerState, id: u32) -> Result<Option<bool>, ChaosError> {
        if state.wizards.has_disconnected(id)? {
            return Ok(None);
        }
        loop {
            select! {
                _ = &mut self.quit_rx => {
                    self.tx.shutdown().await?;
                }
                Some(msg) = self.rx.recv() => {
                    match msg {
                        RecieveMsg::Disconnected { id: disconnected_id } => {
                            state.wizards.get_mut(disconnected_id)?.disconnected = true;
                            if id == disconnected_id {
                                return Ok(None);
                            }
                        }
                        RecieveMsg::Message { id: msg_id, msg: Message::Dismount(dismount) } if msg_id == id => {
                            return Ok(dismount);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    async fn creation_ranged_combat(
        &mut self,
        state: &mut ServerState,
        id: u32,
        sx: u8,
        sy: u8,
        dx: u8,
        dy: u8,
    ) -> Result<(), ChaosError> {
        let creation = state.arena.get_creation(sx, sy).clone();
        let color = creation.projectile_color();
        let tile = state.arena.get(dx, dy).clone();
        if let Some(Spawn::Blob(blob)) = tile.spawn {
            if blob.defend_against_attack(creation.stats.base.ranged_combat) {
                if creation.stats.dragon {
                    self.tx.successful_dragon_ranged_attack(id, sx, sy, dx, dy).await?;
                } else {
                    self.tx.successful_ranged_attack(id, sx, sy, dx, dy, false, color).await?;
                }
                state.arena.remove_spawn(dx, dy);
                return Ok(());
            }
        } else if let Some(other) = tile.creation {
            if other.stats.undead && !creation.stats.undead {
                self.tx.undead_cannot_be_attacked(id).await?;
            } else if other.defend_against_attack(creation.stats.base.ranged_combat) {
                if other.stats.magic_wood && tile.wizard.is_some() {
                    let wizard_id = state.arena.get_wizard(dx, dy).id;
                    if creation.stats.dragon {
                        self.tx.successful_dragon_ranged_attack(id, sx, sy, dx, dy).await?;
                    } else {
                        self.tx.successful_ranged_attack(id, sx, sy, dx, dy, false, color).await?;
                    }
                    state.wizards.get_mut(wizard_id)?.alive = false;
                    state.arena.kill_wizard_and_creations(wizard_id);
                    return Ok(());
                } else {
                    if creation.stats.dragon {
                        self.tx.successful_dragon_ranged_attack(id, sx, sy, dx, dy).await?;
                        state.arena.kill_creation(dx, dy, false);
                    } else {
                        let corpse = other.has_a_corpse();
                        self.tx.successful_ranged_attack(id, sx, sy, dx, dy, corpse, color).await?;
                        state.arena.kill_creation(dx, dy, corpse);
                    }
                    return Ok(());
                }
            }
        } else if let Some(wizard) = tile.wizard {
            if wizard.defend_against_attack(creation.stats.base.ranged_combat) {
                if creation.stats.dragon {
                    self.tx.successful_dragon_ranged_attack(id, sx, sy, dx, dy).await?;
                } else {
                    self.tx.successful_ranged_attack(id, sx, sy, dx, dy, false, color).await?;
                }
                state.arena.kill_wizard_and_creations(wizard.id);
                state.wizards.get_mut(wizard.id)?.alive = false;
                return Ok(());
            }
        }
        if creation.stats.dragon {
            self.tx.failed_dragon_ranged_attack(id, sx, sy, dx, dy).await?;
        } else {
            self.tx.failed_ranged_attack(id, sx, sy, dx, dy, color).await?;
        }
        Ok(())
    }

    async fn wizard_ranged_combat(
        &mut self,
        state: &mut ServerState,
        id: u32,
        sx: u8,
        sy: u8,
        dx: u8,
        dy: u8,
    ) -> Result<(), ChaosError> {
        let wizard = state.arena.get_wizard(sx, sy);
        let tile = state.arena.get(dx, dy).clone();
        if let Some(Spawn::Blob(blob)) = tile.spawn {
            if blob.defend_against_attack(wizard.stats.get_ranged_combat()) {
                self.tx
                    .successful_ranged_attack(id, sx, sy, dx, dy, false, BrightWhite)
                    .await?;
                state.arena.remove_spawn(dx, dy);
                return Ok(());
            }
        } else if let Some(other) = tile.creation {
            if other.defend_against_attack(wizard.stats.get_ranged_combat()) {
                if other.stats.magic_wood && tile.wizard.is_some() {
                    self.tx
                        .successful_ranged_attack(id, sx, sy, dx, dy, false, BrightWhite)
                        .await?;
                    state.wizards.get_mut(other.id)?.alive = false;
                    state.arena.kill_wizard_and_creations(other.id);
                    return Ok(());
                } else {
                    let corpse = other.has_a_corpse();
                    self.tx
                        .successful_ranged_attack(id, sx, sy, dx, dy, corpse, BrightWhite)
                        .await?;
                    state.arena.kill_creation(dx, dy, corpse);
                    return Ok(());
                }
            }
        } else if let Some(other) = tile.wizard {
            if other.defend_against_attack(wizard.stats.get_ranged_combat()) {
                self.tx
                    .successful_ranged_attack(id, sx, sy, dx, dy, false, BrightWhite)
                    .await?;
                state.arena.kill_wizard_and_creations(other.id);
                state.wizards.get_mut(other.id)?.alive = false;
                return Ok(());
            }
        }
        self.tx.failed_ranged_attack(id, sx, sy, dx, dy, BrightWhite).await?;
        Ok(())
    }

    async fn check_for_creation_ranged_combat(
        &mut self,
        state: &mut ServerState,
        id: u32,
        x: u8,
        y: u8,
    ) -> Result<(), ChaosError> {
        let creation = state.arena.get_creation(x, y);
        let range = creation.stats.base.range;
        if range > 0 {
            loop {
                let tiles = state.arena.ranged_combat_tiles(x, y, range);
                self.tx.choose_ranged_combat(id, range, &tiles).await?;
                if let Some((dx, dy)) = self.chosen_tile(state, id, tiles).await? {
                    if state.arena.line_of_sight(x, y, dx, dy) {
                        return self.creation_ranged_combat(state, id, x, y, dx, dy).await;
                    } else {
                        self.tx.no_line_of_sight(id).await?;
                    }
                } else {
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    async fn check_for_wizard_ranged_combat(&mut self, state: &mut ServerState, id: u32, x: u8, y: u8) -> Result<(), ChaosError> {
        let wizard = state.arena.get_wizard(x, y);
        let range = wizard.stats.get_range();
        if range > 0 {
            loop {
                let tiles = state.arena.ranged_combat_tiles(x, y, range);
                self.tx.choose_ranged_combat(id, range, &tiles).await?;
                if let Some((dx, dy)) = self.chosen_tile(state, id, tiles).await? {
                    if state.arena.line_of_sight(x, y, dx, dy) {
                        return self.wizard_ranged_combat(state, id, x, y, dx, dy).await;
                    } else {
                        self.tx.no_line_of_sight(id).await?;
                    }
                } else {
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    async fn check_engaged(&mut self, state: &mut ServerState, id: u32, x: u8, y: u8, manoeuvre: u8) -> Result<bool, ChaosError> {
        for (dx, dy) in state.arena.neighbouring_foes(x, y, id) {
            let tile = state.arena.get(dx, dy).clone();
            if let Some(other) = tile.creation {
                if other.is_engaged(manoeuvre) {
                    return Ok(true);
                }
            } else if let Some(other) = tile.wizard {
                if other.is_engaged(manoeuvre) {
                    return Ok(true);
                }
            } else {
                unreachable!()
            }
        }
        Ok(false)
    }

    pub async fn shadow_wood_attack(&mut self, state: &mut ServerState, id: u32, x: u8, y: u8) -> Result<(), ChaosError> {
        state.arena.get_mut_creation(x, y).moves_left = 0;
        let undead = state.arena.get_creation(x, y).stats.undead;
        loop {
            let tiles = state.arena.creation_combat_tiles(x, y, id);
            if tiles.is_empty() {
                self.tx.no_possible_moves(id).await?;
                return Ok(());
            }
            self.tx.engaged_in_combat(id, &tiles).await?;
            if let Some((dx, dy)) = self.chosen_tile(state, id, tiles).await? {
                if let Some(ref creation) = state.arena.get(dx, dy).creation {
                    if !undead && creation.stats.undead {
                        self.tx.undead_cannot_be_attacked(id).await?;
                        continue;
                    }
                }
                return self.creation_attack(state, id, x, y, dx, dy).await;
            } else {
                return Ok(());
            }
        }
    }

    pub async fn movement_loop(&mut self, state: &mut ServerState, id: u32) -> Result<(), ChaosError> {
        state.arena.reset_moves(id);
        loop {
            if !state.wizards.is_alive(id)? {
                return Ok(());
            }
            if state.wizards.check_for_winning_condition() {
                return Ok(());
            }
            let tiles = state.arena.tiles_with_moves_left(id);
            if tiles.is_empty() {
                return Ok(());
            }
            self.tx.choose_piece(id, &tiles).await?;
            match self.chosen_tile(state, id, tiles).await? {
                Some((x, y)) => {
                    let tile = state.arena.get(x, y).clone();
                    if let Some(GameCreation {
                        stats:
                            CreationStats {
                                flying,
                                shelter: false,
                                magic_wood,
                                shadow_wood,
                                base,
                                ..
                            },
                        ..
                    }) = tile.creation
                    {
                        if shadow_wood {
                            self.shadow_wood_attack(state, id, x, y).await?;
                            continue;
                        }
                        if self.check_engaged(state, id, x, y, base.manoeuvre).await? {
                            self.creation_engaged_in_combat(state, id, x, y).await?;
                            continue;
                        }
                        if tile.wizard.is_some() {
                            if magic_wood {
                                self.move_wizard(state, id, x, y).await?;
                                continue;
                            } else {
                                self.tx.ask_for_dismount(id).await?;
                                match self.dismount_loop(state, id).await? {
                                    Some(true) => {
                                        state.arena.get_mut_creation(x, y).moves_left = 0;
                                        self.move_wizard(state, id, x, y).await?;
                                        continue;
                                    }
                                    Some(false) => {
                                        state.arena.get_mut_wizard(x, y).moves_left = 0;
                                    }
                                    None => continue,
                                }
                            }
                        }
                        if flying {
                            self.fly_creation(state, id, x, y).await?;
                        } else {
                            self.move_creation(state, id, x, y).await?;
                        }
                    } else if let Some(wizard) = tile.wizard {
                        if !wizard.stats.shadow_form && self.check_engaged(state, id, x, y, wizard.stats.base.manoeuvre).await? {
                            self.wizard_engaged_in_combat(state, id, x, y).await?;
                            continue;
                        }
                        if wizard.stats.magic_wings {
                            self.fly_wizard(state, id, x, y).await?;
                        } else {
                            self.move_wizard(state, id, x, y).await?;
                        }
                    } else {
                        unreachable!();
                    }
                }
                None => return Ok(()),
            }
        }
    }

    async fn fire_attack(&mut self, fire: &GameCreation, state: &mut ServerState, x: u8, y: u8) -> Result<(), ChaosError> {
        let tile = state.arena.get(x, y).clone();
        if let Some(ref creation) = tile.creation {
            if creation.id != fire.id && creation.stats.attackable {
                if creation.defend_against_attack(5) {
                    state.arena.kill_creation(x, y, false);
                    if tile.wizard.is_none() {
                        self.tx.spawn_fire(x, y, Some(fire)).await?;
                        state.arena.spawn_fire(x, y, fire.clone());
                    }
                } else {
                    self.tx.spawn_fire(x, y, None).await?;
                }
            }
        } else if let Some(ref wizard) = tile.wizard {
            if wizard.id != fire.id {
                if wizard.defend_against_attack(5) {
                    state.arena.kill_wizard_and_creations(wizard.id);
                    state.wizards.get_mut(wizard.id).unwrap().alive = false;
                    self.tx.spawn_fire(x, y, Some(fire)).await?;
                    state.arena.spawn_fire(x, y, fire.clone());
                    if state.wizards.check_for_winning_condition() {
                        return Ok(());
                    }
                } else {
                    self.tx.spawn_fire(x, y, None).await?;
                }
            }
        } else {
            self.tx.spawn_fire(x, y, Some(fire)).await?;
            state.arena.spawn_fire(x, y, fire.clone());
        }
        Ok(())
    }

    async fn blob_mutate(&mut self, blob: &GameCreation, state: &mut ServerState, x: u8, y: u8) -> Result<(), ChaosError> {
        let tile = state.arena.get(x, y).clone();
        if let Some(ref creation) = tile.creation {
            if creation.id != blob.id {
                self.tx.spawn_blob(x, y, Some(blob)).await?;
                state.arena.spawn_blob(x, y, blob.clone());
            }
        } else if let Some(ref wizard) = tile.wizard {
            if wizard.id != blob.id {
                if wizard.defend_against_attack(5) {
                    state.arena.kill_wizard_and_creations(wizard.id);
                    state.wizards.get_mut(wizard.id).unwrap().alive = false;
                    self.tx.spawn_blob(x, y, Some(blob)).await?;
                    state.arena.spawn_blob(x, y, blob.clone());
                    if state.wizards.check_for_winning_condition() {
                        return Ok(());
                    }
                } else {
                    self.tx.spawn_fire(x, y, None).await?;
                }
            }
        } else {
            self.tx.spawn_blob(x, y, Some(blob)).await?;
            state.arena.spawn_blob(x, y, blob.clone());
        }
        Ok(())
    }

    async fn do_fire(&mut self, state: &mut ServerState) -> Result<(), ChaosError> {
        let mut rng = StdRng::from_entropy();
        for (x, y) in state.arena.all_spawn_tiles() {
            if let Some(spawn) = state.arena.get(x, y).spawn.clone() {
                match rng.gen_range(0..=9) {
                    0 | 1 => {
                        self.tx.remove_spawn(x, y).await?;
                        state.arena.remove_spawn(x, y);
                    }
                    2 => {
                        if y > 0 && state.arena.get(x, y - 1).spawn.is_none() {
                            match spawn {
                                Spawn::Fire(ref fire) => {
                                    self.fire_attack(fire, state, x, y - 1).await?;
                                }
                                Spawn::Blob(ref blob) => {
                                    self.blob_mutate(blob, state, x, y - 1).await?;
                                }
                            }
                        }
                    }
                    3 => {
                        if y > 0 && x < state.arena.width - 1 && state.arena.get(x + 1, y - 1).spawn.is_none() {
                            match spawn {
                                Spawn::Fire(ref fire) => {
                                    self.fire_attack(fire, state, x + 1, y - 1).await?;
                                }
                                Spawn::Blob(ref blob) => {
                                    self.blob_mutate(blob, state, x + 1, y - 1).await?;
                                }
                            }
                        }
                    }
                    4 => {
                        if x < state.arena.width - 1 && state.arena.get(x + 1, y).spawn.is_none() {
                            match spawn {
                                Spawn::Fire(ref fire) => {
                                    self.fire_attack(fire, state, x + 1, y).await?;
                                }
                                Spawn::Blob(ref blob) => {
                                    self.blob_mutate(blob, state, x + 1, y).await?;
                                }
                            }
                        }
                    }
                    5 => {
                        if y < state.arena.height - 1
                            && x < state.arena.width - 1
                            && state.arena.get(x + 1, y + 1).spawn.is_none()
                        {
                            match spawn {
                                Spawn::Fire(ref fire) => {
                                    self.fire_attack(fire, state, x + 1, y + 1).await?;
                                }
                                Spawn::Blob(ref blob) => {
                                    self.blob_mutate(blob, state, x + 1, y + 1).await?;
                                }
                            }
                        }
                    }
                    6 => {
                        if y < state.arena.height - 1 && state.arena.get(x, y + 1).spawn.is_none() {
                            match spawn {
                                Spawn::Fire(ref fire) => {
                                    self.fire_attack(fire, state, x, y + 1).await?;
                                }
                                Spawn::Blob(ref blob) => {
                                    self.blob_mutate(blob, state, x, y + 1).await?;
                                }
                            }
                        }
                    }
                    7 => {
                        if x > 0 && y < state.arena.height - 1 && state.arena.get(x - 1, y + 1).spawn.is_none() {
                            match spawn {
                                Spawn::Fire(ref fire) => {
                                    self.fire_attack(fire, state, x - 1, y + 1).await?;
                                }
                                Spawn::Blob(ref blob) => {
                                    self.blob_mutate(blob, state, x - 1, y + 1).await?;
                                }
                            }
                        }
                    }
                    8 => {
                        if x > 0 && state.arena.get(x - 1, y).spawn.is_none() {
                            match spawn {
                                Spawn::Fire(ref fire) => {
                                    self.fire_attack(fire, state, x - 1, y).await?;
                                }
                                Spawn::Blob(ref blob) => {
                                    self.blob_mutate(blob, state, x - 1, y).await?;
                                }
                            }
                        }
                    }
                    9 => {
                        if x > 0 && y > 0 && state.arena.get(x - 1, y - 1).spawn.is_none() {
                            match spawn {
                                Spawn::Fire(ref fire) => {
                                    self.fire_attack(fire, state, x - 1, y - 1).await?;
                                }
                                Spawn::Blob(ref blob) => {
                                    self.blob_mutate(blob, state, x - 1, y - 1).await?;
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    async fn do_shelter_turn(&mut self, state: &mut ServerState) -> Result<(), ChaosError> {
        for (x, y) in state.arena.all_combustable_shelter_tiles() {
            if state.arena.get_creation(x, y).should_disappear() {
                self.tx.shelter_disappears(x, y).await?;
                state.arena.kill_creation(x, y, false);
            }
        }
        Ok(())
    }

    async fn do_magic_wood(&mut self, state: &mut ServerState) -> Result<(), ChaosError> {
        let mut rng = StdRng::from_entropy();
        for (x, y) in state.arena.wizards_in_trees() {
            if rng.gen_range(0..=9) >= 8 {
                let id = state.arena.get_wizard(x, y).id;
                let server_wizard = state.wizards.get_mut(id)?;
                if server_wizard.spells.len() < 20 {
                    let random_spell = Spell::random();
                    let wizard = state.arena.get_mut_wizard(x, y);
                    wizard.stats.number_of_spells += 1;
                    self.tx.debuff_wizard(wizard.id, &wizard.stats).await?;
                    self.tx.send_spell(wizard.id, &random_spell).await?;
                    self.tx.new_spell(wizard.id, x, y).await?;
                    server_wizard.spells.push(random_spell);
                    state.arena.get_mut(x, y).creation = None;
                }
            }
        }
        Ok(())
    }

    pub async fn game_loop(&mut self, wizards: LobbyWizards) -> Result<Vec<Player>, ChaosError> {
        let mut state = ServerState {
            wizards: wizards.into(),
            arena: Arena::new(),
        };
        self.tx.send_wizards(&state.wizards).await?;
        for (x, y, wizard) in state.wizards.starting_positions()? {
            let game_wizard = GameWizard::from(wizard);
            self.tx.add_wizard(&game_wizard, x, y).await?;
            state.arena.get_mut(x, y).wizard = Some(game_wizard);
        }
        let number_of_turns = state.wizards.len() * 2 + 15;
        for _ in 0..number_of_turns {
            let spells = self.select_spells(&mut state).await?;
            for (id, spell, illusion) in spells {
                self.do_spell(&mut state, id, spell, illusion).await?;
                if state.wizards.check_for_winning_condition() {
                    return Ok(state.wizards.winners());
                }
            }
            self.do_shelter_turn(&mut state).await?;
            self.do_magic_wood(&mut state).await?;
            self.do_fire(&mut state).await?;
            for id in state.wizards.all_active_ids() {
                if !state.wizards.is_alive(id)? {
                    continue;
                }
                if state.wizards.get(id)?.alive {
                    self.tx.turn(id).await?;
                    self.movement_loop(&mut state, id).await?;
                    if state.wizards.check_for_winning_condition() {
                        return Ok(state.wizards.winners());
                    }
                }
            }
            self.tx.turn_end().await?;
        }
        Ok(state.wizards.winners())
    }

    pub async fn end(mut self, winners: Vec<Player>) -> Result<(), ChaosError> {
        self.tx.results(&winners).await?;
        self.quit_rx.await.ok();
        self.tx.shutdown().await.ok();
        Ok(())
    }
}

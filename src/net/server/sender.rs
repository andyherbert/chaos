use crate::config::Player;
use crate::data::creation::GameCreation;
use crate::data::spells::Spell;
use crate::data::stats::WizardStats;
use crate::data::wizard::{GameWizard, LobbyWizards, ServerWizards};
use crate::gfx::color::Color;
use crate::net::{Message, NetworkError, SendMsg};
use tokio::sync::mpsc;

pub struct Sender {
    tx: mpsc::Sender<SendMsg>,
}

impl Sender {
    pub fn new(tx: mpsc::Sender<SendMsg>) -> Self {
        Self { tx }
    }

    async fn send_to_all(&mut self, msg: SendMsg) -> Result<(), NetworkError> {
        self.tx.send(msg).await?;
        Ok(())
    }

    async fn send_to_id(&mut self, to: u32, id: u32, msg: Message) -> Result<(), NetworkError> {
        self.tx.send(SendMsg::MessageToId { to, id, msg }).await?;
        Ok(())
    }

    async fn send_to_all_except(&mut self, id: u32, msg: Message) -> Result<(), NetworkError> {
        self.tx.send(SendMsg::MessageToAllExcept { id, msg }).await?;
        Ok(())
    }

    pub async fn shutdown(&mut self) -> Result<(), NetworkError> {
        self.tx
            .send(SendMsg::MessageToAll {
                id: None,
                msg: Message::Shutdown,
            })
            .await?;
        self.tx.send(SendMsg::Shutdown).await?;
        Err(NetworkError::Shutdown)
    }

    pub async fn send_all_wizards_to(&mut self, id: u32, wizards: &LobbyWizards) -> Result<(), NetworkError> {
        for wizard in wizards.players() {
            self.send_to_id(id, wizard.id, Message::Join(wizard.player.clone())).await?;
            if wizard.ready {
                self.send_to_id(id, wizard.id, Message::Ready(true)).await?;
            }
        }
        Ok(())
    }

    pub async fn join(&mut self, id: u32, player: &Player) -> Result<(), NetworkError> {
        self.send_to_all(SendMsg::MessageToAll {
            id: Some(id),
            msg: Message::Join(player.clone()),
        })
        .await
    }

    pub async fn ready(&mut self, id: u32, ready: bool) -> Result<(), NetworkError> {
        self.send_to_all(SendMsg::MessageToAll {
            id: Some(id),
            msg: Message::Ready(ready),
        })
        .await
    }

    pub async fn leave(&mut self, id: u32) -> Result<(), NetworkError> {
        self.send_to_all(SendMsg::MessageToAll {
            id: Some(id),
            msg: Message::Leave(id),
        })
        .await
    }

    pub async fn send_wizards(&mut self, wizards: &ServerWizards) -> Result<(), NetworkError> {
        for wizard in wizards.iter() {
            self.send_to_id(wizard.id, wizard.id, Message::Start(wizard.clone())).await?;
        }
        Ok(())
    }

    pub async fn add_wizard(&mut self, wizard: &GameWizard, x: u8, y: u8) -> Result<(), NetworkError> {
        self.send_to_all(SendMsg::MessageToAll {
            id: Some(wizard.id),
            msg: Message::AddWizard {
                wizard: wizard.clone(),
                x,
                y,
            },
        })
        .await
    }

    pub async fn choose_spell(&mut self, id: u32) -> Result<(), NetworkError> {
        self.send_to_id(id, id, Message::ChooseSpell).await
    }

    pub async fn cast_spell(&mut self, id: u32, spell: &Spell) -> Result<(), NetworkError> {
        self.send_to_all(SendMsg::MessageToAll {
            id: Some(id),
            msg: Message::CastSpell {
                spell_name: spell.name.clone(),
                range: spell.range,
            },
        })
        .await
    }

    pub async fn waiting_for_other_players(&mut self, amount: usize) -> Result<(), NetworkError> {
        self.send_to_all(SendMsg::MessageToAll {
            id: None,
            msg: Message::WaitingForOtherPlayers(amount as u32),
        })
        .await
    }

    pub async fn buff_wizard(&mut self, id: u32, stats: &WizardStats) -> Result<(), NetworkError> {
        self.send_to_all(SendMsg::MessageToAll {
            id: Some(id),
            msg: Message::BuffWizard(stats.clone()),
        })
        .await
    }

    pub async fn debuff_wizard(&mut self, id: u32, stats: &WizardStats) -> Result<(), NetworkError> {
        self.send_to_all(SendMsg::MessageToAll {
            id: Some(id),
            msg: Message::DeBuffWizard(stats.clone()),
        })
        .await
    }

    pub async fn spell_succeeds(&mut self, alignment: i8) -> Result<(), NetworkError> {
        self.send_to_all(SendMsg::MessageToAll {
            id: None,
            msg: Message::SpellSucceeds(alignment),
        })
        .await
    }

    pub async fn spell_fails(&mut self) -> Result<(), NetworkError> {
        self.send_to_all(SendMsg::MessageToAll {
            id: None,
            msg: Message::SpellFails,
        })
        .await
    }

    pub async fn creation_spell(&mut self, id: u32, x: u8, y: u8, creation: Option<&GameCreation>) -> Result<(), NetworkError> {
        self.send_to_all(SendMsg::MessageToAll {
            id: Some(id),
            msg: Message::CreationSpell {
                x,
                y,
                creation: creation.cloned(),
            },
        })
        .await
    }

    pub async fn cast_fire(&mut self, id: u32, x: u8, y: u8, fire: Option<&GameCreation>) -> Result<(), NetworkError> {
        self.send_to_all(SendMsg::MessageToAll {
            id: Some(id),
            msg: Message::CastFire {
                x,
                y,
                fire: fire.cloned(),
            },
        })
        .await
    }

    pub async fn cast_blob(&mut self, id: u32, x: u8, y: u8, blob: Option<&GameCreation>) -> Result<(), NetworkError> {
        self.send_to_all(SendMsg::MessageToAll {
            id: Some(id),
            msg: Message::CastBlob {
                x,
                y,
                blob: blob.cloned(),
            },
        })
        .await
    }

    pub async fn disbelieve(&mut self, id: u32, x: u8, y: u8, success: bool) -> Result<(), NetworkError> {
        self.send_to_all(SendMsg::MessageToAll {
            id: Some(id),
            msg: Message::Disbelieve { x, y, success },
        })
        .await
    }

    pub async fn move_wizard(&mut self, id: u32, x: u8, y: u8) -> Result<(), NetworkError> {
        self.send_to_all(SendMsg::MessageToAll {
            id: Some(id),
            msg: Message::MoveWizard { x, y },
        })
        .await
    }

    pub async fn move_creation(&mut self, id: u32, sx: u8, sy: u8, dx: u8, dy: u8) -> Result<(), NetworkError> {
        self.send_to_all(SendMsg::MessageToAll {
            id: Some(id),
            msg: Message::MoveCreation { sx, sy, dx, dy },
        })
        .await
    }

    pub async fn movement_range(&mut self, id: u32, range: u8, flying: bool, tiles: &[(u8, u8)]) -> Result<(), NetworkError> {
        self.send_to_id(
            id,
            id,
            Message::MovementRange {
                range,
                flying,
                tiles: tiles.to_vec(),
            },
        )
        .await
    }

    pub async fn movement_points(&mut self, id: u32, points: u8, tiles: &[(u8, u8)]) -> Result<(), NetworkError> {
        self.send_to_id(
            id,
            id,
            Message::MovementPoints {
                points,
                tiles: tiles.to_vec(),
            },
        )
        .await
    }

    pub async fn ask_for_dismount(&mut self, id: u32) -> Result<(), NetworkError> {
        self.send_to_id(id, id, Message::AskForDismount).await
    }

    pub async fn engaged_in_combat(&mut self, id: u32, tiles: &[(u8, u8)]) -> Result<(), NetworkError> {
        self.send_to_id(id, id, Message::EngagedInCombat(tiles.to_vec())).await
    }

    pub async fn choose_piece(&mut self, id: u32, tiles: &[(u8, u8)]) -> Result<(), NetworkError> {
        self.send_to_id(id, id, Message::ChoosePiece(tiles.to_vec())).await
    }

    pub async fn choose_target(&mut self, id: u32, tiles: &[(u8, u8)]) -> Result<(), NetworkError> {
        self.send_to_id(id, id, Message::ChooseTarget(tiles.to_vec())).await
    }

    pub async fn turn(&mut self, id: u32) -> Result<(), NetworkError> {
        self.send_to_all_except(id, Message::Turn).await
    }

    pub async fn turn_end(&mut self) -> Result<(), NetworkError> {
        self.send_to_all(SendMsg::MessageToAll {
            id: None,
            msg: Message::TurnEnd,
        })
        .await
    }

    pub async fn failed_attack(&mut self, id: u32, x: u8, y: u8) -> Result<(), NetworkError> {
        self.send_to_all(SendMsg::MessageToAll {
            id: Some(id),
            msg: Message::FailedAttack { x, y },
        })
        .await
    }

    pub async fn successful_attack(&mut self, id: u32, x: u8, y: u8, corpse: bool) -> Result<(), NetworkError> {
        self.send_to_all(SendMsg::MessageToAll {
            id: Some(id),
            msg: Message::SuccessfulAttack { x, y, corpse },
        })
        .await
    }

    pub async fn choose_ranged_combat(&mut self, id: u32, range: u8, tiles: &[(u8, u8)]) -> Result<(), NetworkError> {
        self.send_to_id(
            id,
            id,
            Message::ChooseRangedCombat {
                range,
                tiles: tiles.to_vec(),
            },
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn successful_ranged_attack(
        &mut self,
        id: u32,
        sx: u8,
        sy: u8,
        dx: u8,
        dy: u8,
        corpse: bool,
        color: Color,
    ) -> Result<(), NetworkError> {
        self.send_to_all(SendMsg::MessageToAll {
            id: Some(id),
            msg: Message::SuccessfulRangedAttack {
                sx,
                sy,
                dx,
                dy,
                corpse,
                color,
            },
        })
        .await
    }

    pub async fn failed_ranged_attack(
        &mut self,
        id: u32,
        sx: u8,
        sy: u8,
        dx: u8,
        dy: u8,
        color: Color,
    ) -> Result<(), NetworkError> {
        self.send_to_all(SendMsg::MessageToAll {
            id: Some(id),
            msg: Message::FailedRangedAttack { sx, sy, dx, dy, color },
        })
        .await
    }

    pub async fn successful_dragon_ranged_attack(&mut self, id: u32, sx: u8, sy: u8, dx: u8, dy: u8) -> Result<(), NetworkError> {
        self.send_to_all(SendMsg::MessageToAll {
            id: Some(id),
            msg: Message::SuccessfulDragonRangedAttack { sx, sy, dx, dy },
        })
        .await
    }

    pub async fn failed_dragon_ranged_attack(&mut self, id: u32, sx: u8, sy: u8, dx: u8, dy: u8) -> Result<(), NetworkError> {
        self.send_to_all(SendMsg::MessageToAll {
            id: Some(id),
            msg: Message::FailedDragonRangedAttack { sx, sy, dx, dy },
        })
        .await
    }

    pub async fn undead_cannot_be_attacked(&mut self, id: u32) -> Result<(), NetworkError> {
        self.send_to_id(id, id, Message::UndeadCannotBeAttacked).await
    }

    pub async fn results(&mut self, players: &[Player]) -> Result<(), NetworkError> {
        self.send_to_all(SendMsg::MessageToAll {
            id: None,
            msg: Message::Results(players.to_vec()),
        })
        .await
    }

    pub async fn no_line_of_sight(&mut self, id: u32) -> Result<(), NetworkError> {
        self.send_to_id(id, id, Message::NoLineOfSight).await
    }

    pub async fn subversion(&mut self, id: u32, x: u8, y: u8, success: bool) -> Result<(), NetworkError> {
        self.send_to_all(SendMsg::MessageToAll {
            id: Some(id),
            msg: Message::Subversion { x, y, success },
        })
        .await
    }

    pub async fn raise_dead(&mut self, id: u32, x: u8, y: u8, success: bool) -> Result<(), NetworkError> {
        self.send_to_all(SendMsg::MessageToAll {
            id: Some(id),
            msg: Message::RaiseDead { x, y, success },
        })
        .await
    }

    pub async fn magic_bolt(&mut self, id: u32, x: u8, y: u8, success: bool) -> Result<(), NetworkError> {
        self.send_to_all(SendMsg::MessageToAll {
            id: Some(id),
            msg: Message::MagicBolt { x, y, success },
        })
        .await
    }

    pub async fn lightning(&mut self, id: u32, x: u8, y: u8, success: bool) -> Result<(), NetworkError> {
        self.send_to_all(SendMsg::MessageToAll {
            id: Some(id),
            msg: Message::Lightning { x, y, success },
        })
        .await
    }

    pub async fn shelter_disappears(&mut self, x: u8, y: u8) -> Result<(), NetworkError> {
        self.send_to_all(SendMsg::MessageToAll {
            id: None,
            msg: Message::ShelterDisappears { x, y },
        })
        .await
    }

    pub async fn magical_attack(&mut self, id: u32, x: u8, y: u8, success: bool) -> Result<(), NetworkError> {
        self.send_to_all(SendMsg::MessageToAll {
            id: Some(id),
            msg: Message::MagicalAttack { x, y, success },
        })
        .await
    }

    pub async fn spawn_fire(&mut self, x: u8, y: u8, fire: Option<&GameCreation>) -> Result<(), NetworkError> {
        self.send_to_all(SendMsg::MessageToAll {
            id: None,
            msg: Message::SpawnFire {
                x,
                y,
                fire: fire.cloned(),
            },
        })
        .await
    }

    pub async fn spawn_blob(&mut self, x: u8, y: u8, blob: Option<&GameCreation>) -> Result<(), NetworkError> {
        self.send_to_all(SendMsg::MessageToAll {
            id: None,
            msg: Message::SpawnBlob {
                x,
                y,
                blob: blob.cloned(),
            },
        })
        .await
    }

    pub async fn remove_spawn(&mut self, x: u8, y: u8) -> Result<(), NetworkError> {
        self.send_to_all(SendMsg::MessageToAll {
            id: None,
            msg: Message::RemoveSpawn { x, y },
        })
        .await
    }

    pub async fn shadow_wood_info(&mut self, id: u32) -> Result<(), NetworkError> {
        self.send_to_id(id, id, Message::ShadowWoodInfo).await
    }

    pub async fn no_possible_moves(&mut self, id: u32) -> Result<(), NetworkError> {
        self.send_to_id(id, id, Message::NoPossibleMoves).await
    }

    pub async fn send_spell(&mut self, id: u32, spell: &Spell) -> Result<(), NetworkError> {
        self.send_to_id(id, id, Message::SendSpell { spell: spell.clone() }).await
    }

    pub async fn new_spell(&mut self, id: u32, x: u8, y: u8) -> Result<(), NetworkError> {
        self.send_to_all(SendMsg::MessageToAll {
            id: Some(id),
            msg: Message::NewSpell { x, y },
        })
        .await
    }
}

use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

use serenity::{all::GuildId, prelude::TypeMapKey};

use crate::session::{VoiceSessionData, VoiceSessionHandle};

pub struct BotDataKey;

pub struct BotData {
    pub(crate) sessions: HashMap<GuildId, VoiceSessionHandle>,
    pub(crate) downloader_client: reqwest::Client
}

impl TypeMapKey for BotDataKey {
    type Value = BotData;
}

impl Default for BotData {
    fn default() -> Self {
        Self {
            sessions: HashMap::new(),
            downloader_client: reqwest::Client::new()
        }
    }
}
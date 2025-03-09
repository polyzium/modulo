/*
 * This file is part of Modulo.
 *
 * Copyright (C) 2024-present Polyzium
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program. If not, see <https://www.gnu.org/licenses/>.
 */

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
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

use std::{collections::VecDeque, ffi::CString, io::{Read, Seek}, sync::Arc};

use libopenmpt_sys::{openmpt_module, openmpt_module_ctl_set_boolean, openmpt_module_ctl_set_text, openmpt_module_destroy, openmpt_module_get_metadata, openmpt_module_get_num_subsongs, openmpt_module_get_selected_subsong, openmpt_module_read_interleaved_float_stereo, openmpt_module_select_subsong, openmpt_module_set_render_param, OPENMPT_MODULE_RENDER_INTERPOLATIONFILTER_LENGTH};
use serenity::{all::{ChannelId, Context, CreateMessage, GuildId}, prelude::TypeMap};
use songbird::{input::RawAdapter, Call};
use symphonia::core::io::MediaSource;
use tokio::{spawn, sync::{mpsc::{channel, Receiver, Sender}, Mutex, RwLock}};
use anyhow::Result;

use crate::{botdata::BotDataKey, misc::escape_markdown};

// Raw FFI in Rust kinda sucks
// To ensure safety, please use the module in ONLY one session!!!
unsafe impl Send for OpenMptModuleSafe {}
unsafe impl Sync for OpenMptModuleSafe {}
pub struct OpenMptModuleSafe(pub *mut openmpt_module);
impl Drop for OpenMptModuleSafe {
    fn drop(&mut self) {
        unsafe { openmpt_module_destroy(self.0); }
    }
}

pub struct WrappedModule {
    pub filename: String,
    pub filehash: String,
    pub module: OpenMptModuleSafe
}

pub enum VoiceSessionNotificationMessage {
    EndOfQueue,
    PlayingNextInQueue(String),
    PlayingSubsong(i32),
    Leave,
}

pub enum VoiceSessionControlMessage {
    PlayNextInQueue,
    // TODO
}

pub enum Interpolation {
    Default,
    None,
    Linear,
    Cubic,
    Sinc8
}

impl Interpolation {
    pub fn to_openmpt_value(&self) -> i32 {
        match self {
            Interpolation::Default => 0,
            Interpolation::None => 1,
            Interpolation::Linear => 2,
            Interpolation::Cubic => 4,
            Interpolation::Sinc8 => 8,
        }
    }

    pub fn from_openmpt_value(value: i32) -> Self {
        match value {
            0 => Interpolation::Default,
            1 => Interpolation::None,
            2 => Interpolation::Linear,
            4 => Interpolation::Cubic,
            8 => Interpolation::Sinc8,
            _ => panic!("Value out of range")
        }
    }
}

pub struct VoiceSessionData {
    pub(crate) current_module: Option<WrappedModule>,
    pub paused: bool,
    pub(crate) interpolation: Interpolation,
    pub(crate) amiga_enabled: bool,
    pub(crate) amiga_mode: String,
    pub(crate) autosubsong_enabled: bool,
    // pub(crate) context: Context,
    pub(crate) text_channel_id: ChannelId,
    pub(crate) notification_handle: Sender<VoiceSessionNotificationMessage>,
    pub(crate) module_queue: VecDeque<WrappedModule>,
    pub current_vote: Option<crate::vote::Vote>
}

#[derive(Clone)]
pub struct VoiceSessionHandle {
    pub data: Arc<RwLock<VoiceSessionData>>,
    pub control_tx: Sender<VoiceSessionControlMessage>,
    pub call: Arc<Mutex<Call>>,
}

pub struct VoiceSession {
    pub(crate) data: Arc<RwLock<VoiceSessionData>>,
    control_tx: Sender<VoiceSessionControlMessage>,
    control_rx: Receiver<VoiceSessionControlMessage>
}

impl VoiceSession {
    pub fn new(ctx: &Context, text_channel_id: ChannelId) -> (Self, Sender<VoiceSessionControlMessage>) {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<VoiceSessionNotificationMessage>(64);
        let ctx2 = ctx.clone();
        let text_channel_id2 = text_channel_id.clone();

        spawn(async move {
            loop {
                if let Some(message) = rx.recv().await {
                    match message {
                        VoiceSessionNotificationMessage::EndOfQueue => {
                            let _ = text_channel_id2.send_message(&ctx2, CreateMessage::new().content("End of queue reached, no more songs to play"))
                                .await;
                            },
                        VoiceSessionNotificationMessage::Leave => break,
                        VoiceSessionNotificationMessage::PlayingNextInQueue(title) => {
                            let mut escaped_title = escape_markdown(&title);
                            let _ = text_channel_id2.send_message(&ctx2, CreateMessage::new().content("Now playing: **".to_string()+&escaped_title+"**"))
                                .await;
                        },
                        VoiceSessionNotificationMessage::PlayingSubsong(subsong_number) => {
                            let _ = text_channel_id2.send_message(&ctx2, CreateMessage::new().content(format!("Playing subsong {subsong_number}")))
                                .await;
                        }
                    };
                }
            }
        });

        let (control_tx, control_rx) = channel::<VoiceSessionControlMessage>(8);
        let this = Self {
            data: Arc::new(RwLock::new(VoiceSessionData {
                current_module: None,
                paused: false,
                interpolation: Interpolation::Default,
                amiga_enabled: false,
                amiga_mode: "auto".to_owned(),
                autosubsong_enabled: false,
                // context: ctx.clone(),
                text_channel_id,
                notification_handle: tx,
                module_queue: VecDeque::with_capacity(16),
                current_vote: None,
            })),
            control_rx,
            control_tx: control_tx.clone(),
        };

        (this, control_tx)
    }

    fn play_next_in_queue(&self) {
        let mut data_l = self.data.blocking_write();
        if data_l.module_queue.len() == 0 {
            data_l.current_module = None;
            data_l.notification_handle.blocking_send(VoiceSessionNotificationMessage::EndOfQueue).unwrap();
        } else {
            let Some(queued_module) = data_l.module_queue.pop_front() else { unreachable!() };
            data_l.current_module = Some(queued_module);
            let Some(current_module) = &data_l.current_module else { unreachable!() };
            unsafe {
                openmpt_module_set_render_param(
                    current_module.module.0,
                    OPENMPT_MODULE_RENDER_INTERPOLATIONFILTER_LENGTH as std::os::raw::c_int,
                    data_l.interpolation.to_openmpt_value()
                );

                let ctl = CString::new("render.resampler.emulate_amiga").unwrap();
                openmpt_module_ctl_set_boolean(current_module.module.0, ctl.as_ptr(), data_l.amiga_enabled as i32);

                let ctl = CString::new("render.resampler.emulate_amiga_type").unwrap();
                let value = CString::new(data_l.amiga_mode.clone()).unwrap();
                openmpt_module_ctl_set_text(current_module.module.0, ctl.as_ptr(), value.as_ptr());
            };
            let Some(module) = &data_l.current_module else { unreachable!() };

            let key = std::ffi::CString::new("title").unwrap();
            let title_raw = unsafe {openmpt_module_get_metadata(module.module.0, key.as_ptr())};
            let mut module_title = unsafe {std::ffi::CStr::from_ptr(title_raw)}
                .to_str().unwrap();
            if module_title.is_empty() {
                module_title = &module.filename;
            }

            data_l.notification_handle.blocking_send(VoiceSessionNotificationMessage::PlayingNextInQueue(module_title.to_string())).unwrap();
        }
    }
}

impl Read for VoiceSession {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let (_r_begin, floats, _r_end) = unsafe { buf.align_to_mut::<f32>() };
        floats.fill(0.0); // Fill with silence
        let data_l = self.data.blocking_write();
        if let Some(module_wrapped) = &data_l.current_module {
            if !data_l.paused {
                unsafe {
                    let frames_read = openmpt_module_read_interleaved_float_stereo(module_wrapped.module.0, 48000, floats.len()/2, floats.as_mut_ptr());
                    if frames_read < floats.len()/2 {
                        if !data_l.autosubsong_enabled {
                            self.control_tx.blocking_send(VoiceSessionControlMessage::PlayNextInQueue).unwrap();
                        } else {
                            let current_subsong = openmpt_module_get_selected_subsong(module_wrapped.module.0);
                            if current_subsong == openmpt_module_get_num_subsongs(module_wrapped.module.0) - 1 {
                                self.control_tx.blocking_send(VoiceSessionControlMessage::PlayNextInQueue).unwrap();
                            } else {
                                openmpt_module_select_subsong(module_wrapped.module.0, current_subsong+1);
                                data_l.notification_handle.blocking_send(VoiceSessionNotificationMessage::PlayingSubsong(current_subsong + 1)).unwrap();
                            }
                        }
                    }
                }
            }
        }
        drop(data_l);

        if let Ok(controlmsg) = self.control_rx.try_recv() {
            match controlmsg {
                VoiceSessionControlMessage::PlayNextInQueue => { self.play_next_in_queue(); },
            }
        }

        Ok(buf.len())
    }
}

impl Seek for VoiceSession {
    fn seek(&mut self, _pos: std::io::SeekFrom) -> std::io::Result<u64> {
        // See is_seekable below
        unreachable!()
    }
}

impl MediaSource for VoiceSession {
    fn is_seekable(&self) -> bool {
        // Sessions are not seekable, module seeking is handled by libopenmpt.
        false
    }

    fn byte_len(&self) -> Option<u64> {
        None
    }
}

pub async fn initiate_session(ctx: &Context, guild_id: GuildId, voice_channel_id: ChannelId, text_channel_id: ChannelId) -> Result<VoiceSessionHandle> {
    {
        let mut lock = ctx.data.write().await;
        let botdata = lock.get_mut::<crate::BotDataKey>().unwrap();
        if let Some(_) = botdata.sessions.get(&guild_id) {
            return Err(anyhow::anyhow!("The bot is already in the voice channel or a session already exists for this guild id"));
        }
    }

    let manager = songbird::get(&ctx)
    .await
    .expect("Songbird Voice client placed in at initialisation.")
    .clone();

    match manager.join(guild_id, voice_channel_id).await {
        Ok(handler_lock) => {
            let mut handler = handler_lock.lock().await;

            let (session, control) = VoiceSession::new(&ctx, text_channel_id);

            let mut lock = ctx.data.write().await;
            let botdata = lock.get_mut::<BotDataKey>().unwrap();
            let handle = VoiceSessionHandle {
                data: session.data.clone(),
                control_tx: control,
                call: handler_lock.clone(),
            };
            let handle2 = handle.clone();
            botdata.sessions.insert(guild_id, handle);

            let pcm = RawAdapter::new(session, 48000, 2);
            let _ = handler.play_input(pcm.into());

            return Ok(handle2);
        }
        Err(err) => {
            return Err(anyhow::Error::new(err));
        },
    }
}
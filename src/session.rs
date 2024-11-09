use std::{collections::VecDeque, io::{Read, Seek}, sync::Arc};

use libopenmpt_sys::{openmpt_module, openmpt_module_destroy, openmpt_module_get_metadata, openmpt_module_read_interleaved_float_stereo};
use serenity::{all::{ChannelId, Context, CreateMessage, GuildId}, prelude::TypeMap};
use songbird::input::RawAdapter;
use symphonia::core::io::MediaSource;
use tokio::{spawn, sync::{mpsc::{channel, Receiver, Sender}, RwLock}};
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
    // pub filename: String,
    pub filehash: String,
    pub module: OpenMptModuleSafe
}

pub enum VoiceSessionNotificationMessage {
    EndOfQueue,
    PlayingNextInQueue(String),
    Leave,
}

pub enum VoiceSessionControlMessage {
    PlayNextInQueue,
    // TODO
}

pub struct VoiceSessionData {
    pub(crate) current_module: Option<WrappedModule>,
    pub paused: bool,
    // pub(crate) context: Context,
    // pub(crate) text_channel_id: ChannelId,
    pub(crate) notification_handle: Sender<VoiceSessionNotificationMessage>,
    pub(crate) module_queue: VecDeque<WrappedModule>,
    pub current_vote: Option<crate::vote::Vote>
}

#[derive(Clone)]
pub struct VoiceSessionHandle {
    pub data: Arc<RwLock<VoiceSessionData>>,
    pub control_tx: Sender<VoiceSessionControlMessage>
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
                            if escaped_title.is_empty() {
                                escaped_title = "[No title]".to_string();
                            }
                            let _ = text_channel_id2.send_message(&ctx2, CreateMessage::new().content("Now playing: **".to_string()+&escaped_title+"**"))
                                .await;
                        },
                    };
                }
            }
        });

        let (control_tx, control_rx) = channel::<VoiceSessionControlMessage>(8);
        let this = Self {
            data: Arc::new(RwLock::new(VoiceSessionData {
                current_module: None,
                paused: false,
                // context: ctx.clone(),
                // text_channel_id,
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
            let Some(module) = &data_l.current_module else { unreachable!() };

            let key = std::ffi::CString::new("title").unwrap();
            let title_raw = unsafe {openmpt_module_get_metadata(module.module.0, key.as_ptr())};
            let module_title = unsafe {std::ffi::CStr::from_ptr(title_raw)}
                .to_str().unwrap();

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
                        self.control_tx.blocking_send(VoiceSessionControlMessage::PlayNextInQueue).unwrap();
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
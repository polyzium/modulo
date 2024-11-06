use std::{io::{Read, Seek}, sync::{Arc, RwLock}};

use libopenmpt_sys::{openmpt_module, openmpt_module_read_interleaved_float_stereo};
use serenity::all::{ChannelId, Context, CreateMessage};
use symphonia::core::io::MediaSource;
use tokio::{spawn, sync::mpsc::Sender};

// Raw FFI in Rust kinda sucks
// To ensure safety, please use the module in ONLY one session!!!
unsafe impl Send for OpenMptModuleSafe {}
unsafe impl Sync for OpenMptModuleSafe {}
pub struct OpenMptModuleSafe(pub *mut openmpt_module);

pub enum VoiceSessionNotificationMessage {
    EndOfSong,
    Leave,
}

pub struct VoiceSessionData {
    pub(crate) module: Option<OpenMptModuleSafe>,
    pub(crate) context: Context,
    pub(crate) text_channel_id: ChannelId,
    pub(crate) async_handle: Sender<VoiceSessionNotificationMessage>,
}

// impl VoiceSessionData {
//     pub fn shutdown(&self) {
//         self.async_handle.blocking_send(VoiceSessionNotificationMessage::Leave).unwrap();
//     }

//     // pub async fn shutdown_async(&self) {
//     //     self.async_handle.send(VoiceSessionNotificationMessage::Leave).await.unwrap();
//     // }
// }

pub struct VoiceSession {
    pub(crate) data: Arc<RwLock<VoiceSessionData>>,
}

impl VoiceSession {
    pub fn new(ctx: &Context, text_channel_id: ChannelId) -> Self {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<VoiceSessionNotificationMessage>(64);
        let ctx2 = ctx.clone();
        let text_channel_id2 = text_channel_id.clone();

        spawn(async move {
            loop {
                if let Some(message) = rx.recv().await {
                    match message {
                        VoiceSessionNotificationMessage::EndOfSong => {
                            let _ = text_channel_id2.send_message(&ctx2, CreateMessage::new().content("End of song reached, stopped playback"))
                                .await;
                            },
                        VoiceSessionNotificationMessage::Leave => break,
                    };
                }
            }
        });

        Self {
            data: Arc::new(RwLock::new(VoiceSessionData {
                module: None,
                context: ctx.clone(),
                text_channel_id,
                async_handle: tx
            })),
        }
    }
}

impl Read for VoiceSession {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let (_r_begin, floats, _r_end) = unsafe { buf.align_to_mut::<f32>() };
        floats.fill(0.0); // Fill with silence
        let mut data_l = self.data.write().unwrap();
        if let Some(module) = &data_l.module {
            unsafe {
                let frames_read = openmpt_module_read_interleaved_float_stereo(module.0, 48000, floats.len()/2, floats.as_mut_ptr());
                if frames_read < floats.len()/2 {
                    data_l.module = None;
                    data_l.async_handle.blocking_send(VoiceSessionNotificationMessage::EndOfSong).unwrap();
                }
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
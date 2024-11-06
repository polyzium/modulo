use serenity::all::{ChannelId, ChannelType, CommandInteraction, Context, CreateInteractionResponse, CreateInteractionResponseMessage};
use serenity::builder::CreateCommand;
use songbird::input::RawAdapter;
use symphonia::core::io::MediaSource;

use std::f32::consts::TAU;
use std::io::{ErrorKind, Read, Seek};
use std::sync::{Arc, Mutex};

pub struct GoertzelOscillator {
    coeff: f32,
    last_sample: f32,
    current_sample: f32,
}

impl GoertzelOscillator {
    pub fn new(freq: f32, phase: f32, samplerate: u32) -> GoertzelOscillator {
        let normalized_freq = freq / samplerate as f32;
        let coeff = 2.0 * (normalized_freq * TAU).cos();

        GoertzelOscillator {
            coeff,
            last_sample: phase.cos(),
            current_sample: (normalized_freq * TAU + phase).cos(),
        }
    }

    pub fn process(&mut self) -> f32 {
        let next_sample = self.current_sample * self.coeff - self.last_sample;

        self.last_sample = self.current_sample;
        self.current_sample = next_sample;

        next_sample
    }
}

impl Read for GoertzelOscillator {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let buflen = buf.len();
        let (_r_begin, floats, _r_end) = unsafe { buf.align_to_mut::<f32>() };
        for out in floats.iter_mut() {
            *out = self.process()
        };

        log::trace!("GoertzelOscillator::read called, {} bytes in, {} bytes out", buflen, floats.len()*4);

        Ok(floats.len()*4)
    }
}

impl Seek for GoertzelOscillator {
    fn seek(&mut self, _pos: std::io::SeekFrom) -> std::io::Result<u64> {
        Err(ErrorKind::Unsupported.into())
    }
}

impl MediaSource for GoertzelOscillator {
    fn is_seekable(&self) -> bool {
        false
    }

    fn byte_len(&self) -> Option<u64> {
        None
    }
}

pub async fn handle(ctx: Context, interaction: &CommandInteraction) {
    let (guild_id, voice_channel_id) = {
        let guild_id = interaction.guild_id.unwrap();
        let voice_channel_id: Option<ChannelId> = {
            let channels = guild_id
            .to_partial_guild(&ctx.http).await.unwrap()
            .channels(&ctx.http).await.unwrap();

            let channel = channels.values()
                .find(|channel| {
                    if channel.kind != ChannelType::Voice { return false };
                    let members = channel.members(&ctx).unwrap();
                    let member = members
                        .iter()
                        .find(|member| member.user.id == interaction.user.id);
                    member.is_some()
                });
            match channel {
                Some(channel) => Some(channel.id),
                None => None,
            }
        };

        (guild_id, voice_channel_id)
    };

    let connect_to = match voice_channel_id {
        Some(channel) => channel,
        None => {
            let response = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                .content("Not in a voice channel".to_string())
            );
            interaction.create_response(ctx.http, response).await.unwrap();
            return;
        },
    };

    let manager = songbird::get(&ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Ok(handler_lock) = manager.join(guild_id, connect_to).await {
        let mut handler = handler_lock.lock().await;

        let osc = GoertzelOscillator::new(1000.0, 0.0, 48000);
        let src = RawAdapter::new(osc, 48000, 1);
        let _ = handler.play_input(src.into());
    }
}

pub fn register() -> CreateCommand {
    CreateCommand::new("test").description("Play a 1000 hz sine wave")
}
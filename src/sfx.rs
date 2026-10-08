//! Short synthesized sound effects. The game ships no asset files, so the
//! hits, parries, and KOs are built as tiny WAV buffers at startup.

use raylib::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cue {
    Hit,
    Parry,
    Block,
    Ult,
    Ko,
    Swing,
    Jump,
}

pub struct Bank<'a> {
    hit: Sound<'a>,
    parry: Sound<'a>,
    block: Sound<'a>,
    ult: Sound<'a>,
    ko: Sound<'a>,
    swing: Sound<'a>,
    jump: Sound<'a>,
}

impl<'a> Bank<'a> {
    pub fn load(audio: &'a RaylibAudio) -> Option<Self> {
        Some(Bank {
            hit: sound(audio, &thud(90.0, 0.09, 0.45))?,
            parry: sound(audio, &tone(880.0, 0.12, 0.35, 18.0))?,
            block: sound(audio, &tone(180.0, 0.07, 0.3, 30.0))?,
            ult: sound(audio, &sweep(220.0, 880.0, 0.32, 0.4))?,
            ko: sound(audio, &thud(55.0, 0.38, 0.55))?,
            swing: sound(audio, &noise(0.05, 0.22))?,
            jump: sound(audio, &tone(520.0, 0.06, 0.22, 22.0))?,
        })
    }

    pub fn play(&self, cue: Cue) {
        let sound = match cue {
            Cue::Hit => &self.hit,
            Cue::Parry => &self.parry,
            Cue::Block => &self.block,
            Cue::Ult => &self.ult,
            Cue::Ko => &self.ko,
            Cue::Swing => &self.swing,
            Cue::Jump => &self.jump,
        };
        sound.play();
    }
}

fn sound<'a>(audio: &'a RaylibAudio, samples: &[i16]) -> Option<Sound<'a>> {
    let bytes = pcm_wav(samples, 22_050);
    let wave = audio.new_wave_from_memory(".wav", &bytes).ok()?;
    audio.new_sound_from_wave(&wave).ok()
}

pub fn pcm_wav(samples: &[i16], rate: u32) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for sample in samples {
        out.extend_from_slice(&sample.to_le_bytes());
    }
    out
}

fn tone(freq: f32, dur: f32, vol: f32, decay: f32) -> Vec<i16> {
    let sr = 22_050.0;
    let n = (sr * dur) as usize;
    (0..n)
        .map(|i| {
            let t = i as f32 / sr;
            let env = (-decay * t).exp() * (1.0 - t / dur).max(0.0);
            let s = (t * freq * std::f32::consts::TAU).sin();
            (s * env * vol * 32_000.0) as i16
        })
        .collect()
}

fn noise(dur: f32, vol: f32) -> Vec<i16> {
    let sr = 22_050.0;
    let n = (sr * dur) as usize;
    let mut state = 0x1234_5678u32;
    (0..n)
        .map(|i| {
            state = state.wrapping_mul(1664525).wrapping_add(1013904223);
            let t = i as f32 / sr;
            let env = (1.0 - t / dur).powi(2);
            let s = (state >> 16) as i16 as f32 / 32_768.0;
            (s * env * vol * 32_000.0) as i16
        })
        .collect()
}

fn thud(freq: f32, dur: f32, vol: f32) -> Vec<i16> {
    let body = tone(freq, dur, vol, 8.0);
    let grit = noise(dur * 0.45, vol * 0.45);
    body.into_iter()
        .enumerate()
        .map(|(i, s)| s.saturating_add(*grit.get(i).unwrap_or(&0)))
        .collect()
}

fn sweep(from: f32, to: f32, dur: f32, vol: f32) -> Vec<i16> {
    let sr = 22_050.0;
    let n = (sr * dur) as usize;
    let mut phase = 0.0f32;
    (0..n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let freq = from + (to - from) * t;
            phase += freq / sr;
            let env = (t * (1.0 - t) * 4.0).min(1.0);
            let s = (phase * std::f32::consts::TAU).sin();
            (s * env * vol * 32_000.0) as i16
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_header_describes_mono_pcm() {
        let bytes = pcm_wav(&[0, 1000, -1000], 22_050);
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        assert_eq!(&bytes[36..40], b"data");
        assert_eq!(bytes.len(), 44 + 6);
    }
}

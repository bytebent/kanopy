//! Minimal WAV writer: interleaved stereo PCM, 16- or 24-bit, streamed to any `Write + Seek`.

use std::io::{self, Seek, SeekFrom, Write};

pub struct WavWriter<W: Write + Seek> {
    w: W,
    bits: u16,
    frames: u32,
}

impl<W: Write + Seek> WavWriter<W> {
    pub fn new(mut w: W, sample_rate: u32, bits: u16) -> io::Result<Self> {
        assert!(bits == 16 || bits == 24);
        let block_align = 2 * bits / 8;
        w.write_all(b"RIFF\0\0\0\0WAVEfmt ")?;
        w.write_all(&16u32.to_le_bytes())?;
        w.write_all(&1u16.to_le_bytes())?; // PCM
        w.write_all(&2u16.to_le_bytes())?;
        w.write_all(&sample_rate.to_le_bytes())?;
        w.write_all(&(sample_rate * block_align as u32).to_le_bytes())?;
        w.write_all(&block_align.to_le_bytes())?;
        w.write_all(&bits.to_le_bytes())?;
        w.write_all(b"data\0\0\0\0")?;
        Ok(Self { w, bits, frames: 0 })
    }

    /// Write frames; samples are clipped to [-1, 1].
    pub fn write(&mut self, left: &[f32], right: &[f32]) -> io::Result<()> {
        let mut buf = Vec::with_capacity(left.len() * 6);
        for (&l, &r) in left.iter().zip(right) {
            for x in [l, r] {
                let x = x.clamp(-1.0, 1.0);
                if self.bits == 16 {
                    buf.extend_from_slice(&((x * 32767.0).round() as i16).to_le_bytes());
                } else {
                    let v = (x * 8_388_607.0).round() as i32;
                    buf.extend_from_slice(&v.to_le_bytes()[..3]);
                }
            }
        }
        self.frames += left.len() as u32;
        self.w.write_all(&buf)
    }

    pub fn finish(mut self) -> io::Result<W> {
        let data = self.frames * 2 * self.bits as u32 / 8;
        self.w.seek(SeekFrom::Start(4))?;
        self.w.write_all(&(36 + data).to_le_bytes())?;
        self.w.seek(SeekFrom::Start(40))?;
        self.w.write_all(&data.to_le_bytes())?;
        self.w.flush()?;
        Ok(self.w)
    }
}

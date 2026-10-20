//! WAV: interleaved stereo PCM, 16- or 24-bit, streamed to any `Write + Seek`, with optional
//! extra chunks before the audio (kanopy stores its forest file in a `knpy` chunk). Also reads
//! those chunks back.

use std::io::{self, Read, Seek, SeekFrom, Write};

pub struct WavWriter<W: Write + Seek> {
    w: W,
    bits: u16,
    frames: u64,
    data_size_at: u64,
    header_len: u64,
}

impl<W: Write + Seek> WavWriter<W> {
    pub fn new(w: W, sample_rate: u32, bits: u16) -> io::Result<Self> {
        Self::with_chunks(w, sample_rate, bits, &[])
    }

    /// Extra chunks (id, body) go between the format and the audio.
    pub fn with_chunks(
        mut w: W,
        sample_rate: u32,
        bits: u16,
        chunks: &[([u8; 4], Vec<u8>)],
    ) -> io::Result<Self> {
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
        let mut pos = 36u64;
        for (id, body) in chunks {
            w.write_all(id)?;
            w.write_all(&(body.len() as u32).to_le_bytes())?;
            w.write_all(body)?;
            pos += 8 + body.len() as u64;
            if body.len() % 2 == 1 {
                w.write_all(&[0])?; // chunks are word-aligned
                pos += 1;
            }
        }
        w.write_all(b"data\0\0\0\0")?;
        Ok(Self {
            w,
            bits,
            frames: 0,
            data_size_at: pos + 4,
            header_len: pos + 8,
        })
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
        self.frames += left.len() as u64;
        self.w.write_all(&buf)
    }

    pub fn finish(mut self) -> io::Result<W> {
        let data = self.frames * 2 * self.bits as u64 / 8;
        let riff = (self.header_len - 8 + data).min(u32::MAX as u64) as u32;
        self.w.seek(SeekFrom::Start(4))?;
        self.w.write_all(&riff.to_le_bytes())?;
        self.w.seek(SeekFrom::Start(self.data_size_at))?;
        self.w
            .write_all(&(data.min(u32::MAX as u64) as u32).to_le_bytes())?;
        self.w.flush()?;
        Ok(self.w)
    }
}

/// The body of the first chunk with this id in a WAV file, if any.
pub fn read_chunk<R: Read + Seek>(mut r: R, id: &[u8; 4]) -> io::Result<Option<Vec<u8>>> {
    let mut head = [0u8; 12];
    r.read_exact(&mut head)?;
    if &head[0..4] != b"RIFF" || &head[8..12] != b"WAVE" {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "not a WAV file"));
    }
    loop {
        let mut ch = [0u8; 8];
        if r.read_exact(&mut ch).is_err() {
            return Ok(None);
        }
        let len = u32::from_le_bytes([ch[4], ch[5], ch[6], ch[7]]) as u64;
        if &ch[0..4] == id {
            let mut body = vec![0u8; len as usize];
            r.read_exact(&mut body)?;
            return Ok(Some(body));
        }
        if &ch[0..4] == b"data" {
            return Ok(None); // kanopy writes its chunks before the audio
        }
        r.seek(SeekFrom::Current((len + len % 2) as i64))?;
    }
}

use crate::DONACDUM_MP3_BYTES;
use anyhow::Context;
use symphonia::core::audio::Signal;

pub fn decode_raw_audio_buffer() -> anyhow::Result<(symphonia::core::audio::SignalSpec, Vec<f32>)> {
    let mut raw_audio_buffer: Vec<f32> = Vec::with_capacity(1024 * 5);
    let mut hint = symphonia::core::probe::Hint::new();
    hint.with_extension("mp3");

    let media_source = symphonia::core::io::MediaSourceStream::new(
        Box::new(std::io::Cursor::new(DONACDUM_MP3_BYTES)),
        Default::default(),
    );

    let mut probed = symphonia::default::get_probe()
        .format(
            &hint,
            media_source,
            &Default::default(),
            &Default::default(),
        )
        .context("failed to probe")?;

    let track = probed
        .format
        .default_track()
        .context("missing default track")?;
    let track_id = track.id;

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &Default::default())
        .context("failed to make decoder")?;

    let mut spec = None;

    loop {
        let packet = match probed.format.next_packet() {
            Ok(packet) => packet,
            Err(symphonia::core::errors::Error::IoError(e)) => {
                if e.kind() == std::io::ErrorKind::UnexpectedEof {
                    break;
                } else {
                    anyhow::bail!("Failed to get next packet: {:?}", e);
                }
            }
            Err(e) => {
                anyhow::bail!("Failed to get next packet: {:?}", e);
            }
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = decoder.decode(&packet).context("packet decode failed")?;

        let decoded = match decoded {
            symphonia::core::audio::AudioBufferRef::F32(decoded) => decoded,
            _ => {
                anyhow::bail!("Unsupported mp3 audio buffer type");
            }
        };

        raw_audio_buffer.reserve(decoded.chan(0).len() * 2);
        for (a, b) in decoded
            .chan(0)
            .iter()
            .copied()
            .zip(decoded.chan(1).iter().copied())
        {
            raw_audio_buffer.push(a);
            raw_audio_buffer.push(b);
        }

        if spec.is_none() {
            spec = Some(*decoded.spec());
        }
    }
    decoder.close();

    let spec = spec.context("missing spec")?;

    Ok((spec, raw_audio_buffer))
}

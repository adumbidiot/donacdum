///! https://gamedev.net/forums/topic/699061-implementing-flac-playback-through-wasapi/5391519/
mod util;

use self::util::decode_raw_audio_buffer;
use anyhow::Context;
use std::convert::TryInto;
use std::os::windows::raw::HANDLE;
use std::sync::Arc;
use win_core_audio::AudioClientShareMode;
use win_core_audio::DataFlow;
use win_core_audio::DeviceState;
use win_core_audio::MultiMediaDeviceEnumerator;
use winapi::shared::minwindef::FALSE;
use winapi::shared::winerror::FAILED;
use winapi::um::combaseapi::CoInitializeEx;
use winapi::um::handleapi::CloseHandle;
use winapi::um::objbase::COINIT_APARTMENTTHREADED;
use winapi::um::synchapi::CreateEventW;
use winapi::um::synchapi::WaitForSingleObject;

const DONACDUM_MP3_BYTES: &[u8] =
    include_bytes!("../assets/Payday 2 - DonAcDum EarRape-311954012.mp3");

pub fn init_sta_com_runtime() -> std::io::Result<()> {
    let code = unsafe { CoInitializeEx(std::ptr::null_mut(), COINIT_APARTMENTTHREADED) };
    if FAILED(code) {
        return Err(std::io::Error::from_raw_os_error(code));
    }
    Ok(())
}

pub struct Event(HANDLE);

impl Event {
    pub fn new() -> std::io::Result<Self> {
        let handle =
            unsafe { CreateEventW(std::ptr::null_mut(), FALSE, FALSE, std::ptr::null_mut()) };
        if handle.is_null() {
            return Err(std::io::Error::last_os_error());
        }

        Ok(Self(handle.cast()))
    }
}

impl Drop for Event {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0.cast());
        }
    }
}

fn main() {
    let code = match real_main() {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("{:?}", e);
            1
        }
    };
    std::process::exit(code);
}

fn real_main() -> anyhow::Result<()> {
    let (spec, raw_audio_buffer) =
        decode_raw_audio_buffer().context("failed to decode raw audio buffer")?;
    let raw_audio_buffer: Arc<Vec<_>> = Arc::from(raw_audio_buffer);

    // The stream is 2 channel f32 pcm here at 44.1 khz
    eprintln!("Hertz: {}", spec.rate);
    eprintln!("# of Channels: {}", spec.channels.count());

    init_sta_com_runtime().context("failed to init com runtime")?;

    let device_enumerator =
        MultiMediaDeviceEnumerator::new().context("failed to create device enumerator")?;

    let audio_devices_collection = device_enumerator
        .enum_audio_endpoints(DataFlow::Render, DeviceState::ACTIVE)
        .context("failed to enumerate audio endpoints")?;

    let num_audio_devices = audio_devices_collection
        .get_count()
        .context("failed to get # of audio devices")?;

    eprintln!("Located {} audio devices", num_audio_devices);

    let mut handles = Vec::with_capacity(num_audio_devices.try_into().unwrap_or(0));

    let share_mode = AudioClientShareMode::Shared;
    for i in 0..num_audio_devices {
        let raw_audio_buffer = raw_audio_buffer.clone();
        let handle = std::thread::spawn(move || {
            init_sta_com_runtime().context("failed to init com runtime")?;

            let device_enumerator =
                MultiMediaDeviceEnumerator::new().context("failed to create device enumerator")?;

            let audio_devices_collection = device_enumerator
                .enum_audio_endpoints(DataFlow::Render, DeviceState::ACTIVE)
                .context("failed to enumerate audio endpoints")?;

            let audio_device = audio_devices_collection
                .item(i)
                .context("failed to get audio device")?;

            /*
            // Maximize pain
            {
                let audio_endpoint_volume = audio_device
                    .activate_audio_endpoint_volume()
                    .context("failed to get audio endpoint volume interface")?;
                audio_endpoint_volume
                    .set_master_volume_level_scalar(1.0)
                    .context("failed to set master volume")?;
                audio_endpoint_volume
                    .set_mute(false)
                    .context("failed to set mute")?;
            }
            */

            let audio_client = audio_device
                .activate_audio_client()
                .context("failed to get audio client")?;

            let (_default_period, minimum_period) = audio_client
                .get_device_period()
                .context("failed to get device period")?;

            let mix_format = audio_client
                .get_mix_format()
                .context("failed to get mix format")?;

            /*
            let test_format = dbg!(WaveFormatExtensible::new(
                WaveFormatType::Float,
                spec.channels.count().try_into().unwrap(),
                spec.rate,
                32
            ));

            dbg!(audio_client.is_format_supported(share_mode, &test_format));
            */

            let audio_buffer = samplerate::convert(
                spec.rate,
                mix_format.samples_per_sec(),
                mix_format.num_channels().into(),
                samplerate::ConverterType::SincBestQuality,
                &raw_audio_buffer,
            )
            .context("failed to convert audio buffer")?;

            let mut audio_buffer_iter = audio_buffer.chunks(2);

            audio_client
                .initialize(share_mode, minimum_period, minimum_period, &mix_format)
                .context("failed to initialize audio client")?;

            let event_handle = Event::new().context("failed to make event handle")?;
            audio_client
                .set_event_handle(event_handle.0.cast())
                .context("failed to set event handle")?;

            let mut buffer_size = audio_client
                .get_buffer_size()
                .context("failed to get buffer size")?;

            let render_client = audio_client
                .get_service_audio_render_client()
                .context("failed to get render client")?;

            unsafe {
                let ptr = render_client
                    .get_buffer(buffer_size)
                    .context("failed to get buffer")?;
                for i in 0..(buffer_size as usize) {
                    let data = audio_buffer_iter
                        .next()
                        .context("failed to preload buffer")?;
                    ptr.cast::<f32>().add(i * 2).write(data[0]);
                    ptr.cast::<f32>().add(i * 2 + 1).write(data[1]);
                }
                render_client
                    .release_buffer(buffer_size)
                    .context("failed to release buffer")?;
            }

            audio_client.start().context("failed to start")?;

            loop {
                // Wait
                unsafe {
                    let ret = WaitForSingleObject(event_handle.0.cast(), 0xFFFFFFFF);
                    // eprintln!("{:X}", ret);
                }

                // Update current buffer size
                {
                    buffer_size = audio_client
                        .get_buffer_size()
                        .context("failed to get buffer size")?;
                    // println!("Buffer Size: {}", buffer_size);

                    let current_padding = audio_client
                        .get_current_padding()
                        .context("failed to get current padding")?;
                    // println!("Current Padding: {}", current_padding);
                    buffer_size -= current_padding;
                }

                if buffer_size != 0 {
                    unsafe {
                        let ptr = render_client
                            .get_buffer(buffer_size)
                            .context("failed to get buffer")?;
                        for i in 0..(buffer_size as usize) {
                            let data = match audio_buffer_iter.next() {
                                Some(data) => data,
                                None => {
                                    audio_buffer_iter = audio_buffer.chunks(2);
                                    audio_buffer_iter
                                        .next()
                                        .expect("just refreshed iter is empty")
                                }
                            };
                            ptr.cast::<f32>().add(i * 2).write(data[0]);
                            ptr.cast::<f32>().add(i * 2 + 1).write(data[1]);
                        }
                        render_client
                            .release_buffer(buffer_size)
                            .context("failed to release buffer")?;
                    }
                }
            }

            std::thread::sleep(std::time::Duration::from_millis(1000));
            audio_client.stop().context("failed to stop")?;

            Result::<_, anyhow::Error>::Ok(())
        });

        handles.push(handle);
    }

    for handle in handles {
        let _ = handle.join().is_ok();
    }

    Ok(())
}

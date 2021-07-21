mod mmdeviceapi;

pub use self::mmdeviceapi::*;
use std::convert::TryFrom;
use std::convert::TryInto;
use std::os::windows::raw::HANDLE;
use std::ptr::NonNull;
use std::time::Duration;
use winapi::shared::basetsd::UINT32;
use winapi::shared::guiddef::GUID;
use winapi::shared::ksmedia::KSDATAFORMAT_SUBTYPE_IEEE_FLOAT;
use winapi::shared::ksmedia::KSDATAFORMAT_SUBTYPE_PCM;
use winapi::shared::minwindef::DWORD;
use winapi::shared::minwindef::FALSE;
use winapi::shared::minwindef::TRUE;
use winapi::shared::minwindef::UINT;
use winapi::shared::minwindef::WORD;
use winapi::shared::mmreg::WAVEFORMATEX;
use winapi::shared::mmreg::WAVEFORMATEXTENSIBLE;
use winapi::shared::mmreg::WAVE_FORMAT_EXTENSIBLE;
use winapi::shared::mmreg::WAVE_FORMAT_IEEE_FLOAT;
use winapi::shared::mmreg::WAVE_FORMAT_PCM;
use winapi::shared::winerror::FAILED;
use winapi::shared::winerror::S_OK;
use winapi::shared::wtypes::PROPERTYKEY;
use winapi::shared::wtypes::VT_EMPTY;
use winapi::shared::wtypes::VT_LPWSTR;
use winapi::um::audioclient::IAudioClient;
use winapi::um::audioclient::IAudioRenderClient;
use winapi::um::audioclient::IID_IAudioClient;
use winapi::um::audioclient::IID_IAudioRenderClient;
use winapi::um::audiosessiontypes::AUDCLNT_SHAREMODE;
use winapi::um::audiosessiontypes::AUDCLNT_SHAREMODE_EXCLUSIVE;
use winapi::um::audiosessiontypes::AUDCLNT_SHAREMODE_SHARED;
use winapi::um::audiosessiontypes::AUDCLNT_STREAMFLAGS_EVENTCALLBACK;
use winapi::um::combaseapi::CoTaskMemFree;
use winapi::um::combaseapi::CLSCTX_ALL;
use winapi::um::coml2api::STGM_READ;
use winapi::um::coml2api::STGM_READWRITE;
use winapi::um::coml2api::STGM_WRITE;
use winapi::um::endpointvolume::IAudioEndpointVolume;
use winapi::um::functiondiscoverykeys_devpkey::PKEY_DeviceInterface_FriendlyName;
use winapi::um::functiondiscoverykeys_devpkey::PKEY_Device_DeviceDesc;
use winapi::um::functiondiscoverykeys_devpkey::PKEY_Device_FriendlyName;
use winapi::um::mmdeviceapi::IMMDevice;
use winapi::um::mmdeviceapi::IMMDeviceCollection;
use winapi::um::propidl::PropVariantClear;
use winapi::um::propidl::PROPVARIANT;
use winapi::um::propsys::IPropertyStore;

/// A collection of audio devices
pub struct MultiMediaDeviceCollection(NonNull<IMMDeviceCollection>);

impl MultiMediaDeviceCollection {
    /// Get the number of items in this collection.
    ///
    /// # Error
    /// Returns an error if the number of items could not be retrieved.
    // WINAPI BUG: THIS IS DEFINITELY MUT
    #[allow(clippy::unnecessary_mut_passed)]
    pub fn get_count(&self) -> std::io::Result<UINT> {
        let mut count = 0;
        let code = unsafe { self.0.as_ref().GetCount(&mut count) };
        if FAILED(code) {
            return Err(std::io::Error::from_raw_os_error(code));
        }

        Ok(count)
    }

    /// Get the audio device at the given index
    ///
    /// # Error
    /// Returns an error if the device could not be retrived.
    ///
    /// # Panics
    /// Panics if the function succeeds yet the ptr is null.
    pub fn item(&self, index: UINT) -> std::io::Result<MultiMediaDevice> {
        let mut ptr = std::ptr::null_mut();
        let code = unsafe { self.0.as_ref().Item(index, &mut ptr) };

        if FAILED(code) {
            return Err(std::io::Error::from_raw_os_error(code));
        }

        let ptr = NonNull::new(ptr).expect("ptr is null");

        Ok(MultiMediaDevice(ptr))
    }
}

impl Drop for MultiMediaDeviceCollection {
    fn drop(&mut self) {
        unsafe {
            self.0.as_ref().Release();
        }
    }
}

/// An Audio Device
pub struct MultiMediaDevice(NonNull<IMMDevice>);

impl MultiMediaDevice {
    /// A property key for the friendly name of the audio adapater
    pub const DEVICE_INTERFACE_FRIENDLY_NAME: PropertyKey =
        PropertyKey(PKEY_DeviceInterface_FriendlyName);

    /// A property key for the device description
    pub const DEVICE_DESC: PropertyKey = PropertyKey(PKEY_Device_DeviceDesc);

    /// A property key for the device friendly name
    pub const DEVICE_FRIENDLY_NAME: PropertyKey = PropertyKey(PKEY_Device_FriendlyName);

    /// Get an [`AudioClient`].
    /// # Error
    /// Returns an error if the client could not be retrieved.
    ///
    /// # Panics
    /// Panics if the ptr is null on success
    pub fn activate_audio_client(&self) -> std::io::Result<AudioClient> {
        let mut ptr = std::ptr::null_mut();
        let code = unsafe {
            self.0.as_ref().Activate(
                &IID_IAudioClient,
                CLSCTX_ALL,
                std::ptr::null_mut(),
                &mut ptr,
            )
        };
        if FAILED(code) {
            return Err(std::io::Error::from_raw_os_error(code));
        }
        let ptr = NonNull::new(ptr.cast()).expect("ptr is null");
        Ok(AudioClient(ptr))
    }

    /// Get an [`AudioEndpointVolume`].
    /// # Error
    /// Returns an error if the interface could not be retrieved.
    ///
    /// # Panics
    /// Panics if the ptr is null on success
    pub fn activate_audio_endpoint_volume(&self) -> std::io::Result<AudioEndpointVolume> {
        let mut ptr = std::ptr::null_mut();
        let code = unsafe {
            self.0.as_ref().Activate(
                &IID_IAudioEndpointVolume,
                CLSCTX_ALL,
                std::ptr::null_mut(),
                &mut ptr,
            )
        };
        if FAILED(code) {
            return Err(std::io::Error::from_raw_os_error(code));
        }
        let ptr = NonNull::new(ptr.cast()).expect("ptr is null");
        Ok(AudioEndpointVolume(ptr))
    }

    /// Get the Id of this device
    ///
    /// # Error
    /// Returns an error if the state could not be retrieved.
    ///
    /// # Panics
    /// Panics if the ptr is null on success
    pub fn get_id(&self) -> std::io::Result<skylight::CoTaskMemWideString> {
        let mut ptr = std::ptr::null_mut();
        let code = unsafe { self.0.as_ref().GetId(&mut ptr) };
        if FAILED(code) {
            return Err(std::io::Error::from_raw_os_error(code));
        }
        let ptr = NonNull::new(ptr).expect("ptr is null");
        Ok(unsafe { skylight::CoTaskMemWideString::from_raw(ptr) })
    }

    /// Get the state of this device
    ///
    /// # Error
    /// Returns an error if the state could not be retrieved.
    ///
    /// # Panics
    /// Panics if the device state is invalid
    pub fn get_state(&self) -> std::io::Result<DeviceState> {
        let mut state = 0;
        let code = unsafe { self.0.as_ref().GetState(&mut state) };

        if FAILED(code) {
            return Err(std::io::Error::from_raw_os_error(code));
        }

        Ok(DeviceState::from_bits(state).expect("invalid device state"))
    }

    /// Open the property store.
    ///
    /// # Error
    /// Failed to get the property store
    ///
    /// # Panics
    /// Panics if the property store ptr was null on success.
    pub fn open_property_store(&self, mode: StorageAccessMode) -> std::io::Result<PropertyStore> {
        let mut ptr = std::ptr::null_mut();
        let code = unsafe { self.0.as_ref().OpenPropertyStore(mode.bits(), &mut ptr) };
        if FAILED(code) {
            return Err(std::io::Error::from_raw_os_error(code));
        }
        let ptr = NonNull::new(ptr).expect("ptr is null");
        Ok(PropertyStore(ptr))
    }
}

impl std::fmt::Debug for MultiMediaDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let property_store = self.open_property_store(StorageAccessMode::READ);
        let device_interface_friendly_name = property_store
            .as_ref()
            .map(|property_store| property_store.get_value(Self::DEVICE_INTERFACE_FRIENDLY_NAME));
        let device_description = property_store
            .as_ref()
            .map(|property_store| property_store.get_value(Self::DEVICE_DESC));
        let device_friendly_name = property_store
            .as_ref()
            .map(|property_store| property_store.get_value(Self::DEVICE_FRIENDLY_NAME));

        f.debug_struct("MultiMediaDevice")
            .field("id", &self.get_id())
            .field("state", &self.get_state())
            .field("property_store", &property_store)
            .field(
                "device_interface_friendly_name",
                &device_interface_friendly_name,
            )
            .field("device_description", &device_description)
            .field("device_friendly_name", &device_friendly_name)
            .finish()
    }
}

impl Drop for MultiMediaDevice {
    fn drop(&mut self) {
        unsafe {
            self.0.as_ref().Release();
        }
    }
}

bitflags::bitflags! {
    pub struct StorageAccessMode: DWORD {
        const READ = STGM_READ;
        const WRITE = STGM_WRITE;
        const READWRITE = STGM_READWRITE;
    }
}

pub struct PropertyStore(NonNull<IPropertyStore>);

impl PropertyStore {
    /// Get the property key at the given index.
    ///
    /// # Error
    /// Fails if the property key could not be acquired
    pub fn get_at(&self, index: DWORD) -> std::io::Result<PropertyKey> {
        let mut key = unsafe { std::mem::zeroed() };
        let code = unsafe { self.0.as_ref().GetAt(index, &mut key) };
        if FAILED(code) {
            return Err(std::io::Error::from_raw_os_error(code));
        }
        Ok(PropertyKey(key))
    }

    /// Get the number of properties
    ///
    /// # Error
    /// Fails if the count could not be retrieved.
    pub fn get_count(&self) -> std::io::Result<DWORD> {
        let mut count = 0;
        let code = unsafe { self.0.as_ref().GetCount(&mut count) };
        if FAILED(code) {
            return Err(std::io::Error::from_raw_os_error(code));
        }
        Ok(count)
    }

    /// Get a property
    ///
    /// # Error
    /// Fails if the value could not be acquired.
    pub fn get_value(&self, key: PropertyKey) -> std::io::Result<PropVariant> {
        let mut prop_variant = PropVariant::new();
        let code = unsafe {
            self.0
                .as_ref()
                .GetValue(key.as_raw_ptr(), prop_variant.as_raw_mut_ptr())
        };
        if FAILED(code) {
            return Err(std::io::Error::from_raw_os_error(code));
        }

        Ok(prop_variant)
    }
}

impl std::fmt::Debug for PropertyStore {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let count = self.get_count();
        f.debug_struct("PropertyStore")
            .field("count", &count)
            .finish()
    }
}

impl Drop for PropertyStore {
    fn drop(&mut self) {
        unsafe {
            self.0.as_ref().Release();
        }
    }
}

pub struct PropertyKey(PROPERTYKEY);

impl PropertyKey {
    pub fn as_raw_ptr(&self) -> *const PROPERTYKEY {
        &self.0
    }
}

pub struct PropVariant(PROPVARIANT);

impl PropVariant {
    /// Create an empty [`PropVariant`].
    pub fn new() -> Self {
        let prop = unsafe {
            let mut prop: PROPVARIANT = std::mem::zeroed();
            prop.vt = VT_EMPTY as u16;
            prop
        };

        Self(prop)
    }

    /// Get a raw const pointer to the inner data.
    pub fn as_raw_ptr(&self) -> *const PROPVARIANT {
        &self.0
    }

    /// Get a raw mut pointer to the inner data.
    pub fn as_raw_mut_ptr(&mut self) -> *mut PROPVARIANT {
        &mut self.0
    }

    /// Try to clear this [`PropVariant`].
    pub fn clear(&mut self) -> std::io::Result<()> {
        let code = unsafe { PropVariantClear(&mut self.0) };
        if code != S_OK {
            return Err(std::io::Error::from_raw_os_error(code));
        }
        Ok(())
    }

    /// Returns true if this contains a wide string.
    pub fn is_wide_string(&self) -> bool {
        u32::from(self.0.vt) == VT_LPWSTR
    }

    /// Try to get this as a wide string.
    ///
    /// This does an O(n) length check.
    /// # Errors
    /// Returns `None` if this does not contain a wide string.
    pub fn as_wide_string(&self) -> Option<&widestring::U16CStr> {
        if !self.is_wide_string() {
            return None;
        }

        Some(unsafe {
            let ptr = *self.0.data.pwszVal();
            widestring::U16CStr::from_ptr_str(ptr)
        })
    }
}

impl Default for PropVariant {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for PropVariant {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match u32::from(self.0.vt) {
            VT_LPWSTR => write!(
                f,
                "WideString({:?})",
                self.as_wide_string()
                    .expect("expected a wide string")
                    .to_string_lossy(),
            ),
            vt => write!(f, "Unknown({:?})", vt),
        }
    }
}

impl Drop for PropVariant {
    fn drop(&mut self) {
        let _ = self.clear().is_ok();
    }
}

/// An audio client, representing one connection
pub struct AudioClient(NonNull<IAudioClient>);

impl AudioClient {
    /// Get the buffer size, in audio frames
    ///
    /// # Errors
    /// Returns an error if the buffer size could not be retrieved.
    pub fn get_buffer_size(&self) -> std::io::Result<UINT32> {
        let mut size = 0;
        let code = unsafe { self.0.as_ref().GetBufferSize(&mut size) };
        if FAILED(code) {
            return Err(std::io::Error::from_raw_os_error(code));
        }
        Ok(size)
    }

    /// Get the default and minimum device period
    ///
    /// # Errors
    /// Errors if the period could not be retrieved.
    ///
    /// # Panics
    /// Panics if the period is invalid
    pub fn get_device_period(&self) -> std::io::Result<(Duration, Duration)> {
        let mut default_period = 0;
        let mut minimum_period = 0;
        let code = unsafe {
            self.0
                .as_ref()
                .GetDevicePeriod(&mut default_period, &mut minimum_period)
        };
        if FAILED(code) {
            return Err(std::io::Error::from_raw_os_error(code));
        }
        let default_period = default_period / 100;
        let default_period =
            Duration::from_nanos(default_period.try_into().expect("invalid period"));
        let minimum_period = minimum_period / 100;
        let minimum_period =
            Duration::from_nanos(minimum_period.try_into().expect("invalid period"));
        Ok((default_period, minimum_period))
    }

    /// Get the mix format.
    ///
    /// This can be called before [`AudioClient::initialize`].
    ///
    /// # Errors
    /// Fails if the mix format could not be acquired.
    ///
    /// # Panics
    /// Panics if the property store ptr was null on success.
    pub fn get_mix_format(&self) -> std::io::Result<WaveFormatExtensible> {
        let mut ptr = std::ptr::null_mut();
        let code = unsafe { self.0.as_ref().GetMixFormat(&mut ptr) };
        if FAILED(code) {
            return Err(std::io::Error::from_raw_os_error(code));
        }
        let ptr = NonNull::new(ptr).expect("ptr was null");
        Ok(WaveFormatExtensible(ptr))
    }

    /// Initialize the audio client.
    ///
    /// # Errors
    /// Fails if the client could not be initialized.
    ///
    /// # Panics
    /// Panics if the duration is too large.
    pub fn initialize(
        &self,
        share_mode: AudioClientShareMode,
        buffer_duration: Duration,
        period_duration: Duration,
        format: &WaveFormatExtensible,
    ) -> std::io::Result<()> {
        let code = unsafe {
            self.0.as_ref().Initialize(
                share_mode.into(),
                AUDCLNT_STREAMFLAGS_EVENTCALLBACK,
                (buffer_duration.as_nanos() * 100)
                    .try_into()
                    .expect("duration is too large"),
                (period_duration.as_nanos() * 100)
                    .try_into()
                    .expect("duration is too large"),
                format.0.as_ptr(),
                std::ptr::null_mut(),
            )
        };
        if FAILED(code) {
            return Err(std::io::Error::from_raw_os_error(code));
        }

        Ok(())
    }

    /// Check if a format is supported
    pub fn is_format_supported(
        &self,
        share_mode: AudioClientShareMode,
        format: &WaveFormatExtensible,
    ) -> std::io::Result<(bool, Option<WaveFormatExtensible>)> {
        let mut ptr = std::ptr::null_mut();
        let code = unsafe {
            self.0
                .as_ref()
                .IsFormatSupported(share_mode.into(), format.0.as_ptr(), &mut ptr)
        };
        if FAILED(code) {
            return Err(std::io::Error::from_raw_os_error(code));
        }
        Ok((code == S_OK, NonNull::new(ptr).map(WaveFormatExtensible)))
    }

    /// Set the event handle
    pub fn set_event_handle(&self, handle: HANDLE) -> std::io::Result<()> {
        let code = unsafe { self.0.as_ref().SetEventHandle(handle.cast()) };
        if FAILED(code) {
            return Err(std::io::Error::from_raw_os_error(code));
        }
        Ok(())
    }

    /// Get a render client
    pub fn get_service_audio_render_client(&self) -> std::io::Result<AudioRenderClient> {
        let mut ptr = std::ptr::null_mut();
        let code = unsafe {
            self.0
                .as_ref()
                .GetService(&IID_IAudioRenderClient, &mut ptr)
        };
        if FAILED(code) {
            return Err(std::io::Error::from_raw_os_error(code));
        }

        let ptr = NonNull::new(ptr.cast()).expect("ptr was null");
        Ok(AudioRenderClient(ptr))
    }

    /// Start
    pub fn start(&self) -> std::io::Result<()> {
        let code = unsafe { self.0.as_ref().Start() };
        if FAILED(code) {
            return Err(std::io::Error::from_raw_os_error(code));
        }
        Ok(())
    }

    /// Stop
    pub fn stop(&self) -> std::io::Result<()> {
        let code = unsafe { self.0.as_ref().Stop() };
        if FAILED(code) {
            return Err(std::io::Error::from_raw_os_error(code));
        }
        Ok(())
    }

    /// Get the current padding
    pub fn get_current_padding(&self) -> std::io::Result<u32> {
        let mut padding = 0;
        let code = unsafe { self.0.as_ref().GetCurrentPadding(&mut padding) };
        if FAILED(code) {
            return Err(std::io::Error::from_raw_os_error(code));
        }
        Ok(padding)
    }
}

impl Drop for AudioClient {
    fn drop(&mut self) {
        unsafe {
            self.0.as_ref().Release();
        }
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash)]
pub enum AudioClientShareMode {
    Shared,
    Exclusive,
}

impl From<AudioClientShareMode> for AUDCLNT_SHAREMODE {
    fn from(m: AudioClientShareMode) -> Self {
        match m {
            AudioClientShareMode::Shared => AUDCLNT_SHAREMODE_SHARED,
            AudioClientShareMode::Exclusive => AUDCLNT_SHAREMODE_EXCLUSIVE,
        }
    }
}

/// https://sourceforge.net/p/comtypes/mailman/comtypes-users/?limit=250
#[allow(non_upper_case_globals)]
const IID_IAudioEndpointVolume: GUID = GUID {
    Data1: 0x5CDF2C82,
    Data2: 0x841E,
    Data3: 0x4546,
    Data4: 0x97220CF74078229A_u64.to_be_bytes(),
};

pub struct WaveFormatExtensible(NonNull<WAVEFORMATEX>);

impl WaveFormatExtensible {
    /*
    /// Make a new format.
    ///
    /// # Panics
    /// Panics on alloc failure or invalid parameters.
    pub fn new(
        format_type: WaveFormatType,
        channels: u16,
        samples_per_sec: u32,
        bits_per_sample: u16,
    ) -> Self {
        if channels > 2 {
            todo!("More than 2 channels unsupported");
        }

        // let valid_bits_per_sample = bits_per_sample;

        let block_align;
        // let sub_format;
        match format_type {
            WaveFormatType::Pcm => {
                if bits_per_sample != 8 && bits_per_sample != 16 {
                    panic!("invalid bits per sample '{}'", bits_per_sample);
                }

                block_align = (channels * bits_per_sample) / 8;
                // sub_format = KSDATAFORMAT_SUBTYPE_PCM;
            }
            WaveFormatType::Float => {
                if bits_per_sample != 32 {
                    panic!("invalid bits per sample '{}'", bits_per_sample);
                }

                block_align = (channels * bits_per_sample) / 8;
                // sub_format = KSDATAFORMAT_SUBTYPE_IEEE_FLOAT;
            }
            WaveFormatType::Extensible => {
                todo!("extensible")
            }
        }

        let avg_bytes_per_sec = samples_per_sec * u32::from(block_align);

        let format_ptr: *mut WAVEFORMATEX =
            unsafe { CoTaskMemAlloc(std::mem::size_of::<WAVEFORMATEX>()).cast() };

        let format: NonNull<WAVEFORMATEX> = NonNull::new(format_ptr.cast()).expect("alloc failure");
        let format_ptr = format.as_ptr();

        unsafe {
            std::ptr::addr_of_mut!((*format_ptr).wFormatTag).write(format_type.into());
            std::ptr::addr_of_mut!((*format_ptr).nChannels).write(channels);
            std::ptr::addr_of_mut!((*format_ptr).nSamplesPerSec).write(samples_per_sec);
            std::ptr::addr_of_mut!((*format_ptr).nAvgBytesPerSec).write(avg_bytes_per_sec);
            std::ptr::addr_of_mut!((*format_ptr).nBlockAlign).write(block_align);
            std::ptr::addr_of_mut!((*format_ptr).wBitsPerSample).write(bits_per_sample);
            std::ptr::addr_of_mut!((*format_ptr).cbSize).write(0);

            /*
            std::ptr::addr_of_mut!((*format_ptr).Samples).write(valid_bits_per_sample);
            std::ptr::addr_of_mut!((*format_ptr).dwChannelMask)
                .write(SPEAKER_FRONT_LEFT | SPEAKER_FRONT_RIGHT);
            std::ptr::addr_of_mut!((*format_ptr).SubFormat).write(sub_format);
            */
        }

        Self(format)
    }
    */

    fn as_raw_wave_format_extensible(&self) -> Option<&WAVEFORMATEXTENSIBLE> {
        if !self.is_wave_format_extensible() {
            return None;
        }

        unsafe {
            let ptr: *mut WAVEFORMATEXTENSIBLE = self.0.as_ptr().cast();
            Some(ptr.as_ref().expect("ptr is null"))
        }
    }

    /// Get the wave format type
    pub fn wave_format_type(&self) -> Result<WaveFormatType, WORD> {
        unsafe { self.0.as_ref().wFormatTag.try_into() }
    }

    /// Get the ks data format type if it exists.
    pub fn ks_data_format_type(&self) -> Option<Result<KsDataFormatType, GUID>> {
        Some(self.as_raw_wave_format_extensible()?.SubFormat.try_into())
    }

    /// Return true if the data is type is WAVE_FORMAT_EXTENSIBLE
    pub fn is_wave_format_extensible(&self) -> bool {
        self.wave_format_type() == Ok(WaveFormatType::Extensible)
    }

    /// Get the number of channels
    pub fn num_channels(&self) -> u16 {
        unsafe { self.0.as_ref().nChannels }
    }

    /// Get the samples_per_sec
    pub fn samples_per_sec(&self) -> u32 {
        unsafe { self.0.as_ref().nSamplesPerSec }
    }
}

impl std::fmt::Debug for WaveFormatExtensible {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("WaveFormatExtensible")
            .field("wave_format_type", &self.wave_format_type())
            .field("ks_data_format_type", &self.ks_data_format_type())
            .field("num_channels", &self.num_channels())
            .field("samples_per_sec", &self.samples_per_sec())
            .finish()
    }
}

impl Drop for WaveFormatExtensible {
    fn drop(&mut self) {
        unsafe { CoTaskMemFree(self.0.as_ptr().cast()) }
    }
}

/// The Wave Format Type
#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub enum WaveFormatType {
    Pcm,
    Float,
    Extensible,
}

impl TryFrom<WORD> for WaveFormatType {
    type Error = WORD;

    fn try_from(word: WORD) -> Result<Self, Self::Error> {
        match word {
            WAVE_FORMAT_PCM => Ok(Self::Pcm),
            WAVE_FORMAT_IEEE_FLOAT => Ok(Self::Float),
            WAVE_FORMAT_EXTENSIBLE => Ok(Self::Extensible),
            _ => Err(word),
        }
    }
}

impl From<WaveFormatType> for WORD {
    fn from(t: WaveFormatType) -> Self {
        match t {
            WaveFormatType::Pcm => WAVE_FORMAT_PCM,
            WaveFormatType::Float => WAVE_FORMAT_IEEE_FLOAT,
            WaveFormatType::Extensible => WAVE_FORMAT_EXTENSIBLE,
        }
    }
}

/// The Ks Data Format type
#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub enum KsDataFormatType {
    Pcm,
    Float,
}

impl TryFrom<GUID> for KsDataFormatType {
    type Error = GUID;

    fn try_from(guid: GUID) -> Result<Self, Self::Error> {
        if guid_eq(guid, KSDATAFORMAT_SUBTYPE_PCM) {
            Ok(Self::Pcm)
        } else if guid_eq(guid, KSDATAFORMAT_SUBTYPE_IEEE_FLOAT) {
            Ok(Self::Float)
        } else {
            Err(guid)
        }
    }
}

fn guid_eq(guid1: GUID, guid2: GUID) -> bool {
    guid1.Data1 == guid2.Data1
        && guid1.Data2 == guid2.Data2
        && guid1.Data3 == guid2.Data3
        && guid1.Data4 == guid2.Data4
}

pub struct AudioEndpointVolume(NonNull<IAudioEndpointVolume>);

impl AudioEndpointVolume {
    /// Set the master volume level on a scale of 0.0 - 1.0
    ///
    /// # Error
    /// Returns an error if the master volume level could not be set
    pub fn set_master_volume_level_scalar(&self, level: f32) -> std::io::Result<()> {
        let code = unsafe {
            self.0
                .as_ref()
                .SetMasterVolumeLevelScalar(level, std::ptr::null())
        };
        if FAILED(code) {
            return Err(std::io::Error::from_raw_os_error(code));
        }
        Ok(())
    }

    /// Set the mute state
    ///
    /// # Error
    /// Returns an error if the mute state could not be set
    pub fn set_mute(&self, mute: bool) -> std::io::Result<()> {
        let mute = if mute { TRUE } else { FALSE };
        let code = unsafe { self.0.as_ref().SetMute(mute, std::ptr::null()) };
        if FAILED(code) {
            return Err(std::io::Error::from_raw_os_error(code));
        }
        Ok(())
    }
}

impl Drop for AudioEndpointVolume {
    fn drop(&mut self) {
        unsafe {
            self.0.as_ref().Release();
        }
    }
}

pub struct AudioRenderClient(NonNull<IAudioRenderClient>);

impl AudioRenderClient {
    /// Get the buffer.
    ///
    /// # Errors
    /// Errors if the buffer could not be acquired
    ///
    /// # Panics
    /// Panics if count == 0 or the buffer ptr is null on success.
    pub fn get_buffer(&self, count: u32) -> std::io::Result<*mut u8> {
        assert_ne!(count, 0);

        let mut ptr = std::ptr::null_mut();
        let code = unsafe { self.0.as_ref().GetBuffer(count, &mut ptr) };
        if FAILED(code) {
            return Err(std::io::Error::from_raw_os_error(code));
        }

        // Undocumented, but this ptr can be null on success sometimes.
        // I don't really know what causes it or why it happens.
        // Use this as a stopgap for now.
        assert!(!ptr.is_null());

        Ok(ptr)
    }

    /// Release buffer
    pub fn release_buffer(&self, count: u32) -> std::io::Result<()> {
        let code = unsafe { self.0.as_ref().ReleaseBuffer(count, 0) };
        if FAILED(code) {
            return Err(std::io::Error::from_raw_os_error(code));
        }

        Ok(())
    }
}

impl Drop for AudioRenderClient {
    fn drop(&mut self) {
        unsafe {
            self.0.as_ref().Release();
        }
    }
}

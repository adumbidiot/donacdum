use crate::MultiMediaDeviceCollection;
use std::ptr::NonNull;
use winapi::shared::minwindef::DWORD;
use winapi::shared::winerror::FAILED;
use winapi::um::combaseapi::CLSCTX_ALL;
use winapi::um::mmdeviceapi::eAll;
use winapi::um::mmdeviceapi::eCapture;
use winapi::um::mmdeviceapi::eRender;
use winapi::um::mmdeviceapi::CLSID_MMDeviceEnumerator;
use winapi::um::mmdeviceapi::EDataFlow;
use winapi::um::mmdeviceapi::IMMDeviceEnumerator;
use winapi::um::mmdeviceapi::DEVICE_STATEMASK_ALL;
use winapi::um::mmdeviceapi::DEVICE_STATE_ACTIVE;
use winapi::um::mmdeviceapi::DEVICE_STATE_DISABLED;
use winapi::um::mmdeviceapi::DEVICE_STATE_NOTPRESENT;
use winapi::um::mmdeviceapi::DEVICE_STATE_UNPLUGGED;

/// An enumerator of Audio Devices
pub struct MultiMediaDeviceEnumerator(NonNull<IMMDeviceEnumerator>);

impl MultiMediaDeviceEnumerator {
    /// Create a new [`MultiMediaDeviceEnumerator`].
    ///
    /// # Errors
    /// Returns an error on failure
    ///
    /// # Panics
    /// Panics if the function succeeds yet the ptr is null.
    pub fn new() -> std::io::Result<Self> {
        let ptr = unsafe {
            skylight::create_instance(&CLSID_MMDeviceEnumerator, CLSCTX_ALL)
                .map_err(std::io::Error::from_raw_os_error)?
        };
        Ok(Self(NonNull::new(ptr).expect("ptr is null")))
    }

    /// Get a collection of audio endpoints.
    ///
    /// # Errors
    /// Returns an error if the collection could not be aquired.
    ///
    /// # Panics
    /// Panics if the function succeeds yet the ptr is null.
    pub fn enum_audio_endpoints(
        &self,
        data_flow: DataFlow,
        device_state: DeviceState,
    ) -> std::io::Result<MultiMediaDeviceCollection> {
        let mut ptr = std::ptr::null_mut();
        let code = unsafe {
            self.0
                .as_ref()
                .EnumAudioEndpoints(data_flow.into(), device_state.bits(), &mut ptr)
        };
        if FAILED(code) {
            return Err(std::io::Error::from_raw_os_error(code));
        }

        let ptr = NonNull::new(ptr).expect("ptr is null");

        Ok(MultiMediaDeviceCollection(ptr))
    }
}

impl Drop for MultiMediaDeviceEnumerator {
    fn drop(&mut self) {
        unsafe {
            self.0.as_ref().Release();
        }
    }
}

/// The flow of audio data
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum DataFlow {
    Render,
    Capture,
    All,
}

impl From<DataFlow> for EDataFlow {
    fn from(flow: DataFlow) -> Self {
        match flow {
            DataFlow::Render => eRender,
            DataFlow::Capture => eCapture,
            DataFlow::All => eAll,
        }
    }
}

bitflags::bitflags! {
    pub struct DeviceState: DWORD {
        const ACTIVE = DEVICE_STATE_ACTIVE;
        const DISABLED = DEVICE_STATE_DISABLED;
        const NOTPRESENT = DEVICE_STATE_NOTPRESENT;
        const UNPLUGGED = DEVICE_STATE_UNPLUGGED;
        const ALL = DEVICE_STATEMASK_ALL;
    }
}

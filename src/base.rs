use std::fmt::Display;
use std::os::raw::c_int;
use std::ffi::CString;

use ffi;
use stream::OutStream;

const MAX_CHANNELS: u32 = 24;

/// Result wrapper that always contains a `ffi::enums::SioError` in error case.
pub type SioResult<T> = Result<T, ffi::enums::SioError>;

/// The base struct which can connect to various audio backends
/// and provides methods to get in-/output `Device`s.
pub struct SoundIo {
    context: *mut ffi::SoundIo,
    name: CString,
}
impl SoundIo {
    pub fn new() -> Self {
        SoundIo {
            context: unsafe { ffi::soundio_create() },
            name: CString::new("rsoundio").unwrap(),
        }
    }

    /// Returns the number builtin channel layouts.
    pub fn channel_layout_builtin_count() -> u32 {
        unsafe { ffi::soundio_channel_layout_builtin_count() as u32 }
    }

    // NOTE: Links to other types in rustdoc are not implemented
    // yet,
    // [see](https://internals.rust-lang.org/t/rustdoc-link-to-other-types-from-doc-comments/968).

    /// Tries to connect on all available backends in order.
    ///
    /// Possible errors:
    ///
    /// - `ffi::enums::SioError::Invalid`
    /// - `ffi::enums::SioError::NoMem`
    /// - `ffi::enums::SioError::SystemResources`
    /// - `ffi::enums::SioError::NoSuchClient`
    pub fn connect(&self) -> SioResult<()> {
        match unsafe { ffi::soundio_connect(self.context) } {
            ffi::enums::SioError::None => Ok(()),
            err @ _ => Err(err),
        }
    }

    /// Instead of calling ::soundio_connect you may call this function to try a
    /// specific backend.
    /// Possible errors:
    ///
    /// - `ffi::enums::SioError::Invalid`
    /// - `ffi::enums::SioError::BackendUnavailable`
    /// - `ffi::enums::SioError::SystemResources`
    /// - `ffi::enums::SioError::NoSuchClient`
    /// - `ffi::enums::SioError::InitAudioBackend`
    /// - `ffi::enums::SioError::BackendDisconnected`
    pub fn connect_backend(&self, backend: ffi::enums::SioBackend) -> SioResult<()> {
        match unsafe { ffi::soundio_connect_backend(self.context, backend) } {
            ffi::enums::SioError::None => Ok(()),
            err @ _ => Err(err),
        }
    }

    /// Returns the number of available backens.
    pub fn backend_count(&self) -> u32 {
        unsafe { ffi::soundio_backend_count(self.context) as u32 }
    }

    /// Returns a backend at the specified index.
    /// If the index is not in range [0, backend_count), then
    /// `None` is returned.
    pub fn backend(&self, idx: u32) -> Option<ffi::enums::SioBackend> {
        match unsafe { ffi::soundio_get_backend(self.context, idx as c_int) } {
            ffi::enums::SioBackend::None => None,
            backend @ _ => Some(backend),
        }
    }

    /// Returns the current backend or `None` if neither
    /// `connect` nor `connect_backend` or `disconnect` was called.
    pub fn current_backend(&self) -> Option<ffi::enums::SioBackend> {
        match unsafe { (*self.context).current_backend } {
            ffi::enums::SioBackend::None => None,
            backend @ _ => Some(backend),
        }
    }

    /// Returns `true` if libsoundio was compiled against `backend`.
    /// Otherwise `false` is returned.
    pub fn have_backend(&self, backend: ffi::enums::SioBackend) -> bool {
        unsafe { ffi::soundio_have_backend(backend) == 1u8 }
    }

    /// Atomically update information for all connected devices.
    /// It is performant to call this function many times per second.
    ///
    /// When you call this, the following callbacks might be called:
    ///
    /// - `on_device_change`
    /// - `on_backend_disconnect`
    ///
    /// Note that if you do not care about learning about updated devices,
    /// you might call this function only once ever and never call `wait_events`.
    pub fn flush_events(&self) {
        unsafe { ffi::soundio_flush_events(self.context) }
    }

    /// This function calls `flush_events` then blocks until another event
    /// is ready or you call `wakeup`.
    /// Be ready for spurious wakeups.
    pub fn wait_events(&self) {
        unsafe { ffi::soundio_wait_events(self.context) }
    }

    /// Makes `wait_events` stop blocking.
    pub fn wakeup(&self) {
        unsafe { ffi::soundio_wakeup(self.context) }
    }

    /// If necessary you can manually trigger a device rescan. Normally you will
    /// not ever have to call this function, as libsoundio listens to system events
    /// for device changes and responds to them by rescanning devices and preparing
    /// the new device information for you to be atomically replaced when you call
    /// `flush_events`. However you might run into cases where you want to
    /// force trigger a device rescan, for example if an ALSA device has a
    /// `Device::probe_error.`
    ///
    /// After you call this you still have to use `soundio_flush_events` or
    /// `soundio_wait_events` and then wait for the
    /// `on_devices_change` callback.
    ///
    /// This can be called from any thread context except for
    /// `OutStream::write_callback` and `InStream::read_callback`.
    pub fn force_device_scan(&self) {
        unsafe { ffi::soundio_force_device_scan(self.context) }
    }

    /// Disconnects the audio backend.
    pub fn disconnect(&self) {
        unsafe { ffi::soundio_disconnect(self.context) }
    }

    /// When you call `flush_events` a snapshot of all device state is
    /// saved and these functions merely access the snapshot data. When you want
    /// to check for new devices, call `flush_events`. Or you can call
    /// `wait_events` to block until devices change. If an error occurs
    /// scanning devices in a background thread, `backend_disconnect` is called
    /// with the error code.
    ///
    /// Get the number of input devices.
    /// Returns `None` if you never called `flush_events`.
    pub fn input_device_count(&self) -> Option<u32> {
        let cnt = unsafe { ffi::soundio_input_device_count(self.context) };
        if cnt < 0 {
            None
        } else {
            Some(cnt as u32)
        }
    }

    /// Get the number of output devices.
    /// Returns `None` if you never called `flush_events`.
    pub fn output_device_count(&self) -> Option<u32> {
        let cnt = unsafe { ffi::soundio_output_device_count(self.context) };
        if cnt < 0 {
            None
        } else {
            Some(cnt as u32)
        }
    }

    /// Always returns a device.
    /// `idx` must be in [0, `input_device_count`)
    /// Returns `None` if you never called `flush_events` or if you provide
    /// invalid parameter values.
    pub fn input_device(&self, idx: u32) -> Option<Device> {
        let dev_ptr = unsafe { ffi::soundio_get_input_device(self.context, idx as c_int) };
        if dev_ptr.is_null() {
            None
        } else {
            Some(Device::new(dev_ptr))
        }
    }

    /// Always returns a device.
    /// `idx` must be in [0, `output_device_count`)
    /// Returns `None` if you never called `flush_events` or if you provide
    /// invalid parameter values.
    pub fn output_device(&self, idx: u32) -> Option<Device> {
        let dev_ptr = unsafe { ffi::soundio_get_output_device(self.context, idx as c_int) };
        if dev_ptr.is_null() {
            None
        } else {
            Some(Device::new(dev_ptr))
        }
    }

    /// Returns the index of the default input device or `None`
    /// if there are no devices or if you never called
    /// `flush_events`.
    pub fn default_input_device_index(&self) -> Option<u32> {
        match unsafe { ffi::soundio_default_input_device_index(self.context) } {
            -1 => None,
            idx @ _ => Some(idx as u32),
        }
    }

    /// Returns the index of the default output device or `None`
    /// if there are no devices or if you never called
    /// `flush_events`.
    pub fn default_output_device_index(&self) -> Option<u32> {
        match unsafe { ffi::soundio_default_output_device_index(self.context) } {
            -1 => None,
            idx @ _ => Some(idx as u32),
        }
    }

    /// Returns the default output `Device` of the backend.
    /// `None` if you aren't connected to a backend.
    pub fn default_output_device(&self) -> Option<Device> {
        self.default_output_device_index().and_then(|idx| self.output_device(idx))
    }

    /// Returns the default input `Device` of the backend.
    /// `None` if you aren't connected to a backend.
    pub fn default_input_device(&self) -> Option<Device> {
        self.default_input_device_index().and_then(|idx| self.input_device(idx))
    }

    /// Sets the application name which is shown in the
    /// system audio mixer.
    /// Call this **before** connecting to an audio backend, otherwise
    /// the setting won't have any effect.
    /// Semicolons `:` will be replaced with `_`.
    /// If the `name` contains a `NULL` byte, `SioError::EncodingString` is returned.
    pub fn set_name<T: Into<String>>(&mut self, name: T) -> SioResult<()> {
        let s = name.into().replace(":", "_");
        self.name = try!(CString::new(s).map_err(|_| ffi::enums::SioError::EncodingString));
        unsafe { (*self.context).app_name = self.name.as_ptr() };
        Ok(())
    }

    /// Returns the application name.
    /// If the name is not a valid UTF-8 string a `SioError::EncodingString` is returned.
    pub fn name(&self) -> SioResult<String> {
        unsafe { ffi::utils::ptr_to_string((*self.context).app_name) }
    }
}
impl Drop for SoundIo {
    fn drop(&mut self) {
        unsafe {
            self.disconnect();
            ffi::soundio_destroy(self.context)
        }
    }
}

/// Provides methods on channel layouts. Layout variants are defined
/// in `ffi::enums::SioChannelLayoutId`.
#[derive(Debug)]
pub struct ChannelLayout {
    layout: *const ffi::SoundIoChannelLayout,
}
impl ChannelLayout {
    pub fn new(raw_layout: *const ffi::SoundIoChannelLayout) -> Self {
        ChannelLayout { layout: raw_layout }
    }

    /// Returns a builtin channel layout or `None` if
    /// `idx` *not* in [0, `SoundIo::channel_layout_builtin_count`).
    pub fn builtin(idx: u32) -> Option<Self> {
        if idx < SoundIo::channel_layout_builtin_count() as u32 {
            Some(ChannelLayout::new(unsafe {
                ffi::soundio_channel_layout_get_builtin(idx as c_int)
            }))
        } else {
            None
        }
    }

    /// Get the default builtin channel layout for the given number of channels.
    pub fn default(channel_count: u32) -> Option<Self> {
        if channel_count < MAX_CHANNELS {
            Some(ChannelLayout::new(unsafe {
                ffi::soundio_channel_layout_get_default(channel_count as c_int)
            }))
        } else {
            None
        }
    }

    /// Return the index of `channel` in the layout, or `None` if not found.
    pub fn find_channel(&self, channel: ffi::enums::SioChannelId) -> Option<u32> {
        match unsafe { ffi::soundio_channel_layout_find_channel(self.layout, channel) } {
            -1 => None,
            idx @ _ => Some(idx as u32),
        }
    }

    /// Populates the name field of layout if it matches a builtin one.
    pub fn detect_builtin(&mut self) -> bool {
        // This is a hack because of the transmute.
        unsafe {
            let mut_layout: *mut ffi::SoundIoChannelLayout = ::std::mem::transmute(self.layout);
            ffi::soundio_channel_layout_detect_builtin(mut_layout) == 1
        }
    }

    /// Iterates over `preferred_layouts`. Returns the first channel layout in
    /// `preferred_layouts` which matches one of the channel layouts in
    /// `available_layouts`.
    /// Returns `None` if none matches.
    pub fn best_matching_channel_layout(preferred_layouts: &[ChannelLayout],
                                        available_layouts: &[ChannelLayout])
                                        -> Option<ChannelLayout> {
        // do some magic with the slices
        let raw_preferred_layouts: Vec<_> = preferred_layouts.iter()
                                                             .map(|l| unsafe { (*l.layout) })
                                                             .collect();
        let raw_available_layouts: Vec<_> = available_layouts.iter()
                                                             .map(|l| unsafe { (*l.layout) })
                                                             .collect();
        let layout_ptr = unsafe {
            ffi::soundio_best_matching_channel_layout(raw_preferred_layouts.as_ptr(),
                                                      preferred_layouts.len() as c_int,
                                                      raw_available_layouts.as_ptr(),
                                                      available_layouts.len() as c_int)
        };
        if layout_ptr.is_null() {
            None
        } else {
            Some(ChannelLayout::new(layout_ptr))
        }
    }

    /// Returns the number of channels in the layout.
    pub fn channel_count(&self) -> u32 {
        unsafe { (*self.layout).channel_count as u32 }
    }
}
impl PartialEq for ChannelLayout {
    fn eq(&self, other: &Self) -> bool {
        unsafe { ffi::soundio_channel_layout_equal(self.layout, other.layout) == 1u8 }
    }

    fn ne(&self, other: &Self) -> bool {
        !self.eq(other)
    }
}
impl Display for ChannelLayout {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        let str_ptr = unsafe { (*self.layout).name };
        write!(f, "{}", ffi::utils::ptr_to_string(str_ptr).unwrap())
    }
}

/// Provides methods on an audio device.
#[derive(Debug)]
pub struct Device {
    device: *mut ffi::SoundIoDevice,
}
impl Device {
    pub fn new(dev_ptr: *mut ffi::SoundIoDevice) -> Self {
        Device { device: dev_ptr }
    }

    /// Add 1 to the reference count of `device`.
    #[allow(dead_code)]
    fn inc_ref(&self) {
        unsafe { ffi::soundio_device_ref(self.device) }
    }

    // Called automatically on `Device` drop.
    /// Remove 1 to the reference count of `device`. Clean up if it was the last
    /// reference.
    fn dec_ref(&self) {
        unsafe { ffi::soundio_device_unref(self.device) }
    }

    /// Sorts channel layouts by channel count, descending.
    pub fn sort_channel_layouts(&self) {
        unsafe { ffi::soundio_device_sort_channel_layouts(self.device) }
    }

    /// Convenience function.
    /// Returns whether `format` is included in the device's
    /// supported formats.
    pub fn supports_format(&self, format: ffi::enums::SioFormat) -> bool {
        unsafe { ffi::soundio_device_supports_format(self.device, format) == 1u8 }
    }

    /// Convenience function.
    /// Returns whether `layout` is included in the device's
    /// supported channel layouts.
    pub fn supports_layout(&self, layout: &ChannelLayout) -> bool {
        unsafe { ffi::soundio_device_supports_layout(self.device, layout.layout) == 1u8 }
    }

    /// Convenience function.
    /// Returns whether `sample_rate` is included in the
    /// device's supported sample rates.
    pub fn supports_sample_rate(&self, sample_rate: u32) -> bool {
        unsafe {
            ffi::soundio_device_supports_sample_rate(self.device, sample_rate as c_int) == 1u8
        }
    }

    /// Convenience function.
    /// Returns the available sample rate nearest to
    /// `sample_rate`, rounding up.
    pub fn nearest_sample_rate(&self, sample_rate: u32) -> u32 {
        unsafe { ffi::soundio_device_nearest_sample_rate(self.device, sample_rate as c_int) as u32 }
    }

    /// Returns an OutStream struct with default settings.
    /// Sets all fields to defaults.
    /// Returns `ffi::enums::SioError::NoMem` if and only if memory could not be allocated.
    pub fn create_outstream(&self) -> SioResult<OutStream> {
        let stream_ptr = unsafe { ffi::soundio_outstream_create(self.device) };
        if stream_ptr.is_null() {
            Err(ffi::enums::SioError::NoMem)
        } else {
            Ok(OutStream::new(stream_ptr))
        }
    }

    /// Returns the number of references on this device.
    pub fn ref_count(&self) -> u32 {
        unsafe { (*self.device).ref_count as u32 }
    }

    /// This is set to a `ffi::enums::SioError` representing the result of the device
    /// probe. Ideally this will be `ffi::enums::SioError::None` in which case all the
    /// fields of the device will be populated. If there is an error code here
    /// then information about formats, sample rates, and channel layouts might
    /// be missing.
    ///
    /// Possible errors:
    ///
    /// - `ffi::enums::SioError::OpeningDevice`
    /// - `ffi::enums::SioError::NoMem`
    pub fn probe_error(&self) -> Option<ffi::enums::SioError> {
        match unsafe { (*self.device).probe_error } {
            ffi::enums::SioError::None => None,
            error @ _ => Some(error),
        }
    }
}
impl Display for Device {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        let str_ptr = unsafe { (*self.device).name };
        write!(f, "{}", ffi::utils::ptr_to_string(str_ptr).unwrap())
    }
}
impl Drop for Device {
    fn drop(&mut self) {
        self.dec_ref()
    }
}
impl PartialEq for Device {
    fn eq(&self, other: &Self) -> bool {
        unsafe { ffi::soundio_device_equal(self.device, other.device) == 1u8 }
    }
}

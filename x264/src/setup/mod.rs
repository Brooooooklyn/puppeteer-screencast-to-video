use core::mem;
use x264_sys::*;

use crate::{Encoder, Encoding, Error, Result};

mod preset;
mod tune;

pub use self::preset::*;
pub use self::tune::*;

/// Builds a new encoder.
pub struct Setup {
  raw: x264_param_t,
}

impl Setup {
  /// Creates a new builder with the specified preset and tune.
  pub fn preset(preset: Preset, tune: Tune, fast_decode: bool, zero_latency: bool) -> Self {
    let mut raw = mem::MaybeUninit::uninit();

    // Name validity verified at compile-time.
    assert_eq!(0, unsafe {
      x264_param_default_preset(
        raw.as_mut_ptr(),
        preset.to_cstr(),
        tune.to_cstr(fast_decode, zero_latency),
      )
    });

    Self {
      raw: unsafe { raw.assume_init() },
    }
  }

  /// Makes the first pass faster.
  pub fn fastfirstpass(mut self) -> Self {
    unsafe {
      x264_param_apply_fastfirstpass(&mut self.raw);
    }
    self
  }

  /// The video's framerate, represented as a rational number.
  ///
  /// The value is in frames per second.
  pub fn fps(mut self, num: u32, den: u32) -> Self {
    self.raw.i_fps_num = num;
    self.raw.i_fps_den = den;
    self
  }

  /// The encoder's timebase, used in rate control with timestamps.
  ///
  /// The value is in seconds per tick.
  pub fn timebase(mut self, num: u32, den: u32) -> Self {
    self.raw.i_timebase_num = num;
    self.raw.i_timebase_den = den;
    self
  }

  /// Please file an issue if you know what this does, because I have no idea.
  pub fn annexb(mut self, annexb: bool) -> Self {
    self.raw.b_annexb = if annexb { 1 } else { 0 };
    self
  }

  /// Approximately restricts the bitrate.
  ///
  /// The value is in metric kilobits per second.
  pub fn bitrate(mut self, bitrate: i32) -> Self {
    self.raw.rc.i_bitrate = bitrate;
    self
  }

  /// The lowest profile, with guaranteed compatibility with all decoders.
  pub fn baseline(mut self) -> Self {
    unsafe {
      x264_param_apply_profile(&mut self.raw, b"baseline\0" as *const u8 as *const i8);
    }
    self
  }

  /// A useless middleground between the baseline and high profiles.
  pub fn main(mut self) -> Self {
    unsafe {
      x264_param_apply_profile(&mut self.raw, b"main\0" as *const u8 as *const i8);
    }
    self
  }

  /// The highest profile, which almost all encoders support.
  pub fn high(mut self) -> Self {
    unsafe {
      x264_param_apply_profile(&mut self.raw, b"high\0" as *const u8 as *const i8);
    }
    self
  }

  /// thread count
  pub fn threads(mut self, threads: u32) -> Self {
    self.raw.i_threads = threads as i32;
    self
  }

  /// Build the encoder.
  pub fn build<C>(mut self, csp: C, width: u32, height: u32) -> Result<Encoder>
  where
    C: Into<Encoding>,
  {
    self.raw.i_csp = csp.into().into_raw() as i32;
    self.raw.i_width = width as i32;
    self.raw.i_height = height as i32;
    self.raw.i_bitdepth = 8;

    let raw = unsafe { x264_encoder_open_161(&mut self.raw) };

    if raw.is_null() {
      Err(Error)
    } else {
      Ok(unsafe { Encoder::from_raw(raw, self.raw) })
    }
  }
}

impl Default for Setup {
  fn default() -> Self {
    let raw = unsafe {
      let mut raw = mem::MaybeUninit::uninit();
      x264_param_default(raw.as_mut_ptr());
      raw.assume_init()
    };

    Self { raw }
  }
}

use core::{mem, ptr};

use x264_sys::*;

use crate::{Data, Encoding, Error, Image, Picture, Result, Setup};

/// Encodes video.
pub struct Encoder {
  raw: *mut x264_t,
  params: x264_param_t,
}

impl Encoder {
  /// Creates a new builder with default options.
  ///
  /// For more options see `Setup::new`.
  pub fn builder() -> Setup {
    Setup::default()
  }

  #[doc(hidden)]
  pub unsafe fn from_raw(raw: *mut x264_t, mut params: x264_param_t) -> Self {
    x264_encoder_parameters(raw, &mut params);
    Self { raw, params }
  }

  /// Feeds a frame to the encoder.
  pub fn encode(&mut self, pts: i64, image: Image) -> Result<(Data, Picture)> {
    let image = image.raw();

    let mut maybe_uninit_picture = mem::MaybeUninit::uninit();
    unsafe { x264_picture_init(maybe_uninit_picture.as_mut_ptr()) };
    let mut picture = unsafe { maybe_uninit_picture.assume_init() };
    picture.i_pts = pts;
    picture.img = image;

    let mut len = 0;
    let mut stuff = mem::MaybeUninit::uninit();
    let mut raw = mem::MaybeUninit::uninit();

    let err = unsafe {
      x264_encoder_encode(
        self.raw,
        stuff.as_mut_ptr(),
        &mut len,
        &mut picture,
        raw.as_mut_ptr(),
      )
    };

    if err < 0 {
      Err(Error)
    } else {
      let data = unsafe { Data::from_raw_parts(stuff.assume_init(), len as usize) };
      let picture = unsafe { Picture::from_raw(raw.assume_init()) };
      Ok((data, picture))
    }
  }

  /// Gets the video headers, which should be sent first.
  pub fn headers(&mut self) -> Result<Data> {
    let mut len = 0;
    let mut stuff = mem::MaybeUninit::uninit();

    let err = unsafe { x264_encoder_headers(self.raw, stuff.as_mut_ptr(), &mut len) };

    if err < 0 {
      Err(Error)
    } else {
      Ok(unsafe { Data::from_raw_parts(stuff.assume_init(), len as usize) })
    }
  }

  /// Begins flushing the encoder, to handle any delayed frames.
  ///
  /// ```rust
  /// # use x264::{Colorspace, Setup};
  /// # let encoder = Setup::default().build(Colorspace::RGB, 1920, 1080).unwrap();
  /// #
  /// let mut flush = encoder.flush();
  ///
  /// while let Some(result) = flush.next() {
  ///     if let Ok((data, picture)) = result {
  ///         // Handle data.
  ///     }
  /// }
  /// ```
  pub fn flush(self) -> Flush {
    Flush { encoder: self }
  }

  /// The width required of any input images.
  pub fn width(&self) -> u32 {
    self.params.i_width as u32
  }
  /// The height required of any input images.
  pub fn height(&self) -> u32 {
    self.params.i_height as u32
  }
  /// The encoding required of any input images.
  pub fn encoding(&self) -> Encoding {
    unsafe { Encoding::from_raw(self.params.i_csp as u32) }
  }
}

impl Drop for Encoder {
  fn drop(&mut self) {
    unsafe {
      x264_encoder_close(self.raw);
    }
  }
}

/// Iterate through any delayed frames.
pub struct Flush {
  encoder: Encoder,
}

impl Flush {
  /// Keeps flushing.
  pub fn next(&mut self) -> Option<Result<(Data, Picture)>> {
    let enc = self.encoder.raw;

    if unsafe { x264_encoder_delayed_frames(enc) } == 0 {
      return None;
    }

    let mut len = 0;
    let mut stuff = mem::MaybeUninit::uninit();
    let mut raw = mem::MaybeUninit::uninit();

    let err = unsafe {
      x264_encoder_encode(
        enc,
        stuff.as_mut_ptr(),
        &mut len,
        ptr::null_mut(),
        raw.as_mut_ptr(),
      )
    };

    Some(if err < 0 {
      Err(Error)
    } else {
      Ok(unsafe {
        (
          Data::from_raw_parts(stuff.assume_init(), len as usize),
          Picture::from_raw(raw.assume_init()),
        )
      })
    })
  }
}

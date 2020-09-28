#[macro_use]
extern crate napi;
#[macro_use]
extern crate napi_derive;
#[macro_use]
extern crate serde_derive;

use std::io::Cursor;
use std::result::Result as STDResult;

use napi::{CallContext, Env, Error, JsBuffer, JsObject, Module, Result, Status, Task};
use serde::{de, Deserialize, Deserializer};

#[cfg(all(unix, not(target_env = "musl")))]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

#[cfg(windows)]
#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

register_module!(pstv, init);

#[derive(Serialize, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PPTFrames {
  end_time: u64,
  start_time: u64,
  frames: Vec<PPTFrameMeta>,
}

#[derive(Serialize, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PPTFrameMeta {
  offset_top: u32,
  page_scale_factor: f32,
  device_width: u16,
  device_height: u16,
  scroll_offset_x: f32,
  scroll_offset_y: f32,
  timestamp: u64,
  #[serde(deserialize_with = "de_base64")]
  data: Vec<u8>,
}

fn de_base64<'de, D>(deserializer: D) -> STDResult<Vec<u8>, D::Error>
where
  D: Deserializer<'de>,
{
  let s = <&str>::deserialize(deserializer)?;
  base64::decode(s).map_err(de::Error::custom)
}

struct AsyncTask(PPTFrames);

impl Task for AsyncTask {
  type Output = Vec<u8>;
  type JsValue = JsBuffer;

  fn compute(&mut self) -> Result<Self::Output> {
    // let mut mp4_file = mp4::read_mp4(
    //   ::std::fs::File::open(
    //     "/Users/longyinan/workspace/bytedance/maiev/.local-tos/maiev/screenshot/1692.mp4",
    //   )
    //   .unwrap(),
    // )
    // .unwrap();
    // for track in mp4_file.tracks() {
    //   println!(
    //     "{:?}, {:?}, {:?}, {:?}",
    //     track.box_type(),
    //     track.track_id(),
    //     track.sequence_parameter_set(),
    //     track.picture_parameter_set()
    //   );
    // }
    // let sample_length = mp4_file.sample_count(1).unwrap();
    // for n in 1..sample_length + 1 {
    //   let sample = mp4_file.read_sample(1, n).unwrap().unwrap();
    //   println!(
    //     "{}, {}, {}, {}, {}",
    //     sample.start_time,
    //     sample.duration,
    //     sample.is_sync,
    //     sample.rendering_offset,
    //     sample.bytes.len(),
    //   );
    // }
    // Ok(vec![])
    let output: Vec<u8> = vec![];
    let mut cursor = Cursor::new(output);
    let config = mp4::Mp4Config {
      major_brand: mp4::FourCC::from("isom"),
      minor_version: 512,
      compatible_brands: vec![
        mp4::FourCC::from("isom"),
        mp4::FourCC::from("iso2"),
        mp4::FourCC::from("avc1"),
        mp4::FourCC::from("mp41"),
      ],
      timescale: 1000,
    };
    let mut mp4_writer = mp4::Mp4Writer::write_start(&mut cursor, &config)
      .map_err(|e| Error::from_reason(format!("{}", e)))?;
    let mut last_frame_timestamp = self.0.start_time;
    let mut start_time = 0;
    let first_frame = self
      .0
      .frames
      .get(0)
      .ok_or(Error::new(Status::InvalidArg, format!("Frames empty")))?;
    let frame_width = first_frame.device_width;
    let frame_height = first_frame.device_height;
    mp4_writer
      .add_track(&mp4::TrackConfig {
        track_type: mp4::TrackType::Video,
        timescale: 1000,
        language: "und".to_owned(),
        media_conf: mp4::MediaConfig::AvcConfig(mp4::AvcConfig {
          width: frame_width * 2,
          height: frame_height * 2,
          seq_param_set: vec![
            103, 66, 192, 50, 218, 0, 169, 3, 183, 185, 112, 22, 200, 0, 0, 3, 0, 8, 0, 0, 3, 1,
            144, 120, 193, 149,
          ],
          pic_param_set: vec![104, 206, 15, 200],
        }),
      })
      .map_err(|e| Error::from_reason(format!("{}", e)))?;
    let mut frame_encoder =
      x264::Setup::preset(x264::Preset::Veryfast, x264::Tune::Animation, true, true)
        .build(
          x264::Colorspace::RGB,
          frame_width as u32 * 2,
          frame_height as u32 * 2,
        )
        .map_err(|_| Error::from_reason("Create h264 encoder failed".to_owned()))?;
    for (index, frame) in self.0.frames.iter().enumerate() {
      let duration = frame.timestamp - last_frame_timestamp;
      start_time = start_time + duration;
      let mut image_decoder = jpeg_decoder::Decoder::new(frame.data.as_slice());
      let pixels = image_decoder
        .decode()
        .map_err(|e| Error::new(Status::GenericFailure, format!("Decode frame failed {}", e)))?;
      let image = x264::Image::rgb(
        frame_width as u32 * 2,
        frame_height as u32 * 2,
        pixels.as_slice(),
      );
      let (frame_data, _) = frame_encoder
        .encode((index as i64) * 60, image)
        .map_err(|_| Error::from_reason("Encode image to frame data failed".to_owned()))?;
      let frame_data_slice = frame_data.entirety();
      let sample = mp4::Mp4Sample {
        start_time,
        duration: duration as u32,
        rendering_offset: 0,
        is_sync: start_time == 0,
        bytes: mp4::Bytes::from_static(unsafe {
          ::std::slice::from_raw_parts(frame_data_slice.as_ptr(), frame_data_slice.len())
        }),
      };
      mp4_writer
        .write_sample(1, &sample)
        .map_err(|e| Error::from_reason(format!("{}", e)))?;
      last_frame_timestamp = frame.timestamp;
    }
    mp4_writer
      .write_end()
      .map_err(|e| Error::from_reason(format!("{}", e)))?;
    Ok(cursor.into_inner())
  }

  fn resolve(&self, env: &mut Env, output: Self::Output) -> Result<Self::JsValue> {
    env.create_buffer_with_data(output)
  }
}

fn init(module: &mut Module) -> Result<()> {
  module.create_named_method("encode", encode)?;
  Ok(())
}

#[js_function(1)]
fn encode(ctx: CallContext) -> Result<JsObject> {
  let frames_meta = ctx.get::<JsBuffer>(0)?;

  let task = AsyncTask(
    serde_json::from_slice(frames_meta.data)
      .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))?,
  );
  ctx.env.spawn(task)
}

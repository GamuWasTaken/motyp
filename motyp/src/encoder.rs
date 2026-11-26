use std::{fs::File, path::PathBuf};

use muxide::api::{MuxerBuilder, MuxerStats, VideoCodec};
use openh264::{
    OpenH264API,
    encoder::{BitRate, Complexity, EncoderConfig, FrameRate, FrameType, RateControlMode},
    formats::YUVSlices,
};
use yuv::{YuvConversionMode, YuvRange, YuvStandardMatrix, rgba_to_yuv420};

use crate::{Error, Profile, renderer::RenderedFrame};

pub struct Config {
    pub bitrate: u32,
    pub fps: f32,
    pub height: u32,
    pub output: PathBuf,
    pub profile: Profile,
    pub width: u32,
}
impl From<&super::Config> for Config {
    fn from(value: &super::Config) -> Self {
        Self {
            bitrate: value.bitrate,
            fps: value.fps,
            height: value.height,
            output: value.output.clone(),
            profile: value.profile,
            width: value.width,
        }
    }
}
impl TryFrom<&super::Config> for Encoder {
    type Error = Error;

    fn try_from(value: &super::Config) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

pub struct Encoder {
    config: Config,
    encoder: openh264::encoder::Encoder,
    muxer: muxide::api::Muxer<File>,
    yuv: yuv::YuvPlanarImageMut<'static, u8>,
    bitstream: Vec<u8>,
}

impl Encoder {
    pub fn new(config: impl Into<Config>) -> Result<Self, Error> {
        let config = config.into();
        let rate_control_mode = match config.profile {
            crate::Profile::Dev => RateControlMode::Bufferbased,
            crate::Profile::Release => RateControlMode::Off,
        };

        let encoder_config = EncoderConfig::new()
            .bitrate(BitRate::from_bps(config.bitrate))
            .max_frame_rate(FrameRate::from_hz(config.fps))
            .rate_control_mode(rate_control_mode)
            .skip_frames(false)
            .adaptive_quantization(false);

        let encoder_config = match config.profile {
            crate::Profile::Dev => encoder_config
                .complexity(Complexity::Low)
                .background_detection(false)
                .scene_change_detect(false),
            crate::Profile::Release => encoder_config
                .complexity(Complexity::High)
                .background_detection(true)
                .scene_change_detect(true),
        };

        let encoder =
            openh264::encoder::Encoder::with_api_config(OpenH264API::from_source(), encoder_config)
                .map_err(Error::openh264_init)?;

        let file = File::create(&config.output).map_err(Error::output_not_creatable)?;
        let muxer = MuxerBuilder::new(file)
            .video(
                VideoCodec::H264,
                config.width,
                config.height,
                config.fps as _,
            )
            .build()
            .map_err(Error::muxer_init)?;
        let yuv = yuv::YuvPlanarImageMut::alloc(
            config.width,
            config.height,
            yuv::YuvChromaSubsampling::Yuv420,
        );

        Ok(Self {
            config,
            encoder,
            muxer,
            yuv,
            bitstream: Vec::new(),
        })
    }

    pub fn push_frame(&mut self, frame: RenderedFrame) -> Result<(), Error> {
        rgba_to_yuv420(
            &mut self.yuv,
            &frame.data.take(),
            self.config.width * 4,
            YuvRange::Limited,
            YuvStandardMatrix::Bt601,
            YuvConversionMode::Balanced,
        )
        .map_err(Error::yuv_failed)?;

        let source = YUVSlices::new(
            (
                self.yuv.y_plane.borrow(),
                self.yuv.u_plane.borrow(),
                self.yuv.v_plane.borrow(),
            ),
            (self.config.width as usize, self.config.height as usize),
            (
                self.yuv.y_stride as usize,
                self.yuv.u_stride as usize,
                self.yuv.v_stride as usize,
            ),
        );

        let encoded_stream = self
            .encoder
            .encode(&source)
            .map_err(Error::openh264_failed)?;
        self.bitstream.clear();
        encoded_stream.write_vec(&mut self.bitstream);

        self.muxer
            .write_video(
                (frame.id as f32 / self.config.fps) as _,
                &self.bitstream,
                matches!(encoded_stream.frame_type(), FrameType::IDR | FrameType::I),
            )
            .map_err(Error::muxer_failed)?;

        Ok(())
    }

    pub fn finish(self) -> Result<MuxerStats, Error> {
        self.muxer
            .finish_with_stats()
            .map_err(Error::output_not_writeable)
    }
}

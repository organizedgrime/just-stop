use ::image::RgbaImage;
use iced::widget::image::Handle;
use nokhwa::pixel_format::RgbAFormat;
use nokhwa::Camera;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct CameraFeed {
    camera: Arc<Mutex<nokhwa::Camera>>,
    current_fame: Arc<Mutex<Option<Handle>>>,
    mirror: bool,
    aspect_ratio: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CameraMessage {
    CaptureFrame,
    NewFrame(Handle),
}

fn decode_frame(camera: &mut Camera) -> RgbaImage {
    camera
        .frame()
        .expect("camera frame")
        .decode_image::<RgbAFormat>()
        .expect("decode frame")
}

impl CameraFeed {
    pub fn frame(&mut self) -> RgbaImage {
        self.camera
            .lock()
            .expect("lock camera mutex")
            .frame()
            .expect("camera frame")
            .decode_image::<RgbAFormat>()
            .expect("decode frame")
    }

    pub fn update(&mut self, message: CameraMessage) {
        match message {
            CameraMessage::CaptureFrame => {
                let mut frame = self.frame();
                let mirror = self.mirror;
                let message = async move {
                    tokio::task::spawn_blocking(move || {
                        if mirror {
                            image::imageops::flip_horizontal_in_place(&mut frame);
                        }
                        Handle::from_rgba(frame.width(), frame.height(), frame.into_raw())
                    })
                    .await
                    .unwrap()
                };
                self.update(message.await);
            }
            CameraMessage::NewFrame(handle) => todo!(),
        }
    }
}

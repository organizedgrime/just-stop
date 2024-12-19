use std::io::Read;

use iced::time::{self, Duration, Instant};

use iced::widget::image::Handle;
use iced::widget::{button, column, image, pick_list, row, scrollable, text, vertical_space};
use iced::{application, Center, Element, Fill, Subscription};
use nokhwa::pixel_format::RgbFormat;
use nokhwa::utils::{
    yuyv422_predicted_size, CameraFormat, CameraIndex, CameraInfo, RequestedFormat,
    RequestedFormatType, Resolution,
};
use nokhwa::{native_api_backend, query, CallbackCamera, Camera, FormatDecoder};

pub fn main() -> iced::Result {
    application("Just Stop", JustStop::update, JustStop::view)
        .subscription(JustStop::subscription)
        .run()
}

#[derive(Default)]
struct JustStop {
    available_devices: Vec<String>,
    available_indices: Vec<CameraIndex>,
    selected_device: Option<String>,
    camera: Option<CallbackCamera>,
    handle: Option<Handle>,
    buffer: Vec<u8>,
}

#[derive(Debug, Clone)]
enum Message {
    CaptureFrame,
    DeviceSelected(String),
    RefreshDeviceList,
}

impl JustStop {
    fn update(&mut self, message: Message) {
        use Message::*;
        match message {
            CaptureFrame => {
                if let Some(camera) = &mut self.camera {
                    println!("i want another frame!");
                    if camera.is_stream_open().unwrap() {
                        if let Ok(buffer) = camera.poll_frame() {
                            // self.buffer
                            //     .resize(yuyv422_predicted_size(buffer.buffer().len(), true), 0);
                            self.handle = Some(Handle::from_bytes(buffer.buffer_bytes()));
                            // buffer
                            //     .decode_image_to_buffer::<RgbFormat>(&mut self.buffer)
                            //     .unwrap();
                            // self.handle = Some(Handle::from_rgba(
                            //     buffer.resolution().width(),
                            //     buffer.resolution().height(),
                            //     self.buffer.clone(),
                            // ));
                        }
                    }
                }
            }
            DeviceSelected(device_name) => {
                // Close the stream if it's still open
                if let Some(camera) = &mut self.camera {
                    if camera.is_stream_open().unwrap() {
                        camera.stop_stream().unwrap();
                    }
                }

                let index = self
                    .available_devices
                    .iter()
                    .position(|name| *name == device_name)
                    .unwrap()
                    .clone();

                let requested = RequestedFormat::new::<RgbFormat>(
                    RequestedFormatType::AbsoluteHighestFrameRate,
                );

                self.camera = Some(
                    CallbackCamera::new(
                        self.available_indices[index].clone(),
                        requested,
                        |buffer| {
                            // buffer
                            //     .clone()
                            //     .decode_image_to_buffer::<RgbFormat>(&mut self.image_data)
                            //     .unwrap();
                        },
                    )
                    .unwrap(),
                );

                // self.camera =
                //     Some(Camera::new(self.available_indices[index].clone(), requested).unwrap());

                // Open the stream
                if let Some(camera) = &mut self.camera {
                    camera.open_stream().unwrap();
                }
            }
            RefreshDeviceList => {
                let backend = native_api_backend().unwrap();
                let devices = query(backend).unwrap();
                self.available_devices = devices.iter().map(CameraInfo::human_name).collect();
                self.available_indices = devices.iter().map(CameraInfo::index).cloned().collect();
            }
        }
    }

    fn view(&self) -> Element<Message> {
        let pick_list = pick_list(
            &self.available_devices[..],
            self.selected_device.clone(),
            Message::DeviceSelected,
        )
        .placeholder("Choose a language...");

        let refresh_button = button(text("refresh list")).on_press(Message::RefreshDeviceList);

        //let c = if let Some(fb) = &self.frame_buffer.map(nokhwa::Buffer::buffer) {
        // if let Some(fb) = self.frame_buffer {
        //     let img = fb.decode_image().unwrap();
        // }
        let c = if let Some(fb) = self.handle.as_ref() {
            println!("yippee!");
            row![image(fb)]
        } else {
            row![]
        };
        // let bytes = if let Some(fb) = &self.frame_buffer {
        //         if fb.buffer()
        // } else {
        //     &[]
        // };

        let content = column!["Select a camera", pick_list, refresh_button, c]
            .width(Fill)
            .align_x(Center)
            .spacing(10);

        scrollable(content).into()
    }

    fn subscription(&self) -> Subscription<Message> {
        if let Some(camera) = &self.camera {
            let millis = (1000.0 / camera.frame_rate().unwrap() as f32) as u64;
            println!("waiting {millis}ms to request another frame");
            time::every(Duration::from_millis(millis)).map(|_| Message::CaptureFrame)
        } else {
            Subscription::none()
        }
    }
}

use iced::time::{self, Duration, Instant};

use iced::widget::image::Handle;
use iced::widget::{button, column, image, pick_list, scrollable, text, vertical_space};
use iced::{application, Center, Element, Fill, Subscription};
use nokhwa::pixel_format::RgbFormat;
use nokhwa::utils::{CameraIndex, CameraInfo, RequestedFormat, RequestedFormatType};
use nokhwa::{native_api_backend, query, Camera, FormatDecoder};

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
    camera: Option<Camera>,
    frame_buffer: Option<nokhwa::Buffer>,
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
                    if camera.is_stream_open() {
                        self.frame_buffer = camera.frame().ok();
                    }
                }
            }
            DeviceSelected(device_name) => {
                // Close the stream if it's still open
                if let Some(camera) = &mut self.camera {
                    if camera.is_stream_open() {
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

                self.camera =
                    Some(Camera::new(self.available_indices[index].clone(), requested).unwrap());

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
        let bytes = if let Some(fb) = &self.frame_buffer {
            fb.buffer().to_vec()
        } else {
            vec![]
        };

        let content = column![
            vertical_space().height(600),
            "Select a camera",
            pick_list,
            refresh_button,
            image(Handle::from_bytes(bytes)),
            vertical_space().height(600),
        ]
        .width(Fill)
        .align_x(Center)
        .spacing(10);

        scrollable(content).into()
    }

    fn subscription(&self) -> Subscription<Message> {
        if self.camera.is_some() {
            time::every(Duration::from_millis(10)).map(|_| Message::CaptureFrame)
        } else {
            Subscription::none()
        }
    }
}

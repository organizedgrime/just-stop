use std::collections::HashMap;
use std::fmt::Display;

use iced::widget::{button, column, pick_list, scrollable, text, vertical_space};
use iced::{Center, Element, Fill};
use nokhwa::pixel_format::RgbFormat;
use nokhwa::utils::{CameraIndex, CameraInfo, RequestedFormat, RequestedFormatType};
use nokhwa::{native_api_backend, query, Camera};

pub fn main() -> iced::Result {
    iced::run("Pick List - Iced", JustStop::update, JustStop::view)
}

#[derive(Default)]
struct JustStop {
    available_devices: Vec<String>,
    available_indices: Vec<CameraIndex>,
    selected_device: Option<String>,
    camera: Option<Camera>,
}

#[derive(Debug, Clone)]
enum Message {
    DeviceSelected(String),
    RefreshDeviceList,
}

impl JustStop {
    fn update(&mut self, message: Message) {
        use Message::*;
        match message {
            DeviceSelected(device_name) => {
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

        //let p2 = pick_list(options, selected, on_selected).placeholder("");

        let content = column![
            vertical_space().height(600),
            "Which is your favorite language?",
            pick_list,
            refresh_button,
            vertical_space().height(600),
        ]
        .width(Fill)
        .align_x(Center)
        .spacing(10);

        scrollable(content).into()
    }
}

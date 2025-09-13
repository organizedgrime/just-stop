use anyhow::{Context as _, Result};
use ffmpeg_sidecar::command::FfmpegCommand;
use ffmpeg_sidecar::event::{FfmpegEvent, LogLevel};
use inquire::Select;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use v4l::capability::Flags;
use v4l::video::Capture;
use v4l::{Capabilities, Device, FourCC};

#[derive(Debug, Clone)]
struct DeviceInfo {
    index: usize,
    path: String,
    name: String,
    driver: String,
    capabilities: Flags,
}

impl DeviceInfo {
    pub fn path(&self) -> String {
        format!("/dev/video{}", self.index)
    }

    pub fn device(&self) -> Result<Device> {
        Device::new(self.index).map_err(|e| {
            anyhow::format_err!(
                "Failed to open {}: {}. Is your webcam connected?",
                self.path(),
                e
            )
        })
    }
}

struct Config {
    input: DeviceInfo,
    output: DeviceInfo,
}

impl std::fmt::Display for DeviceInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} - {} ({})", self.path, self.name, self.driver)
    }
}

fn discover_devices() -> Result<Vec<DeviceInfo>> {
    let nodes = v4l::context::enum_devices();
    let mut devices = Vec::new();

    for node in nodes {
        let index = node.index();
        if let Ok(device) = Device::new(index) {
            if let Ok(caps) = device.query_caps() {
                devices.push(DeviceInfo {
                    index,
                    path: node.path().to_string_lossy().to_string(),
                    name: node.name().unwrap_or_else(|| "Unknown Device".to_string()),
                    driver: caps.driver,
                    capabilities: caps.capabilities,
                });
            }
        }
    }

    Ok(devices)
}

fn pick_device<'a>(
    devices: &'a Vec<DeviceInfo>,
    kind: &'a str,
    requirements: Flags,
) -> Result<DeviceInfo> {
    let eligible_devices: Vec<&DeviceInfo> = devices
        .into_iter()
        .filter(|device| (device.capabilities & requirements).bits() != 0)
        .collect();

    if eligible_devices.is_empty() {
        anyhow::bail!("No {kind} devices found");
    }

    // Multiple devices - let user pick
    Select::new(&format!("Select {kind} device:"), eligible_devices)
        .prompt()
        .context("Device selection cancelled")
        .cloned()
}

fn select_device_settings(info: &DeviceInfo) -> Result<()> {
    let device = info.device()?;
    let formats = device.enum_formats()?;

    Select::new(&format!("Select format for {}:", info.path), formats)
        .prompt()
        .context("Format selection cancelled")?;

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Scanning for video devices...");
    let devices = discover_devices()?;
    let input = pick_device(&devices, "input", Flags::VIDEO_CAPTURE)?;
    let output = pick_device(&devices, "output", Flags::VIDEO_OUTPUT)?;
    // let config = Config { input, output };

    println!(
        "Input: /dev/video{} -> Output: /dev/video{}",
        input.index, output.index
    );

    select_device_settings(&input)?;
    select_device_settings(&output)?;

    println!("Press Ctrl+C to stop the stream");

    // Set up graceful shutdown
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        println!("\nReceived Ctrl+C, shutting down gracefully...");
        r.store(false, Ordering::SeqCst);
    })?;

    // Check and set up devices
    // let (input_device, output_device) = setup_devices()?;

    // Start the streaming loop
    // stream_with_grid_filter(input_device, output_device, running)?;

    println!("Stream ended gracefully");
    Ok(())
}

/* fn setup_devices() -> Result<(Device, Device), Box<dyn std::error::Error>> {
    println!("Setting up video devices...");

    // Open and configure input device

    println!("Input device capabilities:");
    let input_caps = input_device.query_caps()?;
    println!("  Driver: {}", input_caps.driver);
    println!("  Card: {}", input_caps.card);
    println!("  Capabilities: 0x{:x}", input_caps.capabilities);

    // Check if input device supports video capture - FIXED LOGIC
    if (input_caps.capabilities & v4l::capability::Flags::VIDEO_CAPTURE).bits() == 0 {
        return Err("Input device does not support video capture".into());
    }

    println!("Available formats:");
    for format in input_device.enum_formats()? {
        println!("  {} ({})", format.fourcc, format.description);
        for framesize in input_device.enum_framesizes(format.fourcc)? {
            for discrete in framesize.size.to_discrete() {
                println!("    Size: {}", discrete);
                for frameinterval in input_device.enum_frameintervals(
                    framesize.fourcc,
                    discrete.width,
                    discrete.height,
                )? {
                    println!("      Interval:  {}", frameinterval);
                }
            }
        }

        println!()
    }

    let input_format = input_device.format()?;
    println!(
        "Input format: {}x{}, fourcc: {}",
        input_format.width,
        input_format.height,
        FourCC::from(input_format.fourcc)
    );

    // Open output device (virtual camera)
    let output_device = Device::new(2)
        .map_err(|e| format!("Failed to open /dev/video2: {}. Create virtual camera with: sudo modprobe v4l2loopback devices=1 video_nr=2", e))?;

    println!("Output device capabilities:");
    let output_caps = output_device.query_caps()?;
    println!("  Driver: {}", output_caps.driver);
    println!("  Card: {}", output_caps.card);

    // Check if output device supports video output - FIXED LOGIC
    if (output_caps.capabilities & v4l::capability::Flags::VIDEO_OUTPUT).bits() == 0 {
        return Err("Output device does not support video output".into());
    }

    Ok((input_device, output_device))
} */

fn stream_with_grid_filter(
    input: DeviceInfo,
    output: DeviceInfo,
    running: Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting ffmpeg with grid filter...");

    // Build ffmpeg command - FIXED REDUNDANT ARGS
    let mut ffmpeg = FfmpegCommand::new()
        .input(format!("/dev/video{}", input.index))
        .args(["-f", "v4l2"]) // Input format
        // .args(["-video_size", "1920x1080"]) // Set reasonable default size
        .args(["-framerate", "30"]) // Input framerate
        .filter("drawgrid=width=iw/3:height=ih/4:thickness=3:color=pink@1.0")
        .args(["-f", "v4l2"]) // Output format
        .args(["-pix_fmt", "yuv420p"]) // Pixel format
        .output(format!("/dev/video{}", output.index))
        .spawn()?;

    println!("FFmpeg process started");
    println!("Grid configuration: 3 columns × 4 rows in pink color");

    // Monitor ffmpeg events
    let iter = ffmpeg.iter()?;
    for event in iter {
        if !running.load(Ordering::SeqCst) {
            break;
        }

        match event {
            FfmpegEvent::Log(LogLevel::Info, msg) => {
                println!("FFmpeg info: {}", msg);
            }
            FfmpegEvent::Log(LogLevel::Warning, msg) => {
                println!("FFmpeg warning: {}", msg);
            }
            FfmpegEvent::Log(LogLevel::Error, msg) => {
                eprintln!("FFmpeg error: {}", msg);
            }
            FfmpegEvent::Progress(progress) => {
                if progress.frame % 30 == 0 {
                    // Log every second at 30fps
                    println!(
                        "Processed {} frames, time: {:.2}s",
                        progress.frame, progress.time
                    );
                }
            }
            FfmpegEvent::LogEOF => {
                println!("FFmpeg log ended");
                break;
            }
            _ => {}
        }
    }

    // If we're shutting down, kill the ffmpeg process
    if !running.load(Ordering::SeqCst) {
        println!("Terminating ffmpeg process...");
        ffmpeg.kill()?;
    }

    // Wait for process to fully exit
    let exit_status = ffmpeg.wait()?;
    if exit_status.success() {
        println!("FFmpeg exited successfully");
    } else {
        println!("FFmpeg exited with code: {:?}", exit_status.code());
    }

    Ok(())
}

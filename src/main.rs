use anyhow::{Context as _, Result};
use ffmpeg_sidecar::command::FfmpegCommand;
use ffmpeg_sidecar::event::{FfmpegEvent, LogLevel};
use inquire::Select;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use v4l::capability::Flags;
use v4l::{Device, FourCC, Fraction, video::Capture};
mod conf;
mod device;
mod pixel;

use crate::pixel::create_fourcc_to_ffmpeg_map_owned;
use conf::*;
use device::*;

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
                    capabilities: caps.capabilities.into(),
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
) -> Result<JustDevice> {
    let eligible_devices: Vec<&DeviceInfo> = devices
        .into_iter()
        .filter(|device| (Flags::from(device.capabilities) & requirements).bits() != 0)
        .collect();

    if eligible_devices.is_empty() {
        anyhow::bail!("No {kind} devices found");
    }

    let info: DeviceInfo = Select::new(&format!("Select {kind} device:"), eligible_devices)
        .prompt()
        .context("Device selection cancelled")
        .cloned()?;

    let device = info.device()?;

    let formats: Vec<JustFormatDescription> = device
        .enum_formats()?
        .into_iter()
        .map(JustFormatDescription)
        .collect();

    let format = Select::new(&format!("Select format for {}:", info.path), formats)
        .prompt()
        .context("Format selection cancelled")?;

    println!("format: {}", format.0);

    /* let sizes: Vec<FrameSize> = device.enum_framesizes(format.fourcc())?;
        let mut discretes: Vec<JustFrameSize> = vec![];
        for size in sizes {
            for discrete in size.size.to_discrete() {
                discretes.push(JustFrameSize {
                    fourcc: size.fourcc,
                    width: discrete.width,
                    height: discrete.height,
                });
            }
        }

        let size = Select::new("Select frame size:", discretes)
            .prompt()
            .context("Frame size selection cancelled")?;

        let intervals = device.enum_frameintervals(size.fourcc, size.width, size.height)?;
        let mut fractions = vec![];
        for interval in intervals {
            if let FrameIntervalEnum::Discrete(fraction) = interval.interval {
                fractions.push(fraction);
            } else {
                anyhow::bail!("Stepwise fps not yet implemented");
            }
        }

        let fraction = Select::new("Select fraction (aka fps):", fractions)
            .prompt()
            .context("Fraction selection cancelled")?;
    */

    let settings = DeviceSettings {
        format: format.fourcc().repr,
        size: JustFrameSize {
            // fourcc: FourCC::default(),
            width: 1920,
            height: 1080,
        },
        fraction: JustFraction {
            numerator: 1,
            denominator: 30,
        },
    };

    println!("settings selected:\n{settings:?}");

    Ok(JustDevice { info, settings })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Conf::setup()?;

    println!("🔍 Scanning for video devices...");
    let devices = discover_devices()?;
    let input = pick_device(&devices, "input", Flags::VIDEO_CAPTURE)?;
    let output = pick_device(&devices, "output", Flags::VIDEO_OUTPUT)?;

    println!("Input: {} -> Output: {}", input.info.path, output.info.path);
    println!("Press Ctrl+C to stop the stream");

    // Set up graceful shutdown
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        println!("\nReceived Ctrl+C, shutting down gracefully...");
        r.store(false, Ordering::SeqCst);
    })?;

    let config = Conf { input, output };
    // Start the streaming loop
    stream_with_grid_filter(config, running)?;

    println!("Stream ended gracefully");
    Ok(())
}

fn stream_with_grid_filter(
    config: Conf,
    running: Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting ffmpeg with grid filter...");

    let map = create_fourcc_to_ffmpeg_map_owned();

    println!("GOT THE MAP");
    let Conf { input, output } = config;

    let stringfmt = FourCC::new(&input.settings.format).to_string();
    let pixfmt = map.get(&stringfmt).unwrap();

    // Build ffmpeg command
    let mut ffmpeg = FfmpegCommand::new()
        .args(["-f", "v4l2"]) // Force v4l2 for input
        // .args(["-video_size", &input.settings.size.to_string()]) // Set reasonable default size
        .args(["-r", &input.settings.ffmpeg_r()]) // Input framerate
        .input(input.info.path)
        .filter_complex("hue=s=0")
        .args(["-f", "v4l2"]) // Output format
        .args(["-pix_fmt", &pixfmt])
        .args(["-fflags", "+genpts"])
        .args(["-use_wallclock_as_timestamps", "1"])
        .output(&output.info.path)
        .print_command()
        .spawn()?;

    println!("FFmpeg process started");

    thread::sleep(Duration::from_secs(2));

    let mut ffplay = Command::new("ffplay")
        .args(["-i", &output.info.path])
        .spawn()?;

    println!("Grid configuration: 3 columns × 4 rows in pink color");

    // Monitor ffmpeg events
    let iter = ffmpeg.iter()?;
    for event in iter {
        if !running.load(Ordering::SeqCst) {
            break;
        } else if let Some(result) = ffplay.try_wait()? {
            println!("{}", result.to_string());
            ffmpeg.kill()?;
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
        ffplay.kill()?;
    }

    // Wait for process to fully exit
    let exit_status = ffmpeg.wait()?;
    if exit_status.success() {
        println!("FFmpeg exited successfully");
    } else {
        println!("FFmpeg exited with code: {:?}", exit_status.code());
    }
    // Wait for process to fully exit
    let exit_status = ffplay.wait()?;
    if exit_status.success() {
        println!("FFPlay exited successfully");
    } else {
        println!("FFPlay exited with code: {:?}", exit_status.code());
    }

    Ok(())
}

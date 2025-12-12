use anyhow::{Context as _, Result};
use ffmpeg_sidecar::command::FfmpegCommand;
use ffmpeg_sidecar::event::{FfmpegEvent, LogLevel};
use inquire::Select;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use v4l::capability::Flags;
use v4l::{video::Capture, Device, FourCC, Fraction};
mod conf;
mod device;
mod pixel;

use crate::pixel::create_fourcc_to_ffmpeg_map_owned;
use conf::*;
use device::*;

// Trigger file paths
const TMPDIR: &str = "/tmp/just_stop";
const TRIGGER_CAPTURE: &str = "/tmp/just_stop/capture.trigger";
const TRIGGER_DELETION: &str = "/tmp/just_stop/delete.trigger";
const TRIGGER_PLAYBACK: &str = "/tmp/just_stop/playback.trigger";
const PHOTO_DIR: &str = "./photos";
const FILE_PREFIX: &str = "photo";

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

    // For output devices (like v4l2 loopback), formats may not be enumerable
    // Use a default format if none are available
    let format = if formats.is_empty() {
        println!("No formats available for {}, using default YUYV", info.path);
        FourCC::new(b"YUYV")
    } else {
        let selected = Select::new(&format!("Select format for {}:", info.path), formats)
            .prompt()
            .context("Format selection cancelled")?;
        println!("format: {}", selected.0);
        selected.fourcc()
    };

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
        format: format.repr,
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
    // Ensure config directory exists
    Conf::setup()?;

    // Clean up any orphaned ffmpeg/ffplay processes from previous runs
    println!("Cleaning up any existing processes...");
    Command::new("pkill")
        .args(["-f", "ffmpeg.*video"])
        .output()
        .ok();
    Command::new("pkill").arg("ffplay").output().ok();
    thread::sleep(Duration::from_millis(500));

    // Try to load existing config, or create new one interactively
    let config = match Conf::load() {
        Ok(conf) => {
            println!("✓ Loaded existing configuration");
            println!(
                "Input: {} -> Output: {}",
                conf.input.info.path, conf.output.info.path
            );
            conf
        }
        Err(_) => {
            println!("No existing config found, creating new configuration...");
            println!("🔍 Scanning for video devices...");
            let devices = discover_devices()?;
            let input = pick_device(&devices, "input", Flags::VIDEO_CAPTURE)?;
            let output = pick_device(&devices, "output", Flags::VIDEO_OUTPUT)?;

            let config = Conf { input, output };

            // Save the newly created config
            config.save()?;
            println!("✓ Configuration saved");
            println!(
                "Input: {} -> Output: {}",
                config.input.info.path, config.output.info.path
            );

            config
        }
    };

    // Create necessary directories
    fs::create_dir_all(TMPDIR)?;
    fs::create_dir_all(PHOTO_DIR)?;

    println!("Press Ctrl+C to stop the stream");

    // Set up graceful shutdown
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        println!("\nReceived Ctrl+C, shutting down gracefully...");
        r.store(false, Ordering::SeqCst);
    })?;

    // Start the streaming loop
    let mut restart = true;
    while restart {
        restart = stream_with_grid_filter(&config, running.clone())?;
    }

    // Cleanup
    fs::remove_dir_all(TMPDIR).ok();

    println!("Stream ended gracefully");
    Ok(())
}

fn get_latest_photo() -> Option<PathBuf> {
    fs::read_dir(PHOTO_DIR)
        .ok()?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext == "bmp")
                .unwrap_or(false)
        })
        .filter_map(|entry| {
            let metadata = entry.metadata().ok()?;
            let modified = metadata.modified().ok()?;
            Some((entry.path(), modified))
        })
        .max_by_key(|(_, modified)| *modified)
        .map(|(path, _)| path)
}

fn get_photo_count() -> usize {
    fs::read_dir(PHOTO_DIR)
        .map(|entries| {
            entries
                .filter_map(|entry| entry.ok())
                .filter(|entry| {
                    entry
                        .path()
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .map(|ext| ext == "bmp")
                        .unwrap_or(false)
                })
                .count()
        })
        .unwrap_or(0)
}

fn capture_photo(output_device_path: &str) -> Result<()> {
    let timestamp = chrono::Local::now().format("%Y_%m_%d_%H_%M_%S");
    let count = get_photo_count();
    let filename_only = format!("{}_{:03}_{}.bmp", FILE_PREFIX, count, timestamp);
    let filename = format!("{}/{}", PHOTO_DIR, filename_only);
    let symlink = format!("{}/latest.bmp", PHOTO_DIR);

    println!("Capturing photo to {}...", filename);

    // Capture from the output device (virtual cam) which is already receiving the stream
    // This avoids the "device busy" issue since we can read from v4l2loopback
    let output = Command::new("ffmpeg")
        .args(["-f", "v4l2"])
        .args(["-i", output_device_path])
        .args(["-frames:v", "1"])
        .args(["-lossless", "1"])
        .args(["-y", &filename])
        .output()?;

    if output.status.success() {
        println!("✓ Captured: {}", filename);

        // Remove old symlink if it exists
        let _ = fs::remove_file(&symlink);

        // Create symlink with just the filename (not full path)
        Command::new("ln")
            .arg("-s")
            .args([&filename_only, &symlink])
            .output()?;
        Ok(())
    } else {
        anyhow::bail!(
            "Failed to capture photo: {}",
            String::from_utf8_lossy(&output.stderr)
        )
    }
}

fn delete_latest_photo() -> Result<()> {
    if let Some(latest) = get_latest_photo() {
        println!("Deleting {}...", latest.display());
        fs::remove_file(&latest)?;
        println!("✓ Deleted");
        Ok(())
    } else {
        anyhow::bail!("No photos to delete")
    }
}

fn create_playback(output_device: &str) -> Result<()> {
    let playback_file = format!("{}/playback.mp4", TMPDIR);

    // Remove old playback file if it exists
    if Path::new(&playback_file).exists() {
        fs::remove_file(&playback_file)?;
    }

    println!("Creating playback video...");

    let pattern = format!("{}/*.bmp", PHOTO_DIR);

    // Create the video from photos
    let output = Command::new("ffmpeg")
        .args(["-framerate", "12"])
        .args(["-pattern_type", "glob"])
        .args(["-i", &pattern])
        .args([
            "-filter_complex",
            "[0:v]fps=30,scale=height=ih:width=iw,format=yuv420p[output]",
        ])
        .args(["-map", "[output]"])
        .args(["-c:v", "libx264"])
        .args(["-y", &playback_file])
        .output()?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to create playback: {}",
            String::from_utf8_lossy(&output.stderr)
        )
    }

    println!("Playing back video...");

    // Play the video to the output device
    let output = Command::new("ffmpeg")
        .args(["-re"])
        .args(["-i", &playback_file])
        .args(["-f", "v4l2"])
        .arg(output_device)
        .output()?;

    if output.status.success() {
        println!("✓ Playback complete");
        Ok(())
    } else {
        anyhow::bail!(
            "Failed to play video: {}",
            String::from_utf8_lossy(&output.stderr)
        )
    }
}

fn stream_with_grid_filter(
    config: &Conf,
    running: Arc<AtomicBool>,
) -> Result<bool, Box<dyn std::error::Error>> {
    println!("Starting ffmpeg with grid filter...");

    let map = create_fourcc_to_ffmpeg_map_owned();

    println!("GOT THE MAP");
    let Conf { input, output } = config;

    // Clone paths for later use
    let input_path = input.info.path.clone();
    let output_path = output.info.path.clone();
    let framerate = input.settings.ffmpeg_r();

    let stringfmt = FourCC::new(&input.settings.format).to_string();
    let pixfmt = map.get(&stringfmt).unwrap().clone();

    // Build ffmpeg command
    let latest_photo = format!("{}/latest.bmp", PHOTO_DIR);
    let filter = if Path::new(&latest_photo).exists() {
        // Overlay the latest photo with 55% opacity
        format!(
            "[0:v]hue=s=0[base];movie={}:loop=0,setpts=N/(FRAME_RATE*TB),format=yuva420p,colorchannelmixer=aa=0.55[overlay];[base][overlay]overlay",
            latest_photo
        )
    } else {
        // No photo yet, just apply hue filter
        "hue=s=0".to_string()
    };

    let mut ffmpeg = FfmpegCommand::new()
        .args(["-f", "v4l2"]) // Force v4l2 for input
        .args(["-r", &framerate]) // Input framerate
        .input(&input_path)
        .filter_complex(&filter)
        .args(["-f", "v4l2"])
        .args(["-pix_fmt", &pixfmt])
        .args(["-fflags", "+genpts"])
        .args(["-use_wallclock_as_timestamps", "1"])
        .output(&output_path)
        .print_command()
        .spawn()?;

    println!("FFmpeg process started");

    thread::sleep(Duration::from_secs(2));

    let mut ffplay = Command::new("ffplay").args(["-i", &output_path]).spawn()?;

    println!("Grid configuration: 3 columns × 4 rows in pink color");

    // Monitor ffmpeg events
    let iter = ffmpeg.iter()?;
    for event in iter {
        // Check for trigger files
        if Path::new(TRIGGER_CAPTURE).exists() {
            fs::remove_file(TRIGGER_CAPTURE)?;
            println!("\n📸 Capture triggered");

            // Only kill ffplay, keep ffmpeg running so we can capture from output device
            ffplay.kill()?;

            if let Err(e) = capture_photo(&output_path) {
                eprintln!("Capture failed: {}", e);
            }

            ffmpeg.kill()?;

            // Restart ffplay after capture
            return Ok(true);
        }

        if Path::new(TRIGGER_DELETION).exists() {
            fs::remove_file(TRIGGER_DELETION)?;
            println!("\n🗑️  Delete triggered");
            if let Err(e) = delete_latest_photo() {
                eprintln!("Delete failed: {}", e);
            }
            return Ok(true);
        }

        if Path::new(TRIGGER_PLAYBACK).exists() {
            fs::remove_file(TRIGGER_PLAYBACK)?;
            println!("\n🎬 Playback triggered");
            // Kill current stream for playback
            ffmpeg.kill()?;
            ffplay.kill()?;

            if let Err(e) = create_playback(&output_path) {
                eprintln!("Playback failed: {}", e);
            }

            // Break to restart stream
            return Ok(true);
        }

        if !running.load(Ordering::SeqCst) {
            ffmpeg.kill()?;
            ffplay.kill()?;
            return Ok(false);
        } else if let Some(result) = ffplay.try_wait()? {
            println!("{}", result.to_string());
            ffmpeg.kill()?;
            return Ok(false);
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

    println!("Stream ended gracefully");
    Ok(false)
}

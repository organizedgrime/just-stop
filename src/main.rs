use anyhow::Result;
use ffmpeg_sidecar::command::FfmpegCommand;
use ffmpeg_sidecar::event::{FfmpegEvent, LogLevel};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use v4l::prelude::*;
use v4l::video::Capture;
use v4l::{Device, FourCC};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting webcam stream with pink grid overlay...");
    println!("Input: /dev/video0 -> Output: /dev/video2");
    println!("Press Ctrl+C to stop the stream");

    // Set up graceful shutdown
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        println!("\nReceived Ctrl+C, shutting down gracefully...");
        r.store(false, Ordering::SeqCst);
    })?;

    // Check and set up devices
    let (input_device, output_device) = setup_devices()?;

    // Start the streaming loop
    stream_with_grid_filter(input_device, output_device, running)?;

    println!("Stream ended gracefully");
    Ok(())
}

fn setup_devices() -> Result<(Device, Device), Box<dyn std::error::Error>> {
    println!("Setting up video devices...");

    // Open and configure input device
    let input_device = Device::new(0).map_err(|e| {
        format!(
            "Failed to open /dev/video0: {}. Is your webcam connected?",
            e
        )
    })?;

    println!("Input device capabilities:");
    let input_caps = input_device.query_caps()?;
    println!("  Driver: {}", input_caps.driver);
    println!("  Card: {}", input_caps.card);
    println!("  Capabilities: 0x{:x}", input_caps.capabilities);

    // Check if input device supports video capture - FIXED LOGIC
    if (input_caps.capabilities & v4l::capability::Flags::VIDEO_CAPTURE).bits() == 0 {
        return Err("Input device does not support video capture".into());
    }

    // Get current format from input device
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
}

fn stream_with_grid_filter(
    _input_device: Device,
    _output_device: Device,
    running: Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting ffmpeg with grid filter...");

    // Build ffmpeg command - FIXED REDUNDANT ARGS
    let mut ffmpeg = FfmpegCommand::new()
        .input("/dev/video0")
        .args(["-f", "v4l2"]) // Input format
        .args(["-video_size", "640x480"]) // Set reasonable default size
        .args(["-framerate", "30"]) // Input framerate
        .filter("drawgrid=width=iw/3:height=ih/4:thickness=3:color=pink@1.0")
        .args(["-f", "v4l2"]) // Output format
        .args(["-pix_fmt", "yuv420p"]) // Pixel format
        .output("/dev/video2")
        .spawn()?;

    println!("FFmpeg process started");
    println!("Grid configuration: 3 columns × 4 rows in pink color");

    // Monitor ffmpeg process
    while running.load(Ordering::SeqCst) {
        // Check if ffmpeg process is still running
        match ffmpeg.as_inner_mut().try_wait()? {
            Some(status) => {
                println!("FFmpeg process has terminated with status: {:?}", status);
                break;
            }
            None => {
                // Process is still running
                thread::sleep(Duration::from_millis(100));
            }
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

// Enhanced version with event monitoring - FIXED EVENT HANDLING
#[allow(dead_code)]
fn stream_with_events(
    _input_device: Device,
    _output_device: Device,
    running: Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting ffmpeg with event monitoring...");

    let mut ffmpeg = FfmpegCommand::new()
        .input("/dev/video0")
        .args(["-f", "v4l2"])
        .args(["-video_size", "640x480"])
        .args(["-framerate", "30"])
        .filter("drawgrid=width=iw/3:height=ih/4:thickness=3:color=pink@1.0")
        .args(["-f", "v4l2"])
        .args(["-pix_fmt", "yuv420p"])
        .output("/dev/video2")
        .spawn()?;

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

    Ok(())
}

// Utility function to check device availability
#[allow(dead_code)]
fn check_device_exists(device_path: &str) -> bool {
    Path::new(device_path).exists()
}

// Function to gracefully restart stream if needed
#[allow(dead_code)]
fn restart_stream_if_needed(input_device: &Device, output_device: &Device) -> Result<bool> {
    // Check if devices are still accessible
    match input_device.query_caps() {
        Ok(_) => {
            match output_device.query_caps() {
                Ok(_) => Ok(false), // No restart needed
                Err(_) => {
                    println!("Output device lost, restart needed");
                    Ok(true)
                }
            }
        }
        Err(_) => {
            println!("Input device lost, restart needed");
            Ok(true)
        }
    }
}

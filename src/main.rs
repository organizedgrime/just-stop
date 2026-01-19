use anyhow::{Context as _, Result, anyhow};
use crossbeam_channel::{Sender, unbounded};
use ffmpeg_sidecar::{
    command::FfmpegCommand,
    event::{FfmpegEvent, LogLevel},
};
use inquire::Select;
use std::time::Duration;
use std::{
    fs::{self, File},
    io::Write,
};
use std::{io::BufRead as _, thread};
use std::{
    io::BufReader,
    sync::atomic::{AtomicBool, Ordering},
};
use std::{
    os::unix::fs::FileTypeExt,
    path::{Path, PathBuf},
};
use std::{process::Stdio, sync::Arc};
use std::{
    process::{Child, Command},
    thread::sleep,
};
use v4l::{Device, FourCC, video::Capture};
use v4l::{capability::Flags, v4l2};
mod conf;
mod pixel;

use crate::{
    conf::effects::{GridFilter, JustEffects},
    pixel::create_fourcc_to_ffmpeg_map_owned,
};
use conf::stream::*;
use conf::*;

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
            if let Ok(control) = device.query_controls() {
                println!("{control:?}");
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

#[derive(PartialEq)]
enum Message {
    Start,
    Stop,
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
            println!("Input: {} -> Output: {}", conf.input.info.path, conf.output);
            conf
        }
        Err(_) => {
            println!("No existing config found, creating new configuration...");
            println!("🔍 Scanning for video devices...");
            let devices = discover_devices()?;
            let input = pick_device(&devices, "input", Flags::VIDEO_CAPTURE)?;
            let output = JustStream {
                tcp: false,
                port: 8090,
            };
            let effects = JustEffects {
                grid: GridFilter {
                    color: "0xFF0000".to_string(),
                    rows: 9,
                    cols: 16,
                    opacity: 0.55,
                },
                advanced: false,
                onion_opacity: 0.55,
                hflip: false,
                vflip: false,
            };

            let config = Conf {
                input,
                output,
                effects,
            };

            // Save the newly created config
            config.save()?;
            println!("✓ Configuration saved");
            println!(
                "Input: {} -> Output: {}",
                config.input.info.path, config.output
            );

            config
        }
    };

    let file_manager = FileManager::new("photos", "./photos", "/tmp/just-stop")?;
    let mut command =
        file_manager.build_command(config.input.path(), &config.output, &config.effects);

    println!("Press Ctrl+C to stop the stream");

    // Set up graceful shutdown
    let running = Arc::new(AtomicBool::new(true));
    let running_handler = running.clone();
    ctrlc::set_handler(move || {
        println!("\nReceived Ctrl+C");
        running_handler.store(false, Ordering::SeqCst);
    })?;
    let (s, r) = unbounded::<Message>();

    // let output_path = file_manager.output_socket_file();
    let output_path = config.output.to_string();
    let tmpdir = file_manager.tmp.clone();

    thread::spawn(move || {
        let mut ffplay_pid: Option<u32> = None;
        loop {
            println!("loop repeats");
            // File::create(&Path::new(NOTIFICATION_FILE)).expect("clear notification");

            if r.is_empty() {
                // Check for trigger files
                file_manager.monitor_triggers().unwrap();
            } else {
                match r.recv() {
                    Ok(message) => {
                        if message == Message::Start {
                            println!("Received Start message");
                            // while !Path::new(&output_path).exists() {
                            //     println!("waiting");
                            //     sleep(Duration::from_millis(333));
                            // }

                            // FFplay from the output socket
                            // ffplay -f rawvideo -pixel_format yuv420p -video_size 1920x1080 -framerate 60 /tmp/output.pipe
                            let mut ffplay_cmd = Command::new("ffplay");
                            ffplay_cmd
                                // .args(["-f", "mpegts"])
                                // .args(["-framerate", "60"])
                                .args(["-fflags", "nobuffer"])
                                .args(["-flags", "low_delay"])
                                // .arg("-framedrop")
                                // .args(["-probesize", "32"])
                                // // .args(["-"])
                                // .args(["-analyzeduration", "0"])
                                .stdout(Stdio::piped())
                                .stderr(Stdio::piped())
                                // .args(["-f", "rawvideo"])
                                //
                                // .args(["-pixel_format", "yuv420p"])
                                // .args(["-video_size", "1920x1080"])
                                // .arg(file_manager.output_socket());
                                .arg(&output_path);

                            println!("ffplay cmd: {:?}", ffplay_cmd);

                            if ffplay_pid.is_some() {
                                println!("already healthy");
                            } else if let Ok(mut child) = ffplay_cmd.spawn() {
                                ffplay_pid = Some(child.id());
                                println!("spawned ffplay: ${ffplay_pid:?}");

                                // Take ownership of stdout and stderr
                                let stdout = child.stdout.take().unwrap();
                                let stderr = child.stderr.take().unwrap();

                                // Spawn thread for stdout logging
                                std::thread::spawn(move || {
                                    let reader = BufReader::new(stdout);
                                    for line in reader.lines() {
                                        if let Ok(line) = line {
                                            println!("ffplay info: {}", line);
                                        }
                                    }
                                });

                                // Spawn thread for stderr logging
                                std::thread::spawn(move || {
                                    let reader = BufReader::new(stderr);
                                    for line in reader.lines() {
                                        if let Ok(line) = line {
                                            eprintln!("ffplay error: {}", line);
                                        }
                                    }
                                });
                            } else {
                                println!("error occurred spawning ffplay");
                            }
                        } else {
                            println!("Received Stop message");
                            if let Some(pid) = ffplay_pid {
                                Command::new("kill")
                                    .args(["-9", &pid.to_string()])
                                    .output()
                                    .expect("unable to kill ffplay; it is running");
                                println!("Killed ffplay");
                            } else {
                                println!("cannot kill ffplay; it isn't running");
                                return;
                            }

                            // Cleanup
                            fs::remove_dir_all(&file_manager.tmp).ok();

                            println!("Stream ended gracefully");
                        }
                    }
                    Err(e) => {
                        println!("failed to receive message")
                    }
                }
            }

            // Start playing the output, always
            // if let Ok(ffplay) =
            // {};
            thread::sleep(Duration::from_millis(333));
        }
    });

    // let mut ffplay = Command::new("vlc")
    //     .arg(format!("v4l2://{}", &config.output.path()))
    //     .spawn()?;
    //
    // while !Path::new(&preview_path).exists() {
    //     println!("waiting for preview file to be generated");
    //     sleep(Duration::from_millis(333));
    // }
    // s.send(Message::Start)?;

    // thread::sleep(Duration::from_secs(5));
    // // Start the streaming loop
    let mut restart = true;
    while restart {
        restart = stream(&config, running.clone(), &s, &mut command)?;
    }

    // thread::sleep(Duration::from_secs(5));

    fs::remove_dir_all(tmpdir).ok();
    // mirror_process.kill()?;
    s.send(Message::Stop)?;
    // file_manager.cleanup()?;
    Command::new("pkill").arg("ffplay").output().ok();

    println!("the program is now over");
    Ok(())
}

fn stream(
    config: &Conf,
    running: Arc<AtomicBool>,
    s: &Sender<Message>,
    command: &mut FfmpegCommand,
) -> Result<bool, Box<dyn std::error::Error>> {
    println!("Starting ffmpeg with grid filter...");

    let map = create_fourcc_to_ffmpeg_map_owned();

    println!("GOT THE MAP");
    let Conf {
        input,
        output,
        effects,
    } = config;

    // Clone paths for later use
    let input_path = input.info.path.clone();
    let output_path = output.to_string();
    let framerate = input.settings.ffmpeg_r();

    let stringfmt = FourCC::new(&input.settings.format).to_string();
    let pixfmt = map.get(&stringfmt).unwrap().clone();

    // if get_latest_photo().is_none() {
    //     println!("initializing latest.png");
    //     init_snapshot_latest(&input_path)?;
    // } else {
    //     println!("already initialized snapshot.png");
    //     symlink_latest()?;
    //     println!("initialized symlink");
    // }

    let mut ffmpeg = command.spawn()?;

    println!("FFmpeg process started");

    s.send(Message::Start)?;
    println!("Grid configuration: 3 columns × 4 rows in pink color");

    // Monitor ffmpeg events
    for event in ffmpeg.iter()? {
        if !running.load(Ordering::SeqCst) {
            println!("Grid configuration: 3 columns × 4 rows in pink color");
            s.send(Message::Stop)?;
            ffmpeg.kill()?;
            return Ok(false);
        }

        match event {
            FfmpegEvent::Log(LogLevel::Info, msg) => {
                println!("ffmpeg info: {}", msg);
            }
            FfmpegEvent::Log(LogLevel::Warning, msg) => {
                println!("ffmpeg warning: {}", msg);
            }
            FfmpegEvent::Log(LogLevel::Error, msg) => {
                eprintln!("ffmpeg error: {}", msg);
            }
            FfmpegEvent::Progress(progress) => {
                if progress.frame % 30 == 0 {
                    // Log every second at 30fps
                    println!(
                        "ffmpeg Processed {} frames, time: {:.2}s",
                        progress.frame, progress.time
                    );
                }
            }
            FfmpegEvent::LogEOF => {
                println!("ffmpeg log ended");
                break;
            }
            _ => {}
        }
    }

    s.send(Message::Stop)?;
    Ok(false)
}

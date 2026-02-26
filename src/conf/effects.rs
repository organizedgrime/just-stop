use confique::Config;
use serde::Serialize;

#[derive(Serialize, Config)]
pub struct JustEffects {
    pub advanced: bool,

    #[config(nested)]
    pub grid: GridFilter,

    pub onion_opacity: f32,

    pub hflip: bool,
    pub vflip: bool,
}

impl JustEffects {
    fn photo_graph(&self) -> String {
        [
            // Flip camera
            &format!(
                "[0:v]{}[cam]",
                [
                    "scale=1920:1080",
                    if self.vflip { "vflip" } else { "null" },
                    if self.hflip { "hflip" } else { "null" }
                ]
                .join(",")
            ),
            // Latest
            "[1:v]scale=1920:1080[latest]",
            // Split cam into snapshot and stream
            "[cam]split=2[snapshot][stream]",
        ]
        .join(";")
    }

    pub fn filter_complex(&self, notification_file: &str) -> String {
        // Notification text for displaying messages
        let notification_filter = format!("drawtext=textfile={}:reload=1:fontcolor=white:fontsize=100:box=1:boxcolor=black:x=(w-text_w)/2:y=(h-text_h)/2", notification_file);
        // Grid overlay
        let grid_filter = self.grid.to_string();
        // Half sized
        let thumb_filter = "scale=width=iw/2:height=ih/2";

        if self.advanced {
            [
                self.photo_graph(),
                format!("[stream]split=2[stream1][stream2]"),
                format!("[latest]split=2[latest1][latest2]"),
                format!("[stream2]{}[stream_thumb]", thumb_filter),
                format!("[latest2]{}[latest_thumb]", thumb_filter),
                // Onion skin
                format!(
                    "[stream1][latest1]blend=all_mode=normal:all_opacity={}[mux]",
                    self.onion_opacity
                ),
                // Stack thumbnails on top of each other
                format!("[stream_thumb][latest_thumb]vstack=inputs=2[left_stack]"),
                // Add grid and notifications to main view
                format!("[mux]{},{}[main]", grid_filter, notification_filter),
                // Stack thumbnail and main view
                format!("[left_stack][main]hstack=inputs=2[output]"),
            ]
            .join(";")
        } else {
            [
                self.photo_graph(),
                // Onion skin
                format!(
                    "[stream][latest]blend=all_mode=normal:all_opacity={}[mux]",
                    self.onion_opacity
                ),
                // Add grid and notifications
                format!("[mux]{},{}[output]", grid_filter, notification_filter),
            ]
            .join(";")
        }
    }
}

#[derive(Serialize, Config)]
pub struct GridFilter {
    pub color: String,
    pub rows: usize,
    pub cols: usize,
    pub opacity: f32,
}

impl std::fmt::Display for GridFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "drawgrid=w=iw/{}:h=ih/{}:t=4:c={}@{}",
            self.cols, self.rows, self.color, self.opacity
        )
    }
}

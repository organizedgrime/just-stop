use std::collections::HashMap;
use v4l::FourCC;

// pub fn ffmpeg_format(format: &FourCC) -> vec![&str] {
//     let stringfmt = format.to_string();
//     let pixfmt = FOURCC_TO_PIXFMT[&stringfmt];
//     if is_compressed_format(&stringfmt) {
//         return ["-input_format", pixfmt];
//     } else {
//         return ["-pixel_format", pixfmt];
//     }
// }

pub fn is_compressed_format(fourcc: &str) -> bool {
    matches!(
        fourcc,
        "MJPG" | "JPEG" | "H264" | "H265" | "VP8 " | "VP9 " | "XVID" | "DIVX"
    )
}

/// Creates a hardcoded HashMap mapping FOURCC codes to FFmpeg pixel format names
///
/// FOURCC codes are 4-character identifiers used by V4L2 devices
/// FFmpeg pixel format names are the strings used with -pixel_format parameter
pub fn create_fourcc_to_ffmpeg_map() -> HashMap<&'static str, &'static str> {
    HashMap::from([
        // YUV Formats
        ("YUYV", "yuyv422"), // YUV 4:2:2 packed
        ("UYVY", "uyvy422"), // YUV 4:2:2 packed (different byte order)
        ("NV12", "nv12"),    // YUV 4:2:0 semi-planar
        ("NV21", "nv21"),    // YUV 4:2:0 semi-planar (swapped UV)
        ("YU12", "yuv420p"), // YUV 4:2:0 planar
        ("YV12", "yuv420p"), // YUV 4:2:0 planar (swapped UV)
        ("I420", "yuv420p"), // YUV 4:2:0 planar (alias for YU12)
        ("YUV2", "yuyv422"), // Alternative name for YUYV
        ("YUNV", "yuyv422"), // Alternative name for YUYV
        ("V422", "yuyv422"), // Alternative name for YUYV
        ("VYUY", "yvyu422"), // YUV 4:2:2 packed (different order)
        ("NV16", "nv16"),    // YUV 4:2:2 semi-planar
        ("NV61", "nv61"),    // YUV 4:2:2 semi-planar (swapped UV)
        ("422P", "yuv422p"), // YUV 4:2:2 planar
        ("411P", "yuv411p"), // YUV 4:1:1 planar
        ("Y41P", "yuv411p"), // YUV 4:1:1 planar
        ("444P", "yuv444p"), // YUV 4:4:4 planar
        // RGB Formats
        ("RGB3", "rgb24"),    // RGB 24-bit
        ("BGR3", "bgr24"),    // BGR 24-bit
        ("RGB4", "rgb32"),    // RGB 32-bit
        ("BGR4", "bgr32"),    // BGR 32-bit
        ("RGBA", "rgba"),     // RGBA 32-bit
        ("RGBO", "rgba"),     // RGBA 32-bit (alternative)
        ("BGRA", "bgra"),     // BGRA 32-bit
        ("BGRX", "bgr0"),     // BGRX 32-bit
        ("RGBP", "rgb565le"), // RGB 16-bit 565
        ("RGBR", "rgb565be"), // RGB 16-bit 565 big endian
        ("RGBQ", "rgb555le"), // RGB 15-bit 555
        ("RGBO", "rgb555be"), // RGB 15-bit 555 big endian
        ("RGB1", "rgb8"),     // RGB 8-bit
        ("RGB0", "rgb4"),     // RGB 4-bit
        // Grayscale Formats
        ("GREY", "gray"),     // 8-bit grayscale
        ("GRAY", "gray"),     // Alternative spelling
        ("Y800", "gray"),     // Alternative name
        ("Y8  ", "gray"),     // 8-bit Y only
        ("Y16 ", "gray16le"), // 16-bit grayscale little endian
        ("Y16B", "gray16be"), // 16-bit grayscale big endian
        ("Y10 ", "gray10le"), // 10-bit grayscale
        ("Y12 ", "gray12le"), // 12-bit grayscale
        // Bayer Formats (Raw sensor data)
        ("RGGB", "bayer_rggb8"),    // 8-bit Bayer RGGB
        ("GRBG", "bayer_grbg8"),    // 8-bit Bayer GRBG
        ("GBRG", "bayer_gbrg8"),    // 8-bit Bayer GBRG
        ("BGGR", "bayer_bggr8"),    // 8-bit Bayer BGGR
        ("BYR2", "bayer_rggb16le"), // 16-bit Bayer
        ("RG10", "bayer_rggb10"),   // 10-bit Bayer RGGB
        ("BA81", "bayer_bggr8"),    // Alternative Bayer format
        // Compressed Formats (use with -input_format instead of -pixel_format)
        ("MJPG", "mjpeg"), // Motion JPEG
        ("JPEG", "mjpeg"), // JPEG (alternative)
        ("H264", "h264"),  // H.264
        ("H265", "hevc"),  // H.265/HEVC
        ("VP8 ", "vp8"),   // VP8
        ("VP9 ", "vp9"),   // VP9
        ("XVID", "mpeg4"), // XVID (MPEG-4)
        ("DIVX", "mpeg4"), // DivX (MPEG-4)
        // Planar formats
        ("YM24", "yuv444p"), // YUV 4:4:4 planar
        ("YM42", "yuv422p"), // YUV 4:2:2 planar
        ("YM12", "yuv420p"), // YUV 4:2:0 planar
        // 10-bit and 16-bit formats
        ("P010", "p010le"), // 10-bit YUV 4:2:0 semi-planar
        ("P016", "p016le"), // 16-bit YUV 4:2:0 semi-planar
        ("Y210", "y210le"), // 10-bit YUV 4:2:2 packed
        ("Y216", "y216le"), // 16-bit YUV 4:2:2 packed
    ])
}
/// Alternative function that returns owned strings instead of static references
pub fn create_fourcc_to_ffmpeg_map_owned() -> HashMap<String, String> {
    let static_map = create_fourcc_to_ffmpeg_map();
    static_map
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

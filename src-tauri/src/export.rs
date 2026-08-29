use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba, RgbaImage, imageops};
use imageproc::drawing::draw_text_mut;
use ab_glyph::{FontVec, PxScale};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::fs;
use tauri::{AppHandle, Emitter};

fn load_font() -> Result<FontVec, String> {
    let font_paths: &[&str] = if cfg!(target_os = "macos") {
        &[
            "/System/Library/Fonts/PingFang.ttc",
            "/System/Library/Fonts/STHeiti Light.ttc",
            "/System/Library/Fonts/STHeiti Medium.ttc",
            "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
            "/System/Library/Fonts/Helvetica.ttc",
            "/Library/Fonts/Arial Unicode.ttf",
        ]
    } else {
        &[
            "C:\\Windows\\Fonts\\msyh.ttc",
            "C:\\Windows\\Fonts\\msyhbd.ttc",
            "C:\\Windows\\Fonts\\simhei.ttf",
            "C:\\Windows\\Fonts\\simsun.ttc",
            "C:\\Windows\\Fonts\\arial.ttf",
        ]
    };
    for path in font_paths {
        if let Ok(data) = fs::read(path) {
            if let Ok(font) = FontVec::try_from_vec(data) {
                return Ok(font);
            }
        }
    }
    Err("No system font found".to_string())
}

fn fill_gradient(img: &mut RgbaImage, c1: [u8; 4], c2: [u8; 4]) {
    let (w, h) = img.dimensions();
    for y in 0..h {
        for x in 0..w {
            let t = (x as f32 / w as f32 + y as f32 / h as f32) / 2.0;
            let r = (c1[0] as f32 * (1.0 - t) + c2[0] as f32 * t) as u8;
            let g = (c1[1] as f32 * (1.0 - t) + c2[1] as f32 * t) as u8;
            let b = (c1[2] as f32 * (1.0 - t) + c2[2] as f32 * t) as u8;
            img.put_pixel(x, y, Rgba([r, g, b, 255]));
        }
    }
}

fn fill_rect(img: &mut RgbaImage, x: u32, y: u32, w: u32, h: u32, color: Rgba<u8>) {
    for py in y..(y + h).min(img.height()) {
        for px in x..(x + w).min(img.width()) {
            img.put_pixel(px, py, color);
        }
    }
}

fn resize_cover(img: &DynamicImage, target_w: u32, target_h: u32) -> RgbaImage {
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return ImageBuffer::new(target_w, target_h);
    }
    let target_ratio = target_w as f32 / target_h as f32;
    let src_ratio = w as f32 / h as f32;

    let (crop_w, crop_h) = if src_ratio > target_ratio {
        let ch = h;
        let cw = ((h as f32 * target_ratio) as u32).max(1);
        (cw, ch)
    } else {
        let cw = w;
        let ch = ((w as f32 / target_ratio) as u32).max(1);
        (cw, ch)
    };

    let crop_x = (w.saturating_sub(crop_w)) / 2;
    let crop_y = (h.saturating_sub(crop_h)) / 2;

    let img_rgba = img.to_rgba8();
    let cropped = imageops::crop_imm(&img_rgba, crop_x, crop_y, crop_w, crop_h).to_image();
    imageops::resize(&cropped, target_w, target_h, imageops::FilterType::Lanczos3)
}

fn resize_contain(img: &DynamicImage, target_w: u32, target_h: u32) -> (RgbaImage, u32, u32) {
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return (ImageBuffer::new(target_w, target_h), 0, 0);
    }
    let scale = (target_w as f32 / w as f32).min(target_h as f32 / h as f32);
    let new_w = (w as f32 * scale) as u32;
    let new_h = (h as f32 * scale) as u32;
    let resized = imageops::resize(&img.to_rgba8(), new_w, new_h, imageops::FilterType::Lanczos3);
    let off_x = (target_w - new_w) / 2;
    let off_y = (target_h - new_h) / 2;
    (resized, off_x, off_y)
}

pub fn open_file_location(path: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        // Command::arg 不做 shell 解析，路径含 &、#、空格等均安全；
        // explorer 的 /select, 参数在 Windows 中支持含空格路径。
        Command::new("explorer.exe")
            .arg(format!("/select,{}", path))
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg("-R")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn generate_collage(
    app: &AppHandle,
    album_name: &str,
    date_range: &str,
    location: &str,
    photo_paths: &[String],
    output_path: &str,
) -> Result<String, String> {
    let font = load_font()?;

    let count = photo_paths.len();
    if count == 0 {
        return Err("No photos to export".to_string());
    }

    let _ = app.emit("export-progress", (0i32, count as i32));

    let cols = std::cmp::min(4, (count as f64).sqrt().ceil() as u32);
    let rows = ((count as f64) / (cols as f64)).ceil() as u32;
    let cell = 300u32;
    let gap = 12u32;
    let pad = 30u32;
    let header_h = 120u32;
    let footer_h = 50u32;

    let canvas_w = cols * cell + (cols - 1) * gap + pad * 2;
    let canvas_h = header_h + rows * cell + (rows - 1) * gap + footer_h + pad;

    let mut canvas: RgbaImage = ImageBuffer::new(canvas_w, canvas_h);
    fill_gradient(&mut canvas, [15, 23, 42, 255], [30, 27, 75, 255]);

    // Title (centered)
    let (tw, _th) = imageproc::drawing::text_size(PxScale::from(40.0), &font, album_name);
    let title_x = ((canvas_w as i64 - tw as i64) / 2).max(0) as u32;
    draw_text_mut(&mut canvas, Rgba([241, 245, 249, 255]), title_x as i32, 25, PxScale::from(40.0), &font, album_name);

    // Subtitle
    let mut parts: Vec<&str> = Vec::new();
    if !date_range.is_empty() {
        parts.push(date_range);
    }
    parts.push("");
    let count_str = format!("{} 张照片", count);
    let mut subtitle_parts: Vec<String> = Vec::new();
    if !date_range.is_empty() {
        subtitle_parts.push(date_range.to_string());
    }
    subtitle_parts.push(count_str);
    if !location.is_empty() {
        subtitle_parts.push(format!("📍 {}", location));
    }
    let subtitle = subtitle_parts.join("  ·  ");
    let (sw, _sh) = imageproc::drawing::text_size(PxScale::from(20.0), &font, &subtitle);
    let sub_x = ((canvas_w as i64 - sw as i64) / 2).max(0) as u32;
    draw_text_mut(&mut canvas, Rgba([148, 163, 184, 255]), sub_x as i32, 80, PxScale::from(20.0), &font, &subtitle);

    // Photo grid
    for (i, path) in photo_paths.iter().enumerate() {
        let _ = app.emit("export-progress", ((i + 1) as i32, count as i32));

        let row = (i as u32) / cols;
        let col = (i as u32) % cols;
        let x = pad + col * (cell + gap);
        let y = header_h + row * (cell + gap);

        // 应用 EXIF 方向，保证竖拍照片在影集里方向正确
        match crate::image_utils::open_with_orientation(path) {
            Ok(img) => {
                let resized = resize_cover(&img, cell, cell);
                imageops::overlay(&mut canvas, &resized, x as i64, y as i64);
                // border
                for px in x..(x + cell) {
                    if px < canvas_w {
                        canvas.put_pixel(px, y, Rgba([255, 255, 255, 26]));
                        canvas.put_pixel(px, y + cell - 1, Rgba([255, 255, 255, 26]));
                    }
                }
                for py in y..(y + cell) {
                    if py < canvas_h {
                        canvas.put_pixel(x, py, Rgba([255, 255, 255, 26]));
                        canvas.put_pixel(x + cell - 1, py, Rgba([255, 255, 255, 26]));
                    }
                }
            }
            Err(_) => {
                fill_rect(&mut canvas, x, y, cell, cell, Rgba([51, 65, 85, 255]));
            }
        }
    }

    // Footer
    let footer_text = "PhotoMap · 智能影集";
    let (fw, _fh) = imageproc::drawing::text_size(PxScale::from(16.0), &font, footer_text);
    let fx = ((canvas_w as i64 - fw as i64) / 2).max(0) as u32;
    draw_text_mut(&mut canvas, Rgba([100, 116, 139, 255]), fx as i32, (canvas_h - 25) as i32, PxScale::from(16.0), &font, footer_text);

    canvas.save(output_path).map_err(|e| e.to_string())?;

    open_file_location(output_path)?;
    Ok(output_path.to_string())
}

pub fn generate_video(
    app: &AppHandle,
    album_name: &str,
    date_range: &str,
    photo_paths: &[String],
    output_path: &str,
) -> Result<String, String> {
    // 每次运行使用独立临时目录，结束后无论成功失败都清理，避免并发冲突与残留
    let run_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let temp_dir = std::env::temp_dir().join(format!("photomap_export_{}", run_id));

    let result = generate_video_inner(app, album_name, date_range, photo_paths, output_path, &temp_dir);
    let _ = fs::remove_dir_all(&temp_dir);
    result
}

fn generate_video_inner(
    app: &AppHandle,
    album_name: &str,
    date_range: &str,
    photo_paths: &[String],
    output_path: &str,
    temp_dir: &Path,
) -> Result<String, String> {
    let font = load_font()?;

    let count = photo_paths.len();
    if count == 0 {
        return Err("No photos to export".to_string());
    }

    let _ = app.emit("export-progress", (0i32, count as i32));

    let vw = 1280u32;
    let vh = 720u32;

    fs::create_dir_all(&temp_dir).ok();

    let mut concat_list = String::new();
    let mut frame_files: Vec<PathBuf> = Vec::new();

    // Title frame
    let title_path = temp_dir.join("title.png");
    let mut title_img: RgbaImage = ImageBuffer::new(vw, vh);
    fill_gradient(&mut title_img, [15, 23, 42, 255], [30, 27, 75, 255]);
    let (tw, _th) = imageproc::drawing::text_size(PxScale::from(56.0), &font, album_name);
    let tx = ((vw as i64 - tw as i64) / 2).max(0) as u32;
    draw_text_mut(&mut title_img, Rgba([241, 245, 249, 255]), tx as i32, (vh / 2 - 40) as i32, PxScale::from(56.0), &font, album_name);
    let mut sub_parts: Vec<String> = Vec::new();
    if !date_range.is_empty() {
        sub_parts.push(date_range.to_string());
    }
    sub_parts.push(format!("{} 张照片", count));
    let subtitle = sub_parts.join("  ·  ");
    let (sw, _sh) = imageproc::drawing::text_size(PxScale::from(24.0), &font, &subtitle);
    let sx = ((vw as i64 - sw as i64) / 2).max(0) as u32;
    draw_text_mut(&mut title_img, Rgba([148, 163, 184, 255]), sx as i32, (vh / 2 + 30) as i32, PxScale::from(24.0), &font, &subtitle);
    let footer = "PhotoMap · 智能影集";
    let (fw, _fh) = imageproc::drawing::text_size(PxScale::from(16.0), &font, footer);
    let fx = ((vw as i64 - fw as i64) / 2).max(0) as u32;
    draw_text_mut(&mut title_img, Rgba([71, 85, 105, 255]), fx as i32, (vh - 40) as i32, PxScale::from(16.0), &font, footer);
    title_img.save(&title_path).map_err(|e| e.to_string())?;
    concat_list.push_str(&format!("file '{}'\nduration 2.0\n", title_path.display()));
    frame_files.push(title_path);

    // Photo frames
    for (i, path) in photo_paths.iter().enumerate() {
        let _ = app.emit("export-progress", ((i + 1) as i32, count as i32));

        let frame_path = temp_dir.join(format!("frame_{}.jpg", i));
        // 应用 EXIF 方向，保证竖拍照片在视频里方向正确
        match crate::image_utils::open_with_orientation(path) {
            Ok(img) => {
                let mut frame: RgbaImage = ImageBuffer::new(vw, vh);
                fill_gradient(&mut frame, [10, 14, 26, 255], [20, 20, 40, 255]);

                // top bar
                fill_rect(&mut frame, 0, 0, vw, 50, Rgba([15, 23, 42, 200]));
                draw_text_mut(&mut frame, Rgba([241, 245, 249, 255]), 30, 15, PxScale::from(20.0), &font, album_name);
                let counter = format!("{} / {}", i + 1, count);
                let (cw, _ch) = imageproc::drawing::text_size(PxScale::from(18.0), &font, &counter);
                draw_text_mut(&mut frame, Rgba([100, 116, 139, 255]), (vw - cw - 30) as i32, 15, PxScale::from(18.0), &font, &counter);

                // photo (contain fit, centered)
                let (resized, off_x, off_y) = resize_contain(&img, vw - 80, vh - 160);
                imageops::overlay(&mut frame, &resized, (40 + off_x) as i64, (60 + off_y) as i64);

                // bottom bar
                fill_rect(&mut frame, 0, vh - 50, vw, 50, Rgba([15, 23, 42, 200]));
                let filename = Path::new(path).file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                let (fnw, _fnh) = imageproc::drawing::text_size(PxScale::from(16.0), &font, filename);
                draw_text_mut(&mut frame, Rgba([148, 163, 184, 255]), ((vw - fnw) / 2) as i32, (vh - 30) as i32, PxScale::from(16.0), &font, filename);

                let jpg = DynamicImage::ImageRgba8(frame).to_rgb8();
                jpg.save(&frame_path).map_err(|e| e.to_string())?;
                concat_list.push_str(&format!("file '{}'\nduration 2.5\n", frame_path.display()));
                frame_files.push(frame_path);
            }
            Err(_) => continue,
        }
    }

    // End frame
    let end_path = temp_dir.join("end.png");
    let mut end_img: RgbaImage = ImageBuffer::new(vw, vh);
    fill_gradient(&mut end_img, [15, 23, 42, 255], [30, 27, 75, 255]);
    let end_text = "影集结束";
    let (ew, _eh) = imageproc::drawing::text_size(PxScale::from(48.0), &font, end_text);
    let ex = ((vw as i64 - ew as i64) / 2).max(0) as u32;
    draw_text_mut(&mut end_img, Rgba([241, 245, 249, 255]), ex as i32, (vh / 2 - 30) as i32, PxScale::from(48.0), &font, end_text);
    let end_sub = format!("{} · 共 {} 张照片", album_name, count);
    let (esw, _esh) = imageproc::drawing::text_size(PxScale::from(20.0), &font, &end_sub);
    let esx = ((vw as i64 - esw as i64) / 2).max(0) as u32;
    draw_text_mut(&mut end_img, Rgba([148, 163, 184, 255]), esx as i32, (vh / 2 + 20) as i32, PxScale::from(20.0), &font, &end_sub);
    let footer2 = "PhotoMap · 智能影集";
    let (f2w, _f2h) = imageproc::drawing::text_size(PxScale::from(16.0), &font, footer2);
    let f2x = ((vw as i64 - f2w as i64) / 2).max(0) as u32;
    draw_text_mut(&mut end_img, Rgba([71, 85, 105, 255]), f2x as i32, (vh - 40) as i32, PxScale::from(16.0), &font, footer2);
    end_img.save(&end_path).map_err(|e| e.to_string())?;
    concat_list.push_str(&format!("file '{}'\nduration 2.0\n", end_path.display()));
    concat_list.push_str(&format!("file '{}'\n", end_path.display()));
    frame_files.push(end_path);

    // Write concat list
    let list_path = temp_dir.join("concat_list.txt");
    fs::write(&list_path, &concat_list).map_err(|e| e.to_string())?;

    let _ = app.emit("export-progress", (-1i32, count as i32));

    // Try ffmpeg
    let has_ffmpeg = Command::new("ffmpeg").arg("-version").output().is_ok();

    let final_path = if has_ffmpeg {
        let result = Command::new("ffmpeg")
            .args(["-y", "-f", "concat", "-safe", "0", "-i"])
            .arg(&list_path)
            .args(["-vf", "fps=30,format=yuv420p", "-c:v", "libx264", "-preset", "medium", "-crf", "23"])
            .arg(output_path)
            .output();

        match result {
            Ok(output) if output.status.success() => output_path.to_string(),
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("ffmpeg 编码失败: {}", stderr));
            }
            Err(e) => {
                return Err(format!("ffmpeg 启动失败: {}. 请确保已安装 ffmpeg 并添加到系统 PATH。", e));
            }
        }
    } else {
        return Err("未检测到 ffmpeg，无法生成视频。请安装 ffmpeg 并添加到系统 PATH 后重试。".to_string());
    };

    open_file_location(&final_path)?;
    Ok(final_path)
}

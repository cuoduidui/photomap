use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

/// 检查 ffmpeg 是否可用，避免在用户机器缺少 ffmpeg 时输出难以理解的错误
fn ensure_ffmpeg() -> Result<(), String> {
    let status = Command::new("ffmpeg")
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    match status {
        Ok(s) if s.success() => Ok(()),
        _ => Err("未找到 ffmpeg，请先安装 ffmpeg 并加入系统 PATH（例如 winget install ffmpeg）后重试。".to_string()),
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SlideshowConfig {
    pub trip_id: String,
    pub title: String,
    pub narration: String,
    pub photo_paths: Vec<String>,
    pub output_path: String,
    pub photo_duration: f32, // 每张照片停留秒数
    pub transition_duration: f32, // 转场秒数
    pub width: u32,
    pub height: u32,
    pub music_path: Option<String>,
}

pub fn generate_slideshow(
    config: &SlideshowConfig,
    app: AppHandle,
    progress: Arc<AtomicUsize>,
) -> Result<(), String> {
    ensure_ffmpeg()?;

    let photo_count = config.photo_paths.len();
    if photo_count == 0 {
        return Err("没有照片".to_string());
    }

    // 验证照片文件存在
    let valid_photos: Vec<&String> = config.photo_paths
        .iter()
        .filter(|p| Path::new(p).exists())
        .collect();

    if valid_photos.is_empty() {
        return Err("所有照片文件不存在".to_string());
    }

    let total = valid_photos.len();
    progress.store(5, Ordering::SeqCst);
    let _ = app.emit("slideshow-progress", (5u32, total as u32));

    // 创建临时目录用于存放处理后的图片
    let temp_dir = std::env::temp_dir().join(format!("slideshow_{}", config.trip_id));
    std::fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;

    // Step 1: 将所有照片缩放到目标尺寸并添加 Ken Burns 效果（使用 zoompan 滤镜）
    // 简化方案：先把所有图片统一尺寸，然后用 ffmpeg concat + xfade 转场
    let mut image_files = Vec::new();
    for (i, photo_path) in valid_photos.iter().enumerate() {
        let output_img = temp_dir.join(format!("img_{:04}.jpg", i));
        
        // 先把图片缩放到目标尺寸，保持比例并填充
        let status = Command::new("ffmpeg")
            .arg("-y")
            .arg("-i").arg(photo_path)
            .arg("-vf").arg(format!(
                "scale={w}:{h}:force_original_aspect_ratio=decrease,\
                 pad={w}:{h}:(ow-iw)/2:(oh-ih)/2:color=black,\
                 format=yuvj420p",
                w = config.width,
                h = config.height
            ))
            .arg("-q:v").arg("2")
            .arg(&output_img)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|e| format!("ffmpeg 处理图片失败: {}", e))?;

        if !status.success() {
            // 如果失败，跳过这张
            continue;
        }
        image_files.push(output_img);

        let pct = 5 + ((i + 1) * 25 / total);
        progress.store(pct, Ordering::SeqCst);
        let _ = app.emit("slideshow-progress", (pct as u32, total as u32));
    }

    if image_files.is_empty() {
        return Err("没有可用的照片".to_string());
    }

    progress.store(35, Ordering::SeqCst);
    let _ = app.emit("slideshow-progress", (35u32, total as u32));

    // Step 2: 生成 concat 文件列表
    let list_file = temp_dir.join("files.txt");
    let mut list_content = String::new();
    let per_image_duration = config.photo_duration + config.transition_duration;
    
    for img in &image_files {
        list_content.push_str(&format!(
            "file '{}'\nduration {}\n",
            img.to_string_lossy().replace("'", "'\\''"),
            per_image_duration
        ));
    }
    // 最后一张重复一次（concat 要求最后一个 duration 可能被忽略）
    if let Some(last) = image_files.last() {
        list_content.push_str(&format!(
            "file '{}'\nduration {}\n",
            last.to_string_lossy().replace("'", "'\\''"),
            config.photo_duration
        ));
    }
    
    std::fs::write(&list_file, list_content).map_err(|e| e.to_string())?;

    progress.store(45, Ordering::SeqCst);
    let _ = app.emit("slideshow-progress", (45u32, total as u32));

    // Step 3: 使用 concat demuxer 生成基础视频 + 淡入淡出效果
    // 简化版：直接用 concat，然后添加整体淡入淡出
    let video_path = temp_dir.join("video_no_audio.mp4");
    
    // 使用 concat + zoompan 方案更简单：直接用 concat demuxer 然后加全局滤镜
    let img_count = image_files.len() as f32;
    let zoompan_frames = ((per_image_duration * 25.0) * img_count + 50.0) as i32;
    let fade_out_start = img_count * per_image_duration + 1.0;
    
    let status = Command::new("ffmpeg")
        .arg("-y")
        .arg("-f").arg("concat")
        .arg("-safe").arg("0")
        .arg("-i").arg(&list_file)
        .arg("-vf").arg(format!(
            "zoompan=z='min(zoom+0.001,1.1)':d={}:s={}x{}:fps=25,\
             fade=t=in:st=0:d=1.0,fade=t=out:st={}:d=1.0",
            zoompan_frames,
            config.width,
            config.height,
            fade_out_start
        ))
        .arg("-c:v").arg("libx264")
        .arg("-preset").arg("medium")
        .arg("-crf").arg("23")
        .arg("-pix_fmt").arg("yuv420p")
        .arg("-movflags").arg("+faststart")
        .arg(&video_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| format!("视频合成失败: {}", e))?;

    if !status.success() {
        return Err("视频合成失败".to_string());
    }

    progress.store(80, Ordering::SeqCst);
    let _ = app.emit("slideshow-progress", (80u32, total as u32));

    // Step 4: 添加背景音乐（如果有）
    let final_output = PathBuf::from(&config.output_path);
    if let Some(parent) = final_output.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建输出目录失败: {}", e))?;
    }
    
    if let Some(music_path) = &config.music_path {
        if Path::new(music_path).exists() {
            let status = Command::new("ffmpeg")
                .arg("-y")
                .arg("-i").arg(&video_path)
                .arg("-i").arg(music_path)
                .arg("-c:v").arg("copy")
                .arg("-c:a").arg("aac")
                .arg("-b:a").arg("192k")
                .arg("-shortest")
                .arg("-map").arg("0:v:0")
                .arg("-map").arg("1:a:0")
                .arg("-af").arg(format!(
                    "afade=t=in:st=0:d=1.0,afade=t=out:st={}:d=2.0",
                    img_count * per_image_duration
                ))
                .arg(&final_output)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map_err(|e| format!("添加音乐失败: {}", e))?;

            if !status.success() {
                // 如果加音乐失败，就用无音频版本
                std::fs::copy(&video_path, &final_output).map_err(|e| format!("复制视频文件失败: {}", e))?;
            }
        } else {
            std::fs::copy(&video_path, &final_output).map_err(|e| format!("复制视频文件失败: {}", e))?;
        }
    } else {
        std::fs::copy(&video_path, &final_output).map_err(|e| format!("复制视频文件失败: {}", e))?;
    }

    progress.store(100, Ordering::SeqCst);
    let _ = app.emit("slideshow-progress", (100u32, total as u32));

    // 清理临时文件
    let _ = std::fs::remove_dir_all(&temp_dir);

    Ok(())
}

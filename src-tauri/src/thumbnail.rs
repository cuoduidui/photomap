use image::{DynamicImage, GenericImageView};
use std::path::{Path, PathBuf};

// 缩略图尺寸 - 减小尺寸以降低内存和磁盘占用
const THUMB_WIDTH: u32 = 150;
const THUMB_HEIGHT: u32 = 150;
// 缩略图质量 - 使用 Triangle 算法平衡速度和质量
const FILTER_TYPE: image::imageops::FilterType = image::imageops::FilterType::Triangle;

pub fn generate_thumbnail(
    src_path: &str,
    thumbs_dir: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    std::fs::create_dir_all(thumbs_dir)?;

    // 先检查缩略图是否已存在（根据文件路径生成 hash）
    use md5::{Md5, Digest};
    let mut hasher = Md5::new();
    hasher.update(src_path.as_bytes());
    let file_hash = format!("{:x}", hasher.finalize());
    let thumb_name = format!("{}.jpg", file_hash);
    let thumb_path = PathBuf::from(thumbs_dir).join(&thumb_name);
    
    if thumb_path.exists() {
        return Ok(thumb_path.to_string_lossy().to_string());
    }

    // 应用 EXIF 方向，保证竖拍照片缩略图方向正确
    let img = crate::image_utils::open_with_orientation(src_path)?;
    
    // 快速判断是否需要缩放
    let (w, h) = img.dimensions();
    let thumbnail = if w <= THUMB_WIDTH && h <= THUMB_HEIGHT {
        img
    } else {
        img.resize(THUMB_WIDTH, THUMB_HEIGHT, FILTER_TYPE)
    };

    // 保存为 JPEG 格式，质量 75，减小文件体积
    let output = std::fs::File::create(&thumb_path)?;
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(std::io::BufWriter::new(output), 75);
    encoder.encode_image(&thumbnail)?;

    Ok(thumb_path.to_string_lossy().to_string())
}

pub fn generate_thumbnail_from_image(
    img: &DynamicImage,
    thumbs_dir: &str,
    name: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    std::fs::create_dir_all(thumbs_dir)?;
    let thumbnail = img.resize(THUMB_WIDTH, THUMB_HEIGHT, FILTER_TYPE);
    let thumb_name = format!("{}_thumb.jpg", name);
    let thumb_path = PathBuf::from(thumbs_dir).join(&thumb_name);
    
    let output = std::fs::File::create(&thumb_path)?;
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(std::io::BufWriter::new(output), 75);
    encoder.encode_image(&thumbnail)?;
    
    Ok(thumb_path.to_string_lossy().to_string())
}

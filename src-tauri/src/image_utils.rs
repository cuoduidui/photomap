//! 图片加载辅助：统一处理 EXIF Orientation，保证竖拍照片显示方向正确。

use image::{DynamicImage, ImageDecoder, ImageReader};

/// 打开图片并应用 EXIF 方向（手机/相机竖拍照片会自动旋转到正确方向）。
pub fn open_with_orientation(path: &str) -> Result<DynamicImage, image::ImageError> {
    let reader = ImageReader::open(path)?.with_guessed_format()?;
    let mut img = reader.decode()?;
    apply_orientation(path, &mut img);
    Ok(img)
}

/// 读取图片的 EXIF Orientation 并原地应用。
pub fn apply_orientation(path: &str, img: &mut DynamicImage) {
    if let Ok(reader) = ImageReader::open(path) {
        if let Ok(reader) = reader.with_guessed_format() {
                if let Ok(mut decoder) = reader.into_decoder() {
                if let Ok(orientation) = decoder.orientation() {
                    img.apply_orientation(orientation);
                }
            }
        }
    }
}

//! # 人脸检测（启发式）
//!
//! 本模块采用「肤色区域(YCbCr) + 连通分量 + 外观比例」的启发式来定位疑似人脸区域，
//! **并非真实的人脸身份识别**。它不涉及任何人脸检测模型（如 retinaface / blazeface），
//! 也不使用身份向量模型（如 ArcFace），因此：
//! - `detect_faces` 可能漏检（侧脸、遮挡、暗光）或误检（肤色背景块）。
//! - `cluster_faces` 仅按颜色直方图特征做余弦相似度聚类，**聚的是「外观相近的区域」，
//!   不是「同一个人」**；同一人不同光照/姿态可能被拆成多类，不同人相似配色可能被并为一类。
//!
//! 如需可用的人脸识别，应接入真实模型（前端 ONNX 检测 + ArcFace 特征，或本地 Ollama
//! 多模态），本模块仅作为低成本的兜底方案。

use image::{DynamicImage, GenericImageView, ImageBuffer, Rgb};
use image::imageops::FilterType;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct FaceRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone)]
pub struct FaceDetection {
    pub rect: FaceRect,
    pub features: Vec<f32>,
    pub photo_id: String,
}

pub fn detect_faces(img: &DynamicImage) -> Vec<FaceRect> {
    let (width, height) = img.dimensions();
    if width < 60 || height < 60 {
        return Vec::new();
    }

    let rgb = img.to_rgb8();

    let mut skin_mask = vec![false; (width * height) as usize];
    let mut skin_count = 0usize;
    for y in 0..height {
        for x in 0..width {
            let pixel = rgb.get_pixel(x, y);
            let r = pixel[0] as f32;
            let g = pixel[1] as f32;
            let b = pixel[2] as f32;

            let y_val = 0.299 * r + 0.587 * g + 0.114 * b;
            let cb = -0.169 * r - 0.331 * g + 0.500 * b + 128.0;
            let cr = 0.500 * r - 0.419 * g - 0.081 * b + 128.0;

            if y_val > 80.0 && cb > 85.0 && cb < 130.0 && cr > 135.0 && cr < 175.0 {
                skin_mask[(y * width + x) as usize] = true;
                skin_count += 1;
            }
        }
    }

    let components = find_connected_components(&skin_mask, width as usize, height as usize);

    let mut faces = Vec::new();
    let min_dim = std::cmp::min(width, height);
    let min_face = (min_dim as f32 * 0.04) as u32;
    let max_face_w = width / 2;
    let max_face_h = height / 2;

    for component in &components {
        if component.len() < (min_face * min_face / 4) as usize {
            continue;
        }

        let mut min_x = u32::MAX;
        let mut min_y = u32::MAX;
        let mut max_x = 0u32;
        let mut max_y = 0u32;

        for &(x, y) in component {
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }

        let w = max_x - min_x + 1;
        let h = max_y - min_y + 1;

        let aspect = w as f32 / h as f32;
        if aspect < 0.75 || aspect > 1.45 {
            continue;
        }

        if w < min_face || h < min_face {
            continue;
        }
        if w > max_face_w || h > max_face_h {
            continue;
        }

        let fill_ratio = component.len() as f32 / (w * h) as f32;
        if fill_ratio < 0.5 {
            continue;
        }

        faces.push(FaceRect {
            x: min_x,
            y: min_y,
            width: w,
            height: h,
        });
    }

    faces.sort_by(|a, b| (b.width * b.height).cmp(&(a.width * a.height)));
    faces.truncate(2);

    faces
}

fn find_connected_components(mask: &[bool], width: usize, height: usize) -> Vec<Vec<(u32, u32)>> {
    let mut visited = vec![false; width * height];
    let mut components = Vec::new();

    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            if mask[idx] && !visited[idx] {
                let mut component = Vec::new();
                let mut stack = vec![(x, y)];
                visited[idx] = true;

                while let Some((cx, cy)) = stack.pop() {
                    component.push((cx as u32, cy as u32));

                    for dy in 0..3i32 {
                        for dx in 0..3i32 {
                            if dx == 1 && dy == 1 {
                                continue;
                            }
                            let nx = cx as i32 + dx - 1;
                            let ny = cy as i32 + dy - 1;
                            if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
                                let nidx = ny as usize * width + nx as usize;
                                if mask[nidx] && !visited[nidx] {
                                    visited[nidx] = true;
                                    stack.push((nx as usize, ny as usize));
                                }
                            }
                        }
                    }
                }

                components.push(component);
            }
        }
    }

    components
}

/// 提取人脸特征：4x4空间网格 x 6维HSV直方图 + 亮度梯度特征
pub fn extract_face_features(img: &DynamicImage, face: &FaceRect) -> Vec<f32> {
    let cropped = crop_face(img, face);
    let resized = image::imageops::resize(&cropped, 64, 64, FilterType::Lanczos3);

    let grid_size = 4usize; // 4x4 = 16个网格
    let h_bins = 6usize;
    let s_bins = 3usize;
    let bins_per_cell = h_bins * s_bins; // 18

    let mut features = Vec::with_capacity(grid_size * grid_size * bins_per_cell + grid_size * 2);

    let cell_w = 64 / grid_size as u32;
    let cell_h = 64 / grid_size as u32;

    for gy in 0..grid_size {
        for gx in 0..grid_size {
            let mut hist = vec![0.0f32; bins_per_cell];
            let mut count = 0f32;

            for cy in 0..cell_h {
                for cx in 0..cell_w {
                    let px = (gx as u32 * cell_w + cx) as u32;
                    let py = (gy as u32 * cell_h + cy) as u32;
                    let pixel = resized.get_pixel(px, py);
                    let (hue, sat, val) = rgb_to_hsv(pixel[0], pixel[1], pixel[2]);

                    let h_idx = (hue * h_bins as f32) as usize % h_bins;
                    let s_idx = (sat * s_bins as f32) as usize % s_bins;
                    let idx = h_idx * s_bins + s_idx;
                    hist[idx] += val; // 用亮度加权
                    count += 1.0;
                }
            }

            // 归一化
            if count > 0.0 {
                for v in hist.iter_mut() {
                    *v /= count;
                }
            }

            features.extend(hist);
        }
    }

    // 添加每行平均亮度作为轮廓特征
    for y in 0..64 {
        let mut row_sum = 0.0f32;
        for x in 0..64 {
            let p = resized.get_pixel(x, y);
            row_sum += (p[0] as f32 * 0.299 + p[1] as f32 * 0.587 + p[2] as f32 * 0.114) / 255.0;
        }
        features.push(row_sum / 64.0);
    }

    features
}

fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let r = r as f32 / 255.0;
    let g = g as f32 / 255.0;
    let b = b as f32 / 255.0;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    let hue = if delta < 1e-6 {
        0.0
    } else if max == r {
        ((g - b) / delta) % 6.0 / 6.0
    } else if max == g {
        ((b - r) / delta + 2.0) / 6.0
    } else {
        ((r - g) / delta + 4.0) / 6.0
    };

    let hue = if hue < 0.0 { hue + 1.0 } else { hue };
    let sat = if max < 1e-6 { 0.0 } else { delta / max };

    (hue, sat, max)
}

fn crop_face(img: &DynamicImage, face: &FaceRect) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    let rgb = img.to_rgb8();
    let view = image::imageops::crop_imm(&rgb, face.x, face.y, face.width, face.height);
    view.to_image()
}

/// 使用余弦相似度对「外观特征」聚类（基于颜色直方图，非身份向量）。
/// 注意：这只是把外观相近的人脸区域聚到一起，并不等同于识别「同一个人」。
pub fn cluster_faces(detections: Vec<FaceDetection>, threshold: f32) -> Vec<Vec<FaceDetection>> {
    if detections.is_empty() {
        return Vec::new();
    }

    let mut clusters: Vec<Vec<FaceDetection>> = Vec::new();

    for detection in detections {
        let mut best_cluster = None;
        let mut best_sim = threshold;

        for (i, cluster) in clusters.iter().enumerate() {
            // 与聚类质心（平均值）比较
            let centroid_sim = cosine_similarity(&detection.features, &cluster_average(cluster));

            if centroid_sim > best_sim {
                best_sim = centroid_sim;
                best_cluster = Some(i);
            }
        }

        if let Some(i) = best_cluster {
            clusters[i].push(detection);
        } else {
            clusters.push(vec![detection]);
        }
    }

    // 只保留 >= 3张不同照片的聚类
    clusters.retain(|c| {
        let unique_photos: std::collections::HashSet<_> = c.iter().map(|d| &d.photo_id).collect();
        unique_photos.len() >= 3
    });

    clusters.sort_by(|a, b| b.len().cmp(&a.len()));
    clusters.truncate(15);

    clusters
}

fn cluster_average(cluster: &[FaceDetection]) -> Vec<f32> {
    if cluster.is_empty() {
        return Vec::new();
    }
    let len = cluster[0].features.len();
    let mut avg = vec![0.0f32; len];
    for d in cluster {
        for i in 0..len {
            avg[i] += d.features[i];
        }
    }
    let n = cluster.len() as f32;
    for v in avg.iter_mut() {
        *v /= n;
    }
    avg
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let n = a.len().min(b.len());
    let mut dot = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;
    for i in 0..n {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }
    if norm_a < 1e-6 || norm_b < 1e-6 {
        return 0.0;
    }
    dot / (norm_a.sqrt() * norm_b.sqrt())
}

pub fn save_face_thumbnail(
    img: &DynamicImage,
    face: &FaceRect,
    thumbs_dir: &str,
    photo_id: &str,
) -> Option<String> {
    use md5::{Md5, Digest};

    let mut hasher = Md5::new();
    hasher.update(format!("{}_{}_{}_{}", photo_id, face.x, face.y, face.width).as_bytes());
    let hash = format!("{:x}", hasher.finalize());
    let thumb_name = format!("face_{}.jpg", hash);
    let thumb_path = format!("{}/{}", thumbs_dir, thumb_name);

    if Path::new(&thumb_path).exists() {
        return Some(thumb_path);
    }

    let pad_x = (face.width as f32 * 0.2) as u32;
    let pad_y = (face.height as f32 * 0.3) as u32;
    let x = face.x.saturating_sub(pad_x);
    let y = face.y.saturating_sub(pad_y);
    let w = (face.width + pad_x * 2).min(img.width() - x);
    let h = (face.height + pad_y * 2).min(img.height() - y);

    let rgb = img.to_rgb8();
    let view = image::imageops::crop_imm(&rgb, x, y, w, h);
    let cropped = view.to_image();
    let resized = image::imageops::resize(&cropped, 100, 100, FilterType::Lanczos3);

    let output = std::fs::File::create(&thumb_path).ok()?;
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(std::io::BufWriter::new(output), 80);
    encoder.encode_image(&resized).ok()?;

    Some(thumb_path)
}

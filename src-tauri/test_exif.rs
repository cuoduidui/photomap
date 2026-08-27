use exif::{In, Tag, Value};
use std::fs::File;
use std::io::BufReader;

fn main() {
    let path = r"c:\Users\Admin\.trae-cn\attachments\6a8c5f3617e98309a72641ac\049879ac-2656-47cb-b24f-3ef1adc91feb_4a7f6132-2c81-45be-95a5-23feb29dfc89_IMG_20250101_163834.jpg";
    
    let file = File::open(path).unwrap();
    let mut buf_reader = BufReader::new(&file);
    let exif_reader = exif::Reader::new();
    let exif_data = exif_reader.read_from_container(&mut buf_reader).unwrap();

    println!("=== 所有 IFD 中的字段 ===");
    let ifds = [In::PRIMARY, In::EXIF, In::GPS, In::INTEROP, In::THUMBNAIL];
    for ifd in &ifds {
        println!("\n--- IFD: {:?} ---", ifd);
        for f in exif_data.fields() {
            if f.ifd_num == *ifd {
                println!("  {}: {}", f.tag, f.display_value());
            }
        }
    }

    println!("\n\n=== 测试 GPSLatitude ===");
    for ifd in &ifds {
        if let Some(field) = exif_data.get_field(Tag::GPSLatitude, *ifd) {
            println!("  IFD {:?}: {:?}", ifd, field.value);
        }
    }

    println!("\n=== 测试 DateTimeOriginal ===");
    for ifd in &ifds {
        if let Some(field) = exif_data.get_field(Tag::DateTimeOriginal, *ifd) {
            println!("  IFD {:?}: {}", ifd, field.display_value());
        }
    }
}

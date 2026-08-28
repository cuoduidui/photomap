// 用法: cargo run --example exif_dump -- <图片路径>
// 将指定图片的 EXIF 字段打印到终端，便于调试。
use exif::Reader;
use std::{env, fs::File, io::BufReader};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("用法: cargo run --example exif_dump -- <图片路径>");
        std::process::exit(1);
    }

    let path = &args[1];
    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("无法打开 {}: {}", path, e);
            std::process::exit(1);
        }
    };

    let exif_data = match Reader::new().read_from_container(&mut BufReader::new(&file)) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("读取 EXIF 失败: {}", e);
            std::process::exit(1);
        }
    };

    println!("=== 所有 IFD 中的字段 ===");
    for f in exif_data.fields() {
        println!("IFD {}: {}: {}", f.ifd_num, f.tag, f.display_value());
    }
}

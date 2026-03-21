use anyhow::Result;
use clap::{Parser, Subcommand};
use image::ImageEncoder;
use std::io::Write;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "MangaMeeya development tasks")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate test data sets for benchmarks and scale tests.
    GenerateTestdata {
        /// Output directory
        #[arg(short, long, default_value = "testdata/generated")]
        output: PathBuf,

        /// Generate large datasets (500+ images)
        #[arg(long)]
        large: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::GenerateTestdata { output, large } => {
            generate_testdata(&output, large)?;
        }
    }

    Ok(())
}

fn generate_testdata(output: &PathBuf, large: bool) -> Result<()> {
    std::fs::create_dir_all(output)?;

    // Small dataset: 5 images
    generate_dataset(output, "small", 5, 800, 1200)?;

    // Medium dataset: 50 images
    generate_dataset(output, "medium", 50, 1700, 2400)?;

    if large {
        // Large dataset: 500 images
        generate_dataset(output, "large", 500, 1700, 2400)?;

        // Huge dataset: 2000 images
        generate_dataset(output, "huge", 2000, 1700, 2400)?;
    }

    // Format test images (1 per codec)
    generate_format_samples(output)?;

    // Edge cases
    generate_edge_cases(output)?;

    println!("Test data generated at: {}", output.display());
    Ok(())
}

fn generate_dataset(
    base: &PathBuf,
    name: &str,
    count: usize,
    width: u32,
    height: u32,
) -> Result<()> {
    let dir = base.join(name);
    std::fs::create_dir_all(&dir)?;

    // Generate as folder
    println!("Generating {name} dataset ({count} images, {width}x{height})...");
    for i in 0..count {
        let img = generate_manga_page(width, height, i);
        let path = dir.join(format!("page{:04}.jpg", i + 1));
        img.save_with_format(&path, image::ImageFormat::Jpeg)?;
    }

    // Also create a ZIP
    let zip_path = base.join(format!("{name}.zip"));
    let file = std::fs::File::create(&zip_path)?;
    let mut zip = zip::ZipWriter::new(std::io::BufWriter::new(file));

    for i in 0..count {
        let img = generate_manga_page(width, height, i);
        let mut buf = Vec::new();
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 85);
        encoder.write_image(
            img.as_raw(),
            width,
            height,
            image::ExtendedColorType::Rgb8,
        )?;

        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        zip.start_file(format!("page{:04}.jpg", i + 1), options)?;
        zip.write_all(&buf)?;
    }
    zip.finish()?;
    println!("  -> {name}.zip created");

    Ok(())
}

fn generate_manga_page(width: u32, height: u32, seed: usize) -> image::RgbImage {
    image::RgbImage::from_fn(width, height, |x, y| {
        // Create a visually distinct pattern per page
        let r = ((x.wrapping_mul(3).wrapping_add(seed as u32 * 37)) % 256) as u8;
        let g = ((y.wrapping_mul(5).wrapping_add(seed as u32 * 73)) % 256) as u8;
        let b = (((x + y).wrapping_mul(7).wrapping_add(seed as u32 * 131)) % 256) as u8;
        image::Rgb([r, g, b])
    })
}

fn generate_format_samples(base: &PathBuf) -> Result<()> {
    let dir = base.join("formats");
    std::fs::create_dir_all(&dir)?;

    let img = generate_manga_page(200, 300, 42);

    img.save_with_format(dir.join("sample.jpg"), image::ImageFormat::Jpeg)?;
    img.save_with_format(dir.join("sample.png"), image::ImageFormat::Png)?;
    img.save_with_format(dir.join("sample.bmp"), image::ImageFormat::Bmp)?;
    img.save_with_format(dir.join("sample.gif"), image::ImageFormat::Gif)?;
    img.save_with_format(dir.join("sample.tiff"), image::ImageFormat::Tiff)?;

    println!("  -> format samples created");
    Ok(())
}

fn generate_edge_cases(base: &PathBuf) -> Result<()> {
    let dir = base.join("edge");
    std::fs::create_dir_all(&dir)?;

    // 1x1 image
    let tiny = image::RgbImage::from_pixel(1, 1, image::Rgb([255, 0, 0]));
    tiny.save_with_format(dir.join("1x1.jpg"), image::ImageFormat::Jpeg)?;

    // Very wide (landscape)
    let wide = generate_manga_page(2000, 100, 1);
    wide.save_with_format(dir.join("wide.jpg"), image::ImageFormat::Jpeg)?;

    // Very tall (portrait)
    let tall = generate_manga_page(100, 2000, 2);
    tall.save_with_format(dir.join("tall.jpg"), image::ImageFormat::Jpeg)?;

    // Square
    let square = generate_manga_page(500, 500, 3);
    square.save_with_format(dir.join("square.jpg"), image::ImageFormat::Jpeg)?;

    // Corrupt file
    std::fs::write(dir.join("corrupt.jpg"), b"NOT A REAL JPEG FILE")?;

    // Zero-byte file
    std::fs::write(dir.join("zero.jpg"), b"")?;

    // Truncated JPEG (valid header, truncated body)
    let valid = generate_manga_page(100, 100, 4);
    let mut buf = Vec::new();
    let encoder = image::codecs::jpeg::JpegEncoder::new(&mut buf);
    encoder.write_image(
        valid.as_raw(),
        100,
        100,
        image::ExtendedColorType::Rgb8,
    )?;
    std::fs::write(dir.join("truncated.jpg"), &buf[..buf.len() / 2])?;

    println!("  -> edge case samples created");
    Ok(())
}

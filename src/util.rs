use fast_image_resize::images::Image;
use fast_image_resize::{FilterType, PixelType, ResizeAlg, ResizeOptions, Resizer};
use image::{DynamicImage, GenericImageView, ImageBuffer, Rgb, Rgba};

// Convert an image to grayscale while preserving alpha channels
pub fn convert_to_grayscale_optimized(img: &DynamicImage) -> DynamicImage {
    let (width, height) = img.dimensions();

    match img {
        // Handle RGBA images (with transparency)
        DynamicImage::ImageRgba8(rgba_img) => {
            let mut output = ImageBuffer::new(width, height);
            for (x, y, pixel) in rgba_img.enumerate_pixels() {
                let luma = ((pixel[0] as u32 * 299 + pixel[1] as u32 * 587 + pixel[2] as u32 * 114)
                    / 1000) as u8;
                output.put_pixel(x, y, Rgba([luma, luma, luma, pixel[3]]));
            }
            DynamicImage::ImageRgba8(output)
        }
        // Handle RGB images (no transparency)
        DynamicImage::ImageRgb8(rgb_img) => {
            let mut output = ImageBuffer::new(width, height);
            for (x, y, pixel) in rgb_img.enumerate_pixels() {
                let luma = ((pixel[0] as u32 * 299 + pixel[1] as u32 * 587 + pixel[2] as u32 * 114)
                    / 1000) as u8;
                output.put_pixel(x, y, Rgba([luma, luma, luma, 255]));
            }
            DynamicImage::ImageRgba8(output)
        }
        // Handle any other image format by converting to RGBA first
        _ => {
            let rgba = img.to_rgba8();
            let mut output = ImageBuffer::new(width, height);
            for (x, y, pixel) in rgba.enumerate_pixels() {
                let luma = ((pixel[0] as u32 * 299 + pixel[1] as u32 * 587 + pixel[2] as u32 * 114)
                    / 1000) as u8;
                output.put_pixel(x, y, Rgba([luma, luma, luma, pixel[3]]));
            }
            DynamicImage::ImageRgba8(output)
        }
    }
}

pub fn cap_megapixels(
    dyn_img: &DynamicImage,
    max_pixels: u64, // e.g. 4_000_000 for 4MP
) -> Result<DynamicImage, Box<dyn std::error::Error>> {
    let orig_w = dyn_img.width();
    let orig_h = dyn_img.height();
    let total_pixels = (orig_w as u64) * (orig_h as u64);

    // 1. Pass through untouched if within budget
    if total_pixels <= max_pixels {
        return Ok(dyn_img.clone());
    }

    // 2. Calculate scaling factor based on area ratio
    let scale = (max_pixels as f64 / total_pixels as f64).sqrt() as f32;
    let target_w = ((orig_w as f32 * scale).round() as u32).max(1);
    let target_h = ((orig_h as f32 * scale).round() as u32).max(1);

    // 3. Prepare source buffer
    let rgb8 = dyn_img.to_rgb8();
    let src_image = Image::from_vec_u8(orig_w, orig_h, rgb8.into_raw(), PixelType::U8x3)?;

    // 4. Allocate destination buffer
    let mut dst_image = Image::new(target_w, target_h, PixelType::U8x3);

    // 5. Downscale with Lanczos3 filter to eliminate screentone Moiré grid artifacts
    let options = ResizeOptions::new().resize_alg(ResizeAlg::Convolution(FilterType::CatmullRom));
    let mut resizer = Resizer::new();
    resizer.resize(&src_image, &mut dst_image, &options)?;

    // 6. Wrap back into DynamicImage
    let buffer = ImageBuffer::<Rgb<u8>, _>::from_raw(target_w, target_h, dst_image.into_vec())
        .ok_or("Failed to construct image buffer")?;

    Ok(DynamicImage::ImageRgb8(buffer))
}

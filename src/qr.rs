use crate::util::public_url;
use image::ImageEncoder;
use qrcode::QrCode;
use qrcode::render::svg;

pub fn qr_svg(short_code: &str) -> std::result::Result<String, String> {
    let url = format!("{}/{short_code}", public_url());
    let code =
        QrCode::new(url.as_bytes()).map_err(|e| format!("failed to generate QR code: {e}"))?;
    Ok(code
        .render::<svg::Color>()
        .dark_color(svg::Color("#0f172a"))
        .light_color(svg::Color("#e2e8f0"))
        .min_dimensions(6, 6)
        .build())
}

pub fn qr_png_bytes(short_code: &str) -> std::result::Result<Vec<u8>, String> {
    let url = format!("{}/{short_code}", public_url());
    let code =
        QrCode::new(url.as_bytes()).map_err(|e| format!("failed to generate QR code: {e}"))?;
    let img = code
        .render::<image::Luma<u8>>()
        .min_dimensions(6, 6)
        .dark_color(image::Luma([0u8]))
        .light_color(image::Luma([255u8]))
        .build();
    let mut buf = std::io::Cursor::new(Vec::new());
    let encoder = image::codecs::png::PngEncoder::new(&mut buf);
    encoder
        .write_image(
            img.as_raw(),
            img.width(),
            img.height(),
            image::ExtendedColorType::L8,
        )
        .map_err(|e| format!("failed to encode PNG: {e}"))?;
    Ok(buf.into_inner())
}

use crate::cli::AppConfig;
use crate::util;
use axum::{
    extract::{rejection::QueryRejection, Query, RawQuery, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use std::sync::Arc;
use tracing;

#[derive(Deserialize)]
pub struct ImageParams {
    pub url: String,
    pub bw: Option<u8>, // 0-100, where 100 is highest quality
    pub l: Option<u8>,  // Convert to black and white if true
}

pub async fn root_handler() -> &'static str {
    tracing::info!("Triggered root handler");
    "bandwidth-hero-proxy"
}

pub async fn proxy_handler(
    RawQuery(raw_query): RawQuery,
    mut headers: HeaderMap,
    params: Result<Query<ImageParams>, QueryRejection>,
    State(config): State<Arc<AppConfig>>,
) -> Result<Response, (StatusCode, String)> {
    if raw_query.as_deref().unwrap_or("").is_empty() {
        return Ok(root_handler().await.into_response());
    }

    let Query(params) = params.map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "Missing or invalid query parameters. Use /?url=<image_url>&bw=<0|1>&l=<0-100>"
                .to_string(),
        )
    })?;

    tracing::info!(
        "Processing image: {} (quality: {:?}, grayscale: {:?})",
        params.url,
        params.l,
        params.bw,
    );

    headers.remove(header::HOST);
    headers.remove(header::CONTENT_LENGTH);
    headers.remove(header::ACCEPT_ENCODING);
    headers.remove("connection");
    headers.remove("x-forwarded-for");

    // Add hotlink protection header
    if let Ok(referer) = params.url.parse() {
        headers.insert(header::REFERER, referer);
    }

    let response = config
        .client
        .get(&params.url)
        .headers(headers)
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

    let status = response.status();
    if !status.is_success() {
        tracing::warn!("Failed to get image, status code: {}", status);

        let status =
            StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        return Ok((
            status,
            format!("Failed to get image, status code: {}", status),
        )
            .into_response());
    }

    // Get the image data
    let bytes = Arc::new(response.bytes().await.map_err(|e| {
        tracing::error!("Error reading image data: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error reading image: {}", e),
        )
    })?);

    let webp_image = tokio::task::spawn_blocking(move || {
        // Load and decode the image
        let mut img = image::load_from_memory(&bytes).map_err(|e| {
            tracing::error!("Error processing image: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error processing image: {e}"),
            )
        })?;

        // Convert to grayscale if requested
        params.bw.map(|v| {
            if v == 1 {
                img = util::convert_to_grayscale_optimized(&img);
            }
        });

        if let Some(cap) = config.mp_cap {
            let max_pixels = 1_000_000 * cap as u64;
            img = util::cap_megapixels(&img, max_pixels).map_err(|e| {
                tracing::error!("Fast image resize error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Fast image resize error: {}", e),
                )
            })?;
        }

        let img = img.into_rgba8();

        // WebP encoding - quality is straightforward 0-100
        let quality_float = params.l.map_or(80.0, |v| v as f32);
        let webp_encoder = webp::Encoder::from_rgba(&img, img.width(), img.height());

        Ok(webp_encoder.encode(quality_float).to_vec())
    })
    .await
    .map_err(|e| {
        tracing::error!("Blocking task panicked: {e}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal processing error".to_string(),
        )
    })??;

    Ok(([(header::CONTENT_TYPE, "image/webp")], webp_image).into_response())
}

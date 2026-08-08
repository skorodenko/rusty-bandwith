use crate::cli::AppConfig;
use crate::util;
use axum::{
    extract::Query,
    extract::State,
    http::{header, StatusCode},
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
    "bandwidth-hero-proxy"
}

pub async fn proxy_handler(
    Query(params): Query<ImageParams>,
    State(config): State<Arc<AppConfig>>,
) -> Result<Response, (StatusCode, String)> {
    tracing::info!(
        "Processing image: {} (quality: {:?}, grayscale: {:?})",
        params.url,
        params.l,
        params.bw,
    );

    // Download the image
    let response = match reqwest::get(&params.url).await {
        Ok(response) => response,
        Err(e) => {
            tracing::error!("Error fetching image: {}", e);
            return Err((
                StatusCode::BAD_REQUEST,
                format!("Error fetching image: {}", e),
            ));
        }
    };

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

        // WebP encoding - quality is straightforward 0-100
        let quality_float = params.l.map_or(80.0, |v| v as f32);
        let webp_encoder = webp::Encoder::from_image(&img).map_err(|e| {
            tracing::error!("WebP encoding error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("WebP encoding error: {}", e),
            )
        })?;

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

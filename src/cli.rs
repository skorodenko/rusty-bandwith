use clap::{Parser, ValueHint};
use std::net::{IpAddr, Ipv4Addr};

// Server configuration that's shared between threads
pub struct AppConfig {
    pub mp_cap: Option<f32>,
}

// Command line arguments for configuring the server
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Host addr to listen on
    #[arg(long, value_name = "HOST", env = "HOST", default_value_t = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)))]
    pub host: IpAddr,

    /// Port to listen on
    #[arg(long, value_name = "PORT", env = "PORT", default_value_t = 8080, value_hint = ValueHint::Other)]
    pub port: u16,

    /// Megapixel cap for resize
    #[arg(long, value_name = "MPCAP", env = "MPCAP")]
    pub mp_cap: Option<f32>,

    /// Enable JXL encoding instead of WebP
    #[arg(long, env = "JXL", default_value_t = false)]
    pub jxl: bool,

    /// Control JXL encoding speed/effort level
    /// 1 = fastest but lower quality (Lightning)
    /// 8 = slowest but highest quality (Tortoise)
    #[arg(long, value_name = "SPEED", default_value_t = 8)]
    pub speed: u8,
}

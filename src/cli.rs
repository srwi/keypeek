use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Toggle overlay visibility
    #[arg(short, long)]
    pub toggle: bool,

    /// Show settings window
    #[arg(short, long)]
    pub settings: bool,

    /// Connect to a VIA device using the provided JSON path
    #[arg(long)]
    pub via: Option<String>,

    /// Connect to a Vial device using VID:PID (e.g. 1234:5678)
    #[arg(long)]
    pub vial: Option<String>,

    /// Connect to a ZMK device using VID:PID (e.g. 1234:5678)
    #[arg(long)]
    pub zmk: Option<String>,

    /// ZMK serial port name
    #[arg(long)]
    pub serial: Option<String>,

    /// ZMK BLE device ID
    #[arg(long)]
    pub ble: Option<String>,
}

pub fn parse_vid_pid(s: &str) -> Result<(u16, u16), String> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return Err("Format must be VID:PID (hex)".to_string());
    }
    let vid = u16::from_str_radix(parts[0], 16).map_err(|e| format!("Invalid VID: {e}"))?;
    let pid = u16::from_str_radix(parts[1], 16).map_err(|e| format!("Invalid PID: {e}"))?;
    Ok((vid, pid))
}

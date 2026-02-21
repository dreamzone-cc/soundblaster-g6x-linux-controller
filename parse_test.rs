fn main() {
    let line = "  Front Left: Playback 73 [50%] [-27.50dB] [off] Capture 28 [49%] [-20.00dB] [on]";
    
    let mut p_vol = 0.0;
    let mut c_vol = 0.0;
    
    let parts: Vec<&str> = line.split("Capture").collect();
    
    // Parse Playback
    if let Some(p) = parts.get(0) {
        if let Some(start) = p.find('[') {
            if let Some(end) = p[start..].find('%') {
                p_vol = p[start+1..start+end].parse().unwrap_or(0.0) / 100.0;
            }
        }
    }
    
    // Parse Capture
    if let Some(p) = parts.get(1) {
        if let Some(start) = p.find('[') {
            if let Some(end) = p[start..].find('%') {
                c_vol = p[start+1..start+end].parse().unwrap_or(0.0) / 100.0;
            }
        }
    }
    
    println!("p_vol: {}, c_vol: {}", p_vol, c_vol);
}

use anyhow::Result;

pub fn get_icon() -> Result<(u32, u32, Vec<u8>)> {
    let buffer = include_bytes!("../../assets/icon32");
    let width = u32::from_le_bytes(buffer[0..4].try_into()?);
    let height = u32::from_le_bytes(buffer[4..8].try_into()?);
    let buffer = buffer[8..].to_vec();
    Ok((width, height, buffer))
}

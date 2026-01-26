//! 指令数据解析器
//!
//! 当 Transfer 记录不足时，从指令数据 offset 解析代币数量
//!
//! 使用场景：
//! - 某些交易可能没有足够的 Transfer 记录
//! - 需要从指令数据中直接提取代币数量信息
//!
//! 注意：此解析器作为备选方案，优先使用 Transfer 记录解析

/// 指令数据解析错误
#[derive(Debug, thiserror::Error)]
pub enum InstructionDataParseError {
    #[error("数据长度不足，需要至少 {required} 字节，实际只有 {actual} 字节")]
    InsufficientData { required: usize, actual: usize },

    #[error("无效的 offset 位置: {0}")]
    InvalidOffset(usize),

    #[error("无法解析 u64 数值")]
    InvalidU64,
}

/// 从指令数据的指定 offset 解析 u64 数值
///
/// # 参数
/// - `data`: 指令数据
/// - `offset`: 数据起始偏移量（字节）
///
/// # 返回
/// - `Ok(u64)`: 解析出的数值
/// - `Err(InstructionDataParseError)`: 解析失败
///
/// # 示例
/// ```rust
/// use sol_trade_sdk::parser::instruction_data_parser::parse_u64_from_offset;
///
/// let data = vec![0, 0, 0, 0, 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0];
/// let amount = parse_u64_from_offset(&data, 4)?;
/// assert_eq!(amount, 0xf0debc9a78563412);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn parse_u64_from_offset(data: &[u8], offset: usize) -> Result<u64, InstructionDataParseError> {
    // 检查数据长度
    if data.len() < offset + 8 {
        return Err(InstructionDataParseError::InsufficientData {
            required: offset + 8,
            actual: data.len(),
        });
    }

    // Solana 使用 little-endian 字节序
    let bytes = &data[offset..offset + 8];
    let value = u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ]);

    Ok(value)
}

/// 从指令数据的指定 offset 解析 u128 数值
///
/// # 参数
/// - `data`: 指令数据
/// - `offset`: 数据起始偏移量（字节）
///
/// # 返回
/// - `Ok(u128)`: 解析出的数值
/// - `Err(InstructionDataParseError)`: 解析失败
pub fn parse_u128_from_offset(
    data: &[u8],
    offset: usize,
) -> Result<u128, InstructionDataParseError> {
    // 检查数据长度
    if data.len() < offset + 16 {
        return Err(InstructionDataParseError::InsufficientData {
            required: offset + 16,
            actual: data.len(),
        });
    }

    // Solana 使用 little-endian 字节序
    let bytes = &data[offset..offset + 16];
    let value = u128::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7], bytes[8],
        bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
    ]);

    Ok(value)
}

/// 解析代币数量（考虑 decimals）
///
/// # 参数
/// - `amount_raw`: 原始数量（u64）
/// - `decimals`: 代币精度
///
/// # 返回
/// - `f64`: 转换后的人类可读数量
pub fn format_token_amount(amount_raw: u64, decimals: u8) -> f64 {
    amount_raw as f64 / 10_f64.powi(decimals as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_u64_from_offset() {
        // 测试正常情况
        let data = vec![
            0, 1, 2, 3, // 前 4 字节（offset）
            0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, // u64 数据
            4, 5, 6, 7, // 后续数据
        ];

        let result = parse_u64_from_offset(&data, 4).unwrap();
        assert_eq!(result, 0xf0debc9a78563412);

        // 测试 offset = 0
        let data2 = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0, 0, 0, 0];
        let result2 = parse_u64_from_offset(&data2, 0).unwrap();
        assert_eq!(result2, 0x0807060504030201);
    }

    #[test]
    fn test_parse_u64_insufficient_data() {
        let data = vec![0, 1, 2, 3]; // 只有 4 字节
        let result = parse_u64_from_offset(&data, 0);

        assert!(matches!(result, Err(InstructionDataParseError::InsufficientData { .. })));
    }

    #[test]
    fn test_parse_u128_from_offset() {
        // 测试正常情况
        let data = vec![
            0, 1, 2, 3, // 前 4 字节（offset）
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
            0x0f, 0x10, // u128 数据
            4, 5, 6, 7, // 后续数据
        ];

        let result = parse_u128_from_offset(&data, 4).unwrap();
        assert_eq!(result, 0x100f0e0d0c0b0a090807060504030201);
    }

    #[test]
    fn test_parse_u128_insufficient_data() {
        let data = vec![0, 1, 2, 3]; // 只有 4 字节
        let result = parse_u128_from_offset(&data, 0);

        assert!(matches!(result, Err(InstructionDataParseError::InsufficientData { .. })));
    }

    #[test]
    fn test_format_token_amount() {
        // 测试 6 位小数（USDC）
        let amount = 1_500_000;
        let formatted = format_token_amount(amount, 6);
        assert_eq!(formatted, 1.5);

        // 测试 9 位小数（SOL）
        let amount = 2_000_000_000;
        let formatted = format_token_amount(amount, 9);
        assert_eq!(formatted, 2.0);

        // 测试 0 位小数
        let amount = 100;
        let formatted = format_token_amount(amount, 0);
        assert_eq!(formatted, 100.0);
    }
}

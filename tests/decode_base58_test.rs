//! 测试解码 base58 数据

#[test]
fn test_decode_base58() {
    let data = "5zoXf9xCCwZEbjoEtV83ozX";

    // 使用 bs58 crate 解码
    let decoded = bs58::decode(data).into_vec().unwrap();

    println!("Decoded bytes: {:?}", decoded);
    println!("Hex: {:02x?}", decoded);
    println!("First byte: {}", decoded[0]);
    println!("First byte (decimal): {}", decoded[0]);

    // Raydium V4 SWAP discriminator 是 9
    assert_eq!(decoded[0], 9, "第一个字节应该是 9 (SWAP)");
}

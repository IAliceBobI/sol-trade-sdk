//! Jito 区域测试

use sol_trade_sdk::swqos::jito::types::JitoRegion;

#[test]
fn test_all_region_endpoints() {
    // 测试所有区域的 endpoint URL 是否正确
    let test_cases = vec![
        (
            JitoRegion::Default,
            "https://mainnet.block-engine.jito.wtf",
        ),
        (
            JitoRegion::Amsterdam,
            "https://amsterdam.mainnet.block-engine.jito.wtf",
        ),
        (
            JitoRegion::Dublin,
            "https://dublin.mainnet.block-engine.jito.wtf",
        ),
        (
            JitoRegion::Frankfurt,
            "https://frankfurt.mainnet.block-engine.jito.wtf",
        ),
        (
            JitoRegion::London,
            "https://london.mainnet.block-engine.jito.wtf",
        ),
        (
            JitoRegion::NewYork,
            "https://ny.mainnet.block-engine.jito.wtf",
        ),
        (JitoRegion::SLC, "https://slc.mainnet.block-engine.jito.wtf"),
        (
            JitoRegion::Singapore,
            "https://singapore.mainnet.block-engine.jito.wtf",
        ),
        (
            JitoRegion::Tokyo,
            "https://tokyo.mainnet.block-engine.jito.wtf",
        ),
    ];

    for (region, expected_endpoint) in test_cases {
        assert_eq!(
            region.endpoint(),
            expected_endpoint,
            "Region {:?} endpoint mismatch",
            region
        );
    }

    println!("✅ 所有 9 个区域的 endpoint URL 正确");
}

#[test]
fn test_region_from_str() {
    // 测试从字符串解析区域
    let test_cases = vec![
        ("tokyo", JitoRegion::Tokyo),
        ("TOKYO", JitoRegion::Tokyo),
        ("tokyo", JitoRegion::Tokyo),
        ("ny", JitoRegion::NewYork),
        ("newyork", JitoRegion::NewYork),
        ("newyork", JitoRegion::NewYork),
        ("amsterdam", JitoRegion::Amsterdam),
        ("ams", JitoRegion::Amsterdam),
        ("dublin", JitoRegion::Dublin),
        ("dub", JitoRegion::Dublin),
        ("frankfurt", JitoRegion::Frankfurt),
        ("fra", JitoRegion::Frankfurt),
        ("ffm", JitoRegion::Frankfurt),
        ("london", JitoRegion::London),
        ("lon", JitoRegion::London),
        ("slc", JitoRegion::SLC),
        ("saltlakecity", JitoRegion::SLC),
        ("singapore", JitoRegion::Singapore),
        ("sgp", JitoRegion::Singapore),
        ("sg", JitoRegion::Singapore),
        ("default", JitoRegion::Default),
    ];

    for (input, expected) in test_cases {
        let result = JitoRegion::from_str(input);
        assert!(
            result.is_ok(),
            "Failed to parse region from '{}': {:?}",
            input,
            result
        );
        assert_eq!(
            result.unwrap(),
            expected,
            "Region mismatch for input '{}'",
            input
        );
    }

    println!("✅ 所有区域字符串解析正确");

    // 测试无效输入
    assert!(JitoRegion::from_str("invalid").is_err());
    assert!(JitoRegion::from_str("losangeles").is_err()); // 官方不支持
    assert!(JitoRegion::from_str("paris").is_err());

    println!("✅ 无效区域正确返回错误");
}

#[test]
fn test_region_display() {
    // 测试区域的字符串表示
    let test_cases = vec![
        (JitoRegion::Default, "Default"),
        (JitoRegion::Amsterdam, "Amsterdam"),
        (JitoRegion::Dublin, "Dublin"),
        (JitoRegion::Frankfurt, "Frankfurt"),
        (JitoRegion::London, "London"),
        (JitoRegion::NewYork, "NewYork"),
        (JitoRegion::SLC, "SLC"),
        (JitoRegion::Singapore, "Singapore"),
        (JitoRegion::Tokyo, "Tokyo"),
    ];

    for (region, expected) in test_cases {
        assert_eq!(region.to_string(), expected);
    }

    println!("✅ 所有区域的 Display 格式正确");
}

#[test]
fn test_all_regions() {
    // 测试 all_regions() 方法
    let regions = JitoRegion::all_regions();

    assert_eq!(regions.len(), 9, "应该有 9 个区域");

    // 验证包含所有关键区域
    assert!(regions.contains(&JitoRegion::Default));
    assert!(regions.contains(&JitoRegion::Tokyo));
    assert!(regions.contains(&JitoRegion::Singapore));
    assert!(regions.contains(&JitoRegion::Dublin));
    assert!(regions.contains(&JitoRegion::NewYork));

    println!("✅ all_regions() 返回所有 9 个区域");
}

#[test]
fn test_region_default() {
    // 测试 Default trait 实现
    let region = JitoRegion::default();
    assert_eq!(region, JitoRegion::Default);

    println!("✅ JitoRegion::default() 返回 Default");
}

#[test]
fn test_region_recommendations() {
    // 测试区域推荐

    // 亚洲用户应该使用 Tokyo 或 Singapore
    let tokyo = JitoRegion::Tokyo;
    let singapore = JitoRegion::Singapore;
    assert!(tokyo.endpoint().contains("tokyo"));
    assert!(singapore.endpoint().contains("singapore"));

    // 欧洲用户应该使用 Amsterdam, Dublin, Frankfurt, 或 London
    let amsterdam = JitoRegion::Amsterdam;
    let dublin = JitoRegion::Dublin;
    let frankfurt = JitoRegion::Frankfurt;
    let london = JitoRegion::London;

    assert!(amsterdam.endpoint().contains("amsterdam"));
    assert!(dublin.endpoint().contains("dublin"));
    assert!(frankfurt.endpoint().contains("frankfurt"));
    assert!(london.endpoint().contains("london"));

    // 美国东海岸用户应该使用 NewYork
    let ny = JitoRegion::NewYork;
    assert!(ny.endpoint().contains("ny"));

    // 美国西海岸用户应该使用 SLC
    let slc = JitoRegion::SLC;
    assert!(slc.endpoint().contains("slc"));

    println!("✅ 所有区域的地理位置正确");
}

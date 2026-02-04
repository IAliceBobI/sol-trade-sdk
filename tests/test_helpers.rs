//! 测试工具函数
//!
//! 提供测试用的辅助函数，包括 SOL 空投和测试客户端创建

use sol_trade_sdk::{
    SolanaTrade, TradeBuyParams, TradeTokenType,
    common::fast_fn::{
        get_associated_token_address_with_program_id_fast,
        get_associated_token_address_with_program_id_fast_use_seed,
    },
    common::{GasFeeStrategy, TradeConfig},
    constants::{TOKEN_PROGRAM, TOKEN_PROGRAM_2022, WSOL_TOKEN_ACCOUNT},
    swqos::SwqosConfig,
    trading::core::params::{DexParamEnum, PumpSwapParams},
};
use solana_commitment_config::CommitmentConfig;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    native_token::LAMPORTS_PER_SOL, pubkey::Pubkey, signature::Keypair, signer::Signer,
};
use std::sync::Arc;

/// 为测试账户自动空投 SOL
pub async fn airdrop_to_payer(
    rpc_url: &str,
    payer: &Pubkey,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = RpcClient::new(rpc_url.to_string());

    // 检查账户余额
    let balance = client.get_balance(payer).await?;
    println!("账户余额: {} lamports ({:.4} SOL)", balance, balance as f64 / 1e9);

    // 如果余额小于 2 SOL，自动请求空投
    if balance < 2 * LAMPORTS_PER_SOL {
        println!("💧 请求 2 SOL 空投...");
        let airdrop_signature = client.request_airdrop(payer, 2 * LAMPORTS_PER_SOL).await?;
        println!("📤 空投交易签名: {}", airdrop_signature);

        // 等待空投确认
        loop {
            let confirmed = client.confirm_transaction(&airdrop_signature).await?;
            if confirmed {
                break;
            }
        }

        // 验证余额
        let new_balance = client.get_balance(payer).await?;
        println!(
            "✅ 空投成功！新余额: {} lamports ({:.4} SOL)",
            new_balance,
            new_balance as f64 / 1e9
        );
    } else {
        println!("✅ 账户余额充足");
    }
    Ok(())
}

/// 创建测试用的 SolanaTrade 客户端
#[allow(dead_code)]
pub async fn create_test_client() -> SolanaTrade {
    create_test_client_with_seed_optimize(false).await
}

/// 创建测试用的 SolanaTrade 客户端（可选择是否启用 seed 优化）
pub async fn create_test_client_with_seed_optimize(use_seed_optimize: bool) -> SolanaTrade {
    let rpc_url = "http://127.0.0.1:8899".to_string();

    // 使用 Keypair::new() 生成随机测试账户
    let payer = Keypair::new();

    // 空投 SOL
    let payer_pubkey = payer.pubkey();
    if let Err(e) = airdrop_to_payer(&rpc_url, &payer_pubkey).await {
        panic!("空投 SOL 失败，无法继续测试: {}\n  账户: {}\n  RPC: {}", e, payer_pubkey, rpc_url);
    }

    let commitment = CommitmentConfig::confirmed();
    let swqos_configs: Vec<SwqosConfig> = vec![SwqosConfig::Default(rpc_url.clone())];
    let trade_config = TradeConfig::new(rpc_url, swqos_configs, commitment)
        .with_wsol_ata_config(true, use_seed_optimize);
    SolanaTrade::new(Arc::new(payer), trade_config).await
}

/// 获取账户的 WSOL ATA 地址
#[inline]
#[allow(dead_code)]
pub fn get_wsol_ata_address(payer: &Pubkey) -> Pubkey {
    get_associated_token_address_with_program_id_fast(payer, &WSOL_TOKEN_ACCOUNT, &TOKEN_PROGRAM)
}

/// 打印并返回账户的 SOL 和 WSOL 余额（同时使用 get_balance 和 get_token_account_balance）
/// 如果 WSOL 账户不存在（已关闭），返回 (sol_balance, 0)
#[allow(dead_code)]
pub async fn print_balances(
    rpc_url: &str,
    payer: &Pubkey,
) -> Result<(u64, u64), Box<dyn std::error::Error>> {
    let client = RpcClient::new(rpc_url.to_string());

    // 获取 SOL 余额
    let sol_balance = client.get_balance(payer).await?;

    // 获取 WSOL ATA 地址
    let wsol_ata = get_wsol_ata_address(payer);

    // 方式1: 使用 get_balance 获取 WSOL 余额（账户不存在时返回 0）
    let wsol_balance = match client.get_balance(&wsol_ata).await {
        Ok(balance) => balance,
        Err(e) => {
            println!("⚠️  get_balance 查询 WSOL 账户失败: {}，视为余额 0", e);
            0
        },
    };

    // 方式2: 使用 get_token_account_balance 获取 WSOL 余额（账户不存在时返回 0）
    let (wsol_amount, wsol_decimals, wsol_ui_amount_str) =
        match client.get_token_account_balance(&wsol_ata).await {
            Ok(token) => {
                let amount: u64 = token.amount.parse().unwrap_or_else(|e| {
                    println!(
                        "⚠️  解析 WSOL amount 字符串失败: {}，原始值: '{}'，账户: {}，视为余额 0",
                        e, token.amount, wsol_ata
                    );
                    0
                });
                (amount, token.decimals, token.ui_amount_string)
            },
            Err(e) => {
                println!("⚠️  get_token_account_balance 查询 WSOL 账户失败: {}，视为余额 0", e);
                (0, 9, "0".to_string())
            },
        };

    println!("\n========== 账户余额 ==========");
    println!("账户地址: {}", payer);
    println!("WSOL ATA: {}", wsol_ata);
    println!("--------------------------------");
    println!(
        "💰 SOL 余额: {} lamports ({:.4} SOL)",
        sol_balance,
        sol_balance as f64 / LAMPORTS_PER_SOL as f64
    );
    println!("   账户: {}", payer);
    println!(
        "🪙 WSOL 余额 (get_balance): {} lamports ({:.4} SOL)",
        wsol_balance,
        wsol_balance as f64 / LAMPORTS_PER_SOL as f64
    );
    println!("   ATA: {}", wsol_ata);
    println!("🪙 WSOL 余额 (get_token_account_balance): {} lamports", wsol_amount);
    println!("   ATA: {}", wsol_ata);
    println!("🪙 WSOL uiAmountString: {} (decimals: {})", wsol_ui_amount_str, wsol_decimals);
    println!("   ATA: {}", wsol_ata);
    println!("================================\n");

    Ok((sol_balance, wsol_amount))
}

/// 获取指定 mint 的 Token 余额（自动检测 Token Program）
///
/// # 参数
/// * `rpc_url` - RPC URL
/// * `payer` - 钱包地址
/// * `mint` - Token mint 地址
///
/// # 返回
/// * `Ok(u64)` - Token 余额（原始数量）
/// * `Err` - 查询失败
#[allow(dead_code)]
pub async fn get_token_balance(
    rpc_url: &str,
    payer: &Pubkey,
    mint: &Pubkey,
) -> Result<u64, Box<dyn std::error::Error>> {
    let client = RpcClient::new(rpc_url.to_string());

    // 先查询 mint 账户获取 Token Program
    let mint_account = client.get_account(mint).await?;
    let token_program = mint_account.owner;

    // 使用正确的 Token Program 计算 ATA
    let ata = get_associated_token_address_with_program_id_fast(payer, mint, &token_program);

    match client.get_token_account_balance(&ata).await {
        Ok(token) => {
            let amount: u64 = token.amount.parse().unwrap_or_else(|e| {
                println!(
                    "⚠️  解析 token amount 字符串失败: {}，原始值: '{}'，账户: {}",
                    e, token.amount, ata
                );
                0
            });
            Ok(amount)
        },
        Err(_) => Ok(0), // 账户不存在，返回 0
    }
}

/// 打印指定 mint 的 Token 余额并返回（自动检测 Token Program）
///
/// # 参数
/// * `rpc_url` - RPC URL
/// * `payer` - 钱包地址
/// * `mint` - Token mint 地址
/// * `token_name` - Token 名称（用于打印）
///
/// # 返回
/// * `Ok(u64)` - Token 余额（原始数量）
#[allow(dead_code)]
pub async fn print_token_balance(
    rpc_url: &str,
    payer: &Pubkey,
    mint: &Pubkey,
    token_name: &str,
) -> Result<u64, Box<dyn std::error::Error>> {
    let client = RpcClient::new(rpc_url.to_string());

    // 先查询 mint 账户获取 Token Program
    let mint_account = client.get_account(mint).await?;
    let token_program = mint_account.owner;

    // 使用正确的 Token Program 计算 ATA
    let ata = get_associated_token_address_with_program_id_fast(payer, mint, &token_program);

    // 查询余额
    let balance = match client.get_token_account_balance(&ata).await {
        Ok(token) => {
            let amount: u64 = token.amount.parse().unwrap_or_else(|e| {
                println!(
                    "⚠️  解析 token amount 字符串失败: {}，原始值: '{}'，账户: {}",
                    e, token.amount, ata
                );
                0
            });
            amount
        },
        Err(_) => 0,
    };

    println!("  🪙 {} 余额: {}", token_name, balance);
    println!("     Mint: {}", mint);
    println!("     ATA: {}", ata);
    Ok(balance)
}

/// 打印并查询 4 个 ATA 地址的余额
///
/// 包含：
/// 1. TOKEN_PROGRAM (标准)
/// 2. TOKEN_PROGRAM_2022 (标准)
/// 3. TOKEN_PROGRAM (seed 优化)
/// 4. TOKEN_PROGRAM_2022 (seed 优化)
#[allow(dead_code)]
pub async fn print_seed_optimize_balances(
    rpc_url: &str,
    payer: &Pubkey,
    mint: &Pubkey,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = RpcClient::new(rpc_url.to_string());

    // 计算 4 个 ATA 地址
    let ata_token_standard =
        get_associated_token_address_with_program_id_fast(payer, mint, &TOKEN_PROGRAM);
    let ata_token2022_standard =
        get_associated_token_address_with_program_id_fast(payer, mint, &TOKEN_PROGRAM_2022);
    let ata_token_seed = get_associated_token_address_with_program_id_fast_use_seed(
        payer,
        mint,
        &TOKEN_PROGRAM,
        true,
    );
    let ata_token2022_seed = get_associated_token_address_with_program_id_fast_use_seed(
        payer,
        mint,
        &TOKEN_PROGRAM_2022,
        true,
    );

    println!("\n========== Seed 优化 ATA 余额查询 ==========");
    println!("钱包地址: {}", payer);
    println!("Token Mint: {}", mint);
    println!("------------------------------------------");

    // 查询每个地址的余额
    let addresses = [
        ("TOKEN_PROGRAM (标准)", &ata_token_standard),
        ("TOKEN_PROGRAM_2022 (标准)", &ata_token2022_standard),
        ("TOKEN_PROGRAM (seed)", &ata_token_seed),
        ("TOKEN_PROGRAM_2022 (seed)", &ata_token2022_seed),
    ];

    for (name, address) in addresses.iter() {
        match client.get_token_account_balance(address).await {
            Ok(token) => {
                println!("  {:<30} {} ({})", format!("{}:", name), token.ui_amount_string, address);
            },
            Err(_) => {
                // 尝试用 get_balance
                match client.get_balance(address).await {
                    Ok(lamports) => {
                        let sol = lamports as f64 / LAMPORTS_PER_SOL as f64;
                        println!("  {:<30} {:.4} UNIT ({})", format!("{}:", name), sol, address);
                    },
                    Err(_) => {
                        println!("  {:<30} N/A ({})", format!("{}:", name), address);
                    },
                }
            },
        }
    }

    println!("============================================\n");

    Ok(())
}

/// 使用 SOL 购买 Pump 代币（空投用途）
///
/// 这是一个便捷工具函数，封装了 PumpSwap 买入交易的全流程。
/// 用户只需传入购买的 SOL 数量和代币地址，内部自动处理：
/// - 从 RPC 获取池信息
/// - 设置 Gas 策略
/// - 构建买入参数
/// - 执行交易
///
/// # 参数
/// * `client` - TradingClient 实例
/// * `pool` - PumpSwap 池地址
/// * `mint` - 要购买的 Pump 代币 mint 地址
/// * `sol_amount` - 购买的 SOL 数量（lamports），例如 0.01 SOL = 10_000_000 lamports
/// * `slippage_basis_points` - 滑点容忍度（可选，默认为 500，即 5%）
///
/// # 返回
/// * `Ok((bool, Vec<Signature>, Option<TradeError>))` - 交易结果
/// * `Err(anyhow::Error)` - 如果交易执行失败
///
/// # 示例
/// ```ignore
/// // 购买 0.01 SOL 的 Pump 代币
/// let pool = Pubkey::from_str("池地址").unwrap();
/// let mint = Pubkey::from_str("代币地址").unwrap();
/// buy_pump_with_sol(&client, pool, mint, 10_000_000, None).await?;
/// ```
#[allow(dead_code)]
pub async fn buy_pump_with_sol(
    client: &SolanaTrade,
    pool: Pubkey,
    mint: Pubkey,
    sol_amount: u64,
    slippage_basis_points: Option<u64>,
) -> Result<
    (bool, Vec<solana_sdk::signature::Signature>, Option<sol_trade_sdk::swqos::common::TradeError>),
    anyhow::Error,
> {
    println!("\n🛒 开始购买 Pump 代币");
    println!("  - Pool: {}", pool);
    println!("  - Token Mint: {}", mint);
    println!("  - 购买金额: {} lamports ({:.4} SOL)", sol_amount, sol_amount as f64 / 1e9);
    if let Some(slippage) = slippage_basis_points {
        println!("  - 滑点容忍: {} bps ({:.1}%)", slippage, slippage as f64 / 100.0);
    }

    // 1. 从 RPC 获取池信息
    let pump_swap_params = PumpSwapParams::from_pool_address_by_rpc(&client.rpc, &pool)
        .await
        .unwrap_or_else(|e| {
            panic!(
                "从 RPC 获取 PumpSwap Pool 信息失败: {}\n  Pool: {}\n  RPC: {}",
                e,
                pool,
                client.rpc.url()
            )
        });
    println!("  - 池信息获取成功");

    // 2. 从 RPC 获取最新的 blockhash
    let recent_blockhash = client.rpc.get_latest_blockhash().await.map_err(|e| {
        anyhow::anyhow!("获取最新 blockhash 失败: {}\n  RPC: {}", e, client.rpc.url())
    })?;

    // 3. 设置 Gas 策略
    let gas_fee_strategy = GasFeeStrategy::new();
    gas_fee_strategy.set_global_fee_strategy(150_000, 150_000, 500_000, 500_000, 0.001, 0.001);

    // 4. 构建买入参数
    let buy_params = TradeBuyParams {
        dex_type: sol_trade_sdk::DexType::PumpSwap,
        input_token_type: TradeTokenType::SOL,
        mint,
        input_token_amount: sol_amount,
        slippage_basis_points,
        recent_blockhash: Some(recent_blockhash),
        extension_params: DexParamEnum::PumpSwap(pump_swap_params),
        address_lookup_table_account: None,
        wait_transaction_confirmed: true,
        create_input_token_ata: true,
        close_input_token_ata: false, // 推荐：复用 ATA
        create_mint_ata: true,
        durable_nonce: None,
        enable_jito_sandwich_protection: Some(false),
        fixed_output_token_amount: None,
        gas_fee_strategy,
        simulate: false,
        on_transaction_signed: None,
        callback_execution_mode: None,
    };

    // 5. 执行买入交易
    match client.buy(buy_params).await {
        Ok((success, signatures, error)) => {
            if success {
                println!("✅ 买入成功！签名数量: {}", signatures.len());
                for (i, sig) in signatures.iter().enumerate() {
                    println!("  [{}] {}", i + 1, sig);
                }
            } else {
                println!("❌ 买入失败: {:?}", error);
            }
            Ok((success, signatures, error))
        },
        Err(e) => {
            println!("❌ 交易错误: {}", e);
            Err(e)
        },
    }
}

/// 使用固定输出数量购买 Pump 代币
///
/// 指定要购买的代币数量，系统自动计算需要支付的 SOL 金额。
/// 适用于需要精确控制买入代币数量的场景（如空投）。
///
/// # 参数
/// * `client` - TradingClient 实例
/// * `pool` - PumpSwap 池地址
/// * `mint` - 要购买的 Pump 代币 mint 地址
/// * `token_amount` - 要购买的代币数量（整数），例如 10000 个代币
/// * `slippage_basis_points` - 滑点容忍度（可选，默认为 500，即 5%）
///
/// # 返回
/// * `Ok((bool, Vec<Signature>, Option<TradeError>))` - 交易结果
/// * `Err(anyhow::Error)` - 如果交易执行失败
#[allow(dead_code)]
pub async fn buy_pump_with_fixed_output(
    client: &SolanaTrade,
    pool: Pubkey,
    mint: Pubkey,
    token_amount: u64,
    slippage_basis_points: Option<u64>,
) -> Result<
    (bool, Vec<solana_sdk::signature::Signature>, Option<sol_trade_sdk::swqos::common::TradeError>),
    anyhow::Error,
> {
    println!("\n🛒 开始购买 Pump 代币（固定输出数量）");
    println!("  - Pool: {}", pool);
    println!("  - Token Mint: {}", mint);
    println!("  - 目标代币数量: {}", token_amount);
    if let Some(slippage) = slippage_basis_points {
        println!("  - 滑点容忍: {} bps ({:.1}%)", slippage, slippage as f64 / 100.0);
    }

    // 1. 从 RPC 获取池信息
    let pump_swap_params = PumpSwapParams::from_pool_address_by_rpc(&client.rpc, &pool)
        .await
        .unwrap_or_else(|e| {
            panic!(
                "从 RPC 获取 PumpSwap Pool 信息失败: {}\n  Pool: {}\n  RPC: {}",
                e,
                pool,
                client.rpc.url()
            )
        });
    println!("  - 池信息获取成功");

    // 2. 从 RPC 获取最新的 blockhash
    let recent_blockhash = client.rpc.get_latest_blockhash().await.map_err(|e| {
        anyhow::anyhow!("获取最新 blockhash 失败: {}\n  RPC: {}", e, client.rpc.url())
    })?;

    // 3. 设置 Gas 策略
    let gas_fee_strategy = GasFeeStrategy::new();
    gas_fee_strategy.set_global_fee_strategy(150_000, 150_000, 500_000, 500_000, 0.001, 0.001);

    // 4. 构建买入参数（使用 fixed_output_token_amount）
    let buy_params = TradeBuyParams {
        dex_type: sol_trade_sdk::DexType::PumpSwap,
        input_token_type: TradeTokenType::SOL,
        mint,
        input_token_amount: 0, // 使用 fixed_output_token_amount 时不需要
        slippage_basis_points,
        recent_blockhash: Some(recent_blockhash),
        extension_params: DexParamEnum::PumpSwap(pump_swap_params),
        address_lookup_table_account: None,
        wait_transaction_confirmed: true,
        create_input_token_ata: true,
        close_input_token_ata: false,
        create_mint_ata: true,
        durable_nonce: None,
        enable_jito_sandwich_protection: Some(false),
        fixed_output_token_amount: Some(token_amount),
        gas_fee_strategy,
        simulate: false,
        on_transaction_signed: None,
        callback_execution_mode: None,
    };

    // 5. 执行买入交易
    match client.buy(buy_params).await {
        Ok((success, signatures, error)) => {
            if success {
                println!("✅ 买入成功！签名数量: {}", signatures.len());
                for (i, sig) in signatures.iter().enumerate() {
                    println!("  [{}] {}", i + 1, sig);
                }
            } else {
                println!("❌ 买入失败: {:?}", error);
            }
            Ok((success, signatures, error))
        },
        Err(e) => {
            println!("❌ 交易错误: {}", e);
            Err(e)
        },
    }
}

use crate::{
    common::spl_token::close_account,
    constants::{TOKEN_PROGRAM_2022, trade_consts::DEFAULT_SLIPPAGE},
    trading::core::{
        params::{PumpFunParams, SwapParams},
        traits::InstructionBuilder,
    },
};
use crate::{
    instruction::utils::pumpfun::{
        accounts, get_bonding_curve_pda, get_creator, get_user_volume_accumulator_pda,
        global_constants::{self},
    },
    utils::calc::{
        common::{calculate_with_slippage_buy, calculate_with_slippage_sell},
        pumpfun::{get_buy_token_amount_from_sol_amount, get_sell_sol_amount_from_token_amount},
    },
};
use anyhow::{Result, anyhow};
use solana_sdk::instruction::AccountMeta;
use solana_sdk::signature::Keypair;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey, signer::Signer};
use std::sync::Arc;

/// Instruction builder for PumpFun protocol
pub struct PumpFunInstructionBuilder;

#[async_trait::async_trait]
impl InstructionBuilder for PumpFunInstructionBuilder {
    async fn build_buy_instructions(&self, params: &SwapParams) -> Result<Vec<Instruction>> {
        // ========================================
        // Parameter validation and basic data preparation
        // ========================================
        let protocol_params = params
            .protocol_params
            .as_any()
            .downcast_ref::<PumpFunParams>()
            .ok_or_else(|| anyhow!("Invalid protocol params for PumpFun"))?;

        // 🔧 修复：显式检查 Option 以提高代码清晰度
        let input_amount =
            params.input_amount.ok_or_else(|| anyhow!("Input amount is required"))?;
        if input_amount == 0 {
            return Err(anyhow!("Amount cannot be zero"));
        }

        let bonding_curve = &protocol_params.bonding_curve;
        let creator_vault_pda = protocol_params.creator_vault;
        let creator = get_creator(&creator_vault_pda);

        // ========================================
        // Trade calculation and account address preparation
        // ========================================
        let buy_token_amount = match params.fixed_output_amount {
            Some(amount) => amount,
            None => get_buy_token_amount_from_sol_amount(
                bonding_curve.virtual_token_reserves as u128,
                bonding_curve.virtual_sol_reserves as u128,
                bonding_curve.real_token_reserves as u128,
                creator,
                input_amount,
            ),
        };

        let max_sol_cost = calculate_with_slippage_buy(
            input_amount,
            params.slippage_basis_points.unwrap_or(DEFAULT_SLIPPAGE),
        );

        let bonding_curve_addr = if bonding_curve.account == Pubkey::default() {
            get_bonding_curve_pda(&params.output_mint)
                .ok_or_else(|| anyhow!("Bonding curve PDA not found for mint"))?
        } else {
            bonding_curve.account
        };

        // Determine token program based on mayhem mode
        let is_mayhem_mode = bonding_curve.is_mayhem_mode;
        let token_program = protocol_params.token_program;
        let token_program_meta = if protocol_params.token_program == TOKEN_PROGRAM_2022 {
            crate::constants::TOKEN_PROGRAM_2022_META
        } else {
            crate::constants::TOKEN_PROGRAM_META
        };

        let associated_bonding_curve =
            if protocol_params.associated_bonding_curve == Pubkey::default() {
                crate::common::fast_fn::get_associated_token_address_with_program_id_fast(
                    &bonding_curve_addr,
                    &params.output_mint,
                    &token_program,
                )
            } else {
                protocol_params.associated_bonding_curve
            };

        let user_token_account =
            crate::common::fast_fn::get_associated_token_address_with_program_id_fast_use_seed(
                &params.payer.pubkey(),
                &params.output_mint,
                &token_program,
                params.open_seed_optimize,
            );

        let user_volume_accumulator = get_user_volume_accumulator_pda(&params.payer.pubkey())
            .ok_or_else(|| anyhow!("User volume accumulator PDA not found"))?;

        // ========================================
        // Build instructions
        // ========================================
        let mut instructions = Vec::with_capacity(2);

        // Create associated token account
        if params.create_output_mint_ata {
            instructions.extend(
                crate::common::fast_fn::create_associated_token_account_idempotent_fast_use_seed(
                    &params.payer.pubkey(),
                    &params.payer.pubkey(),
                    &params.output_mint,
                    &token_program,
                    params.open_seed_optimize,
                ),
            );
        }

        let mut buy_data = [0u8; 24];
        buy_data[..8].copy_from_slice(&[102, 6, 61, 18, 1, 218, 235, 234]); // Method ID
        buy_data[8..16].copy_from_slice(&buy_token_amount.to_le_bytes());
        buy_data[16..24].copy_from_slice(&max_sol_cost.to_le_bytes());

        // Determine fee recipient based on mayhem mode
        let fee_recipient_meta = if is_mayhem_mode {
            global_constants::MAYHEM_FEE_RECIPIENT_META
        } else {
            global_constants::FEE_RECIPIENT_META
        };

        let accounts: [AccountMeta; 16] = [
            global_constants::GLOBAL_ACCOUNT_META,
            fee_recipient_meta,
            AccountMeta::new_readonly(params.output_mint, false),
            AccountMeta::new(bonding_curve_addr, false),
            AccountMeta::new(associated_bonding_curve, false),
            AccountMeta::new(user_token_account, false),
            AccountMeta::new(params.payer.pubkey(), true),
            crate::constants::SYSTEM_PROGRAM_META,
            token_program_meta,
            AccountMeta::new(creator_vault_pda, false),
            accounts::EVENT_AUTHORITY_META,
            accounts::PUMPFUN_META,
            accounts::GLOBAL_VOLUME_ACCUMULATOR_META,
            AccountMeta::new(user_volume_accumulator, false),
            accounts::FEE_CONFIG_META,
            accounts::FEE_PROGRAM_META,
        ];

        instructions.push(Instruction::new_with_bytes(
            accounts::PUMPFUN,
            &buy_data,
            accounts.to_vec(),
        ));

        Ok(instructions)
    }

    async fn build_sell_instructions(&self, params: &SwapParams) -> Result<Vec<Instruction>> {
        // ========================================
        // Parameter validation and basic data preparation
        // ========================================
        let protocol_params = params
            .protocol_params
            .as_any()
            .downcast_ref::<PumpFunParams>()
            .ok_or_else(|| anyhow!("Invalid protocol params for PumpFun"))?;

        let token_amount = if let Some(amount) = params.input_amount {
            if amount == 0 {
                return Err(anyhow!("Amount cannot be zero"));
            }
            amount
        } else {
            return Err(anyhow!("Amount token is required"));
        };

        let bonding_curve = &protocol_params.bonding_curve;
        let creator_vault_pda = protocol_params.creator_vault;
        let creator = get_creator(&creator_vault_pda);

        // ========================================
        // Trade calculation and account address preparation
        // ========================================
        let sol_amount = get_sell_sol_amount_from_token_amount(
            bonding_curve.virtual_token_reserves as u128,
            bonding_curve.virtual_sol_reserves as u128,
            creator,
            token_amount,
        );

        let min_sol_output = match params.fixed_output_amount {
            Some(fixed) => fixed,
            None => calculate_with_slippage_sell(
                sol_amount,
                params.slippage_basis_points.unwrap_or(DEFAULT_SLIPPAGE),
            ),
        };

        let bonding_curve_addr = if bonding_curve.account == Pubkey::default() {
            get_bonding_curve_pda(&params.input_mint)
                .ok_or_else(|| anyhow!("Bonding curve PDA not found for mint"))?
        } else {
            bonding_curve.account
        };

        // Determine token program based on mayhem mode
        let is_mayhem_mode = bonding_curve.is_mayhem_mode;
        let token_program = protocol_params.token_program;
        let token_program_meta = if protocol_params.token_program == TOKEN_PROGRAM_2022 {
            crate::constants::TOKEN_PROGRAM_2022_META
        } else {
            crate::constants::TOKEN_PROGRAM_META
        };

        let associated_bonding_curve =
            if protocol_params.associated_bonding_curve == Pubkey::default() {
                crate::common::fast_fn::get_associated_token_address_with_program_id_fast(
                    &bonding_curve_addr,
                    &params.input_mint,
                    &token_program,
                )
            } else {
                protocol_params.associated_bonding_curve
            };

        let user_token_account =
            crate::common::fast_fn::get_associated_token_address_with_program_id_fast_use_seed(
                &params.payer.pubkey(),
                &params.input_mint,
                &token_program,
                params.open_seed_optimize,
            );

        // ========================================
        // Build instructions
        // ========================================
        let mut instructions = Vec::with_capacity(2);

        let mut sell_data = [0u8; 24];
        sell_data[..8].copy_from_slice(&[51, 230, 133, 164, 1, 127, 131, 173]); // Method ID
        sell_data[8..16].copy_from_slice(&token_amount.to_le_bytes());
        sell_data[16..24].copy_from_slice(&min_sol_output.to_le_bytes());

        // Determine fee recipient based on mayhem mode
        let fee_recipient_meta = if is_mayhem_mode {
            global_constants::MAYHEM_FEE_RECIPIENT_META
        } else {
            global_constants::FEE_RECIPIENT_META
        };

        let accounts: [AccountMeta; 14] = [
            global_constants::GLOBAL_ACCOUNT_META,
            fee_recipient_meta,
            AccountMeta::new_readonly(params.input_mint, false),
            AccountMeta::new(bonding_curve_addr, false),
            AccountMeta::new(associated_bonding_curve, false),
            AccountMeta::new(user_token_account, false),
            AccountMeta::new(params.payer.pubkey(), true),
            crate::constants::SYSTEM_PROGRAM_META,
            AccountMeta::new(creator_vault_pda, false),
            token_program_meta,
            accounts::EVENT_AUTHORITY_META,
            accounts::PUMPFUN_META,
            accounts::FEE_CONFIG_META,
            accounts::FEE_PROGRAM_META,
        ];

        instructions.push(Instruction::new_with_bytes(
            accounts::PUMPFUN,
            &sell_data,
            accounts.to_vec(),
        ));

        // Optional: Close token account
        if protocol_params.close_token_account_when_sell.unwrap_or(false)
            || params.close_input_mint_ata
        {
            instructions.push(close_account(
                &token_program,
                &user_token_account,
                &params.payer.pubkey(),
                &params.payer.pubkey(),
                &[&params.payer.pubkey()],
            )?);
        }

        Ok(instructions)
    }
}

/// Parameters for creating a new token on PumpFun
#[derive(Clone, Debug)]
pub struct CreateTokenParams {
    /// Mint keypair (must be a signer)
    pub mint: Arc<Keypair>,
    /// Token name
    pub name: String,
    /// Token symbol
    pub symbol: String,
    /// Metadata URI
    pub uri: String,
    /// Creator public key
    pub creator: Pubkey,
    /// Whether to use create_v2 (Token2022 + Mayhem support)
    pub use_v2: bool,
    /// Whether to enable Mayhem mode (only for create_v2)
    pub is_mayhem_mode: bool,
}

impl PumpFunInstructionBuilder {
    /// 构建创建代币指令（传统 Token 程序）
    ///
    /// 此函数用于创建使用传统 SPL Token 程序的代币，使用 Metaplex 存储元数据。
    ///
    /// # 参数
    /// * `params` - 创建代币的参数，包括 mint、name、symbol、uri、creator 等
    ///
    /// # 返回
    /// * `Ok(Instruction)` - 成功返回创建代币的指令
    /// * `Err` - 如果参数验证失败或 PDA 计算失败
    ///
    /// # 账户列表（按顺序）
    /// 1. mint - 代币 mint 账户（签名者，可写）
    /// 2. mint_authority - Mint 权限账户（只读）
    /// 3. bonding_curve - Bonding curve PDA（可写）
    /// 4. associated_bonding_curve - Bonding curve 的关联代币账户（可写）
    /// 5. global - PumpFun 全局配置账户（只读）
    /// 6. mpl_token_metadata - Metaplex Token Metadata 程序（只读）
    /// 7. metadata - Metaplex 元数据 PDA（可写）
    /// 8. user - 用户/创建者账户（签名者，可写）
    /// 9. system_program - 系统程序（只读）
    /// 10. token_program - SPL Token 程序（只读）
    /// 11. associated_token_program - 关联代币程序（只读）
    /// 12. rent - 租金系统账户（只读）
    /// 13. event_authority - 事件权限账户（只读）
    /// 14. program - PumpFun 程序（只读）
    pub fn build_create_instruction(params: &CreateTokenParams) -> Result<Instruction> {
        use crate::constants::{TOKEN_PROGRAM, TOKEN_PROGRAM_META};
        use crate::instruction::utils::pumpfun::{
            accounts, get_bonding_curve_pda, global_constants, seeds,
        };

        // 验证参数：如果 use_v2 为 true，应该使用 build_create_v2_instruction
        if params.use_v2 {
            return Err(anyhow!("Use build_create_v2_instruction for create_v2"));
        }

        // 计算 bonding curve PDA 地址
        // Seeds: ["bonding-curve", mint]
        let bonding_curve = get_bonding_curve_pda(&params.mint.pubkey())
            .ok_or_else(|| anyhow!("Failed to derive bonding curve PDA"))?;

        // 计算 bonding curve 的关联代币账户（ATA）地址
        // 这是 bonding curve PDA 持有的代币账户，用于存储实际代币
        // 使用传统 Token 程序（TOKEN_PROGRAM）
        let associated_bonding_curve =
            crate::common::fast_fn::get_associated_token_address_with_program_id_fast(
                &bonding_curve,
                &params.mint.pubkey(),
                &TOKEN_PROGRAM,
            );

        // 计算 Metaplex Token Metadata PDA 地址
        // Seeds: ["metadata", MPL_TOKEN_METADATA_PROGRAM_ID, mint]
        // 用于存储代币的元数据（名称、符号、图片等）
        let metadata = Pubkey::find_program_address(
            &[
                seeds::METADATA_SEED,                  // "metadata"
                accounts::MPL_TOKEN_METADATA.as_ref(), // Metaplex 程序 ID
                params.mint.pubkey().as_ref(),         // 代币 mint 地址
            ],
            &accounts::MPL_TOKEN_METADATA,
        )
        .0;

        // 构建指令数据
        // 指令标识符（discriminator）: [24, 30, 200, 40, 5, 28, 7, 119]
        let mut data = vec![24u8, 30, 200, 40, 5, 28, 7, 119];

        // 序列化代币名称（name）
        // 格式：4字节长度（小端序） + 名称字节
        let name_bytes = params.name.as_bytes();
        let name_len = name_bytes.len() as u32;
        data.extend_from_slice(&name_len.to_le_bytes());
        data.extend_from_slice(name_bytes);

        // 序列化代币符号（symbol）
        // 格式：4字节长度（小端序） + 符号字节
        let symbol_bytes = params.symbol.as_bytes();
        let symbol_len = symbol_bytes.len() as u32;
        data.extend_from_slice(&symbol_len.to_le_bytes());
        data.extend_from_slice(symbol_bytes);

        // 序列化元数据 URI（uri）
        // 格式：4字节长度（小端序） + URI 字节
        let uri_bytes = params.uri.as_bytes();
        let uri_len = uri_bytes.len() as u32;
        data.extend_from_slice(&uri_len.to_le_bytes());
        data.extend_from_slice(uri_bytes);

        // 序列化创建者地址（creator）
        // 格式：32字节公钥
        data.extend_from_slice(params.creator.as_ref());

        // 构建账户列表（按指令要求的顺序）
        // 账户顺序定义在 docs/pumpfun/idl/pump.json 的 "create" 指令中
        // 每个账户的作用和地址说明如下：
        let accounts = vec![
            // 0: mint - 代币 mint 账户（签名者，可写）
            // 作用：新创建的代币 mint 账户，用于标识和管理代币
            // 地址：由用户生成的 Keypair 的公钥
            AccountMeta::new(params.mint.pubkey(), true),
            // 1: mint_authority - Mint 权限账户（只读）
            // 作用：控制代币铸造权限的 PDA，所有 PumpFun 代币共享同一个 mint authority
            // 地址：TSLvdd1pWpHVjahSpsvCXUbgwsL3JAcvokwaKt1eokM
            // 说明：由 PumpFun 程序控制，确保只有通过程序才能创建和铸造代币
            AccountMeta::new_readonly(global_constants::MINT_AUTHORITY, false),
            // 2: bonding_curve - Bonding curve PDA（可写）
            // 作用：存储代币的虚拟和实际储备量，用于价格计算和交易
            // 地址：由 ["bonding-curve", mint] seeds 派生的 PDA
            AccountMeta::new(bonding_curve, false),
            // 3: associated_bonding_curve - Bonding curve 的关联代币账户（可写）
            // 类型：具体的账户地址（Associated Token Account，ATA）
            // 作用：bonding curve PDA 持有的代币账户，用于存储实际代币余额
            // 地址：由 bonding_curve、TOKEN_PROGRAM、mint 派生的 PDA
            // 计算方式：Pubkey::find_program_address(
            //   &[bonding_curve, TOKEN_PROGRAM_ID, mint],
            //   &ASSOCIATED_TOKEN_PROGRAM_ID
            // )
            // 说明：这是一个数据账户，存储代币余额，可以被写入和读取
            AccountMeta::new(associated_bonding_curve, false),
            // 4: global - PumpFun 全局配置账户（只读）
            // 作用：存储 PumpFun 协议的全局配置参数（初始储备量、手续费等）
            // 地址：4wTV1YmiEkRvAtNtsSGPtUrqRYQMe5SKy2uB4Jjaxnjf
            global_constants::GLOBAL_ACCOUNT_META,
            // 5: mpl_token_metadata - Metaplex Token Metadata 程序（只读）
            // 作用：Metaplex 的 Token Metadata 程序，用于创建和管理代币元数据
            // 地址：metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s
            // 功能：
            //   - 创建元数据账户（createV1/createMetadataAccountV3）：在链上创建并初始化元数据账户
            //   - 更新元数据（updateV1）：允许更新权限修改元数据（名称、符号、URI、创建者等）
            //   - 存储元数据：将元数据信息（name、symbol、uri、creators 等）存储在链上账户中
            //   - 权限管理：管理元数据的更新权限（update authority）
            // 说明：
            //   - 不仅仅是提供解析功能，而是一个完整的链上程序，可以创建和修改元数据
            //   - 提供标准化的代币元数据存储格式，使钱包和 DEX 能够正确显示代币信息
            //   - 元数据存储在链上的 PDA 账户中，可以通过程序指令进行创建和更新
            AccountMeta::new_readonly(accounts::MPL_TOKEN_METADATA, false),
            // 6: metadata - Metaplex 元数据 PDA（可写）
            // 作用：存储当前代币的元数据（name、symbol、uri、creator 等）
            // 地址：由 ["metadata", MPL_TOKEN_METADATA_PROGRAM_ID, mint] seeds 派生的 PDA
            AccountMeta::new(metadata, false),
            // 7: user - 用户/创建者账户（签名者，可写）
            // 作用：代币创建者的账户，用于支付交易费用和接收代币
            // 地址：params.creator
            AccountMeta::new(params.creator, true),
            // 8: system_program - 系统程序（只读）
            // 作用：Solana 核心系统程序，负责账户创建、SOL 转账等基础操作
            // 地址：11111111111111111111111111111111
            // 说明：所有账户创建和 SOL 转账都需要通过 System Program
            crate::constants::SYSTEM_PROGRAM_META,
            // 9: token_program - SPL Token 程序（只读）
            // 作用：SPL Token 标准程序，负责代币的创建、转账、铸造等操作
            // 地址：TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA
            // 说明：类似于以太坊的 ERC-20 标准，定义了代币的基本操作规范
            TOKEN_PROGRAM_META,
            // 10: associated_token_program - 关联代币程序（只读）
            // 类型：程序地址（Program ID）
            // 作用：自动创建和管理关联代币账户（ATA）的链上程序
            // 地址：ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL
            // 功能：
            //   - 提供 create_associated_token_account 指令：创建 ATA 账户
            //   - 提供 create_associated_token_account_idempotent 指令：幂等创建（如果已存在则跳过）
            //   - 自动计算 ATA 地址：根据 owner、mint、token_program 派生 PDA
            // 说明：
            //   - 这是一个可执行程序，不是数据账户
            //   - 用于创建和管理 associated_bonding_curve 这样的 ATA 账户
            //   - 简化代币账户管理，为每个钱包地址自动派生唯一的代币账户
            // 与 associated_bonding_curve 的区别：
            //   - associated_token_program：程序（可执行代码），用于创建账户
            //   - associated_bonding_curve：账户（数据存储），用于存储代币余额
            AccountMeta::new_readonly(crate::constants::ASSOCIATED_TOKEN_PROGRAM_ID, false),
            // 11: rent - 租金系统账户（只读）
            // 作用：Solana 的租金系统变量（Sysvar），提供当前租金费率信息
            // 地址：SysvarRent111111111111111111111111111111111
            // 说明：用于计算账户所需的最小余额以保持账户活跃（rent-exempt）
            crate::constants::RENT_META,
            // 12: event_authority - 事件权限账户（只读）
            // 作用：Native Events Program 的权限账户，用于事件日志的验证和管理
            // 地址：Ce6TQqeHC9p8KetsN6JsjHK7UTZk7nasjjnr7XxXp9F1
            // 说明：提供标准化的事件接口，确保日志的可靠性和可访问性
            accounts::EVENT_AUTHORITY_META,
            // 13: program - PumpFun 程序（只读）
            // 作用：PumpFun 程序本身，执行代币创建逻辑
            // 地址：6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P
            accounts::PUMPFUN_META,
        ];

        Ok(Instruction::new_with_bytes(accounts::PUMPFUN, &data, accounts))
    }

    /// 构建 create_v2 代币创建指令（Token2022 + Mayhem 模式支持）
    ///
    /// 此函数用于创建使用 Token2022 程序的代币，支持 Mayhem 模式。
    /// 与传统的 create 指令不同，create_v2 使用 Token2022 的内置元数据功能，
    /// 不需要单独的 Metaplex metadata 账户。
    ///
    /// # 参数
    /// * `params` - 创建代币的参数，包括 mint、name、symbol、uri、creator、is_mayhem_mode 等
    ///
    /// # 返回
    /// * `Ok(Instruction)` - 成功返回创建代币的指令
    /// * `Err` - 如果参数验证失败或 PDA 计算失败
    ///
    /// # 账户列表（按顺序）
    /// 1. mint - 代币 mint 账户（签名者，可写）
    /// 2. mint_authority - Mint 权限账户（只读）
    /// 3. bonding_curve - Bonding curve PDA（可写）
    /// 4. associated_bonding_curve - Bonding curve 的关联代币账户（Token2022，可写）
    /// 5. global - PumpFun 全局配置账户（只读）
    /// 6. user - 用户/创建者账户（签名者，可写）
    /// 7. system_program - 系统程序（只读）
    /// 8. token_program - Token2022 程序（只读）
    /// 9. associated_token_program - 关联代币程序（只读）
    /// 10. mayhem_program_id - Mayhem 程序 ID（可写，非签名者）
    /// 11. global_params - Mayhem 全局参数 PDA（只读）
    /// 12. sol_vault - Mayhem SOL 金库 PDA（可写，非签名者）
    /// 13. mayhem_state - Mayhem 状态 PDA（可写）
    /// 14. mayhem_token_vault - Mayhem 代币金库 ATA（可写）
    /// 15. event_authority - 事件权限账户（只读）
    /// 16. program - PumpFun 程序（只读）
    pub fn build_create_v2_instruction(params: &CreateTokenParams) -> Result<Instruction> {
        use crate::constants::{TOKEN_PROGRAM_2022, TOKEN_PROGRAM_2022_META};
        use crate::instruction::utils::pumpfun::{
            accounts, get_bonding_curve_pda, global_constants,
        };

        // 验证参数：如果 use_v2 为 false，应该使用 build_create_instruction
        if !params.use_v2 {
            return Err(anyhow!("Use build_create_instruction for create"));
        }

        // 计算 bonding curve PDA 地址
        // Seeds: ["bonding-curve", mint]
        let bonding_curve = get_bonding_curve_pda(&params.mint.pubkey())
            .ok_or_else(|| anyhow!("Failed to derive bonding curve PDA"))?;

        // 计算 bonding curve 的关联代币账户（ATA）地址
        // 这是 bonding curve PDA 持有的代币账户，用于存储实际代币
        // 使用 Token2022 程序（TOKEN_PROGRAM_2022）
        let associated_bonding_curve =
            crate::common::fast_fn::get_associated_token_address_with_program_id_fast(
                &bonding_curve,
                &params.mint.pubkey(),
                &TOKEN_PROGRAM_2022,
            );

        // 计算 Mayhem 程序相关的 PDA 地址
        // Mayhem 是 PumpFun 的扩展功能，用于支持 Token2022 的高级特性

        // Mayhem 全局参数 PDA
        // Seeds: ["global-params"]
        let mayhem_global_params =
            Pubkey::find_program_address(&[b"global-params"], &global_constants::MAYHEM_PROGRAM_ID)
                .0;

        // Mayhem SOL 金库 PDA
        // Seeds: ["sol-vault"]
        let mayhem_sol_vault =
            Pubkey::find_program_address(&[b"sol-vault"], &global_constants::MAYHEM_PROGRAM_ID).0;

        // Mayhem 状态 PDA（每个代币一个）
        // Seeds: ["mayhem-state", mint]
        let mayhem_state = Pubkey::find_program_address(
            &[b"mayhem-state", params.mint.pubkey().as_ref()],
            &global_constants::MAYHEM_PROGRAM_ID,
        )
        .0;

        // Mayhem 代币金库的关联代币账户
        // 这是 mayhem_sol_vault 持有的代币账户
        let mayhem_token_vault =
            crate::common::fast_fn::get_associated_token_address_with_program_id_fast(
                &mayhem_sol_vault,
                &params.mint.pubkey(),
                &TOKEN_PROGRAM_2022,
            );

        // 构建指令数据
        // 指令标识符（discriminator）: [214, 144, 76, 236, 95, 139, 49, 180]
        let mut data = vec![214u8, 144, 76, 236, 95, 139, 49, 180];

        // 序列化代币名称（name）
        // 格式：4字节长度（小端序） + 名称字节
        let name_bytes = params.name.as_bytes();
        let name_len = name_bytes.len() as u32;
        data.extend_from_slice(&name_len.to_le_bytes());
        data.extend_from_slice(name_bytes);

        // 序列化代币符号（symbol）
        // 格式：4字节长度（小端序） + 符号字节
        let symbol_bytes = params.symbol.as_bytes();
        let symbol_len = symbol_bytes.len() as u32;
        data.extend_from_slice(&symbol_len.to_le_bytes());
        data.extend_from_slice(symbol_bytes);

        // 序列化元数据 URI（uri）
        // 格式：4字节长度（小端序） + URI 字节
        let uri_bytes = params.uri.as_bytes();
        let uri_len = uri_bytes.len() as u32;
        data.extend_from_slice(&uri_len.to_le_bytes());
        data.extend_from_slice(uri_bytes);

        // 序列化创建者地址（creator）
        // 格式：32字节公钥
        data.extend_from_slice(params.creator.as_ref());

        // 添加 Mayhem 模式标志（1字节）
        // 0 = false（不启用 Mayhem 模式）
        // 1 = true（启用 Mayhem 模式）
        data.push(if params.is_mayhem_mode { 1 } else { 0 });

        // 构建账户列表（按指令要求的顺序）
        let accounts = vec![
            AccountMeta::new(params.mint.pubkey(), true), // 0: mint - 代币 mint 账户（签名者，可写）
            AccountMeta::new_readonly(global_constants::MINT_AUTHORITY, false), // 1: mint_authority - Mint 权限账户（只读）
            AccountMeta::new(bonding_curve, false), // 2: bonding_curve - Bonding curve PDA（可写）
            AccountMeta::new(associated_bonding_curve, false), // 3: associated_bonding_curve - Bonding curve 的关联代币账户（Token2022，可写）
            global_constants::GLOBAL_ACCOUNT_META, // 4: global - PumpFun 全局配置账户（只读）
            AccountMeta::new(params.creator, true), // 5: user - 用户/创建者账户（签名者，可写）
            crate::constants::SYSTEM_PROGRAM_META, // 6: system_program - 系统程序（只读）
            TOKEN_PROGRAM_2022_META,               // 7: token_program - Token2022 程序（只读）
            AccountMeta::new_readonly(crate::constants::ASSOCIATED_TOKEN_PROGRAM_ID, false), // 8: associated_token_program - 关联代币程序（只读）
            AccountMeta::new(global_constants::MAYHEM_PROGRAM_ID, false), // 9: mayhem_program_id - Mayhem 程序 ID（可写，非签名者）
            AccountMeta::new_readonly(mayhem_global_params, false), // 10: global_params - Mayhem 全局参数 PDA（只读）
            AccountMeta::new(mayhem_sol_vault, false), // 11: sol_vault - Mayhem SOL 金库 PDA（可写，非签名者）
            AccountMeta::new(mayhem_state, false),     // 12: mayhem_state - Mayhem 状态 PDA（可写）
            AccountMeta::new(mayhem_token_vault, false), // 13: mayhem_token_vault - Mayhem 代币金库 ATA（可写）
            accounts::EVENT_AUTHORITY_META, // 14: event_authority - 事件权限账户（只读）
            accounts::PUMPFUN_META,         // 15: program - PumpFun 程序（只读）
        ];

        Ok(Instruction::new_with_bytes(accounts::PUMPFUN, &data, accounts))
    }
}

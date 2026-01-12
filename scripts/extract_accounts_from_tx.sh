#!/usr/bin/env bash
# 从 Solana 交易 HTML/签名中提取所有账户地址
# 用法: ./extract_accounts_from_tx.sh <html_file_or_signature>

set -e

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

INPUT="${1:-}"

# 默认目录
DEFAULT_DIR="$(dirname "$0")/txs"

if [ -z "$INPUT" ]; then
    # 没有参数，使用默认目录
    if [ -d "$DEFAULT_DIR" ]; then
        INPUT="$DEFAULT_DIR"
        echo -e "${BLUE}💡 使用默认目录: $INPUT${NC}"
        echo ""
    else
        echo -e "${YELLOW}用法:${NC}"
        echo "  $0                          # 默认从 scripts/txs/ 读取"
        echo "  $0 <html_file>              # 从单个 HTML 文件提取"
        echo "  $0 <directory>              # 从指定目录批量提取"
        echo "  $0 <transaction_signature>   # 从 Solana Explorer 获取 (待实现)"
        exit 1
    fi
fi

echo -e "${GREEN}================================${NC}"
echo -e "${GREEN}提取 Solana 交易账户${NC}"
echo -e "${GREEN}================================${NC}"
echo ""

# 提取所有 Solana 地址 (base58, 32-44 字符)
if [ -f "$INPUT" ]; then
    # 单个文件
    echo -e "${BLUE}📄 从单个文件提取: $INPUT${NC}"
    ADDRESSES=$(grep -oE '[A-HJ-NP-Za-km-z1-9]{32,44}' "$INPUT" | sort -u)
elif [ -d "$INPUT" ]; then
    # 目录 - 批量处理所有 HTML
    echo -e "${BLUE}📂 从目录批量提取: $INPUT${NC}"
    HTML_FILES=$(find "$INPUT" -name "*.html" -o -name "*.htm")
    
    if [ -z "$HTML_FILES" ]; then
        echo -e "${YELLOW}⚠️  目录中没有找到 HTML 文件${NC}"
        exit 1
    fi
    
    HTML_COUNT=$(echo "$HTML_FILES" | wc -l | tr -d ' ')
    echo -e "${GREEN}找到 $HTML_COUNT 个 HTML 文件${NC}"
    echo ""
    
    # 合并所有 HTML 的地址
    ADDRESSES=""
    while IFS= read -r html_file; do
        echo -e "  ${YELLOW}→${NC} $(basename "$html_file")"
        file_addresses=$(grep -oE '[A-HJ-NP-Za-km-z1-9]{32,44}' "$html_file" 2>/dev/null || true)
        ADDRESSES="$ADDRESSES
$file_addresses"
    done <<< "$HTML_FILES"
    
    # 去重
    ADDRESSES=$(echo "$ADDRESSES" | sort -u)
    echo ""
    echo -e "${GREEN}✓ 已合并所有文件的地址${NC}"
    echo ""
else
    echo -e "${BLUE}从交易签名提取: $INPUT${NC}"
    echo -e "${YELLOW}功能待实现：可使用 Solana CLI 获取${NC}"
    echo "  solana transaction $INPUT"
    exit 1
fi

# 过滤：只保留有效的 Solana 地址（排除纯数字、hex 等）
VALID_ADDRESSES=$(echo "$ADDRESSES" | while read -r addr; do
    # 检查是否是有效 base58 (至少包含一些字母)
    if echo "$addr" | grep -qE '[A-HJ-NP-Za-km-z]'; then
        # 排除明显的 hex 字符串
        if ! echo "$addr" | grep -qE '^[0-9a-f]+$'; then
            echo "$addr"
        fi
    fi
done | sort -u)

# 已知地址分类
get_address_type() {
    case "$1" in
        "11111111111111111111111111111111") echo "System Program" ;;
        "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA") echo "Token Program" ;;
        "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb") echo "Token-2022 Program" ;;
        "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL") echo "Associated Token Program" ;;
        "ComputeBudget111111111111111111111111111111") echo "Compute Budget Program" ;;
        "Sysvar"*) echo "Sysvar Account" ;;
        "So11111111111111111111111111111111111111112") echo "WSOL Mint" ;;
        "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK") echo "⭐ Raydium CLMM Program" ;;
        "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4") echo "⭐ Jupiter V6" ;;
        "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB") echo "Jupiter V4" ;;
        "9H6tua7jkLhdm3w8BvgpTn5LZNU7g4ZynDmCiNN3q6Rp") echo "Unknown DEX" ;;
        "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA") echo "⭐ PumpFun AMM" ;;
        "pfeeUxB6jkeY1Hxd7CsFCAjcbHA9rWtchMGdZ6VojVZ") echo "PumpFun Fee" ;;
        "EZVkeboWeXygtq8LMyENHyXdF5wpYrtExRNH9UwB1qYw") echo "🔥 CLMM Pool (WSOL-JUP)" ;;
        *"pump") echo "🎯 Potential Pump Token" ;;
        *) 
            # 长度判断
            len=${#1}
            if [ $len -eq 44 ]; then
                echo "Pool/Account"
            elif [ $len -eq 43 ] || [ $len -eq 44 ]; then
                echo "Token/PDA"
            else
                echo "Unknown"
            fi
            ;;
    esac
}

# 分类显示
echo -e "${BLUE}找到的地址:${NC}"
echo ""

PROGRAMS=()
POOLS=()
TOKENS=()
OTHERS=()

while IFS= read -r addr; do
    type=$(get_address_type "$addr")
    
    case "$type" in
        *"Program"*|*"Sysvar"*)
            PROGRAMS+=("$addr  # $type")
            ;;
        *"Pool"*|*"CLMM"*|*"AMM"*)
            POOLS+=("$addr  # $type")
            ;;
        *"Mint"*|*"Token"*|*"Pump"*)
            TOKENS+=("$addr  # $type")
            ;;
        *)
            OTHERS+=("$addr  # $type")
            ;;
    esac
done <<< "$VALID_ADDRESSES"

echo -e "${GREEN}═══ 程序地址 (${#PROGRAMS[@]}) ═══${NC}"
for item in "${PROGRAMS[@]}"; do
    echo "  $item"
done
echo ""

echo -e "${GREEN}═══ Pool 地址 (${#POOLS[@]}) ═══${NC}"
for item in "${POOLS[@]}"; do
    echo "  $item"
done
echo ""

echo -e "${GREEN}═══ Token 地址 (${#TOKENS[@]}) ═══${NC}"
for item in "${TOKENS[@]}"; do
    echo "  $item"
done
echo ""

echo -e "${YELLOW}═══ 其他地址 (${#OTHERS[@]}) ═══${NC}"
for item in "${OTHERS[@]}"; do
    echo "  $item"
done
echo ""

# 生成启动脚本
OUTPUT_SCRIPT="/tmp/start-validator-accounts.sh"
cat > "$OUTPUT_SCRIPT" << 'SCRIPT_HEADER'
#!/bin/bash
# 自动生成的 solana-test-validator 启动脚本
# 包含程序和账户地址

set -e

echo "🚀 启动 solana-test-validator..."
echo ""

solana-test-validator \
SCRIPT_HEADER

# 添加所有地址
echo "$VALID_ADDRESSES" | while read -r addr; do
    type=$(get_address_type "$addr")
    echo "  --clone $addr \\  # $type" >> "$OUTPUT_SCRIPT"
done

cat >> "$OUTPUT_SCRIPT" << 'SCRIPT_FOOTER'
  --reset \
  --quiet \
  --ledger /tmp/test-ledger

echo ""
echo "✅ Validator 启动成功！"
echo "   RPC: http://localhost:8899"
SCRIPT_FOOTER

chmod +x "$OUTPUT_SCRIPT"

echo -e "${GREEN}================================${NC}"
echo -e "${GREEN}✅ 完成！${NC}"
echo -e "${GREEN}================================${NC}"
echo ""
echo -e "${GREEN}生成的启动脚本: $OUTPUT_SCRIPT${NC}"
echo -e "${BLUE}执行: $OUTPUT_SCRIPT${NC}"
echo ""
echo -e "${GREEN}总计提取:${NC}"
echo "  - 程序: ${#PROGRAMS[@]}"
echo "  - Pools: ${#POOLS[@]}"
echo "  - Tokens: ${#TOKENS[@]}"
echo "  - 其他: ${#OTHERS[@]}"
echo "  - 总计: $(echo "$VALID_ADDRESSES" | wc -l) 个地址"
echo ""

# 保存地址列表到 JSON
SCRIPT_DIR="$(dirname "$0")"
ACCOUNTS_JSON="$SCRIPT_DIR/accounts.json"

# 辅助函数：生成 JSON 数组
generate_json_array() {
    local -n items=$1
    local first=true
    
    for item in "${items[@]}"; do
        addr=$(echo "$item" | awk '{print $1}')
        type=$(echo "$item" | sed 's/^[^ ]* *# *//')
        
        if [ "$first" = true ]; then
            echo "    {\"address\": \"$addr\", \"type\": \"$type\"}"
            first=false
        else
            echo "    {\"address\": \"$addr\", \"type\": \"$type\"}"
        fi
    done
}

# 构建 JSON
echo "{" > "$ACCOUNTS_JSON"

# Programs
echo '  "programs": [' >> "$ACCOUNTS_JSON"
if [ ${#PROGRAMS[@]} -gt 0 ]; then
    for i in "${!PROGRAMS[@]}"; do
        addr=$(echo "${PROGRAMS[$i]}" | awk '{print $1}')
        type=$(echo "${PROGRAMS[$i]}" | sed 's/^[^ ]* *# *//')
        if [ $i -eq $((${#PROGRAMS[@]} - 1)) ]; then
            echo "    {\"address\": \"$addr\", \"type\": \"$type\"}" >> "$ACCOUNTS_JSON"
        else
            echo "    {\"address\": \"$addr\", \"type\": \"$type\"}," >> "$ACCOUNTS_JSON"
        fi
    done
fi
echo '  ],' >> "$ACCOUNTS_JSON"

# Pools
echo '  "pools": [' >> "$ACCOUNTS_JSON"
if [ ${#POOLS[@]} -gt 0 ]; then
    for i in "${!POOLS[@]}"; do
        addr=$(echo "${POOLS[$i]}" | awk '{print $1}')
        type=$(echo "${POOLS[$i]}" | sed 's/^[^ ]* *# *//')
        if [ $i -eq $((${#POOLS[@]} - 1)) ]; then
            echo "    {\"address\": \"$addr\", \"type\": \"$type\"}" >> "$ACCOUNTS_JSON"
        else
            echo "    {\"address\": \"$addr\", \"type\": \"$type\"}," >> "$ACCOUNTS_JSON"
        fi
    done
fi
echo '  ],' >> "$ACCOUNTS_JSON"

# Tokens
echo '  "tokens": [' >> "$ACCOUNTS_JSON"
if [ ${#TOKENS[@]} -gt 0 ]; then
    for i in "${!TOKENS[@]}"; do
        addr=$(echo "${TOKENS[$i]}" | awk '{print $1}')
        type=$(echo "${TOKENS[$i]}" | sed 's/^[^ ]* *# *//')
        if [ $i -eq $((${#TOKENS[@]} - 1)) ]; then
            echo "    {\"address\": \"$addr\", \"type\": \"$type\"}" >> "$ACCOUNTS_JSON"
        else
            echo "    {\"address\": \"$addr\", \"type\": \"$type\"}," >> "$ACCOUNTS_JSON"
        fi
    done
fi
echo '  ],' >> "$ACCOUNTS_JSON"

# Others
echo '  "others": [' >> "$ACCOUNTS_JSON"
if [ ${#OTHERS[@]} -gt 0 ]; then
    for i in "${!OTHERS[@]}"; do
        addr=$(echo "${OTHERS[$i]}" | awk '{print $1}')
        type=$(echo "${OTHERS[$i]}" | sed 's/^[^ ]* *# *//')
        if [ $i -eq $((${#OTHERS[@]} - 1)) ]; then
            echo "    {\"address\": \"$addr\", \"type\": \"$type\"}" >> "$ACCOUNTS_JSON"
        else
            echo "    {\"address\": \"$addr\", \"type\": \"$type\"}," >> "$ACCOUNTS_JSON"
        fi
    done
fi
echo '  ]' >> "$ACCOUNTS_JSON"

echo "}" >> "$ACCOUNTS_JSON"

echo -e "${GREEN}账户已保存到: $ACCOUNTS_JSON${NC}"

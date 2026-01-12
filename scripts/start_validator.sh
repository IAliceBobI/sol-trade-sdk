#!/usr/bin/env bash
# 从 accounts.json 读取账户并启动 solana-test-validator
# 用法: ./start_validator.sh [--url RPC_URL] [--core-only]

set -e

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

SCRIPT_DIR="$(dirname "$0")"
ACCOUNTS_JSON="$SCRIPT_DIR/accounts.json"

# 解析参数
RPC_URL="https://api.mainnet-beta.solana.com"
CORE_ONLY=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --url)
            RPC_URL="$2"
            shift 2
            ;;
        --core-only)
            CORE_ONLY=true
            shift
            ;;
        -h|--help)
            echo "用法: $0 [OPTIONS]"
            echo ""
            echo "选项:"
            echo "  --url RPC_URL      指定 Solana RPC URL (默认: mainnet-beta)"
            echo "  --core-only        只 clone 核心程序，跳过 Pool 和 Token"
            echo "  -h, --help         显示帮助信息"
            exit 0
            ;;
        *)
            echo "未知参数: $1"
            exit 1
            ;;
    esac
done

echo -e "${GREEN}================================${NC}"
echo -e "${GREEN}启动 Solana Test Validator${NC}"
echo -e "${GREEN}================================${NC}"
echo ""

# 检查 JSON 文件是否存在
if [ ! -f "$ACCOUNTS_JSON" ]; then
    echo -e "${YELLOW}❌ 找不到 accounts.json${NC}"
    echo -e "${YELLOW}请先运行: ./extract_accounts_from_tx.sh${NC}"
    exit 1
fi

# 检查 jq 是否安装
if ! command -v jq &> /dev/null; then
    echo -e "${YELLOW}⚠️  未安装 jq，使用备用方案提取地址...${NC}"
    # 备用方案：使用 grep 提取所有地址
    if [ "$CORE_ONLY" = true ]; then
        ADDRESSES=$(grep -A 999 '"programs":' "$ACCOUNTS_JSON" | grep -B 999 '],"pools":' | grep -oE '"address": "[A-HJ-NP-Za-km-z1-9]{32,44}"' | cut -d'"' -f4)
    else
        ADDRESSES=$(grep -oE '"address": "[A-HJ-NP-Za-km-z1-9]{32,44}"' "$ACCOUNTS_JSON" | cut -d'"' -f4)
    fi
else
    # 使用 jq 提取地址
    if [ "$CORE_ONLY" = true ]; then
        echo -e "${BLUE}🔑 仅 clone 核心程序...${NC}"
        ADDRESSES=$(jq -r '.programs[].address' "$ACCOUNTS_JSON" 2>/dev/null)
    else
        ADDRESSES=$(jq -r '.programs[].address, .pools[].address, .tokens[].address, .others[].address' "$ACCOUNTS_JSON" 2>/dev/null)
    fi
fi

if [ -z "$ADDRESSES" ]; then
    echo -e "${YELLOW}❌ accounts.json 中没有找到账户${NC}"
    exit 1
fi

ACCOUNT_COUNT=$(echo "$ADDRESSES" | wc -l | tr -d ' ')
echo -e "${BLUE}📋 从 accounts.json 加载了 $ACCOUNT_COUNT 个账户${NC}"
echo ""

# 过滤有效地址（只保留核心程序和确认有效的地址）
VALID_ADDRESSES=""
INVALID_COUNT=0

# 必须包含的核心程序
CORE_PROGRAMS="
TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA
TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb
ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL
"

# 不能 clone 的地址（Sysvar 和 ComputeBudget）
EXCLUDE_ADDRESSES="
Sysvar1nstructions1111111111111111111111111
SysvarC1ock11111111111111111111111111111111
ComputeBudget111111111111111111111111111111
"

while IFS= read -r addr; do
    if [ -n "$addr" ]; then
        # 检查是否在排除列表中
        if echo "$EXCLUDE_ADDRESSES" | grep -q "^$addr$"; then
            ((INVALID_COUNT++))
            continue
        fi
        
        # 检查地址长度
        len=${#addr}
        if [ $len -ge 32 ] && [ $len -le 44 ]; then
            # 检查是否是 base58 有效字符（排除 0, O, I, l）
            if echo "$addr" | grep -qE '^[A-HJ-NP-Za-km-z1-9]+$'; then
                # 排除全是相同字符的地址
                if ! echo "$addr" | grep -qE '^(.)\1+$'; then
                    # 排除包含过10个以上连续相同字符的地址
                    if ! echo "$addr" | grep -qE '(.)\1{9,}'; then
                        VALID_ADDRESSES="$VALID_ADDRESSES$addr
"
                    else
                        ((INVALID_COUNT++))
                    fi
                else
                    ((INVALID_COUNT++))
                fi
            else
                ((INVALID_COUNT++))
            fi
        else
            ((INVALID_COUNT++))
        fi
    fi
done <<< "$ADDRESSES"

VALID_COUNT=$(echo "$VALID_ADDRESSES" | grep -v '^$' | wc -l | tr -d ' ')

if [ $INVALID_COUNT -gt 0 ]; then
    echo -e "${YELLOW}⚠️  过滤了 $INVALID_COUNT 个无效地址${NC}"
fi
echo -e "${GREEN}✓ 有效账户: $VALID_COUNT 个${NC}"
echo ""

# 构建 solana-test-validator 命令
CMD="solana-test-validator"

# 添加所有 --clone 参数
while IFS= read -r addr; do
    if [ -n "$addr" ]; then
        CMD="$CMD --clone $addr"
    fi
done <<< "$VALID_ADDRESSES"

# 添加其他参数
CMD="$CMD --url $RPC_URL --reset --quiet --ledger /tmp/test-ledger"

echo -e "${GREEN}🚀 启动命令:${NC}"
echo -e "${BLUE}solana-test-validator \\${NC}"
echo -e "${BLUE}  --url $RPC_URL \\${NC}"
echo -e "${BLUE}  --clone <$VALID_COUNT 个地址> \\${NC}"
echo -e "${BLUE}  --reset --quiet --ledger /tmp/test-ledger${NC}"
echo ""
echo -e "${YELLOW}提示: 按 Ctrl+C 停止 validator${NC}"
echo ""
echo -e "${YELLOW}如果 clone 失败，请尝试:${NC}"
echo -e "${YELLOW}  1. 使用 --core-only 只 clone 核心程序${NC}"
echo -e "${YELLOW}  2. 指定更快的 RPC: --url https://your-rpc-url${NC}"
echo ""

# 执行命令
exec $CMD

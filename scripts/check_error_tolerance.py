#!/usr/bin/env python3
"""
Rust 错误容忍和掩盖问题检查脚本

检查 Rust 代码中可能导致生产事故的错误处理模式，包括：
- unwrap() 的过度使用
- 数据库操作静默失败
- 不当的 unwrap_or_default() 和 unwrap_or()
- let _ = 忽略重要返回值
- assert! 在生产代码中
- expect() 缺少有用的错误信息
- panic! 使用不当
- ok() 静默忽略错误
- parse().unwrap() 模式
- 未经检查的数组/Vec 访问

Usage:
    python3 check_error_tolerance.py [path]

Examples:
    python3 check_error_tolerance.py              # 检查当前目录
    python3 check_error_tolerance.py src/         # 检查 src/ 目录
    python3 check_error_tolerance.py ../my-project # 检查指定项目
"""

import os
import re
import sys
from pathlib import Path
from typing import List, Tuple, Dict
from dataclasses import dataclass
from enum import Enum


class Severity(Enum):
    HIGH = "🔴 高严重度"
    MEDIUM = "🟡 中严重度"
    LOW = "🟢 低严重度"


@dataclass
class Issue:
    """代码问题"""
    file_path: str
    line_number: int
    line_content: str
    severity: Severity
    category: str
    risk: str
    suggestion: str
    example: str = ""


# 检查模式配置
CHECK_PATTERNS = {
    # 高严重度问题
    "unwrap()": {
        "pattern": r"\bunwrap\(\)(?!\s*//.*测试|test)",
        "severity": Severity.HIGH,
        "category": "unwrap() 的过度使用",
        "risk": "生产环境中直接 panic，无法优雅降级",
        "suggestion": "使用 ? 操作符传播错误，或使用 map_err 添加错误上下文",
        "example": """// ❌ 危险
let user_id = get_user_id().unwrap();

// ✅ 更好
let user_id = get_user_id()
    .map_err(|e| Error::UserIdNotFound(e))?;"""
    },

    "unwrap_or_default": {
        "pattern": r"\.unwrap_or_default\(\)",
        "severity": Severity.HIGH,
        "category": "不当的 unwrap_or_default()",
        "risk": "可能掩盖真实错误，导致业务逻辑错误",
        "suggestion": "金额、余额、状态等字段必须显式处理错误",
        "example": """// ❌ 余额查询失败返回 0
let balance = query_balance(user_id).unwrap_or_default();

// ✅ 明确处理错误
let balance = query_balance(user_id)
    .map_err(|e| {
        log::error!("Failed to query balance for {:?}: {:?}", user_id, e);
        Error::BalanceQueryFailed
    })?;"""
    },

    "unwrap_or": {
        "pattern": r"\.unwrap_or\([^)]+\)",
        "severity": Severity.HIGH,
        "category": "不当的 unwrap_or()",
        "risk": "网络错误、配置错误被掩盖为默认值",
        "suggestion": "使用 Result 传播错误，或在启动时明确失败",
        "example": """// ❌ 网络错误被掩盖
let price = fetch_price().unwrap_or(old_price);

// ✅ 启动时明确失败
let price = fetch_price().await
    .map_err(|e| Error::PriceFetchFailed { context: e })?;"""
    },

    "let _ =": {
        "pattern": r"let\s+_\s*=\s*[a-z_]+\(.*\)[;$]",
        "severity": Severity.HIGH,
        "category": "let _ = 忽略 must_use 值",
        "risk": "忽略重要返回值，导致资源泄漏或逻辑错误",
        "suggestion": "显式处理返回值，或使用 semicolon 表示有意识丢弃",
        "example": """// ❌ 忽略事务提交结果
let _ = tx.commit();

// ✅ 显式处理
tx.commit()?;"""
    },

    "assert!": {
        "pattern": r"assert!\([^,)]+(,[^)]+)?\)",
        "severity": Severity.HIGH,
        "category": "assert! 在生产代码中",
        "risk": "release 模式下被优化掉，debug 模式才 panic",
        "suggestion": "使用 if 语句检查并返回错误，或使用 debug_assert!",
        "example": """// ❌ release 模式下不检查
assert!(amount > 0, "Amount must be positive");

// ✅ 运行时始终检查
if amount <= 0 {
    return Err(Error::InvalidAmount { amount });
}"""
    },

    # 中严重度问题
    "expect_short": {
        "pattern": r'\.expect\("[^"]{0,20}"\)',
        "severity": Severity.MEDIUM,
        "category": "expect() 缺少有用的错误信息",
        "risk": "panic 时缺少调试上下文，难以定位问题",
        "suggestion": "包含足够的上下文（地址、ID、参数等）",
        "example": """// ❌ 信息不足
let config = load_config().expect("failed");

// ✅ 包含上下文
let config = load_config().expect(
    "Failed to load config from CONFIG_PATH env var"
);"""
    },

    "panic_short": {
        "pattern": r'panic!\("[^"]{0,30}"\)',
        "severity": Severity.MEDIUM,
        "category": "panic! 使用不当",
        "risk": "panic 信息不完整，调试困难",
        "suggestion": "包含请求参数、时间戳、地址等调试信息",
        "example": """// ❌ 缺少上下文
panic!("Invalid state");

// ✅ 包含调试信息
panic!(
    "Invalid state: expected Active, got {:?} for order {}",
    state, order_id
);"""
    },

    "ok()": {
        "pattern": r"\.ok\(\)\s*;",
        "severity": Severity.MEDIUM,
        "category": "ok() 静默忽略错误",
        "risk": "错误被悄无声息地忽略，可能导致后续问题",
        "suggestion": "至少记录日志，或使用 inspect_err",
        "example": """// ❌ 错误被吞掉
let result = some_operation().ok();

// ✅ 至少记录日志
if let Err(e) = some_operation() {
    log::warn!("Operation failed: {:?}", e);
}"""
    },

    "parse().unwrap": {
        "pattern": r"\.parse\(\)\.unwrap\(\)",
        "severity": Severity.MEDIUM,
        "category": "parse().unwrap() 模式",
        "risk": "字符串解析失败导致 panic",
        "suggestion": "优雅处理解析错误，提供清晰的错误消息",
        "example": """// ❌ 解析失败 panic
let port: u16 = env::var("PORT").unwrap().parse().unwrap();

// ✅ 优雅处理错误
let port: u16 = env::var("PORT")
    .map_err(|e| Error::ConfigMissing("PORT".into()))?
    .parse()
    .map_err(|e| Error::ConfigInvalid {
        key: "PORT",
        value: env::var("PORT").unwrap_or_default(),
        source: e,
    })?;"""
    },

    "direct_index": {
        "pattern": r"[a-z_]+\[[0-9]+\](?!\s*=)",
        "severity": Severity.MEDIUM,
        "category": "未经检查的数组/Vec 访问",
        "risk": "越界访问导致 panic",
        "suggestion": "使用 .get()、.first()、.last() 等安全方法",
        "example": """// ❌ 可能 panic
let item = items[0];

// ✅ 安全访问
let item = items.get(0).ok_or(Error::EmptyList)?;

// ✅ 或使用迭代器
let item = items.first().ok_or(Error::EmptyList)?;"""
    },

    # 低严重度问题
    "todo!": {
        "pattern": r"(todo|unimplemented)!\(",
        "severity": Severity.LOW,
        "category": "todo!() 和 unimplemented!() 在生产代码",
        "risk": "功能未完成，执行到时会 panic",
        "suggestion": "返回明确的错误，或添加 #[cfg(test)] 条件",
        "example": """// ❌ 生产代码中未完成
fn complex_feature(input: Input) -> Output {
    todo!()
}

// ✅ 返回明确的错误
fn complex_feature(input: Input) -> Result<Output, Error> {
    Err(Error::NotImplemented {
        feature: "complex_feature".into()
    })
}"""
    },
}


def should_skip_file(file_path: Path) -> bool:
    """判断文件是否应该跳过"""
    # 跳过测试文件（可选，根据需求调整）
    # if "test" in file_path.parts or file_path.name.startswith("test_"):
    #     return True

    # 跳过 target 目录
    if "target" in file_path.parts:
        return True

    return False


def check_rust_file(file_path: Path) -> List[Issue]:
    """检查单个 Rust 文件"""
    issues = []

    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            lines = f.readlines()
    except Exception as e:
        print(f"⚠️  无法读取文件 {file_path}: {e}", file=sys.stderr)
        return issues

    for line_num, line in enumerate(lines, 1):
        line_stripped = line.strip()

        # 跳过注释行
        if line_stripped.startswith("//"):
            continue

        # 跳过测试代码中的某些检查（可选）
        if "#[test]" in line or "#test" in line:
            # 在测试模式下，可以放宽某些检查
            continue

        # 检查每个模式
        for pattern_name, config in CHECK_PATTERNS.items():
            pattern = config["pattern"]
            if re.search(pattern, line):
                issues.append(Issue(
                    file_path=str(file_path),
                    line_number=line_num,
                    line_content=line_stripped,
                    severity=config["severity"],
                    category=config["category"],
                    risk=config["risk"],
                    suggestion=config["suggestion"],
                    example=config.get("example", "")
                ))

    return issues


def find_rust_files(root_path: Path) -> List[Path]:
    """查找所有 .rs 文件"""
    rust_files = []

    for file_path in root_path.rglob("*.rs"):
        if not should_skip_file(file_path):
            rust_files.append(file_path)

    return rust_files


def format_report(issues: List[Issue]) -> str:
    """格式化检查报告"""
    if not issues:
        return "✅ 未发现错误容忍问题！"

    # 按严重度和文件分组
    issues_by_severity = {
        Severity.HIGH: [],
        Severity.MEDIUM: [],
        Severity.LOW: []
    }

    for issue in issues:
        issues_by_severity[issue.severity].append(issue)

    lines = []
    lines.append(f"# Rust 错误容忍检查报告\n")
    lines.append(f"共发现 {len(issues)} 个问题\n")

    # 按严重度输出
    for severity in [Severity.HIGH, Severity.MEDIUM, Severity.LOW]:
        severity_issues = issues_by_severity[severity]
        if not severity_issues:
            continue

        lines.append(f"\n## {severity.value}\n")

        # 按文件分组
        issues_by_file: Dict[str, List[Issue]] = {}
        for issue in severity_issues:
            file_path = issue.file_path
            if file_path not in issues_by_file:
                issues_by_file[file_path] = []
            issues_by_file[file_path].append(issue)

        for file_path, file_issues in sorted(issues_by_file.items()):
            lines.append(f"\n### 文件: `{file_path}`\n")

            for issue in file_issues:
                lines.append(f"- **行 {issue.line_number}**: `{issue.line_content}`")
                lines.append(f"  - **类别**: {issue.category}")
                lines.append(f"  - **风险**: {issue.risk}")
                lines.append(f"  - **建议**: {issue.suggestion}")

                if issue.example:
                    lines.append(f"  - **示例**:")
                    lines.append(f"    ```rust")
                    for example_line in issue.example.split('\n'):
                        lines.append(f"    {example_line}")
                    lines.append(f"    ```")

                lines.append("")

    # 汇总统计
    lines.append("\n---\n")
    lines.append("## 📊 汇总统计\n\n")
    lines.append("| 严重度 | 问题数量 | 优先级 |")
    lines.append("|--------|----------|--------|")

    high_count = len(issues_by_severity[Severity.HIGH])
    medium_count = len(issues_by_severity[Severity.MEDIUM])
    low_count = len(issues_by_severity[Severity.LOW])

    lines.append(f"| 🔴 高 | {high_count} | P0 - 立即修复 |")
    lines.append(f"| 🟡 中 | {medium_count} | P1 - 尽快修复 |")
    lines.append(f"| 🟢 低 | {low_count} | P2 - 改进代码质量 |")

    return '\n'.join(lines)


def main():
    """主函数"""
    if len(sys.argv) > 1:
        check_path = Path(sys.argv[1])
    else:
        check_path = Path.cwd()

    if not check_path.exists():
        print(f"❌ 错误: 路径不存在: {check_path}", file=sys.stderr)
        sys.exit(1)

    print(f"🔍 检查路径: {check_path}")
    print(f"📁 正在查找 Rust 文件...\n")

    rust_files = find_rust_files(check_path)

    if not rust_files:
        print(f"⚠️  未找到任何 .rs 文件在: {check_path}")
        sys.exit(0)

    print(f"✅ 找到 {len(rust_files)} 个 Rust 文件\n")
    print("🔬 正在分析代码...\n")

    all_issues = []
    for rust_file in rust_files:
        issues = check_rust_file(rust_file)
        all_issues.extend(issues)

    # 输出报告
    report = format_report(all_issues)
    print(report)

    # 根据严重度返回退出码
    high_issues = [i for i in all_issues if i.severity == Severity.HIGH]
    if high_issues:
        print(f"\n❌ 发现 {len(high_issues)} 个高严重度问题，请立即修复！")
        sys.exit(1)
    elif all_issues:
        print(f"\n⚠️  发现 {len(all_issues)} 个问题，建议尽快修复。")
        sys.exit(0)
    else:
        print(f"\n✅ 代码质量检查通过！")
        sys.exit(0)


if __name__ == "__main__":
    main()

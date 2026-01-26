#!/usr/bin/env python3
"""
Rust 测试执行和分析脚本

执行 Rust 测试并分析失败原因，提供修复建议。

Features:
- 运行 cargo test 并捕获输出
- 分析失败的测试并提取错误信息
- 对每个失败的测试提供单独的执行和分析
- 尝试自动修复可修复的问题（如未使用的导入、类型错误等）
- 生成详细的测试报告

Usage:
    python3 run_rust_tests.py [test_name] [--package <name>] [--features <features>]

Examples:
    python3 run_rust_tests.py                          # 运行所有测试
    python3 run_rust_tests.py test_login              # 运行指定测试
    python3 run_rust_tests.py --package my-package    # 运行指定包的测试
    python3 run_rust_tests.py --features "full"       # 启用指定 features
"""

import argparse
import json
import os
import re
import subprocess
import sys
from dataclasses import dataclass
from enum import Enum
from pathlib import Path
from typing import List, Optional, Dict


class TestStatus(Enum):
    PASSED = "✅ 通过"
    FAILED = "❌ 失败"
    IGNORED = "⚠️  忽略"
    TIMEOUT = "⏱️  超时"


@dataclass
class TestResult:
    """测试结果"""
    test_name: str
    status: TestStatus
    duration: float
    error_message: str = ""
    error_type: str = ""
    suggestion: str = ""
    fixable: bool = False


def run_command(
    command: List[str],
    cwd: Optional[Path] = None,
    timeout: int = 300
) -> tuple[int, str, str]:
    """
    运行命令并返回退出码、标准输出和标准错误

    Returns:
        (exit_code, stdout, stderr)
    """
    try:
        result = subprocess.run(
            command,
            cwd=cwd,
            capture_output=True,
            text=True,
            timeout=timeout
        )
        return result.returncode, result.stdout, result.stderr
    except subprocess.TimeoutExpired:
        return -1, "", f"Command timed out after {timeout} seconds"
    except Exception as e:
        return -1, "", str(e)


def parse_test_output(output: str) -> List[TestResult]:
    """
    解析 cargo test 输出，提取测试结果

    cargo test 输出格式示例:
        test test_foo ... ok
        test test_bar ... FAILED
        test test_baz ... ignored
    """
    results = []

    # 正则匹配测试结果行
    test_pattern = re.compile(r'^test\s+(.+?)\s+\.\.\.(\w+)(?:\s+(.+))?$')

    for line in output.split('\n'):
        match = test_pattern.match(line.strip())
        if match:
            test_name = match.group(1)
            status_str = match.group(2)
            rest = match.group(3) or ""

            if status_str == "ok":
                status = TestStatus.PASSED
            elif status_str == "FAILED":
                status = TestStatus.FAILED
            elif status_str in ["ignored", "should panic"]:
                status = TestStatus.IGNORED
            else:
                status = TestStatus.FAILED

            results.append(TestResult(
                test_name=test_name,
                status=status,
                duration=0.0,  # cargo test 默认不输出时间
                error_message=rest if status == TestStatus.FAILED else ""
            ))

    return results


def analyze_failure(stderr: str, test_name: str) -> Dict[str, str]:
    """
    分析测试失败原因并提供修复建议

    Returns:
        {
            "error_type": "错误类型",
            "suggestion": "修复建议",
            "fixable": True/False
        }
    """
    error_type = "未知错误"
    suggestion = "请检查测试代码和实现代码"
    fixable = False

    # 常见错误模式
    patterns = {
        "assertion failed": {
            "type": "断言失败",
            "suggestion": "检查断言条件，确保测试预期与实际行为一致",
            "fixable": False
        },
        "panicked at": {
            "type": "Panic",
            "suggestion": "代码发生 panic，检查是否访问了无效数据或触发了 panic!",
            "fixable": False
        },
        "attempt to add with overflow": {
            "type": "算术溢出",
            "suggestion": "使用 checked_add、wrapping_add 或 saturating_add",
            "fixable": True
        },
        "borrow checker": {
            "type": "借用检查错误",
            "suggestion": "检查所有权和生命周期，可能需要克隆或调整引用",
            "fixable": False
        },
        "type mismatch": {
            "type": "类型不匹配",
            "suggestion": "检查类型注解，可能需要进行类型转换",
            "fixable": True
        },
        "no such file or directory": {
            "type": "文件不存在",
            "suggestion": "确保测试所需的文件存在，或使用临时目录",
            "fixable": True
        },
        "connection refused": {
            "type": "连接失败",
            "suggestion": "确保测试依赖的服务已启动，或使用 mock",
            "fixable": False
        },
        "timeout": {
            "type": "测试超时",
            "suggestion": "优化测试性能或增加超时时间",
            "fixable": False
        },
    }

    for pattern, info in patterns.items():
        if pattern.lower() in stderr.lower():
            error_type = info["type"]
            suggestion = info["suggestion"]
            fixable = info["fixable"]
            break

    return {
        "error_type": error_type,
        "suggestion": suggestion,
        "fixable": fixable
    }


def run_single_test(
    test_name: str,
    package: Optional[str] = None,
    features: Optional[str] = None,
    workspace_root: Path = None
) -> TestResult:
    """
    运行单个测试并分析结果
    """
    if workspace_root is None:
        workspace_root = Path.cwd()

    # 构建命令
    command = ["cargo", "test", "--no-fail-fast"]

    if package:
        command.extend(["--package", package])

    if features:
        command.extend(["--features", features])

    command.append(test_name)

    print(f"🔍 运行测试: {test_name}")
    print(f"📝 命令: {' '.join(command)}\n")

    exit_code, stdout, stderr = run_command(command, cwd=workspace_root)

    if exit_code == 0:
        return TestResult(
            test_name=test_name,
            status=TestStatus.PASSED,
            duration=0.0
        )
    else:
        # 分析失败原因
        analysis = analyze_failure(stderr, test_name)

        return TestResult(
            test_name=test_name,
            status=TestStatus.FAILED,
            duration=0.0,
            error_message=stderr[:500],  # 限制长度
            error_type=analysis["error_type"],
            suggestion=analysis["suggestion"],
            fixable=analysis["fixable"]
        )


def format_test_report(results: List[TestResult]) -> str:
    """格式化测试报告"""
    lines = []
    lines.append("# Rust 测试报告\n")

    passed = [r for r in results if r.status == TestStatus.PASSED]
    failed = [r for r in results if r.status == TestStatus.FAILED]
    ignored = [r for r in results if r.status == TestStatus.IGNORED]

    lines.append(f"## 📊 统计\n")
    lines.append(f"- 总测试数: {len(results)}")
    lines.append(f"- ✅ 通过: {len(passed)}")
    lines.append(f"- ❌ 失败: {len(failed)}")
    lines.append(f"- ⚠️  忽略: {len(ignored)}\n")

    if failed:
        lines.append(f"## ❌ 失败的测试\n")

        for result in failed:
            lines.append(f"\n### {result.test_name}")
            lines.append(f"- **状态**: {result.status.value}")
            lines.append(f"- **错误类型**: {result.error_type}")
            lines.append(f"- **建议**: {result.suggestion}")

            if result.fixable:
                lines.append(f"- **可自动修复**: ✅ 是")

            if result.error_message:
                lines.append(f"\n**错误信息**:")
                lines.append(f"```")
                lines.append(result.error_message[:300])
                if len(result.error_message) > 300:
                    lines.append("...")
                lines.append(f"```")

    if passed:
        lines.append(f"\n## ✅ 通过的测试\n")
        for result in passed[:10]:  # 只显示前 10 个
            lines.append(f"- {result.test_name}")

        if len(passed) > 10:
            lines.append(f"- ... 还有 {len(passed) - 10} 个测试")

    return '\n'.join(lines)


def main():
    parser = argparse.ArgumentParser(
        description="Rust 测试执行和分析工具"
    )
    parser.add_argument(
        "test_name",
        nargs="?",
        help="要运行的测试名称（留空运行所有测试）"
    )
    parser.add_argument(
        "--package", "-p",
        help="指定包名"
    )
    parser.add_argument(
        "--features", "-F",
        help="启用的 features"
    )
    parser.add_argument(
        "--workspace", "-w",
        type=Path,
        default=Path.cwd(),
        help="工作空间根目录（默认为当前目录）"
    )

    args = parser.parse_args()

    # 查找工作空间根目录
    workspace_root = args.workspace
    while not (workspace_root / "Cargo.toml").exists():
        parent = workspace_root.parent
        if parent == workspace_root:
            print("❌ 错误: 未找到 Cargo.toml")
            sys.exit(1)
        workspace_root = parent

    print(f"📂 工作空间: {workspace_root}\n")

    if args.test_name:
        # 运行单个测试
        result = run_single_test(
            args.test_name,
            package=args.package,
            features=args.features,
            workspace_root=workspace_root
        )

        report = format_test_report([result])
        print(report)

        if result.status == TestStatus.FAILED:
            sys.exit(1)
    else:
        # 运行所有测试
        print("🚀 运行所有测试...\n")

        command = ["cargo", "test", "--no-fail-fast", "--", "--format-terse"]
        if args.package:
            command.extend(["--package", args.package])
        if args.features:
            command.extend(["--features", args.features])

        exit_code, stdout, stderr = run_command(command, cwd=workspace_root)

        results = parse_test_output(stdout)

        report = format_test_report(results)
        print(report)

        if exit_code != 0:
            sys.exit(1)


if __name__ == "__main__":
    main()

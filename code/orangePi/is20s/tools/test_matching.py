#!/usr/bin/env python3
"""
ドメインマッチングテスター

デバイスと同一のマッチングロジックでローカルテストを行う。
デプロイ前に辞書の動作確認に使用。

Usage:
    # 単一ドメインテスト
    ./test_matching.py youtube.com
    ./test_matching.py www.gstatic.com

    # 複数ドメインテスト
    ./test_matching.py youtube.com netflix.com amazon.co.jp

    # バッチテスト（ファイルから）
    ./test_matching.py -f test_domains.txt

    # 期待値テスト（CSV形式: domain,expected_service,expected_category）
    ./test_matching.py -t test_cases.csv

    # 辞書ファイル指定
    ./test_matching.py -d ../data/default_domain_services.json youtube.com

    # 詳細モード（マッチしたパターンを表示）
    ./test_matching.py -v youtube.com
"""
import argparse
import json
import sys
from pathlib import Path
from typing import Dict, List, Optional, Tuple

# デフォルト辞書パス
SCRIPT_DIR = Path(__file__).parent
DEFAULT_DICT_PATH = SCRIPT_DIR.parent / "data" / "default_domain_services.json"


def load_dictionary(dict_path: Path) -> Dict[str, Dict[str, str]]:
    """辞書ファイルを読み込み"""
    if not dict_path.exists():
        print(f"Error: Dictionary file not found: {dict_path}", file=sys.stderr)
        sys.exit(1)

    with dict_path.open(encoding="utf-8") as f:
        data = json.load(f)
        return data.get("services", data)


def match_domain(
    domain: str, services: Dict[str, Dict[str, str]], verbose: bool = False
) -> Tuple[Optional[str], Optional[str], Optional[str], Optional[str]]:
    """
    ドメインをマッチング（デバイスと同一ロジック）

    2パスマッチング:
    - Pass 1: dotted patterns（より具体的）を全てチェック
    - Pass 2: dotted patternでマッチしなかった場合のみ substring patterns をチェック

    Returns:
        (service, category, role, matched_pattern) or (None, None, None, None)
    """
    if not domain:
        return (None, None, None, None)

    domain_lower = domain.lower()

    # Pass 1: dotted patterns（より具体的）を先にチェック
    for pattern, info in services.items():
        if "." in pattern:
            # パターンにドットがある場合は厳密マッチ
            # prefix対応: www.gstatic → www.gstatic.com にマッチ
            if (
                domain_lower == pattern
                or domain_lower.endswith("." + pattern)
                or ("." + pattern + ".") in domain_lower
                or domain_lower.startswith(pattern + ".")
            ):
                if verbose:
                    match_type = "exact" if domain_lower == pattern else "dotted"
                    print(f"  [MATCH] pattern='{pattern}' type={match_type} (pass 1)")
                return (
                    info.get("service"),
                    info.get("category"),
                    info.get("role"),
                    pattern,
                )

    # Pass 2: dotted patternでマッチしなかった場合のみ substring patterns をチェック
    for pattern, info in services.items():
        if "." not in pattern:
            # パターンにドットがない場合は部分マッチ
            if pattern in domain_lower:
                if verbose:
                    print(f"  [MATCH] pattern='{pattern}' type=substring (pass 2)")
                return (
                    info.get("service"),
                    info.get("category"),
                    info.get("role"),
                    pattern,
                )

    return (None, None, None, None)


def format_result(
    domain: str,
    service: Optional[str],
    category: Optional[str],
    role: Optional[str],
    pattern: Optional[str] = None,
    show_pattern: bool = False,
) -> str:
    """結果をフォーマット"""
    if service:
        base = f"{domain} -> {service} ({category})"
        if role:
            base += f" [{role}]"
        if show_pattern and pattern:
            base += f" [pattern: {pattern}]"
        return base
    else:
        return f"{domain} -> (unknown)"


def test_single(
    domains: List[str], services: Dict[str, Dict[str, str]], verbose: bool = False
) -> int:
    """単一/複数ドメインテスト"""
    unknown_count = 0

    for domain in domains:
        domain = domain.strip()
        if not domain or domain.startswith("#"):
            continue

        service, category, role, pattern = match_domain(domain, services, verbose)
        result = format_result(domain, service, category, role, pattern, verbose)
        print(result)

        if not service:
            unknown_count += 1

    return unknown_count


def test_batch(
    file_path: Path, services: Dict[str, Dict[str, str]], verbose: bool = False
) -> int:
    """バッチテスト（ファイルからドメインリストを読み込み）"""
    if not file_path.exists():
        print(f"Error: File not found: {file_path}", file=sys.stderr)
        sys.exit(1)

    with file_path.open(encoding="utf-8") as f:
        domains = [line.strip() for line in f if line.strip() and not line.startswith("#")]

    print(f"Testing {len(domains)} domains from {file_path}")
    print("-" * 60)

    unknown_count = test_single(domains, services, verbose)

    print("-" * 60)
    print(f"Total: {len(domains)}, Unknown: {unknown_count}")

    return unknown_count


def test_expectations(
    file_path: Path, services: Dict[str, Dict[str, str]], verbose: bool = False
) -> Tuple[int, int, int]:
    """
    期待値テスト（CSV形式）

    フォーマット: domain,expected_service,expected_category
    expected_service が空または "-" の場合は unknown を期待

    Returns:
        (passed, failed, total)
    """
    if not file_path.exists():
        print(f"Error: File not found: {file_path}", file=sys.stderr)
        sys.exit(1)

    passed = 0
    failed = 0
    results = []

    with file_path.open(encoding="utf-8") as f:
        for line_no, line in enumerate(f, 1):
            line = line.strip()
            if not line or line.startswith("#"):
                continue

            parts = line.split(",")
            if len(parts) < 2:
                print(f"Warning: Invalid line {line_no}: {line}", file=sys.stderr)
                continue

            domain = parts[0].strip()
            expected_service = parts[1].strip() if len(parts) > 1 else ""
            expected_category = parts[2].strip() if len(parts) > 2 else ""

            # "-" は unknown を期待
            if expected_service == "-":
                expected_service = ""

            service, category, role, pattern = match_domain(domain, services, verbose)

            # 判定
            service_match = (service or "") == expected_service
            category_match = (
                not expected_category or (category or "") == expected_category
            )

            if service_match and category_match:
                passed += 1
                status = "PASS"
            else:
                failed += 1
                status = "FAIL"

            results.append(
                {
                    "domain": domain,
                    "status": status,
                    "expected": f"{expected_service or '(unknown)'} / {expected_category or '*'}",
                    "actual": f"{service or '(unknown)'} / {category or ''}",
                    "pattern": pattern,
                }
            )

    # 結果出力
    print(f"Testing {len(results)} cases from {file_path}")
    print("=" * 80)

    for r in results:
        if r["status"] == "FAIL":
            print(f"[FAIL] {r['domain']}")
            print(f"       Expected: {r['expected']}")
            print(f"       Actual:   {r['actual']}")
            if r["pattern"]:
                print(f"       Pattern:  {r['pattern']}")
        elif verbose:
            print(f"[PASS] {r['domain']} -> {r['actual']}")

    print("=" * 80)
    print(f"Passed: {passed}/{len(results)}, Failed: {failed}")

    return passed, failed, len(results)


def main():
    parser = argparse.ArgumentParser(
        description="ドメインマッチングテスター（デバイス同一ロジック）",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  %(prog)s youtube.com                     # 単一テスト
  %(prog)s youtube.com amazon.co.jp        # 複数テスト
  %(prog)s -f domains.txt                  # バッチテスト
  %(prog)s -t test_cases.csv               # 期待値テスト
  %(prog)s -v youtube.com                  # 詳細モード
  %(prog)s -d custom_dict.json domain.com  # 辞書指定
        """,
    )

    parser.add_argument(
        "domains",
        nargs="*",
        help="テスト対象のドメイン",
    )
    parser.add_argument(
        "-d",
        "--dict",
        type=Path,
        default=DEFAULT_DICT_PATH,
        help=f"辞書ファイル (default: {DEFAULT_DICT_PATH})",
    )
    parser.add_argument(
        "-f",
        "--file",
        type=Path,
        help="ドメインリストファイル（1行1ドメイン）",
    )
    parser.add_argument(
        "-t",
        "--test",
        type=Path,
        help="期待値テストファイル（CSV: domain,service,category）",
    )
    parser.add_argument(
        "-v",
        "--verbose",
        action="store_true",
        help="詳細モード（マッチしたパターンを表示）",
    )
    parser.add_argument(
        "--count",
        action="store_true",
        help="辞書のパターン数を表示して終了",
    )

    args = parser.parse_args()

    # 辞書読み込み
    services = load_dictionary(args.dict)

    if args.count:
        print(f"Dictionary: {args.dict}")
        print(f"Patterns: {len(services)}")
        return 0

    # テスト実行
    exit_code = 0

    if args.test:
        # 期待値テスト
        passed, failed, total = test_expectations(args.test, services, args.verbose)
        if failed > 0:
            exit_code = 1
    elif args.file:
        # バッチテスト
        unknown = test_batch(args.file, services, args.verbose)
        if unknown > 0:
            exit_code = 1
    elif args.domains:
        # 単一/複数ドメインテスト
        unknown = test_single(args.domains, services, args.verbose)
        # 単一テストでは exit code は設定しない
    else:
        # インタラクティブモード
        print(f"Dictionary: {args.dict} ({len(services)} patterns)")
        print("Enter domains to test (Ctrl+D to exit):")
        try:
            while True:
                domain = input("> ").strip()
                if domain:
                    service, category, role, pattern = match_domain(
                        domain, services, args.verbose
                    )
                    result = format_result(
                        domain, service, category, role, pattern, True
                    )
                    print(result)
        except EOFError:
            print("\nBye!")

    return exit_code


if __name__ == "__main__":
    sys.exit(main())

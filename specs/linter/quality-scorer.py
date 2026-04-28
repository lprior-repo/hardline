#!/usr/bin/env python3
"""
Quality Scorer for Spec Files

Validates spec files against quality rules and returns a quality score.

Usage:
    quality-scorer.py <spec-file>
    quality-scorer.py --rules <rules-file> <spec-file>

Exit codes:
    0 - Score >= 80 and no security failures
    1 - Score < 80 (below threshold)
    2 - Security rules failed
    3 - Invalid input or other error
"""

import argparse
import json
import sys
import re
from pathlib import Path
from typing import Any

import yaml


class QualityScorerError(Exception):
    """Base exception for quality scorer errors."""

    pass


class RuleLoadError(QualityScorerError):
    """Failed to load rules file."""

    pass


class SpecLoadError(QualityScorerError):
    """Failed to load spec file."""

    pass


class ValidationError(QualityScorerError):
    """Validation error."""

    pass


def load_rules(rules_path: Path) -> dict[str, Any]:
    """Load rules from YAML file.

    Args:
        rules_path: Path to rules.yaml file

    Returns:
        Dictionary containing rules configuration

    Raises:
        RuleLoadError: If rules cannot be loaded
    """
    try:
        with open(rules_path, "r", encoding="utf-8") as f:
            rules = yaml.safe_load(f)
    except FileNotFoundError:
        raise RuleLoadError(f"Rules file not found: {rules_path}")
    except yaml.YAMLError as e:
        raise RuleLoadError(f"Invalid YAML in rules file: {e}")

    if not rules or "rules" not in rules:
        raise RuleLoadError("Rules file must contain 'rules' key")

    return rules


def load_spec(spec_path: Path) -> dict[str, Any]:
    """Load spec from YAML file.

    Args:
        spec_path: Path to spec YAML file

    Returns:
        Dictionary containing spec data

    Raises:
        SpecLoadError: If spec cannot be loaded
    """
    try:
        with open(spec_path, "r", encoding="utf-8") as f:
            spec = yaml.safe_load(f)
    except FileNotFoundError:
        raise SpecLoadError(f"Spec file not found: {spec_path}")
    except yaml.YAMLError as e:
        raise SpecLoadError(f"Invalid YAML in spec file: {e}")

    if not isinstance(spec, dict):
        raise SpecLoadError("Spec file must contain a YAML dictionary")

    return spec


def get_nested_value(data: dict[str, Any], path: str) -> Any:
    """Get nested value from dict using dot notation.

    Args:
        data: Dictionary to search
        path: Dot-separated path (e.g., "identity.id")

    Returns:
        Value at the path, or None if not found
    """
    keys = path.split(".")
    current = data

    for key in keys:
        if isinstance(current, dict):
            current = current.get(key)
        else:
            return None

    return current


def check_completeness(
    spec: dict[str, Any], rules: list[dict[str, Any]]
) -> tuple[int, list[dict[str, Any]]]:
    """Check completeness rules.

    Args:
        spec: Spec dictionary
        rules: List of completeness rules

    Returns:
        Tuple of (score, list of failures)
    """
    total_weight = sum(r.get("weight", 0) for r in rules)
    failures = []
    earned_weight = total_weight

    for rule in rules:
        rule_id = rule["id"]
        weight = rule.get("weight", 0)

        # Check required fields
        if "required_fields" in rule:
            for field in rule["required_fields"]:
                value = get_nested_value(spec, field)
                if value is None or (isinstance(value, str) and not value.strip()):
                    failures.append(
                        {
                            "rule_id": rule_id,
                            "rule_name": rule["name"],
                            "message": rule.get(
                                "error_message", f"Missing required field: {field}"
                            ),
                            "severity": rule.get("severity", "error"),
                        }
                    )
                    earned_weight -= weight
                    break
            continue

        # Check minimum items
        if "min_items" in rule:
            path = rule.get("path", "")
            items = get_nested_value(spec, path)
            if (
                not items
                or not isinstance(items, list)
                or len(items) < rule["min_items"]
            ):
                failures.append(
                    {
                        "rule_id": rule_id,
                        "rule_name": rule["name"],
                        "message": rule.get(
                            "error_message", f"Minimum items not met for: {path}"
                        ),
                        "severity": rule.get("severity", "error"),
                    }
                )
                earned_weight -= weight

    score = int((earned_weight / total_weight) * 100) if total_weight > 0 else 100
    return score, failures


def check_clarity(
    spec: dict[str, Any], rules: list[dict[str, Any]]
) -> tuple[int, list[dict[str, Any]]]:
    """Check clarity rules.

    Args:
        spec: Spec dictionary
        rules: List of clarity rules

    Returns:
        Tuple of (score, list of failures)
    """
    total_weight = sum(r.get("weight", 0) for r in rules)
    failures = []
    earned_weight = total_weight

    spec_text = json.dumps(spec).lower()

    for rule in rules:
        rule_id = rule["id"]
        weight = rule.get("weight", 0)
        severity = rule.get("severity", "warning")

        # Check prohibited patterns
        if "prohibited_patterns" in rule:
            for pattern in rule["prohibited_patterns"]:
                if re.search(pattern, spec_text, re.IGNORECASE):
                    failures.append(
                        {
                            "rule_id": rule_id,
                            "rule_name": rule["name"],
                            "message": rule.get(
                                "error_message", f"Found prohibited pattern: {pattern}"
                            ),
                            "severity": severity,
                            "pattern": pattern,
                        }
                    )
                    if severity == "error":
                        earned_weight -= weight
                    break
            continue

        # Check minimum description length
        if "min_description_length" in rule:
            path = rule.get("path", "")
            items = get_nested_value(spec, path)
            if items and isinstance(items, list):
                for item in items:
                    desc = item.get("description", "") if isinstance(item, dict) else ""
                    if len(desc) < rule["min_description_length"]:
                        failures.append(
                            {
                                "rule_id": rule_id,
                                "rule_name": rule["name"],
                                "message": rule.get(
                                    "error_message", "Description too short"
                                ),
                                "severity": severity,
                            }
                        )
                        earned_weight -= weight
                        break

    score = int((earned_weight / total_weight) * 100) if total_weight > 0 else 100
    return score, failures


def check_testability(
    spec: dict[str, Any], rules: list[dict[str, Any]]
) -> tuple[int, list[dict[str, Any]]]:
    """Check testability rules.

    Args:
        spec: Spec dictionary
        rules: List of testability rules

    Returns:
        Tuple of (score, list of failures)
    """
    total_weight = sum(r.get("weight", 0) for r in rules)
    failures = []
    earned_weight = total_weight

    for rule in rules:
        rule_id = rule["id"]
        weight = rule.get("weight", 0)

        # Check minimum items
        if "min_items" in rule:
            path = rule.get("path", "")
            items = get_nested_value(spec, path)
            if (
                not items
                or not isinstance(items, list)
                or len(items) < rule["min_items"]
            ):
                failures.append(
                    {
                        "rule_id": rule_id,
                        "rule_name": rule["name"],
                        "message": rule.get(
                            "error_message", f"Minimum tests not met for: {path}"
                        ),
                        "severity": "error",
                    }
                )
                earned_weight -= weight
            continue

        # Check required test fields
        if "required_test_fields" in rule:
            path = "acceptance_tests"
            at = get_nested_value(spec, path)
            if at and isinstance(at, dict):
                for test_type in ["happy_paths", "error_paths"]:
                    tests = at.get(test_type, [])
                    if isinstance(tests, list):
                        for test in tests:
                            if isinstance(test, dict):
                                for field in rule["required_test_fields"]:
                                    if field not in test or not test[field]:
                                        failures.append(
                                            {
                                                "rule_id": rule_id,
                                                "rule_name": rule["name"],
                                                "message": f"Test missing required field: {field}",
                                                "severity": "error",
                                            }
                                        )
                                        earned_weight -= weight
                                        break
            continue

        # Check test name length
        if "min_test_name_length" in rule:
            path = "acceptance_tests"
            at = get_nested_value(spec, path)
            if at and isinstance(at, dict):
                for test_type in ["happy_paths", "error_paths"]:
                    tests = at.get(test_type, [])
                    if isinstance(tests, list):
                        for test in tests:
                            if isinstance(test, dict):
                                name = test.get("test_name", "")
                                if len(name) < rule["min_test_name_length"]:
                                    failures.append(
                                        {
                                            "rule_id": rule_id,
                                            "rule_name": rule["name"],
                                            "message": f"Test name too short: {name}",
                                            "severity": "warning",
                                        }
                                    )
                                    earned_weight -= weight
                                    break

    score = int((earned_weight / total_weight) * 100) if total_weight > 0 else 100
    return score, failures


def check_security(
    spec: dict[str, Any], rules: list[dict[str, Any]]
) -> tuple[int, list[dict[str, Any]]]:
    """Check security rules.

    Args:
        spec: Spec dictionary
        rules: List of security rules

    Returns:
        Tuple of (score, list of failures)
    """
    total_weight = sum(r.get("weight", 0) for r in rules)
    failures = []
    earned_weight = total_weight

    spec_text = json.dumps(spec)

    for rule in rules:
        rule_id = rule["id"]
        weight = rule.get("weight", 0)

        # Check prohibited patterns
        if "prohibited_patterns" in rule:
            for pattern in rule["prohibited_patterns"]:
                if re.search(pattern, spec_text, re.IGNORECASE):
                    failures.append(
                        {
                            "rule_id": rule_id,
                            "rule_name": rule["name"],
                            "message": rule.get(
                                "error_message", f"Found security issue: {pattern}"
                            ),
                            "severity": "error",
                            "pattern": pattern,
                        }
                    )
                    earned_weight -= weight
                    break

    score = int((earned_weight / total_weight) * 100) if total_weight > 0 else 100
    return score, failures


def check_specificity(
    spec: dict[str, Any], rules: list[dict[str, Any]]
) -> tuple[int, list[dict[str, Any]]]:
    """Check specificity rules.

    Args:
        spec: Spec dictionary
        rules: List of specificity rules

    Returns:
        Tuple of (score, list of failures)
    """
    total_weight = sum(r.get("weight", 0) for r in rules)
    failures = []
    earned_weight = total_weight

    for rule in rules:
        rule_id = rule["id"]
        weight = rule.get("weight", 0)
        severity = rule.get("severity", "warning")

        # Check prohibited patterns
        if "prohibited_patterns" in rule:
            spec_text = json.dumps(spec).lower()
            for pattern in rule["prohibited_patterns"]:
                if re.search(pattern, spec_text, re.IGNORECASE):
                    failures.append(
                        {
                            "rule_id": rule_id,
                            "rule_name": rule["name"],
                            "message": rule.get(
                                "error_message", f"Found vague language: {pattern}"
                            ),
                            "severity": severity,
                            "pattern": pattern,
                        }
                    )
                    if severity == "error":
                        earned_weight -= weight
                    break
            continue

        # Check required ID pattern
        if "required_id_pattern" in rule:
            path = rule.get("path", "")
            items = get_nested_value(spec, path)
            if items and isinstance(items, list):
                for item in items:
                    if isinstance(item, dict):
                        item_id = item.get("id", "")
                        if not re.match(rule["required_id_pattern"], item_id):
                            failures.append(
                                {
                                    "rule_id": rule_id,
                                    "rule_name": rule["name"],
                                    "message": rule.get(
                                        "error_message", f"Invalid ID format: {item_id}"
                                    ),
                                    "severity": severity,
                                }
                            )
                            earned_weight -= weight
                            break

    score = int((earned_weight / total_weight) * 100) if total_weight > 0 else 100
    return score, failures


def calculate_category_scores(
    spec: dict[str, Any], rules: dict[str, Any]
) -> dict[str, Any]:
    """Calculate scores for each category.

    Args:
        spec: Spec dictionary
        rules: Rules configuration

    Returns:
        Dictionary with category scores and failures
    """
    category_rules = rules.get("rules", {})
    scoring_config = rules.get("scoring", {})

    results = {}
    all_failures = []

    # Calculate each category
    categories = ["completeness", "clarity", "testability", "security", "specificity"]

    for category in categories:
        category_rules_list = category_rules.get(category, [])
        if not category_rules_list:
            results[category] = {"score": 100, "failures": []}
            continue

        if category == "completeness":
            score, failures = check_completeness(spec, category_rules_list)
        elif category == "clarity":
            score, failures = check_clarity(spec, category_rules_list)
        elif category == "testability":
            score, failures = check_testability(spec, category_rules_list)
        elif category == "security":
            score, failures = check_security(spec, category_rules_list)
        elif category == "specificity":
            score, failures = check_specificity(spec, category_rules_list)
        else:
            score, failures = 100, []

        results[category] = {"score": score, "failures": failures}
        all_failures.extend(failures)

    # Calculate weighted total score
    weights = scoring_config.get(
        "category_weights",
        {
            "completeness": 30,
            "clarity": 15,
            "testability": 30,
            "security": 15,
            "specificity": 10,
        },
    )

    total_score = 0
    for category, weight in weights.items():
        if category in results:
            total_score += results[category]["score"] * (weight / 100)

    return {
        "total_score": int(total_score),
        "categories": results,
        "all_failures": all_failures,
        "threshold": scoring_config.get("threshold", 80),
    }


def print_results(result: dict[str, Any], spec_path: Path) -> None:
    """Print quality score results.

    Args:
        result: Quality score result
        spec_path: Path to the spec file
    """
    print(f"\n{'=' * 60}")
    print(f"Quality Score Report: {spec_path.name}")
    print(f"{'=' * 60}")

    # Print total score
    total = result["total_score"]
    threshold = result["threshold"]
    status = "PASS" if total >= threshold else "FAIL"

    if total >= threshold:
        status_color = "\033[0;32m"  # Green
    elif any(f["severity"] == "error" for f in result["all_failures"]):
        status_color = "\033[0;31m"  # Red
    else:
        status_color = "\033[0;33m"  # Yellow

    print(
        f"\nOverall Score: {status_color}{total}/100{'\033[0m'} (Threshold: {threshold}) [{status}]"
    )

    # Print category breakdown
    print(f"\nCategory Breakdown:")
    print("-" * 40)
    for category, data in result["categories"].items():
        score = data["score"]
        failures = data["failures"]

        if score >= 80:
            cat_color = "\033[0;32m"  # Green
        elif score >= 50:
            cat_color = "\033[0;33m"  # Yellow
        else:
            cat_color = "\033[0;31m"  # Red

        error_count = sum(1 for f in failures if f["severity"] == "error")
        warning_count = sum(1 for f in failures if f["severity"] == "warning")

        issues = []
        if error_count > 0:
            issues.append(f"{error_count} errors")
        if warning_count > 0:
            issues.append(f"{warning_count} warnings")

        issue_str = f" ({', '.join(issues)})" if issues else ""

        print(
            f"  {category.capitalize():15} {cat_color}{score:3}{'\033[0m'}{issue_str}"
        )

    # Print failures
    errors = [f for f in result["all_failures"] if f["severity"] == "error"]
    warnings = [f for f in result["all_failures"] if f["severity"] == "warning"]

    if errors:
        print(f"\n\033[0;31mErrors ({len(errors)}):{'\033[0m'}")
        for failure in errors:
            print(f"  [{failure['rule_id']}] {failure['message']}")

    if warnings:
        print(f"\n\033[0;33mWarnings ({len(warnings)}):{'\033[0m'}")
        for failure in warnings:
            print(f"  [{failure['rule_id']}] {failure['message']}")

    print()


def main() -> int:
    """Main entry point.

    Returns:
        Exit code (0, 1, 2, or 3)
    """
    parser = argparse.ArgumentParser(
        description="Quality scorer for spec files",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "spec_file",
        nargs="?",
        help="Path to spec YAML file to validate",
    )
    parser.add_argument(
        "--rules",
        default="rules.yaml",
        help="Path to rules YAML file (default: rules.yaml)",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Output results as JSON",
    )
    parser.add_argument(
        "--skip-schema",
        action="store_true",
        help="Skip schema validation (assume valid)",
    )

    args = parser.parse_args()

    if not args.spec_file:
        parser.print_help()
        return 3

    # Determine paths
    script_dir = Path(__file__).parent
    spec_path = Path(args.spec_file)
    rules_path = script_dir / args.rules

    if not rules_path.is_absolute():
        rules_path = script_dir / rules_path

    # Load rules
    try:
        rules = load_rules(rules_path)
    except RuleLoadError as e:
        print(f"Error: {e}", file=sys.stderr)
        return 3

    # Load spec
    try:
        spec = load_spec(spec_path)
    except SpecLoadError as e:
        print(f"Error: {e}", file=sys.stderr)
        return 3

    # Calculate scores
    result = calculate_category_scores(spec, rules)

    # Output results
    if args.json:
        print(json.dumps(result, indent=2))
    else:
        print_results(result, spec_path)

    # Determine exit code
    # Check for security errors - they are always failures with exit code 2
    has_security_errors = any(
        f["severity"] == "error"
        for f in result["all_failures"]
        if f.get("rule_id", "").startswith("SEC")
    )

    if has_security_errors:
        return 2

    if result["total_score"] < result["threshold"]:
        return 1

    return 0


if __name__ == "__main__":
    sys.exit(main())

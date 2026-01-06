#!/usr/bin/env python3
"""
Claude Code settings.local.json cleanup script

Removes invalid entries that cause parser errors:
- Shell parameter expansion fragments (%%*, ##*)
- Regex patterns with standalone * (not :*)
- Other problematic patterns

Usage:
    python3 .claude/cleanup_settings.py

Or make executable:
    chmod +x .claude/cleanup_settings.py
    ./.claude/cleanup_settings.py
"""

import json
import re
import sys
from pathlib import Path


def find_settings_file():
    """Find settings.local.json in current or parent directories"""
    current = Path.cwd()
    for path in [current] + list(current.parents):
        settings_path = path / ".claude" / "settings.local.json"
        if settings_path.exists():
            return settings_path
    return None


def is_invalid_entry(entry: str) -> tuple[bool, str]:
    """Check if entry is invalid and return reason"""

    # Shell parameter expansion fragments
    if '%%' in entry:
        return True, "Contains shell parameter expansion %%"
    if '##' in entry:
        return True, "Contains shell parameter expansion ##"

    # Standalone * not part of :* prefix matching
    # Match * that is not preceded by :
    if re.search(r'(?<![:])\.\*', entry):
        return True, "Contains regex .* (not :* prefix matching)"

    # Incomplete Bash entries (just variable assignments)
    if re.match(r'^Bash\([a-z_]+="\$[^"]+"\)$', entry):
        return True, "Incomplete Bash entry (variable fragment)"

    return False, ""


def cleanup_settings(dry_run: bool = False):
    """Clean up invalid entries from settings.local.json"""

    settings_path = find_settings_file()
    if not settings_path:
        print("Error: settings.local.json not found")
        sys.exit(1)

    print(f"Processing: {settings_path}")

    with open(settings_path, 'r', encoding='utf-8') as f:
        data = json.load(f)

    if 'permissions' not in data or 'allow' not in data['permissions']:
        print("No permissions.allow list found")
        return

    allow_list = data['permissions']['allow']
    original_count = len(allow_list)

    new_allow_list = []
    removed_entries = []

    for entry in allow_list:
        is_invalid, reason = is_invalid_entry(entry)
        if is_invalid:
            removed_entries.append((entry[:80] + "..." if len(entry) > 80 else entry, reason))
        else:
            new_allow_list.append(entry)

    removed_count = len(removed_entries)

    if removed_count == 0:
        print("No invalid entries found")
        return

    print(f"\nFound {removed_count} invalid entries:")
    for entry, reason in removed_entries:
        print(f"  - {entry}")
        print(f"    Reason: {reason}")

    if dry_run:
        print(f"\n[DRY RUN] Would remove {removed_count} entries")
        return

    data['permissions']['allow'] = new_allow_list

    with open(settings_path, 'w', encoding='utf-8') as f:
        json.dump(data, f, indent=2, ensure_ascii=False)

    print(f"\nRemoved {removed_count} invalid entries")
    print(f"Remaining entries: {len(new_allow_list)} (was {original_count})")


if __name__ == "__main__":
    dry_run = "--dry-run" in sys.argv or "-n" in sys.argv
    if dry_run:
        print("Running in dry-run mode (no changes will be made)\n")
    cleanup_settings(dry_run=dry_run)

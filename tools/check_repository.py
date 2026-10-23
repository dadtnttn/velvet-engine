#!/usr/bin/env python3
"""Check the Git index for unsupported demos, local artifacts and private media."""

from __future__ import annotations

import json
from pathlib import Path
import posixpath
import re
import subprocess
import sys
import tomllib


ROOT = Path(__file__).resolve().parent.parent
MEDIA = {
    ".png", ".jpg", ".jpeg", ".webp", ".gif", ".ico", ".svg",
    ".mp4", ".webm", ".wav", ".ogg", ".mp3", ".flac",
    ".glb", ".gltf", ".bin", ".blend", ".blend1", ".fbx",
    ".obj", ".mtl", ".zip", ".exe", ".pdf",
}


def git_paths(*args: str) -> set[str]:
    result = subprocess.run(
        ["git", "ls-files", "-z", *args], cwd=ROOT, check=True,
        capture_output=True,
    )
    return set(result.stdout.decode("utf-8").rstrip("\0").split("\0")) - {""}


def main() -> int:
    policy = json.loads((ROOT / "tools/repository-policy.json").read_text(encoding="utf-8"))
    paths = git_paths("--cached")
    errors = [f"tracked file is ignored: {p}" for p in sorted(git_paths("--cached", "--ignored", "--exclude-standard"))]
    demos = set(policy["demos"])
    media = set(policy["public_media"])
    media_dirs = tuple(policy["public_media_directories"])
    # Source-only demos read private data (include_str!/include_bytes!) that stays off Git.
    source_only = tuple(f"demos/{name}/" for name in policy.get("source_only_demos", []))
    for path in sorted(paths):
        parts = path.split("/")
        if parts[0] == "demos" and len(parts) > 1 and parts[1] not in demos:
            errors.append(f"unsupported demo: {path}")
        if parts[0] == "demos" and Path(path).suffix.lower() in MEDIA:
            if path not in media and not path.startswith(media_dirs):
                errors.append(f"unreviewed demo media: {path}")
        source = ROOT / path
        if path.endswith(".rs") and source.is_file() and not path.startswith(source_only):
            text = source.read_text(encoding="utf-8")
            for relative in re.findall(r'include_(?:bytes|str)!\s*\(\s*"([^"\n]+)"\s*\)', text):
                included = posixpath.normpath(posixpath.join(posixpath.dirname(path), relative))
                if included not in paths:
                    errors.append(f"compile-time resource missing from Git: {path} -> {included}")
    workspace = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))["workspace"]
    for member in workspace["members"]:
        if f"{member}/Cargo.toml" not in paths:
            errors.append(f"workspace member missing from Git: {member}")
    members = {member.split("/")[1] for member in workspace["members"] if member.startswith("demos/")}
    if members != demos:
        errors.append("Cargo workspace demos differ from tools/repository-policy.json")
    for path in sorted(media):
        if path not in paths:
            errors.append(f"reviewed resource missing from Git: {path}")
    if errors:
        print("Repository policy failed:", file=sys.stderr)
        for error in errors:
            print(f"  {error}", file=sys.stderr)
        return 1
    print(f"Repository policy OK: {len(paths)} tracked files, {len(demos)} supported demos, {len(media)} reviewed resources.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

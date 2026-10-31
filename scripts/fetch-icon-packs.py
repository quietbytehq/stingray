#!/usr/bin/env python3
"""Fetches the bundled icon packs and keeps only the icons Stingray uses.

Each pack is a GPL-3.0 icon theme on GitHub. For every pack this script clones the repository
(shallow), finds one SVG for each icon Stingray needs (folders, home folders, trash and common file
types), and copies it to static/icon-packs/<id>/<light|dark>/<key>.svg with the pack's licence.
It also writes static/icon-packs/index.json, which the app reads.

Run from the repo root:  python3 scripts/fetch-icon-packs.py
"""
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "static" / "icon-packs"

PACKS = [
    {"id": "candy", "name": "Candy", "author": "EliverLara", "repo": "EliverLara/candy-icons"},
    {"id": "kora", "name": "Kora", "author": "bikass", "repo": "bikass/kora"},
    {"id": "newaita", "name": "Newaita Reborn", "author": "cbrnix", "repo": "cbrnix/Newaita-reborn"},
    {"id": "papirus", "name": "Papirus", "author": "Papirus Development Team", "repo": "PapirusDevelopmentTeam/papirus-icon-theme"},
    {"id": "whitesur", "name": "WhiteSur", "author": "Vince Liuice", "repo": "vinceliuice/WhiteSur-icon-theme"},
    {"id": "nordzy", "name": "Nordzy", "author": "alvatip", "repo": "alvatip/Nordzy-icon"},
    {"id": "gruvbox", "name": "Gruvbox Plus", "author": "SylEleuth", "repo": "SylEleuth/gruvbox-plus-icon-pack"},
]

# Stingray key -> freedesktop icon names to try, best first.
ICONS = {
    "folder": ["folder", "inode-directory", "default-folder"],
    "home": ["user-home", "folder-home", "default-user-home"],
    "desktop": ["user-desktop", "folder-desktop", "default-user-desktop"],
    "documents": ["folder-documents", "folder-document", "default-folder-documents"],
    "downloads": ["folder-download", "folder-downloads", "default-folder-download"],
    "music": ["folder-music", "default-folder-music"],
    "pictures": ["folder-pictures", "folder-images", "default-folder-pictures"],
    "videos": ["folder-videos", "folder-video", "default-folder-videos"],
    "trash": ["user-trash", "user-trash-empty", "trashcan_empty"],
    "file": ["text-x-generic", "unknown", "application-octet-stream"],
    "document": ["x-office-document", "application-vnd.openxmlformats-officedocument.wordprocessingml.document", "text-x-generic"],
    "pdf": ["application-pdf", "pdf"],
    "spreadsheet": ["x-office-spreadsheet", "application-vnd.openxmlformats-officedocument.spreadsheetml.sheet"],
    "presentation": ["x-office-presentation", "application-vnd.openxmlformats-officedocument.presentationml.presentation"],
    "image": ["image-x-generic", "image-png", "image-jpeg"],
    "video": ["video-x-generic", "video-mp4"],
    "audio": ["audio-x-generic", "audio-mpeg", "audio-x-mpeg"],
    "archive": ["package-x-generic", "application-zip", "application-x-archive", "application-x-compressed-tar"],
    "code": ["text-x-script", "text-x-python", "application-x-shellscript", "text-x-source"],
}

SKIP = re.compile(r"(symbolic|/actions/|/apps/|/status/|/emblems/|/devices/|/panel/|/animations/|/16/|/16x16|/22|/24|@2x)", re.I)
DARK = re.compile(r"(dark|night|black)", re.I)
LIGHT_ONLY = re.compile(r"(light|white)", re.I)


def score(path: Path) -> int:
    """Prefer scalable and large sizes."""
    s = str(path)
    if "scalable" in s:
        return 1000
    m = re.search(r"/(\d+)(?:x\d+)?/", s)
    return int(m.group(1)) if m else 100


def index_svgs(repo: Path) -> dict:
    found: dict = {}
    for p in repo.rglob("*.svg"):
        rel = "/" + str(p.relative_to(repo))
        if SKIP.search(rel):
            continue
        try:
            real = p.resolve(strict=True)
        except (FileNotFoundError, RuntimeError):
            continue
        if not real.is_file() or real.stat().st_size < 100:
            continue
        variant = "dark" if DARK.search(rel) else "light"
        found.setdefault((p.stem, variant), []).append((score(p), real))
    return found


def pick(found: dict, names: list, variant: str):
    for name in names:
        options = found.get((name, variant))
        if options:
            return max(options, key=lambda o: o[0])[1]
    return None


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    manifest = []
    with tempfile.TemporaryDirectory() as tmp:
        for pack in PACKS:
            repo = Path(tmp) / pack["id"]
            print(f"== {pack['name']}: cloning…", flush=True)
            subprocess.run(["git", "clone", "--depth", "1", "-q", f"https://github.com/{pack['repo']}", str(repo)], check=True)
            found = index_svgs(repo)
            target = OUT / pack["id"]
            shutil.rmtree(target, ignore_errors=True)
            got = {"light": 0, "dark": 0}
            for variant in ("light", "dark"):
                for key, names in ICONS.items():
                    src = pick(found, names, variant)
                    if src is None and variant == "light":
                        src = pick(found, names, "dark")
                    if src is None:
                        continue
                    dest = target / variant / f"{key}.svg"
                    dest.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copyfile(src, dest)
                    got[variant] += 1
            for lic in ("LICENSE", "COPYING", "LICENSE.md", "LICENSE.txt"):
                if (repo / lic).exists():
                    shutil.copyfile(repo / lic, target / "LICENSE")
                    break
            has_dark = got["dark"] >= 10
            if not has_dark:
                shutil.rmtree(target / "dark", ignore_errors=True)
            print(f"   light {got['light']}/{len(ICONS)}, dark {got['dark'] if has_dark else 0}/{len(ICONS)}")
            manifest.append({
                "id": pack["id"], "name": pack["name"], "author": pack["author"],
                "url": f"https://github.com/{pack['repo']}", "license": "GPL-3.0",
                "hasDark": has_dark, "icons": sorted(p.stem for p in (target / "light").glob("*.svg")),
            })
    (OUT / "index.json").write_text(json.dumps(manifest, indent=2) + "\n")
    total = sum(f.stat().st_size for f in OUT.rglob("*") if f.is_file())
    print(f"Done: {len(manifest)} packs, {total / 1024:.0f} KB")
    return 0


if __name__ == "__main__":
    sys.exit(main())

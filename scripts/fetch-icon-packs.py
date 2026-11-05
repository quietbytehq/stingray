#!/usr/bin/env python3
"""Fetches the bundled icon packs and keeps only the icons Stingray uses.

For every pack this clones the repository (shallow), finds one SVG for each icon Stingray needs
(folders, home folders, trash and common file types), and copies it to
static/icon-packs/<id>/<light|dark>/<key>.svg with the pack's licence. Packs that only draw app
icons (Numix Circle, MoreWaita) take the icons they lack from a companion theme listed as "base".
Writes static/icon-packs/index.json, which the app reads.

Run from the repo root:  python3 scripts/fetch-icon-packs.py   (or name pack ids to refresh only those)
"""
import json, re, shutil, subprocess, sys, tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "static" / "icon-packs"
GH = "https://github.com/"

PACKS = [
    {"id": "papirus", "name": "Papirus", "author": "Papirus Development Team", "url": GH + "PapirusDevelopmentTeam/papirus-icon-theme", "license": "GPL-3.0"},
    {"id": "tela", "name": "Tela", "author": "Vince Liuice", "url": GH + "vinceliuice/Tela-icon-theme", "license": "GPL-3.0"},
    {"id": "whitesur", "name": "WhiteSur", "author": "Vince Liuice", "url": GH + "vinceliuice/WhiteSur-icon-theme", "license": "GPL-3.0"},
    {"id": "beautyline", "name": "BeautyLine", "author": "Elbullazul, Garuda Linux", "url": "https://gitlab.com/garuda-linux/themes-and-settings/artwork/beautyline", "license": "GPL-3.0"},
    {"id": "fluent", "name": "Fluent", "author": "Vince Liuice", "url": GH + "vinceliuice/Fluent-icon-theme", "license": "GPL-3.0"},
    {"id": "mcmojave", "name": "McMojave Circle", "author": "Vince Liuice", "url": GH + "vinceliuice/McMojave-circle", "license": "GPL-3.0"},
    {"id": "colloid", "name": "Colloid", "author": "Vince Liuice", "url": GH + "vinceliuice/Colloid-icon-theme", "license": "GPL-3.0"},
    {"id": "candy", "name": "Candy", "author": "EliverLara", "url": GH + "EliverLara/candy-icons", "license": "GPL-3.0"},
    {"id": "kora", "name": "Kora", "author": "bikass", "url": GH + "bikass/kora", "license": "GPL-3.0"},
    {"id": "qogir", "name": "Qogir", "author": "Vince Liuice", "url": GH + "vinceliuice/Qogir-icon-theme", "license": "GPL-3.0"},
    {"id": "reversal", "name": "Reversal", "author": "yeyushengfan258", "url": GH + "yeyushengfan258/Reversal-icon-theme", "license": "GPL-3.0"},
    {"id": "numix", "name": "Numix Circle", "author": "Numix Project", "url": GH + "numixproject/numix-icon-theme-circle", "license": "GPL-3.0", "base": GH + "numixproject/numix-icon-theme"},
    {"id": "morewaita", "name": "MoreWaita", "author": "somepaulo", "url": GH + "somepaulo/MoreWaita", "license": "GPL-3.0", "base": "https://gitlab.gnome.org/GNOME/adwaita-icon-theme"},
    {"id": "vimix", "name": "Vimix", "author": "Vince Liuice", "url": GH + "vinceliuice/Vimix-icon-theme", "license": "CC-BY-SA-4.0"},
    {"id": "breeze", "name": "Breeze", "author": "KDE", "url": GH + "KDE/breeze-icons", "license": "LGPL-2.1"},
    {"id": "newaita", "name": "Newaita Reborn", "author": "cbrnix", "url": GH + "cbrnix/Newaita-reborn", "license": "GPL-3.0"},
    {"id": "nordzy", "name": "Nordzy", "author": "alvatip", "url": GH + "alvatip/Nordzy-icon", "license": "GPL-3.0"},
    {"id": "gruvbox", "name": "Gruvbox Plus", "author": "SylEleuth", "url": GH + "SylEleuth/gruvbox-plus-icon-pack", "license": "GPL-3.0"},
]

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

SKIP = re.compile(r"(symbolic|/actions/|/apps/|/status/|/emblems/|/devices/|/panel/|/animations/|/16/|/16x16|/22|/24|@2x|/src/.*template)", re.I)
DARK = re.compile(r"(dark|night|black)", re.I)


def score(p: Path) -> int:
    s = str(p)
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


def clone(url: str, dest: Path) -> bool:
    r = subprocess.run(["git", "clone", "--depth", "1", "-q", url, str(dest)], capture_output=True, text=True)
    if r.returncode != 0:
        print(f"   could not download {url}: {r.stderr.strip()[:200]}")
        return False
    return True


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    only = set(sys.argv[1:])
    old = {p["id"]: p for p in json.loads((OUT / "index.json").read_text())} if (OUT / "index.json").exists() else {}
    manifest = []
    with tempfile.TemporaryDirectory() as tmp:
        for pack in PACKS:
            if only and pack["id"] not in only:
                if pack["id"] in old:
                    manifest.append(old[pack["id"]])
                continue
            print(f"== {pack['name']}: downloading…", flush=True)
            repo = Path(tmp) / pack["id"]
            if not clone(pack["url"], repo):
                if pack["id"] in old:
                    manifest.append(old[pack["id"]])
                continue
            found = index_svgs(repo)
            base_found = {}
            if pack.get("base") and clone(pack["base"], Path(tmp) / (pack["id"] + "-base")):
                base_found = index_svgs(Path(tmp) / (pack["id"] + "-base"))
            target = OUT / pack["id"]
            shutil.rmtree(target, ignore_errors=True)
            got = {"light": 0, "dark": 0}
            for variant in ("light", "dark"):
                for key, names in ICONS.items():
                    src = pick(found, names, variant) or (pick(found, names, "dark") if variant == "light" else None)
                    if src is None and base_found:
                        src = pick(base_found, names, variant) or pick(base_found, names, "light")
                    if src is None:
                        continue
                    dest = target / variant / f"{key}.svg"
                    dest.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copyfile(src, dest)
                    got[variant] += 1
            for lic in ("LICENSE", "COPYING", "LICENSE.md", "LICENSE.txt", "COPYING.LIB"):
                if (repo / lic).exists():
                    shutil.copyfile(repo / lic, target / "LICENSE")
                    break
            if got["light"] < 12:
                print(f"   only {got['light']} icons found; leaving {pack['name']} out")
                shutil.rmtree(target, ignore_errors=True)
                continue
            has_dark = got["dark"] >= 10
            if not has_dark:
                shutil.rmtree(target / "dark", ignore_errors=True)
            print(f"   {got['light']}/{len(ICONS)} icons{', with dark variants' if has_dark else ''}")
            manifest.append({
                "id": pack["id"], "name": pack["name"], "author": pack["author"],
                "url": pack["url"], "license": pack["license"], "hasDark": has_dark,
                "icons": sorted(p.stem for p in (target / "light").glob("*.svg")),
            })
    (OUT / "index.json").write_text(json.dumps(manifest, indent=2) + "\n")
    total = sum(f.stat().st_size for f in OUT.rglob("*") if f.is_file())
    print(f"Done: {len(manifest)} packs, {total / 1024:.0f} KB")
    return 0


if __name__ == "__main__":
    sys.exit(main())

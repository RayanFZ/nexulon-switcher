"""End-to-end sandbox test for the switching engine.

Runs a built binary against a throwaway data directory so no real launcher
login is ever touched. Exercises add -> switch -> rename -> notes ->
delete (to trash) -> restore.

Set NEXULON_EXE to test a specific build; it defaults to the debug binary.
"""

from __future__ import annotations

import base64
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EXE = Path(os.environ.get("NEXULON_EXE", ROOT / "target" / "debug" / "nexulon-switcher.exe"))
SANDBOX = Path(os.environ.get("TEMP", r"D:\Tmp")) / "nexulon-sandbox"
LIVE = SANDBOX / "live"
DATA = SANDBOX / "data"


def write(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def run(method: str, params: dict | None = None) -> dict:
    args = [str(EXE), "--dump", method]
    if params is not None:
        args.append(json.dumps(params))
    env = dict(os.environ)
    env["NEXULON_DATA_DIR"] = str(DATA)
    result = subprocess.run(args, capture_output=True, text=True, env=env, timeout=120)
    if result.returncode != 0:
        raise SystemExit(f"{method} failed ({result.returncode}): {result.stderr}")
    text = result.stdout.strip()
    if not text:
        raise SystemExit(f"{method} produced no output: {result.stderr}")
    return json.loads(text)


def expect(label: str, actual, wanted) -> None:
    if actual != wanted:
        raise SystemExit(f"FAIL {label}: expected {wanted!r}, got {actual!r}")
    print(f"ok   {label}: {actual!r}")


def main() -> None:
    if not EXE.exists():
        raise SystemExit(f"build the debug binary first: {EXE}")

    if SANDBOX.exists():
        shutil.rmtree(SANDBOX)
    DATA.mkdir(parents=True)
    LIVE.mkdir(parents=True)

    platforms = {
        "Version": "sandbox",
        "Platforms": {
            "TestApp": {
                "Identifiers": ["t"],
                "ExeLocationDefault": "",
                "ExesToEnd": [],
                "PathListToClear": ["SAME_AS_LOGIN_FILES"],
                "LoginFiles": {
                    str(LIVE / "login.dat"): "login.dat",
                    str(LIVE / "sub" / "*.cfg"): "sub",
                    str(LIVE / "id.txt"): "id.txt",
                },
                "AllFilesRequired": False,
                "UniqueIdFile": str(LIVE / "id.txt"),
                "UniqueIdMethod": "CREATE_ID_FILE",
            }
        },
    }
    write(DATA / "Platforms.json", json.dumps(platforms, indent=2))
    write(DATA / "WindowSettings.json", json.dumps({"EnabledBasicPlatforms": ["t"]}))

    def live_state() -> tuple[str, str]:
        return read(LIVE / "login.dat"), read(LIVE / "sub" / "a.cfg")

    # --- account one -----------------------------------------------------
    write(LIVE / "login.dat", "AAA")
    write(LIVE / "sub" / "a.cfg", "A1")
    write(LIVE / "id.txt", "IDTEST01")
    expect("add first", run("add", {"platform": "TestApp", "name": "First"})["name"], "First")
    expect("stored login", read(DATA / "LoginCache/TestApp/First/login.dat"), "AAA")
    expect("stored glob", read(DATA / "LoginCache/TestApp/First/sub/a.cfg"), "A1")

    # --- account two -----------------------------------------------------
    write(LIVE / "login.dat", "BBB")
    write(LIVE / "sub" / "a.cfg", "B1")
    write(LIVE / "id.txt", "IDTEST02")
    expect("add second", run("add", {"platform": "TestApp", "name": "Second"})["name"], "Second")

    listing = run("accounts", {"platform": "TestApp"})
    expect("account count", len(listing["accounts"]), 2)
    expect("current is second", listing["current"]["name"], "Second")

    # --- switch back to first --------------------------------------------
    result = run("switch", {"platform": "TestApp", "account": "First", "clear": True, "start": False})
    expect("switch account", result["account"], "First")
    expect("switch id", result["id"], "IDTEST01")
    expect("live restored", live_state(), ("AAA", "A1"))
    expect("current after switch", run("accounts", {"platform": "TestApp"})["current"]["name"], "First")

    # --- rename / notes --------------------------------------------------
    expect(
        "rename",
        run("rename", {"platform": "TestApp", "account": "First", "new_name": "Renamed"})["name"],
        "Renamed",
    )
    run("notes", {"platform": "TestApp", "account": "Renamed", "notes": "hello"})
    renamed = [a for a in run("accounts", {"platform": "TestApp"})["accounts"] if a["name"] == "Renamed"]
    expect("notes stored", renamed[0]["notes"], "hello")
    expect("renamed id", renamed[0]["id"], "IDTEST01")

    # --- artwork (avatar + cover) ----------------------------------------
    picture = SANDBOX / "picture.png"
    picture.write_bytes(
        base64.b64decode(
            "iVBORw0KGgoAAAANSUhEUgAAAAgAAAAIAQMAAAD+wSzIAAAABlBMVEX///+/v7+jQ3Y5"
            "AAAADklEQVQI12P4AIX8EAgALgAD/aNpbtEAAAAASUVORK5CYII="
        )
    )
    painted = run(
        "artwork_set",
        {"platform": "TestApp", "account": "Second", "kind": "avatar", "path": str(picture)},
    )
    expect("avatar data uri", painted["avatar"].startswith("data:image/png;base64,"), True)
    painted = run(
        "artwork_set",
        {"platform": "TestApp", "account": "Second", "kind": "cover", "path": str(picture)},
    )
    expect("cover data uri", painted["cover"].startswith("data:image/png;base64,"), True)

    # Artwork is keyed by name, so a rename must carry it across.
    run("rename", {"platform": "TestApp", "account": "Second", "new_name": "Third"})
    moved = [a for a in run("accounts", {"platform": "TestApp"})["accounts"] if a["name"] == "Third"]
    expect("artwork survives rename", bool(moved[0]["avatar"]) and bool(moved[0]["cover"]), True)
    index = json.loads(read(DATA / "Artwork/TestApp/index.json"))
    expect("artwork re-keyed", sorted(index), ["Third"])

    run("artwork_clear", {"platform": "TestApp", "account": "Third", "kind": "cover"})
    cleared = [a for a in run("accounts", {"platform": "TestApp"})["accounts"] if a["name"] == "Third"]
    expect("cover cleared", cleared[0]["cover"], None)
    expect("avatar kept", bool(cleared[0]["avatar"]), True)

    # --- delete to trash, then restore -----------------------------------
    trashed = run("delete", {"platform": "TestApp", "account": "Renamed"})
    expect("delete returns a trash token", bool(trashed.get("trash")), True)
    expect(
        "trashed folder exists",
        (DATA / "LoginCache/TestApp/.trash" / trashed["trash"]).is_dir(),
        True,
    )
    remaining = run("accounts", {"platform": "TestApp"})["accounts"]
    expect("remaining accounts", [a["name"] for a in remaining], ["Third"])

    run("restore", {"platform": "TestApp", "account": "Renamed", "trash": trashed["trash"]})
    restored = sorted(a["name"] for a in run("accounts", {"platform": "TestApp"})["accounts"])
    expect("restored accounts", restored, ["Renamed", "Third"])
    expect(
        "restored id mapping",
        [a["id"] for a in run("accounts", {"platform": "TestApp"})["accounts"] if a["name"] == "Renamed"][0],
        "IDTEST01",
    )

    # --- inventory -------------------------------------------------------
    boot = run("bootstrap")
    by_name = {p["name"]: p for p in boot["platforms"]}
    expect("bootstrap has test app", "TestApp" in by_name, True)
    expect("bootstrap account names", sorted(by_name["TestApp"]["accountNames"]), ["Renamed", "Third"])
    expect("bootstrap merged the platform pack", len(boot["platforms"]) >= 28, True)
    expect("bootstrap version", boot["version"], "1.0.0")
    expect("bootstrap data root", Path(boot["dataRoot"]), DATA)
    expect("autostart reports a boolean", isinstance(run("autostart")["enabled"], bool), True)

    enabled = json.loads(read(DATA / "WindowSettings.json"))["EnabledBasicPlatforms"]
    expect("enabled keeps the test id", "t" in enabled, True)
    expect("enabled gained the new platforms", "s" in enabled and "cc" in enabled, True)

    print("\nALL SANDBOX CHECKS PASSED")
    shutil.rmtree(SANDBOX, ignore_errors=True)


if __name__ == "__main__":
    sys.exit(main())

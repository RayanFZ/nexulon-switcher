"""Small Win32 helpers for exercising the app window.

Usage:
    python tools/winutil.py resize <width> <height>
    python tools/winutil.py click <client-x> <client-y>
    python tools/winutil.py wheel <client-x> <client-y> <ticks>
"""

from __future__ import annotations

import ctypes
import sys
import time
from ctypes import wintypes

user32 = ctypes.WinDLL("user32", use_last_error=True)

TITLE = "Nexulon Switcher"


class RECT(ctypes.Structure):
    _fields_ = [
        ("left", wintypes.LONG),
        ("top", wintypes.LONG),
        ("right", wintypes.LONG),
        ("bottom", wintypes.LONG),
    ]


class POINT(ctypes.Structure):
    _fields_ = [("x", wintypes.LONG), ("y", wintypes.LONG)]


user32.FindWindowW.restype = wintypes.HWND


def hwnd() -> int:
    handle = user32.FindWindowW(None, TITLE)
    if not handle:
        raise SystemExit("window not found")
    return handle


def to_screen(handle: int, x: int, y: int) -> tuple[int, int]:
    point = POINT(x, y)
    user32.ClientToScreen(handle, ctypes.byref(point))
    return point.x, point.y


def focus(handle: int) -> None:
    """Bring the app forward before synthetic input; clicks go to whatever is on top."""
    user32.ShowWindow(handle, 9)  # SW_RESTORE
    user32.SetWindowPos(handle, -1, 0, 0, 0, 0, 0x0003)  # topmost, keep size/pos
    user32.SetForegroundWindow(handle)
    user32.SetFocus(handle)
    time.sleep(0.25)


def resize(width: int, height: int) -> None:
    handle = hwnd()
    focus(handle)
    outer = RECT()
    client = RECT()
    user32.GetWindowRect(handle, ctypes.byref(outer))
    user32.GetClientRect(handle, ctypes.byref(client))
    frame_w = (outer.right - outer.left) - (client.right - client.left)
    frame_h = (outer.bottom - outer.top) - (client.bottom - client.top)
    user32.SetWindowPos(
        handle, 0, 40, 40, width + frame_w, height + frame_h, 0x0004
    )
    time.sleep(0.6)


def click(x: int, y: int) -> None:
    handle = hwnd()
    focus(handle)
    sx, sy = to_screen(handle, x, y)
    user32.SetCursorPos(sx, sy)
    time.sleep(0.15)
    user32.mouse_event(0x0002, 0, 0, 0, 0)
    user32.mouse_event(0x0004, 0, 0, 0, 0)
    time.sleep(0.5)


def wheel(x: int, y: int, ticks: int) -> None:
    handle = hwnd()
    focus(handle)
    sx, sy = to_screen(handle, x, y)
    user32.SetCursorPos(sx, sy)
    time.sleep(0.15)
    user32.mouse_event(0x0800, 0, 0, ticks * 120, 0)
    time.sleep(0.5)


def main(argv: list[str]) -> None:
    command = argv[1] if len(argv) > 1 else ""
    if command == "resize":
        resize(int(argv[2]), int(argv[3]))
    elif command == "click":
        click(int(argv[2]), int(argv[3]))
    elif command == "wheel":
        wheel(int(argv[2]), int(argv[3]), int(argv[4]))
    else:
        raise SystemExit(__doc__)


if __name__ == "__main__":
    main(sys.argv)

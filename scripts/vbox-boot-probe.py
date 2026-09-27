#!/usr/bin/env python3
"""Headless VirtualBox probe for NeoDOS.

Adapted from the ad-hoc ``vbox_neotop.py`` / ``vbox_cmd_time.py`` harnesses used
during local VirtualBox debugging. Start a VM headless, wait for the guest
shell prompt, optionally inject commands over PS/2, and dump the serial log.

Requires ``VBoxManage`` in PATH. The VM must already exist (create it with
``neodev vm create --backend virtualbox``).
"""
from __future__ import annotations

import argparse
import re
import subprocess
import sys
import time
from pathlib import Path

PROMPT = "C:\\> "

# US keyboard scancode set 1 (make codes).
SCANCODES = {
    "a": 0x1E, "b": 0x30, "c": 0x2E, "d": 0x20, "e": 0x12, "f": 0x21,
    "g": 0x22, "h": 0x23, "i": 0x17, "j": 0x24, "k": 0x25, "l": 0x26,
    "m": 0x32, "n": 0x31, "o": 0x18, "p": 0x19, "q": 0x10, "r": 0x13,
    "s": 0x1F, "t": 0x14, "u": 0x16, "v": 0x2F, "w": 0x11, "x": 0x2D,
    "y": 0x15, "z": 0x2C, " ": 0x39, ".": 0x34, "\\": 0x2B, ":": 0x27,
    "-": 0x0C, "_": 0x0C, "/": 0x35, "0": 0x0B, "1": 0x02, "2": 0x03,
    "3": 0x04, "4": 0x05, "5": 0x06, "6": 0x07, "7": 0x08, "8": 0x09,
    "9": 0x0A,
}
SHIFT = 0x2A
ENTER = 0x1C


def vm(*args: str) -> subprocess.CompletedProcess:
    return subprocess.run(["VBoxManage", *args], capture_output=True, text=True)


def state(name: str) -> str:
    out = vm("showvminfo", name, "--machinereadable").stdout
    m = re.search(r'VMState="(\w+)"', out)
    return m.group(1) if m else "?"


def wait_state(name: str, target: str, timeout: int = 60) -> bool:
    deadline = time.time() + timeout
    while time.time() < deadline:
        if state(name) == target:
            return True
        time.sleep(1)
    return False


def poweroff(name: str) -> None:
    if state(name) == "running":
        vm("controlvm", name, "acpipowerbutton")
        if not wait_state(name, "poweroff", 40):
            vm("controlvm", name, "poweroff")
            wait_state(name, "poweroff", 40)


def read_serial(path: Path) -> str:
    try:
        return path.read_bytes().decode("utf-8", "replace")
    except OSError:
        return ""


def send_text(name: str, text: str) -> None:
    codes: list[str] = []
    for ch in text:
        shift = ch.isupper() or ch in ':"_'
        code = SCANCODES.get(ch.lower())
        if code is None:
            continue
        if shift:
            codes.append(f"{SHIFT:02x}")
        codes += [f"{code:02x}", f"{(code | 0x80):02x}"]
        if shift:
            codes.append(f"{(SHIFT | 0x80):02x}")
    if codes:
        vm("controlvm", name, "keyboardputscancode", *codes)


def send_enter(name: str) -> None:
    vm("controlvm", name, "keyboardputscancode", f"{ENTER:02x}", f"{(ENTER | 0x80):02x}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--vm", default="NeoDOS", help="VirtualBox VM name")
    parser.add_argument("--serial", required=True, help="guest serial log path")
    parser.add_argument("--timeout", type=int, default=240, help="boot timeout (s)")
    parser.add_argument("--cpus", type=int, help="set CPU count before boot")
    parser.add_argument("--cmd", action="append", default=[], help="command to run after boot (repeatable)")
    parser.add_argument("--wait", type=float, default=8.0, help="wait after each command (s)")
    parser.add_argument("--screenshot", help="save a screenshot PNG here")
    args = parser.parse_args()

    serial = Path(args.serial)
    serial.write_text("")

    poweroff(args.vm)
    if args.cpus is not None:
        vm("modifyvm", args.vm, "--cpus", str(args.cpus))

    print(f"[*] starting '{args.vm}' headless")
    started = vm("startvm", args.vm, "--type", "headless")
    if started.returncode != 0:
        print(started.stderr.strip() or started.stdout.strip(), file=sys.stderr)
        return 1

    deadline = time.time() + args.timeout
    booted = False
    while time.time() < deadline:
        if PROMPT in read_serial(serial):
            booted = True
            break
        if state(args.vm) not in ("running", "paused"):
            break
        time.sleep(3)

    if not booted:
        print("[!] shell prompt not reached before timeout", file=sys.stderr)
        if args.screenshot:
            vm("controlvm", args.vm, "screenshotpng", args.screenshot)
        poweroff(args.vm)
        return 2

    print("[+] shell prompt reached")
    for command in args.cmd:
        print(f"[*] sending: {command}")
        send_text(args.vm, command)
        send_enter(args.vm)
        time.sleep(args.wait)

    if args.screenshot:
        vm("controlvm", args.vm, "screenshotpng", args.screenshot)

    poweroff(args.vm)

    text = read_serial(serial)
    print("===== serial grep =====")
    for pattern in (r"NeoDOS Kernel v", r"All \d+ kernel tests passed", r"\.\.\. FAIL", r"PANIC", r"GPF"):
        hits = [l for l in text.splitlines() if re.search(pattern, l)]
        for line in hits[-3:]:
            print(f"  {line.strip()}")
    print(f"serial log: {serial}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

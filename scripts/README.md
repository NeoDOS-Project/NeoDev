# NeoDev helper scripts

Reusable validation harnesses recovered from local `/tmp` workflows and adapted
to be path-agnostic so they can support future NeoDev improvements.

| Script | Purpose |
|--------|---------|
| `vbox-vdi-sync-check.sh` | Acceptance/regression check that `neodev test --backend virtualbox` regenerates a stale `disk_image.vdi`. |
| `vbox-boot-probe.py` | Headless VirtualBox smoke probe: start the VM, wait for the guest prompt, optionally inject shell commands and dump the serial log. |

## Environment

Both scripts discover the NeoDOS checkout from `NEODOS_ROOT` (or an explicit
flag/argument) and never hardcode a user path.

```bash
export NEODOS_ROOT=/path/to/neodos
```

`vbox-vdi-sync-check.sh` uses the `neodev` binary it finds, in this order:
`--neodev PATH`, `$NEODEV_BIN`, `neodev` on `PATH`, `target/debug/neodev` next to
this repository.

## `vbox-vdi-sync-check.sh`

```bash
# Force IMG to be newer than VDI, then run the VirtualBox test suite.
scripts/vbox-vdi-sync-check.sh --neodos-root "$NEODOS_ROOT"

# Rebuild the image first (neodev build --quick --image) instead of `touch`.
scripts/vbox-vdi-sync-check.sh --build --timeout 240
```

It records `IMG`/`VDI` mtimes before and after, runs the test, and asserts the
VDI is at least as new as the raw image (the contract from
`docs/development/virtualbox.md`). Exit status is non-zero on failure.

## `vbox-boot-probe.py`

```bash
# Boot VM 'NeoDOS' headless and capture its serial log.
scripts/vbox-boot-probe.py --vm NeoDOS --serial "$NEODOS_ROOT/vbox_serial.log"

# Run `ver` after the shell appears and snapshot the screen.
scripts/vbox-boot-probe.py --vm NeoDOS --cmd ver --screenshot /tmp/neodos.png
```

Requires `VBoxManage` in `PATH`. The VM must already exist (create it with
`neodev vm create --backend virtualbox`).

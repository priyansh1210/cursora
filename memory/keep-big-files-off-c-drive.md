---
name: keep-big-files-off-c-drive
description: "On the laptop (PRIYANSH), C: (256 GB SSD) is nearly full; put large installs, VMs and build output on D: (1 TB HDD)"
metadata:
  type: user
---

This is about the laptop (hostname `PRIYANSH`), not the stronger PC ([[dev-machines]]). C: is a 256 GB NVMe SSD with ~44 GB free; D: is a 1 TB HDD with ~490 GB free. The user does not want large things on C:.

**Why:** "C drive is very filled up". They chose D: even though it is slower.

**How to apply:** Install large tools, VMs, distros and build output on D:. The WSL distro `Ubuntu-24.04` (user `priyansh`) lives at `D:\WSL\Ubuntu-24.04`, and `npm run test:linux` builds inside it. Before anything grows C: by more than a few GB, ask first. Machine: 8 GB RAM, i5-11300H, Windows 11 Home (no Hyper-V Manager).

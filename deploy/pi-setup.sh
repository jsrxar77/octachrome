#!/usr/bin/env bash
# One-time Raspberry Pi setup (run as root on the Pi):
#   sudo bash pi-setup.sh <dir containing octachrome, octachrome.service, 99-maschine-mk3.rules>
set -euo pipefail
SRC="${1:-.}"

id octachrome >/dev/null 2>&1 || useradd --system --create-home --groups audio,plugdev octachrome

install -m 0755 "$SRC/octachrome" /usr/local/bin/octachrome
install -m 0644 "$SRC/99-maschine-mk3.rules" /etc/udev/rules.d/99-maschine-mk3.rules
install -m 0644 "$SRC/octachrome.service" /etc/systemd/system/octachrome.service
udevadm control --reload-rules && udevadm trigger

# Real-time limits for the audio group.
cat > /etc/security/limits.d/95-octachrome-audio.conf <<'LIMITS'
@audio - rtprio 95
@audio - memlock unlimited
LIMITS

# Boot to console, no desktop.
systemctl set-default multi-user.target

# Full CPU speed: no frequency ramp-up glitches.
if [ -w /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor ]; then
  for g in /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor; do echo performance > "$g"; done
  cat > /etc/systemd/system/cpu-performance.service <<'UNIT'
[Unit]
Description=CPU governor: performance
[Service]
Type=oneshot
ExecStart=/bin/sh -c 'for g in /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor; do echo performance > $g; done'
[Install]
WantedBy=multi-user.target
UNIT
  systemctl enable cpu-performance.service
fi

systemctl daemon-reload
systemctl enable --now octachrome.service
echo "Octachrome installed. Logs: journalctl -u octachrome -f"

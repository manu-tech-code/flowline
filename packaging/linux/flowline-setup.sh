#!/bin/sh
# One-time setup so Flowline's shortcut and paste work on Linux.
#
#   sh flowline-setup.sh
#
# It does two things, both with sudo, and nothing else:
#   1. adds you to the "input" group, so Flowline can read your keyboard for the
#      global hold-to-talk shortcut (this works on X11 and on Wayland);
#   2. lets that group use /dev/uinput, so Flowline can press Ctrl+V for you.
# Then log out and back in once for the group change to apply.
set -eu

if [ "$(id -u)" = 0 ]; then
  echo "Run this as your normal user (it asks for sudo itself)." >&2
  exit 1
fi

echo "Flowline setup for user: $USER"
echo "This adds you to the 'input' group and a udev rule for /dev/uinput."
printf "Continue? [y/N] "
read -r answer
case "$answer" in y|Y|yes|YES) ;; *) echo "Cancelled."; exit 0 ;; esac

sudo modprobe uinput || true
echo uinput | sudo tee /etc/modules-load.d/flowline.conf >/dev/null
sudo usermod -aG input "$USER"
echo 'KERNEL=="uinput", GROUP="input", MODE="0660", OPTIONS+="static_node=uinput"' \
  | sudo tee /etc/udev/rules.d/99-flowline.rules >/dev/null
sudo udevadm control --reload
sudo udevadm trigger

echo
echo "Done. Log out and back in, then start Flowline."
echo "To undo:  sudo rm /etc/udev/rules.d/99-flowline.rules /etc/modules-load.d/flowline.conf"
echo "          sudo gpasswd -d $USER input"

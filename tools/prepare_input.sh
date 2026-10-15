#!/bin/sh
# Root-only kernel/permission preparation; never creates an input or sensor session.
set -eu
[ "$(id -u)" = 0 ] && [ "$#" = 1 ] || { echo 'Usage as root: prepare_input.sh OPERATOR' >&2; exit 1; }
operator=$1
case "$operator" in *[!a-zA-Z0-9_-]*|'') exit 1;; esac
getent passwd "$operator" >/dev/null
rule=$(printf 'KERNEL=="uinput", SUBSYSTEM=="misc", RUN+="/usr/bin/setfacl -m u:%s:rw /dev/uinput"' "$operator")
grep -Fxq "$rule" /etc/udev/rules.d/80-wigglecontrol-uinput.rules || { echo 'The matching installed named-operator udev rule is required' >&2; exit 1; }
backup=/var/lib/wigglecontrol/rollback
boot=/etc/modules-load.d/wigglecontrol.conf
[ -f "$backup/manifest" ] && [ -f "$backup/uinput.acl" ] || { echo 'Run the full installer first; an original rollback is required' >&2; exit 1; }

# An upgrade must enroll this new path before replacing an existing boot file.
# Preserve the first baseline on retries, including an interrupted extension.
if grep -Fxq "restore $boot" "$backup/manifest"; then
 [ -e "$backup/files$boot" ] || [ -L "$backup/files$boot" ] || { echo 'Boot rollback snapshot incomplete' >&2; exit 1; }
elif ! grep -Fxq "remove $boot" "$backup/manifest"; then
 next=$(mktemp "$backup/manifest.new.XXXXXX")
 trap 'rm -f "$next"' EXIT HUP INT TERM
 cp "$backup/manifest" "$next"
 if [ -e "$boot" ] || [ -L "$boot" ]; then
  mkdir -p "$backup/files/etc/modules-load.d"
  cp -a "$boot" "$backup/files$boot"
  printf 'restore %s\n' "$boot" >> "$next"
 else
  printf 'remove %s\n' "$boot" >> "$next"
 fi
 mv "$next" "$backup/manifest"
 trap - EXIT HUP INT TERM
fi

# The driver must register before a boot-time udev rule can grant access to it.
udevadm control --reload-rules
modprobe uinput
udevadm settle --timeout=10
install -d /etc/modules-load.d
next=$(mktemp /etc/modules-load.d/wigglecontrol.conf.new.XXXXXX)
trap 'rm -f "$next"' EXIT HUP INT TERM
printf '# Kernel interface only; no camera, model or input session.\nuinput\n' > "$next"
chmod 644 "$next"
mv "$next" "$boot"
trap - EXIT HUP INT TERM
setfacl -m "u:$operator:rw" /dev/uinput
echo 'Prepared uinput module and named-operator access; no session started'

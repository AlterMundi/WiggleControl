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
keyboard=/etc/udev/rules.d/80-wigglecontrol-keyboard.rules
keyboard_rule_source="$(dirname "$0")/../packaging/80-wigglecontrol-keyboard.rules"
[ -f "$keyboard_rule_source" ] || { echo "Keyboard classification rule is missing" >&2; exit 1; }
[ -f "$backup/manifest" ] && [ -f "$backup/uinput.acl" ] || { echo 'Run the full installer first; an original rollback is required' >&2; exit 1; }

# Enroll each managed path before replacement, preserving the initial baseline.
# Older installations may have neither entry in their retained manifest.
for prepared_path in "$boot" "$keyboard"; do
 if grep -Fxq "restore $prepared_path" "$backup/manifest"; then
  [ -e "$backup/files$prepared_path" ] || [ -L "$backup/files$prepared_path" ] || { echo 'Input preparation rollback snapshot incomplete' >&2; exit 1; }
 elif ! grep -Fxq "remove $prepared_path" "$backup/manifest"; then
  next=$(mktemp "$backup/manifest.new.XXXXXX")
  trap 'rm -f "$next"' EXIT HUP INT TERM
  cp "$backup/manifest" "$next"
  if [ -e "$prepared_path" ] || [ -L "$prepared_path" ]; then
   mkdir -p "$backup/files$(dirname "$prepared_path")"
   cp -a "$prepared_path" "$backup/files$prepared_path"
   printf 'restore %s\n' "$prepared_path" >> "$next"
  else
   printf 'remove %s\n' "$prepared_path" >> "$next"
  fi
  mv "$next" "$backup/manifest"
  trap - EXIT HUP INT TERM
 fi
done

install -m 644 "$keyboard_rule_source" "$keyboard.new"
mv "$keyboard.new" "$keyboard"

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

#!/bin/sh
# Root-only kernel/permission preparation; never creates an input or sensor session.
set -eu
[ "$(id -u)" = 0 ] || { echo 'Run as receiving administrator' >&2; exit 1; }
standalone=false
if [ "${1:-}" = --standalone ]; then standalone=true; shift; fi
[ "$#" = 1 ] || { echo 'Usage as root: prepare_input.sh [--standalone] OPERATOR' >&2; exit 1; }
operator=$1
case "$operator" in *[!a-zA-Z0-9_-]*|'') exit 1;; esac
getent passwd "$operator" >/dev/null
rule=$(printf 'KERNEL=="uinput", SUBSYSTEM=="misc", RUN+="/usr/bin/setfacl -m u:%s:rw /dev/uinput"' "$operator")
backup=/var/lib/wigglecontrol/rollback
boot=/etc/modules-load.d/wigglecontrol.conf
keyboard=/etc/udev/rules.d/80-wigglecontrol-keyboard.rules
grant=/etc/udev/rules.d/80-wigglecontrol-uinput.rules
keyboard_rule_source="$(dirname "$0")/../packaging/80-wigglecontrol-keyboard.rules"
[ -f "$keyboard_rule_source" ] || { echo "Keyboard classification rule is missing" >&2; exit 1; }
if "$standalone"; then
 backup=/var/lib/wigglecontrol/preview-input-rollback
 boot=/etc/modules-load.d/wigglecontrol-preview.conf
 keyboard=/etc/udev/rules.d/81-wigglecontrol-preview-keyboard.rules
 grant=/etc/udev/rules.d/81-wigglecontrol-preview-uinput.rules
 install -d -m 700 /var/lib/wigglecontrol
 exec 9>/var/lib/wigglecontrol/.preview-input.lock
 flock -x 9
 if [ ! -e "$backup/manifest" ]; then
  [ ! -e "$backup" ] || { echo 'Inspect incomplete preview input rollback' >&2; exit 1; }
  for path in "$boot" "$keyboard" "$grant"; do
   [ ! -e "$path" ] && [ ! -L "$path" ] || { echo "Preserve existing file: $path" >&2; exit 1; }
  done
  modprobe uinput
  udevadm settle --timeout=10
  stage=$(mktemp -d /var/lib/wigglecontrol/preview-input-rollback.new.XXXXXX)
  trap 'rm -rf "$stage"' EXIT HUP INT TERM
  getfacl -p /dev/uinput > "$stage/uinput.acl"
  getfacl -cpn /dev/uinput > "$stage/uinput.before"
  printf '%s:%s\n' "$operator" "$(id -u "$operator")" > "$stage/operator"
  # Calculate the expected ACL on an ordinary private file, without touching
  # the device. This lets retries/removal distinguish our grant from later edits.
  : > "$stage/acl-probe"
  setfacl -n --set-file="$stage/uinput.before" "$stage/acl-probe"
  setfacl -m "u:$operator:rw" "$stage/acl-probe"
  getfacl -cpn "$stage/acl-probe" > "$stage/uinput.expected"
  rm "$stage/acl-probe"
  mkdir "$stage/expected"
  printf '# Kernel interface only; no camera, model or input session.\nuinput\n' > "$stage/expected/boot"
  cp "$keyboard_rule_source" "$stage/expected/keyboard"
  printf '%s\n' "$rule" > "$stage/expected/grant"
  printf 'remove %s\n' "$boot" "$keyboard" "$grant" > "$stage/manifest"
  mv "$stage" "$backup"
  trap - EXIT HUP INT TERM
 fi
 [ "$(cat "$backup/operator")" = "$operator:$(id -u "$operator")" ] || { echo 'Preview input belongs to another operator' >&2; exit 1; }
 expected=$(printf 'remove %s\n' "$boot" "$keyboard" "$grant")
 [ "$(cat "$backup/manifest")" = "$expected" ] || { echo 'Unexpected preview rollback inventory' >&2; exit 1; }
 for role in boot keyboard grant; do
  case "$role" in boot) path=$boot;; keyboard) path=$keyboard;; grant) path=$grant;; esac
  [ ! -L "$path" ] && { [ ! -e "$path" ] || cmp -s "$backup/expected/$role" "$path"; } || { echo "Preserve changed file: $path" >&2; exit 1; }
 done
 # A changed shipped rule needs an explicit migration, not a silent overwrite.
 cmp -s "$keyboard_rule_source" "$backup/expected/keyboard" || { echo 'Keyboard rule differs from retained installation' >&2; exit 1; }
 [ -f "$backup/uinput.acl" ] || { echo 'Preview input snapshot incomplete' >&2; exit 1; }
 modprobe uinput
 udevadm settle --timeout=10
 current=$(getfacl -cpn /dev/uinput)
 [ "$current" = "$(cat "$backup/uinput.before")" ] || [ "$current" = "$(cat "$backup/uinput.expected")" ] || { echo 'Preserve independently changed uinput ACL' >&2; exit 1; }
 install -d /etc/udev/rules.d /etc/modules-load.d
 install -m 644 "$backup/expected/grant" "$grant.new"
 mv "$grant.new" "$grant"
else
 grep -Fxq "$rule" "$grant" || { echo 'The matching installed named-operator udev rule is required' >&2; exit 1; }
fi
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

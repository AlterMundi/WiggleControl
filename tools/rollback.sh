#!/bin/sh
# Stop the owning finite session first; no other application/service is stopped here.
set -eu
[ "$(id -u)" = 0 ] || { echo 'Run as receiving administrator' >&2; exit 1; }
backup=/var/lib/wigglecontrol/rollback
standalone=false
if [ "$#" = 1 ] && [ "$1" = --standalone ]; then
 standalone=true
 backup=/var/lib/wigglecontrol/preview-input-rollback
 exec 9>/var/lib/wigglecontrol/.preview-input.lock
 flock -x 9
 # Validate every selected path/ACL before removing anything. Other operators'
 # permissions and administration changes are not ours to restore over.
 for role in boot keyboard grant; do
  case "$role" in
   boot) path=/etc/modules-load.d/wigglecontrol-preview.conf;;
   keyboard) path=/etc/udev/rules.d/81-wigglecontrol-preview-keyboard.rules;;
   grant) path=/etc/udev/rules.d/81-wigglecontrol-preview-uinput.rules;;
  esac
  [ ! -L "$path" ] && { [ ! -e "$path" ] || cmp -s "$backup/expected/$role" "$path"; } || { echo "Preserve changed file: $path" >&2; exit 1; }
 done
 expected=$(printf 'remove %s\n' /etc/modules-load.d/wigglecontrol-preview.conf /etc/udev/rules.d/81-wigglecontrol-preview-keyboard.rules /etc/udev/rules.d/81-wigglecontrol-preview-uinput.rules)
 [ "$(cat "$backup/manifest")" = "$expected" ] || { echo 'Unexpected preview rollback inventory' >&2; exit 1; }
 current=$(getfacl -cpn /dev/uinput)
 [ "$current" = "$(cat "$backup/uinput.before")" ] || [ "$current" = "$(cat "$backup/uinput.expected")" ] || { echo 'Preserve independently changed uinput ACL' >&2; exit 1; }
elif [ "$#" != 0 ]; then
 echo 'Usage as root: rollback.sh [--standalone]' >&2; exit 1
fi
[ -f "$backup/manifest" ] || exit 1
while IFS=' ' read -r action path; do
 case "$action" in
  restore) [ -e "$backup/files$path" ] || [ -L "$backup/files$path" ] || { echo 'Rollback snapshot incomplete' >&2; exit 1; };;
  remove) :;;
  *) exit 1;;
 esac
done < "$backup/manifest"
while IFS=' ' read -r action path; do
 case "$action" in
  restore) rm -rf "$path"; cp -a "$backup/files$path" "$path";;
  remove) rm -rf "$path";;
 esac
done < "$backup/manifest"
if "$standalone"; then
 # ACL entries only: do not restore a historical device owner/group/mode flag.
 setfacl -n --set-file="$backup/uinput.before" /dev/uinput
else
 setfacl --restore="$backup/uinput.acl"
fi
udevadm control --reload-rules
if "$standalone"; then
 echo 'Independent preview input baseline restored; no session started'
else
 echo 'WiggleControl artifacts/config/input grant restored; unrelated desktop/access unchanged'
fi

#!/bin/sh
# Stop the owning finite session first; no other application/service is stopped here.
set -eu
[ "$(id -u)" = 0 ] || { echo 'Run as receiving administrator' >&2; exit 1; }
backup=/var/lib/wigglecontrol/rollback
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
setfacl --restore="$backup/uinput.acl"
udevadm control --reload-rules
echo 'WiggleControl artifacts/config/input grant restored; unrelated desktop/access unchanged'

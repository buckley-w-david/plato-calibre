#!/bin/sh

echo "Compiling..."
cargo build --quiet --release --target=arm-unknown-linux-gnueabihf
arm-linux-gnueabihf-strip target/arm-unknown-linux-gnueabihf/release/plato-calibre

command -v findmnt >/dev/null
if [[ $? != 0 ]]; then
    echo "This script relies on findmnt, from util-linux!"
    exit 255
fi

KOBO_MOUNTPOINT="$(findmnt -nlo TARGET LABEL=KOBOeReader)"

# Sanity check...
if [[ -z "${KOBO_MOUNTPOINT}" ]] ; then
	echo "Couldn't find a Kobo eReader volume! Is one actually mounted?"
	exit 255
fi

if [[ ! -d "${KOBO_MOUNTPOINT}/.kobo" ]] ; then
	echo "Can't find a .kobo directory, ${KOBO_MOUNTPOINT} doesn't appear to point to a Kobo eReader... Is one actually mounted?"
	exit 255
fi

HOOK_DIR="$KOBO_MOUNTPOINT/.adds/plato/bin/plato-calibre/"
mkdir -p "$HOOK_DIR"

echo "Copying..."
cp target/arm-unknown-linux-gnueabihf/release/plato-calibre "$HOOK_DIR"

if [[ -f Settings.toml ]]; then 
    cp Settings.toml "$HOOK_DIR"
elif [[ -f Settings-sample.toml ]]; then
    cp Settings-sample.toml "$HOOK_DIR"
fi

echo "Done!"

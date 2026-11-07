#!/usr/bin/env bash
# Build a self-contained AppImage for the Rust GTK4 launcher.
set -euo pipefail

if [[ $# -lt 2 ]]; then
    echo "usage: $0 <appimagetool> <output.AppImage> [version]" >&2
    exit 2
fi

appimagetool=$(command -v "$1" || printf '%s\n' "$1")
output=$2
version=${3:-local}
root=$(cd -- "$(dirname -- "$0")/../.." && pwd)
appdir=$root/dist/AppDir
rm -rf "$appdir"
mkdir -p "$appdir/usr/bin" "$appdir/usr/lib" "$appdir/usr/share/doc/bbport-rust"

cp "$root/target/release/bbport-gui" "$appdir/usr/bin/"
cp "$root/target/release/bbport-rs" "$appdir/usr/bin/"
cp "$root/README.md" "$appdir/usr/share/doc/bbport-rust/"

copy_deps() {
    ldd "$1" 2>/dev/null | awk '/=> \// {print $3}' | while read -r library; do
        name=$(basename "$library")
        case "$name" in
            libc.so.*|libm.so.*|libpthread.so.*|libdl.so.*|librt.so.*|ld-linux*|linux-vdso*) continue ;;
        esac
        if [[ ! -e "$appdir/usr/lib/$name" ]]; then
            cp -L "$library" "$appdir/usr/lib/$name"
            copy_deps "$library"
        fi
    done
}
copy_deps "$appdir/usr/bin/bbport-gui"
copy_deps "$appdir/usr/bin/bbport-rs"

prefix=$(pkg-config --variable=prefix gtk4 2>/dev/null || true)
prefix=${prefix:-/usr}
for path in share/glib-2.0/schemas share/icons/Adwaita share/icons/hicolor; do
    if [[ -d "$prefix/$path" ]]; then
        mkdir -p "$appdir/usr/$(dirname "$path")"
        cp -R "$prefix/$path" "$appdir/usr/$path"
    fi
done

pixbuf_libdir=$(pkg-config --variable=libdir gdk-pixbuf-2.0 2>/dev/null || true)
if [[ -z "$pixbuf_libdir" ]]; then
    pixbuf_libdir=/usr/lib
fi
if [[ -d "$pixbuf_libdir/gdk-pixbuf-2.0" ]]; then
    mkdir -p "$appdir/usr/lib"
    cp -R "$pixbuf_libdir/gdk-pixbuf-2.0" "$appdir/usr/lib/"
fi

if command -v glib-compile-schemas >/dev/null 2>&1; then
    glib-compile-schemas "$appdir/usr/share/glib-2.0/schemas"
fi
if command -v gdk-pixbuf-query-loaders >/dev/null 2>&1; then
    pixbuf_dir="$appdir/usr/lib/gdk-pixbuf-2.0/2.10.0"
    if [[ -d "$pixbuf_dir/loaders" ]]; then
        (cd "$pixbuf_dir" && gdk-pixbuf-query-loaders loaders/*.so > loaders.cache)
    fi
fi

cp "$root/launcher/bloodborne.png" "$appdir/bbport-rust.png"
cat > "$appdir/bbport-rust.desktop" <<'EOF'
[Desktop Entry]
Type=Application
Name=bbport-rust
Comment=Bloodborne PC Rust launcher
Exec=bbport-gui
Icon=bbport-rust
Terminal=false
Categories=Game;Utility;
EOF

cat > "$appdir/AppRun" <<'EOF'
#!/bin/sh
HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
export LD_LIBRARY_PATH="$HERE/usr/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export GSETTINGS_SCHEMA_DIR="$HERE/usr/share/glib-2.0/schemas"
export GDK_PIXBUF_MODULEDIR="$HERE/usr/lib/gdk-pixbuf-2.0/2.10.0/loaders"
export GDK_PIXBUF_MODULE_FILE="$HERE/usr/lib/gdk-pixbuf-2.0/2.10.0/loaders.cache"
export XDG_DATA_DIRS="$HERE/usr/share${XDG_DATA_DIRS:+:$XDG_DATA_DIRS}"
exec "$HERE/usr/bin/bbport-gui" "$@"
EOF
chmod +x "$appdir/AppRun"

mkdir -p "$(dirname -- "$output")"
APPIMAGE_EXTRACT_AND_RUN=1 ARCH=x86_64 VERSION="$version" \
    "$appimagetool" --no-appstream "$appdir" "$output"

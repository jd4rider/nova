#!/bin/sh
# Shared pixel probes for the Metal acceptance scripts. sips crops one pixel,
# converts it to uncompressed BMP, and od reads the BGR bytes, so the checks
# need no Python or image library. Coordinates are top-left pixel coordinates.

pixel() {
    shot=$1
    x=$2
    y=$3
    one="target/pixel-$$.png"
    sips -c 1 1 --cropOffset "$y" "$x" "$shot" --out "$one" >/dev/null
    sips -s format bmp "$one" --out "$one.bmp" >/dev/null
    offset=$(od -An -tu4 -j10 -N4 "$one.bmp" | tr -d ' ')
    od -An -tu1 -j"$offset" -N3 "$one.bmp" | awk '{print $3, $2, $1}'
    rm -f "$one" "$one.bmp"
}

# expect_pixel FILE X Y R G B LABEL; tolerance is ±12 per channel.
expect_pixel() {
    shot=$1
    x=$2
    y=$3
    want_r=$4
    want_g=$5
    want_b=$6
    label=$7
    got=$(pixel "$shot" "$x" "$y")
    echo "$got" | awk -v r="$want_r" -v g="$want_g" -v b="$want_b" -v label="$label" '
        {
            dr = $1 - r; if (dr < 0) dr = -dr;
            dg = $2 - g; if (dg < 0) dg = -dg;
            db = $3 - b; if (db < 0) db = -db;
            if (dr <= 12 && dg <= 12 && db <= 12) exit 0;
            printf "metal: %s expected %d,%d,%d got %d,%d,%d\n", label, r, g, b, $1, $2, $3;
            exit 1;
        }'
}

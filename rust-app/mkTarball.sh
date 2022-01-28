#!/bin/sh

cargo-ledger --hex-next-to-json
tar -czvf release.tar.gz --transform 's,.*/,,;s,tarball-,,;s,^,pocket/,' app.json app.hex ../tarball-default.nix pocket.gif --mtime=0

echo
echo "==== Release hashes for release.tar.gz ===="
echo

echo -n "MD5 | "
md5sum release.tar.gz | cut -d' ' -f1
echo -n "SHA256 | "
sha256sum release.tar.gz | cut -d' ' -f1
echo -n "SHA512 | "
sha512sum release.tar.gz | cut -d' ' -f1


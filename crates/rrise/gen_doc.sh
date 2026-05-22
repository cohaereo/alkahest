#
# Copyright (c) 2022 Contributors to the Rrise project
#

cargo doc --no-deps
rm -rf ./docs
echo "<meta http-equiv=\"refresh\" content=\"0; url=rrise/index.html\">" > target/doc/index.html
cp -r target/doc ./docs

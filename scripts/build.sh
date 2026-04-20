export RUST_LOG=debug
mkdir -p build
cd build
cmake ..
make lime
./lime

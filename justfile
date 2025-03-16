
# rebuilds shaders and runs the game
default:
    just rebuild-shaders && cargo run

# rebuilds shaders, run setup-shader-compiler once before starting development
rebuild-shaders:
    cargo gpu build \
        --shader-crate ./crates/shaders/ \
        --capability Int8

# only needs to be done once
setup-shader-compiler:
    cargo gpu install --shader-crate ./crates/shaders/

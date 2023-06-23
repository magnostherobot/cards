# How to Build/Run

For a native build, `cargo build` or `cargo run`.

For a wasm build, I use `wasm-pack` and use the python standard library HTTP
server to host the files for testing:

```sh
wasm-pack build -t web
python3 -m http.server 8000
```

# Thanks

Thanks to Lazyspace for the [card
assets](https://lazyspace.itch.io/pixel-playing-cards) I'm using.

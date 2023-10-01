# ref-275

<p align="center">
  <img src="https://raw.githubusercontent.com/turboflakes/ref-275/main/assets/ref-275-github-header.png">
</p>

`ref-275` is a small WASM app specifically to vote AYE on Kusama [Referendum #275](https://kusama.subsquare.io/referenda/275).

## Development / Build from Source

If you'd like to build from source, first install Rust.

```bash
curl https://sh.rustup.rs -sSf | sh
```

If Rust is already installed run

```bash
rustup update
```

Verify Rust installation by running

```bash
rustc --version
```

Once done, finish installing the support software

```bash
sudo apt install build-essential git clang libclang-dev pkg-config libssl-dev
```

Install Trunk, a WASM bundler:

```
cargo install --locked trunk
```

Run the app locally with:

```
trunk serve --open
```

## Inspiration

This small WASM app was heavily inspired by the wasm-example available [here](https://github.com/paritytech/subxt/tree/master/examples/wasm-example).

All credits goes to the [subxt](https://github.com/paritytech/subxt) Team for the amazing work and examples provided - you rock 🤘🎸

### License

`ref-275` - The entire code within this repository is licensed under the [Apache License 2.0](./LICENSE).

### Quote

> "Instead of worrying about what you cannot control, shift your energy to what you can create."
― Roy T. Bennett

__

Enjoy `ref-275`

# mu2vid

just my personal GUI tool for uploading music album to YouTube.

![img](https://github.com/user-attachments/assets/820d9ed2-ab09-4d55-952c-f28a76d8eb8e)

## Build

To build and run on Windows, it's requires MSYS2 MinGW64.

Install the required MSYS2 packages:

```sh
pacman -S \
    mingw-w64-x86_64-gcc \
    mingw-w64-x86_64-cmake \
    mingw-w64-x86_64-make \
    mingw-w64-x86_64-clang \
    mingw-w64-x86_64-llvm \
    mingw-w64-x86_64-pkgconf
```

Add the GNU Windows target:

```sh
rustup target add x86_64-pc-windows-gnu
```

Set the build environment from Command Prompt:

```bat
set "PATH=C:\msys64\mingw64\bin;C:\msys64\usr\bin;%PATH%"
set "LIBCLANG_PATH=C:\msys64\mingw64\bin"
set "CMAKE_GENERATOR=MinGW Makefiles"
set "CMAKE_BUILD_PARALLEL_LEVEL=8"
```

Build or run with:

```sh
cargo build --target=x86_64-pc-windows-gnu
cargo run --target=x86_64-pc-windows-gnu
```

## License

[GPL v3.0](LICENSE)

@echo off
setlocal

set "MSYS2=C:\msys64"

set "PATH=%MSYS2%\mingw64\bin;%MSYS2%\usr\bin;%PATH%"
set "LIBCLANG_PATH=%MSYS2%\mingw64\bin"
set "CMAKE_GENERATOR=MinGW Makefiles"

cargo run --target x86_64-pc-windows-gnu

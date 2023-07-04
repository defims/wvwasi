# wvwasi
A WebView with WebAssembly System Interface ([WASI](https://github.com/WebAssembly/WASI)) may be the best Javascript/WebAssembly runtime, and `wvwasi` is it.

                                  |
    Javascript/WebAssembly code   |  Javascript/WebAssembly application code
                                  |                 |
                                  |                 v
                                  | WASI syscalls (inserted by compiler toolchain)
                                  |                 |
    ------------------------------+                 |
                                  |                 v
    Javascript/WebAssembly runtime|    wvwasi (implementation WASI in webview)
    (WebView)                     |                 |
                                  |                 v
                                  |        platform-specific calls
                                  |

*(Hence wvwasi isn't for making programs execute on WASI runtimes.
That would either be a wasm32-wasi target complied by rust, or done
through POSIX emulation by the Emscripten or wasi-sdk toolchains.)*

WARNING: This is a alpha. Work in progress.

## Example Usage
Currently only the Windows platform is implemented, other platforms are on the way.
```bash
cargo run --example hello_world --target x86_64-pc-windows-msvc
```

## API
The WASI API is versioned. This documentation is based on the WASI [preview 1][]
snapshot. `wvwasi` implements the WASI system call API with the following
additions/modifications:

### System Calls
This section has been adapted from the official WASI API documentation.

- [`wvwasi.wasiSnapshotPreview1.fd_readdir()`](https://github.com/WebAssembly/WASI/blob/main/legacy/preview1/docs.md#-fd_readdirfd-fd-buf-pointeru8-buf_len-size-cookie-dircookie---resultsize-errno)
- [`wvwasi.wasiSnapshotPreview1.path_remove_directory()`](https://github.com/WebAssembly/WASI/blob/main/legacy/preview1/docs.md#-path_remove_directoryfd-fd-path-string---result-errno)
- [`wvwasi.wasiSnapshotPreview1.path_unlink_file()`](https://github.com/WebAssembly/WASI/blob/main/legacy/preview1/docs.md#-path_unlink_filefd-fd-path-string---result-errno)

[WASI]: https://github.com/WebAssembly/WASI
[preview 1]: https://github.com/WebAssembly/WASI/blob/main/legacy/preview1/docs.md
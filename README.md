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
## How does it achieve high performance?

No magic, just IPC and sharedbuffer.

## Why not implement WASI using [Web APIs](https://developer.mozilla.org/en-US/docs/Web/API)?

1. [Webkit oppose File System Access API](https://webkit.org/standards-positions/).
2. Unable to preopen system folder.
3. You can not customize your own interface with high-performance communication mechanisms.

## API
The WASI API is versioned. This documentation is based on the WASI [preview 1][]
snapshot. `wvwasi` implements the WASI system call API with the following
additions/modifications:

### System Calls
This section has been adapted from the official WASI API documentation.

- [`wvwasi.wasiSnapshotPreview1.fd_readdir()`](https://github.com/WebAssembly/WASI/blob/main/legacy/preview1/docs.md#-fd_readdirfd-fd-buf-pointeru8-buf_len-size-cookie-dircookie---resultsize-errno)
- [`wvwasi.wasiSnapshotPreview1.path_remove_directory()`](https://github.com/WebAssembly/WASI/blob/main/legacy/preview1/docs.md#-path_remove_directoryfd-fd-path-string---result-errno)
- [`wvwasi.wasiSnapshotPreview1.path_unlink_file()`](https://github.com/WebAssembly/WASI/blob/main/legacy/preview1/docs.md#-path_unlink_filefd-fd-path-string---result-errno)
- [`wvwasi.wasiSnapshotPreview1.fd_advise()`](https://github.com/WebAssembly/WASI/blob/main/legacy/preview1/docs.md#-fd_advisefd-fd-offset-filesize-len-filesize-advice-advice---result-errno)
- [`wvwasi.wasiSnapshotPreview1.fd_allocate()`](https://github.com/WebAssembly/WASI/blob/main/legacy/preview1/docs.md#-fd_allocatefd-fd-offset-filesize-len-filesize---result-errno)
- [`wvwasi.wasiSnapshotPreview1.fd_close()`](https://github.com/WebAssembly/WASI/blob/main/legacy/preview1/docs.md#-fd_closefd-fd---result-errno)
- [`wvwasi.wasiSnapshotPreview1.fd_datasync()`](https://github.com/WebAssembly/WASI/blob/main/legacy/preview1/docs.md#-fd_datasyncfd-fd---result-errno)
- [`wvwasi.wasiSnapshotPreview1.fd_fdstat_get()`](https://github.com/WebAssembly/WASI/blob/main/legacy/preview1/docs.md#-fd_fdstat_getfd-fd---resultfdstat-errno)
- [`wvwasi.wasiSnapshotPreview1.fd_fdstat_set_flags()`](https://github.com/WebAssembly/WASI/blob/main/legacy/preview1/docs.md#-fd_fdstat_set_flagsfd-fd-flags-fdflags---result-errno)
- [`wvwasi.wasiSnapshotPreview1.fd_fdstat_set_rights()`](https://github.com/WebAssembly/WASI/blob/main/legacy/preview1/docs.md#-fd_fdstat_set_rightsfd-fd-fs_rights_base-rights-fs_rights_inheriting-rights---result-errno)
- [`wvwasi.wasiSnapshotPreview1.fd_filestat_get()`](https://github.com/WebAssembly/WASI/blob/main/legacy/preview1/docs.md#-fd_filestat_getfd-fd---resultfilestat-errno)
- [`wvwasi.wasiSnapshotPreview1.fd_filestat_set_size()`](https://github.com/WebAssembly/WASI/blob/main/legacy/preview1/docs.md#-fd_filestat_set_sizefd-fd-size-filesize---result-errno)
- [`wvwasi.wasiSnapshotPreview1.fd_filestat_set_times()`](https://github.com/WebAssembly/WASI/blob/main/legacy/preview1/docs.md#-fd_filestat_set_timesfd-fd-atim-timestamp-mtim-timestamp-fst_flags-fstflags---result-errno)
- [`wvwasi.wasiSnapshotPreview1.fd_pread()`](https://github.com/WebAssembly/WASI/blob/main/legacy/preview1/docs.md#-fd_preadfd-fd-iovs-iovec_array-offset-filesize---resultsize-errno)
- [`wvwasi.wasiSnapshotPreview1.fd_prestat_get()`](https://github.com/WebAssembly/WASI/blob/main/legacy/preview1/docs.md#-fd_prestat_getfd-fd---resultprestat-errno)
- [`wvwasi.wasiSnapshotPreview1.fd_prestat_dir_name()`](https://github.com/WebAssembly/WASI/blob/main/legacy/preview1/docs.md#-fd_prestat_dir_namefd-fd-path-pointeru8-path_len-size---result-errno)
- [`wvwasi.wasiSnapshotPreview1.fd_pwrite()`](https://github.com/WebAssembly/WASI/blob/main/legacy/preview1/docs.md#-fd_pwritefd-fd-iovs-ciovec_array-offset-filesize---resultsize-errno)
- [`wvwasi.wasiSnapshotPreview1.fd_read()`](https://github.com/WebAssembly/WASI/blob/main/legacy/preview1/docs.md#-fd_readfd-fd-iovs-iovec_array---resultsize-errno)
- [`wvwasi.wasiSnapshotPreview1.fd_readdir()`](https://github.com/WebAssembly/WASI/blob/main/legacy/preview1/docs.md#-fd_readdirfd-fd-buf-pointeru8-buf_len-size-cookie-dircookie---resultsize-errno)
- [`wvwasi.wasiSnapshotPreview1.fd_renumber()`](https://github.com/WebAssembly/WASI/blob/main/legacy/preview1/docs.md#-fd_renumberfd-fd-to-fd---result-errno)
- [`wvwasi.wasiSnapshotPreview1.fd_seek()`](https://github.com/WebAssembly/WASI/blob/main/legacy/preview1/docs.md#-fd_seekfd-fd-offset-filedelta-whence-whence---resultfilesize-errno)
- [`wvwasi.wasiSnapshotPreview1.fd_sync()`](https://github.com/WebAssembly/WASI/blob/main/legacy/preview1/docs.md#-fd_syncfd-fd---result-errno)
- [`wvwasi.wasiSnapshotPreview1.fd_tell()`](https://github.com/WebAssembly/WASI/blob/main/legacy/preview1/docs.md#-fd_tellfd-fd---resultfilesize-errno)
- [`wvwasi.wasiSnapshotPreview1.fd_write()`](https://github.com/WebAssembly/WASI/blob/main/legacy/preview1/docs.md#-fd_writefd-fd-iovs-ciovec_array---resultsize-errno)
- [`wvwasi.wasiSnapshotPreview1.path_create_directory()`](https://github.com/WebAssembly/WASI/blob/main/legacy/preview1/docs.md#-path_create_directoryfd-fd-path-string---result-errno)
- [`wvwasi.wasiSnapshotPreview1.path_filestat_get()`](https://github.com/WebAssembly/WASI/blob/main/legacy/preview1/docs.md#-path_filestat_getfd-fd-flags-lookupflags-path-string---resultfilestat-errno)
- [`wvwasi.wasiSnapshotPreview1.path_filestat_set_times()`](https://github.com/WebAssembly/WASI/blob/main/legacy/preview1/docs.md#-path_filestat_set_timesfd-fd-flags-lookupflags-path-string-atim-timestamp-mtim-timestamp-fst_flags-fstflags---result-errno)
- [`wvwasi.wasiSnapshotPreview1.path_link()`](https://github.com/WebAssembly/WASI/blob/main/legacy/preview1/docs.md#-path_linkold_fd-fd-old_flags-lookupflags-old_path-string-new_fd-fd-new_path-string---result-errno)
- [`wvwasi.wasiSnapshotPreview1.path_open()`](https://github.com/WebAssembly/WASI/blob/main/legacy/preview1/docs.md#-path_openfd-fd-dirflags-lookupflags-path-string-oflags-oflags-fs_rights_base-rights-fs_rights_inheriting-rights-fdflags-fdflags---resultfd-errno)
- [`wvwasi.wasiSnapshotPreview1.path_readlink()`](https://github.com/WebAssembly/WASI/blob/main/legacy/preview1/docs.md#-path_readlinkfd-fd-path-string-buf-pointeru8-buf_len-size---resultsize-errno)
- [`wvwasi.wasiSnapshotPreview1.path_remove_directory()`](https://github.com/WebAssembly/WASI/blob/main/legacy/preview1/docs.md#-path_remove_directoryfd-fd-path-string---result-errno)
- [`wvwasi.wasiSnapshotPreview1.path_rename()`](https://github.com/WebAssembly/WASI/blob/main/legacy/preview1/docs.md#-path_renamefd-fd-old_path-string-new_fd-fd-new_path-string---result-errno)
- [`wvwasi.wasiSnapshotPreview1.path_symlink()`](https://github.com/WebAssembly/WASI/blob/main/legacy/preview1/docs.md#-path_symlinkold_path-string-fd-fd-new_path-string---result-errno)
- [`wvwasi.wasiSnapshotPreview1.path_unlink_file()`](https://github.com/WebAssembly/WASI/blob/main/legacy/preview1/docs.md#-path_unlink_filefd-fd-path-string---result-errno)

[WASI]: https://github.com/WebAssembly/WASI
[preview 1]: https://github.com/WebAssembly/WASI/blob/main/legacy/preview1/docs.md
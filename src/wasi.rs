use webview2_com::Microsoft::Web::WebView2::Win32::{ ICoreWebView2Environment12, ICoreWebView2SharedBuffer, ICoreWebView2Environment };
// use wiggle::GuestType;
use windows::core::ComInterface;
// use wry::{
//   application::window::Window,
//   Result,
//   http::{Request, Response, header}
// };
// use anyhow::Context;
use wasi_common::snapshots::preview_1::{self, wasi_snapshot_preview1::WasiSnapshotPreview1};
use preview_1::error::Errno;
use crate::webview::{ WvWasiOptions, WvWasiPreopen };

pub struct Wasi<'a> {
  pub ctx: wasi_common::WasiCtx,
  pub shared_buffer: ICoreWebView2SharedBuffer, // windows use ICoreWebView2SharedBuffer, others use a polyfill (host buffer with custom protocol access api)
  pub shared_memory: wiggle::wasmtime::WasmtimeGuestMemory<'a>,
  pub root_fd: u32,
}

impl Wasi<'_> {
  pub fn new<'a>(env: &ICoreWebView2Environment, options: Option<WvWasiOptions<'a>>) -> anyhow::Result<Self> {
    let buf_len = 10240; // TODO config
    let ctx = wasi_cap_std_sync::WasiCtxBuilder::new().build();
    // let stdin = table.push_input_stream(stdin).context("stdin")?; // TODO
    // let stdout = table.push_output_stream(stdout).context("stdout")?; // TODO
    // let stderr = table.push_output_stream(stderr).context("stderr")?; // TODO

    fn get_host_dir(path: &std::path::Path) -> anyhow::Result<wasi_cap_std_sync::dir::Dir> {
      let dir = wasi_cap_std_sync::Dir::open_ambient_dir(
        path,
        wasi_cap_std_sync::ambient_authority()
      )?;
      Ok(wasi_cap_std_sync::dir::Dir::from_cap_std(dir))
    }

    let temp_dir = std::env::temp_dir();// default root path is temp_dir
    let root_fd = ctx.push_dir(
      Box::new(get_host_dir(temp_dir.as_path())?),
      std::path::Path::new("/").to_path_buf()
    )?;
    // let root_fd = types::Fd::from(root_fd); // root_fd == 3

    if let Some(WvWasiOptions { preopens }) = options {
      for WvWasiPreopen { guest_path, path } in preopens.into_iter() {
        if guest_path == "/" {
          ctx.insert_dir(
            root_fd,
            Box::new(get_host_dir(std::path::Path::new(path))?),
      std::path::Path::new(guest_path).to_path_buf()
          );
        } else {
          let _fd = ctx.push_dir(
            Box::new(get_host_dir(std::path::Path::new(path))?),
            std::path::Path::new(guest_path).to_path_buf()
          )?;
        }
      }
    }

    let environment = env.cast::<ICoreWebView2Environment12>()?;
    let shared_buffer = unsafe { environment.CreateSharedBuffer(buf_len.try_into()?)? };

    // guest memory
    let mut shared_buffer_ptr: *mut u8 = &mut 0u8;
    let _ = unsafe { &shared_buffer.Buffer(&mut shared_buffer_ptr) };
    let slice: &mut [u8] = unsafe { std::slice::from_raw_parts_mut(shared_buffer_ptr, buf_len.try_into()?) };
    let shared_memory = wiggle::wasmtime::WasmtimeGuestMemory::new(slice);

    Ok(Wasi {
      ctx,
      shared_buffer,
      shared_memory,
      root_fd
    })
  }

  #[tokio::main(flavor = "current_thread")]
  pub async fn fd_advise(&mut self, request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    use preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1;

    let (
      fd, // The file descriptor.
      offset, // The offset at which to start the allocation.
      len, // The length of the area that is allocated.
      advice, // The advice.
    ) = serde_json::from_str::<(i32, i64, i64, i32)>(std::str::from_utf8(request)?)?;
    dbg!(fd, offset, len, advice);

    // TODO right check
    // TODO argument error detect

    let _ = self.ctx.fd_advise(fd.try_into()?, offset.try_into()?, len.try_into()?, advice.try_into()?).await?;

    Ok(format!(r#"[{}]"#, Into::<u16>::into(Errno::Success)).as_bytes().to_owned())
  }

  // This operation from cloudabi is linux-specific, isn't even
  // supported across all linux filesystems, and has no support on macos
  // or windows. Rather than ship spotty support, it has been removed
  // from preview 2, and we are no longer supporting it in preview 1 as
  // well.
  #[tokio::main(flavor = "current_thread")]
  pub async fn fd_allocate(&mut self, request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    use preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1;

    let (
      fd, // The file descriptor.
      offset, // The offset at which to start the allocation.
      len, // The length of the area that is allocated.
    ) = serde_json::from_str::<(i32, i64, i64)>(std::str::from_utf8(request)?)?;
    dbg!(fd, offset, len);

    // TODO right check
    // TODO argument error detect

    let _ = self.ctx.fd_allocate(fd.try_into()?, offset.try_into()?, len.try_into()?).await?;

    Ok(format!(r#"[{}]"#, Into::<u16>::into(Errno::Success)).as_bytes().to_owned())
  }

  #[tokio::main(flavor = "current_thread")]
  pub async fn fd_close(&mut self, request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    use preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1;

    let (
      fd, // The file descriptor.
    ) = serde_json::from_str::<(u32,)>(std::str::from_utf8(request)?)?;
    dbg!(fd);

    // TODO right check
    // TODO argument error detect

    let _ = self.ctx.fd_close(fd.try_into()?).await?;

    Ok(format!(r#"[{}]"#, Into::<u16>::into(Errno::Success)).as_bytes().to_owned())
  }

  #[tokio::main(flavor = "current_thread")]
  pub async fn fd_datasync(&mut self, request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    use preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1;

    let (
      fd, // The file descriptor.
    ) = serde_json::from_str::<(u32,)>(std::str::from_utf8(request)?)?;
    dbg!(fd);

    // TODO right check
    // TODO argument error detect

    let _ = self.ctx.fd_datasync(fd.try_into()?).await?;

    Ok(format!(r#"[{}]"#, Into::<u16>::into(Errno::Success)).as_bytes().to_owned())
  }

  #[tokio::main(flavor = "current_thread")]
  pub async fn fd_fdstat_get(&mut self, request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    use preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1;

    let (
      fd, // The file descriptor.
      fdstat_ptr // The buffer where the file's attributes are stored.
    ) = serde_json::from_str::<(u32, u32)>(std::str::from_utf8(request)?)?;
    dbg!(fd, fdstat_ptr);
    // TODO right check

    // TODO
    // if iovs_len == 0 {
    //   *nread_ptr = 0;
    //   return ERRNO_SUCCESS;
    // }

    let filestat = self.ctx.fd_fdstat_get(fd.try_into()?).await?;
    dbg!(&filestat);
    let _ = wiggle::GuestPtr::new(&self.shared_memory, fdstat_ptr.try_into()?).write(filestat)?;

    Ok(format!(r#"[{}]"#, Into::<u16>::into(Errno::Success)).as_bytes().to_owned())
  }

  #[tokio::main(flavor = "current_thread")]
  pub async fn fd_fdstat_set_flags(&mut self, request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    use preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1;

    let (
      fd, // The file descriptor.
      flags, // The desired file size.
    ) = serde_json::from_str::<(i32, i32)>(std::str::from_utf8(request)?)?;
    dbg!(fd, flags);
    // TODO right check

    // TODO
    // if iovs_len == 0 {
    //   *nread_ptr = 0;
    //   return ERRNO_SUCCESS;
    // }

    let _ = self.ctx.fd_fdstat_set_flags(fd.try_into()?, flags.try_into()?).await?;

    Ok(format!(r#"[{}]"#, Into::<u16>::into(Errno::Success)).as_bytes().to_owned())
  }

  #[tokio::main(flavor = "current_thread")]
  pub async fn fd_fdstat_set_rights(&mut self, request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    use preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1;

    let (
      fd, // The file descriptor.
      fs_rights_base, // The desired rights of the file descriptor.
      fs_rights_inheriting,
    ) = serde_json::from_str::<(i32, i64, i64)>(std::str::from_utf8(request)?)?;
    dbg!(fd, fs_rights_base, fs_rights_inheriting);
    // TODO right check

    // TODO
    // if iovs_len == 0 {
    //   *nread_ptr = 0;
    //   return ERRNO_SUCCESS;
    // }

    let _ = self.ctx.fd_fdstat_set_rights(fd.try_into()?, fs_rights_base.try_into()?, fs_rights_inheriting.try_into()?).await?;

    Ok(format!(r#"[{}]"#, Into::<u16>::into(Errno::Success)).as_bytes().to_owned())
  }

  #[tokio::main(flavor = "current_thread")]
  pub async fn fd_filestat_get(&mut self, request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    use preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1;

    let (
      fd, // The file descriptor.
      filestat_ptr // The buffer where the file's attributes are stored.
    ) = serde_json::from_str::<(u32, u32)>(std::str::from_utf8(request)?)?;
    dbg!(fd, filestat_ptr);
    // TODO right check

    // TODO
    // if iovs_len == 0 {
    //   *nread_ptr = 0;
    //   return ERRNO_SUCCESS;
    // }

    // fd need to be a file fd, not a folder.
    let filestat = self.ctx.fd_filestat_get(fd.try_into()?).await?;
    dbg!(&filestat);
    let _ = wiggle::GuestPtr::new(&self.shared_memory, filestat_ptr.try_into()?).write(filestat)?;

    Ok(format!(r#"[{}]"#, Into::<u16>::into(Errno::Success)).as_bytes().to_owned())
  }

  #[tokio::main(flavor = "current_thread")]
  pub async fn fd_filestat_set_size(&mut self, request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    use preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1;

    let (
      fd, // The file descriptor.
      size, // The desired file size.
    ) = serde_json::from_str::<(u32, u64)>(std::str::from_utf8(request)?)?;
    dbg!(fd, size);
    // TODO right check

    // TODO
    // if iovs_len == 0 {
    //   *nread_ptr = 0;
    //   return ERRNO_SUCCESS;
    // }

    // fd need to be a file fd, not a folder.
    let _ = self.ctx.fd_filestat_set_size(fd.try_into()?, size.try_into()?).await?;

    Ok(format!(r#"[{}]"#, Into::<u16>::into(Errno::Success)).as_bytes().to_owned())
  }

  #[tokio::main(flavor = "current_thread")]
  pub async fn fd_filestat_set_times(&mut self, request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    use preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1;

    let (
      fd, // The file descriptor.
      atim, // The desired values of the data access timestamp.
      mtim, // The desired values of the data modification timestamp.
      fst_flags, // A bitmask indicating which timestamps to adjust.
    ) = serde_json::from_str::<(i32, i64, i64, i32)>(std::str::from_utf8(request)?)?;
    dbg!(fd, atim, mtim, fst_flags);
    // TODO right check

    // TODO
    // if iovs_len == 0 {
    //   *nread_ptr = 0;
    //   return ERRNO_SUCCESS;
    // }

    // fd need to be a file fd, not a folder.
    let _ = self.ctx.fd_filestat_set_times(fd.try_into()?, atim.try_into()?, mtim.try_into()?, fst_flags.try_into()?).await?;

    Ok(format!(r#"[{}]"#, Into::<u16>::into(Errno::Success)).as_bytes().to_owned())
  }

  #[tokio::main(flavor = "current_thread")]
  pub async fn fd_pread(&mut self, request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    use preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1;

    let (
      fd, // The file descriptor.
      iovs_ptr, // List of scatter/gather vectors in which to store data.
      iovs_len, // The length of the iovs.
      offset, // The offset within the file at which to read.
      nread_ptr // A memory location to store the bytes read.
    ) = serde_json::from_str::<(i32, i32, i32, i64, i32)>(std::str::from_utf8(request)?)?;
    dbg!(fd, iovs_ptr, iovs_len, offset, nread_ptr);
    // TODO right check

    // TODO
    // if iovs_len == 0 {
    //   *nread_ptr = 0;
    //   return ERRNO_SUCCESS;
    // }

    // convert ivos_ptr and ivos_len to preview_1::types::IvosArray
    // iovs_ptr need to align to iovs_array's alignment 4.
    let iovs_array = preview_1::types::IovecArray::new(&self.shared_memory, (iovs_ptr.try_into()?, iovs_len.try_into()?));

    // fd need to be a file fd, not a folder.
    let nread = self.ctx.fd_pread(fd.try_into()?, &iovs_array, offset.try_into()?).await?;
    dbg!(&nread);
    let _ = wiggle::GuestPtr::new(&self.shared_memory, nread_ptr.try_into()?).write(nread)?;

    Ok(format!(r#"[{}]"#, Into::<u16>::into(Errno::Success)).as_bytes().to_owned())
  }

  #[tokio::main(flavor = "current_thread")]
  pub async fn fd_prestat_get(&mut self, request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    use preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1;

    let (
      fd, // The file descriptor.
      buf_ptr, // A pointer to store the prestat information.
    ) = serde_json::from_str::<(u32, u32)>(std::str::from_utf8(request)?)?;
    dbg!(&fd, &buf_ptr);

    // TODO right check
    // TODO argument error detect

    let prestat = self.ctx.fd_prestat_get(fd.try_into()?).await?;
    dbg!(&prestat);
    let _ = wiggle::GuestPtr::new(&self.shared_memory, buf_ptr.try_into()?).write(prestat)?;

    Ok(format!(r#"[{}]"#, Into::<u16>::into(Errno::Success)).as_bytes().to_owned())
  }

  #[tokio::main(flavor = "current_thread")]
  pub async fn fd_prestat_dir_name(&mut self, request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    use preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1;

    let (
      fd, // The file descriptor.
      path, // A memory location to store the path name.
      path_len, // The length of the path.
    ) = serde_json::from_str::<(u32, u32, u32)>(std::str::from_utf8(request)?)?;
    dbg!(&path, &path_len);

    // TODO right check
    // TODO argument error detect

    let path_ptr = &wiggle::GuestPtr::new(&self.shared_memory, path);
    let _ = self.ctx.fd_prestat_dir_name(fd.try_into()?, path_ptr, path_len.try_into()?).await?;

    Ok(format!(r#"[{}]"#, Into::<u16>::into(Errno::Success)).as_bytes().to_owned())
  }

  #[tokio::main(flavor = "current_thread")]
  pub async fn fd_pwrite(&mut self, request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    use preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1;

    let (
      fd, // The file descriptor.
      iovs_ptr, // List of scatter/gather vectors from which to write data.
      iovs_len, // The length of the ciovs.
      offset, // The offset within the file at which to write.
      nwritten_ptr // A memory location to store the bytes written.
    ) = serde_json::from_str::<(u32, u32, u32, u64, u32)>(std::str::from_utf8(request)?)?;
    dbg!(fd, iovs_ptr, iovs_len, nwritten_ptr);
    // TODO right check

    // TODO
    // if iovs_len == 0 {
    //   *nread_ptr = 0;
    //   return ERRNO_SUCCESS;
    // }

    // convert ivos_ptr and ivos_len to preview_1::types::CivosArray
    // iovs_ptr need to align to iovs_array's alignment 4.
    let iovs_array = preview_1::types::CiovecArray::new(&self.shared_memory, (iovs_ptr.try_into()?, iovs_len.try_into()?));
    // dbg!(iovs_array.get(0).unwrap().read()?);

    // fd need to be a file fd, not a folder.
    let nwritten = self.ctx.fd_pwrite(fd.try_into()?, &iovs_array, offset).await?;
    dbg!(&nwritten);
    let _ = wiggle::GuestPtr::new(&self.shared_memory, nwritten_ptr.try_into()?).write(nwritten)?;

    Ok(format!(r#"[{}]"#, Into::<u16>::into(Errno::Success)).as_bytes().to_owned())
  }

  #[tokio::main(flavor = "current_thread")]
  pub async fn fd_read(&mut self, request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    use preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1;

    // https://docs.rs/wasi/latest/wasi/wasi_snapshot_preview1/fn.fd_read.html
    // https://github.com/microsoft/vscode-wasm/blob/main/wasm-wasi-core/src/common/wasi.ts#L2559
    // https://github.com/bytecodealliance/wasmtime/blob/5325894519c098e07857c74da3b0c99126b0f597/crates/wasi-preview1-component-adapter/src/lib.rs#L812C5-L812C5
    let (
      fd, // The file descriptor.
      iovs_ptr, // List of scatter/gather vectors in which to store data.
      iovs_len, // The length of the iovs.
      nread_ptr // A memory location to store the bytes read.
    ) = serde_json::from_str::<(i32, i32, i32, i32)>(std::str::from_utf8(request)?)?;
    dbg!(fd, iovs_ptr, iovs_len, nread_ptr);
    // TODO right check

    // TODO
    // if iovs_len == 0 {
    //   *nread_ptr = 0;
    //   return ERRNO_SUCCESS;
    // }

    // convert ivos_ptr and ivos_len to preview_1::types::IvosArray
    // iovs_ptr need to align to iovs_array's alignment 4.
    let iovs_array = preview_1::types::IovecArray::new(&self.shared_memory, (iovs_ptr.try_into()?, iovs_len.try_into()?));
    dbg!(iovs_array.get(0).unwrap().read()?);

    // fd need to be a file fd, not a folder.
    let nread = self.ctx.fd_read(fd.try_into()?, &iovs_array).await?;
    dbg!(&nread);
    let _ = wiggle::GuestPtr::new(&self.shared_memory, nread_ptr.try_into()?).write(nread)?;

    Ok(format!(r#"[{}]"#, Into::<u16>::into(Errno::Success)).as_bytes().to_owned())
  }

  #[tokio::main(flavor = "current_thread")]
  pub async fn fd_readdir(&mut self, request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    use preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1;

    let (
      fd, // The file descriptor.
      buf_ptr, // The buffer where directory entries are stored.
      buf_len, // The length of the buffer.
      cookie, // The location within the directory to start reading.
      bufused_ptr // The number of bytes stored in the read buffer.
    ) = serde_json::from_str::<(u32, u32, u32, u64, u32)>(std::str::from_utf8(request)?)?;
    dbg!(fd, buf_ptr, buf_len, cookie, bufused_ptr);
    // TODO right check

    let buf = &wiggle::GuestPtr::new(&self.shared_memory, buf_ptr);
    let bufused = self.ctx.fd_readdir(fd.try_into()?, buf, buf_len.try_into()?, cookie).await?;
    dbg!(bufused);
    let _ = wiggle::GuestPtr::new(&self.shared_memory, bufused_ptr).write(bufused)?;

    Ok(format!(r#"[{}]"#, Into::<u16>::into(Errno::Success)).as_bytes().to_owned())
  }

  #[tokio::main(flavor = "current_thread")]
  pub async fn fd_renumber(&mut self, request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    use preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1;

    let (
      fd, // The file descriptor.
      to, // The file descriptor to overwrite.
    ) = serde_json::from_str::<(u32, u32)>(std::str::from_utf8(request)?)?;
    dbg!(fd, to);

    // TODO right check
    // TODO argument error detect

    let _ = self.ctx.fd_renumber(fd.try_into()?, to.try_into()?).await?;

    Ok(format!(r#"[{}]"#, Into::<u16>::into(Errno::Success)).as_bytes().to_owned())
  }

  #[tokio::main(flavor = "current_thread")]
  pub async fn fd_seek(&mut self, request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    use preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1;

    let (
      fd, // The file descriptor.
      offset, // offset The number of bytes to move.
      whence, // whence The base from which the offset is relative.
      newoffset_ptr // new_offset_ptr A memory location to store the new offset.
    ) = serde_json::from_str::<(u32, u64, u32, u32)>(std::str::from_utf8(request)?)?;
    dbg!(fd, offset, whence, newoffset_ptr);

    // TODO right check
    // TODO argument error detect

    let newoffset = self.ctx.fd_seek(fd.try_into()?, offset.try_into()?, (whence as u8).try_into()?).await?;
    dbg!(&newoffset);
    let _ = wiggle::GuestPtr::new(&self.shared_memory, newoffset_ptr.try_into()?).write(newoffset)?;

    Ok(format!(r#"[{}]"#, Into::<u16>::into(Errno::Success)).as_bytes().to_owned())
  }

  #[tokio::main(flavor = "current_thread")]
  pub async fn fd_sync(&mut self, request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    use preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1;

    let (
      fd, // The file descriptor.
    ) = serde_json::from_str::<(u32,)>(std::str::from_utf8(request)?)?;
    dbg!(fd);

    // TODO right check
    // TODO argument error detect

    let _ = self.ctx.fd_sync(fd.try_into()?).await?;

    Ok(format!(r#"[{}]"#, Into::<u16>::into(Errno::Success)).as_bytes().to_owned())
  }

  #[tokio::main(flavor = "current_thread")]
  pub async fn fd_tell(&mut self, request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    use preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1;

    let (
      fd, // The file descriptor.
      offset_ptr, // A memory location to store the current offset of the file descriptor, relative to the start of the file.
    ) = serde_json::from_str::<(u32, u64)>(std::str::from_utf8(request)?)?;
    dbg!(fd, offset_ptr);

    // TODO right check
    // TODO argument error detect

    let offset = self.ctx.fd_tell(fd.try_into()?).await?;
    dbg!(&offset);
    let _ = wiggle::GuestPtr::new(&self.shared_memory, offset_ptr.try_into()?).write(offset)?;

    Ok(format!(r#"[{}]"#, Into::<u16>::into(Errno::Success)).as_bytes().to_owned())
  }

  #[tokio::main(flavor = "current_thread")]
  pub async fn fd_write(&mut self, request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    use preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1;

    let (
      fd, // The file descriptor.
      iovs_ptr, // List of scatter/gather vectors from which to write data.
      iovs_len, // The length of the ciovs.
      nwritten_ptr // A memory location to store the bytes written.
    ) = serde_json::from_str::<(u32, u32, u32, u32)>(std::str::from_utf8(request)?)?;
    dbg!(fd, iovs_ptr, iovs_len, nwritten_ptr);
    // TODO right check

    // TODO
    // if iovs_len == 0 {
    //   *nread_ptr = 0;
    //   return ERRNO_SUCCESS;
    // }

    // convert ivos_ptr and ivos_len to preview_1::types::CivosArray
    // iovs_ptr need to align to iovs_array's alignment 4.
    let iovs_array = preview_1::types::CiovecArray::new(&self.shared_memory, (iovs_ptr.try_into()?, iovs_len.try_into()?));
    // dbg!(iovs_array.get(0).unwrap().read()?);

    // fd need to be a file fd, not a folder.
    let nwritten = self.ctx.fd_write(fd.try_into()?, &iovs_array).await?;
    dbg!(&nwritten);
    let _ = wiggle::GuestPtr::new(&self.shared_memory, nwritten_ptr.try_into()?).write(nwritten)?;

    Ok(format!(r#"[{}]"#, Into::<u16>::into(Errno::Success)).as_bytes().to_owned())
  }

  #[tokio::main(flavor = "current_thread")]
  pub async fn path_open(&mut self, request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    let (
      fd, // The file descriptor.
      dirflags, // Flags determining the method of how the path is resolved.
      path_ptr, // A memory location holding the relative path of the file or directory to open, relative to the path_open::fd directory.
      path_len, // The path length.
      oflags, // The method by which to open the file.
      fs_rights_base, // The initial rights of the newly created file descriptor. The implementation is allowed to return a file descriptor with fewer rights than specified, if and only if those rights do not apply to the type of file being opened. The base rights are rights that will apply to operations using the file descriptor itself, while the inheriting rights are rights that apply to file descriptors derived from it.
      fs_rights_inheriting, // Inheriting rights.
      fdflags, // The fd flags.
      fd_ptr, // A memory location to store the opened file descriptor.
    ) = serde_json::from_str::<(i32, i32, i32, i32, i32, i64, i64, i32, i32)>(std::str::from_utf8(request)?)?;
    dbg!(fd, dirflags, path_ptr, path_len, oflags, fs_rights_base, fs_rights_inheriting, fdflags, fd_ptr);

    let path = &wiggle::GuestPtr::new(&self.shared_memory, (path_ptr.try_into()?, path_len.try_into()?));
    let fd = self.ctx.path_open(
      fd.try_into()?,
      dirflags.try_into()?,
      path,
      oflags.try_into()?,
      fs_rights_base.try_into()?,
      fs_rights_inheriting.try_into()?,
      fdflags.try_into()?,
    ).await?;
    let _ = wiggle::GuestPtr::new(&self.shared_memory, fd_ptr.try_into()?).write(fd)?;

    Ok(format!(r#"[{}]"#, Into::<u16>::into(Errno::Success)).as_bytes().to_owned())
  }

  #[tokio::main(flavor = "current_thread")]
  pub async fn path_remove_directory(&mut self, request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    use preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1;

    let (
      fd,
      path_ptr,
      path_len
    ) = serde_json::from_str::<(i32, i32, i32)>(std::str::from_utf8(request)?)?;
    dbg!(fd, path_ptr, path_len);
    let path = &wiggle::GuestPtr::new(&self.shared_memory, (path_ptr.try_into()?, path_len.try_into()?));

    let _ = self.ctx.path_remove_directory(fd.try_into()?, path).await?;

    Ok(format!(r#"[{}]"#, Into::<u16>::into(Errno::Success)).as_bytes().to_owned())
  }

  #[tokio::main(flavor = "current_thread")]
  pub async fn path_unlink_file(&mut self, request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    use preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1;

    let (
      fd,
      path_ptr,
      path_len,
    ) = serde_json::from_str::<(i32, i32, i32)>(std::str::from_utf8(request)?)?;
    dbg!(fd, path_ptr, path_len);
    let path = &wiggle::GuestPtr::new(&self.shared_memory, (path_ptr.try_into()?, path_len.try_into()?));

    let _ = self.ctx.path_unlink_file(fd.try_into()?, path).await?;

    Ok(format!(r#"[{}]"#, Into::<u16>::into(Errno::Success)).as_bytes().to_owned())
  }
}

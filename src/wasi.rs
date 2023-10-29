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
  pub socket_fd: u32,
}

impl Wasi<'_> {
  pub fn new<'a>(env: &ICoreWebView2Environment, options: Option<WvWasiOptions<'a>>) -> anyhow::Result<Self> {
    let buf_len = 102400; // TODO config
    let mut ctx = wasi_cap_std_sync::WasiCtxBuilder::new().build();
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

    // preopen socket
    let address = "127.0.0.1:9999"; // TODO
    let stdlistener = std::net::TcpListener::bind(address)?;
    // let _ = stdlistener.set_nonblocking(true)?; // nonblock

    let socket = wasi_cap_std_sync::TcpListener::from_std(stdlistener);
    let socket: wasi_cap_std_sync::net::Socket = socket.into();
    let file: Box<dyn wasi_common::file::WasiFile> = socket.into(); // wasi-cap-std-sync-10.0.1/src/lib.rs preopened_socket
    let socket_fd = ctx.push_file(file, wasi_common::file::FileAccessMode::READ | wasi_common::file::FileAccessMode::WRITE)?;

    // root_fd
    let temp_dir = std::env::temp_dir();// default root path is temp_dir
    let root_fd = ctx.push_dir(
      Box::new(get_host_dir(temp_dir.as_path())?),
      std::path::Path::new("/").to_path_buf()
    )?;
    // let root_fd = types::Fd::from(root_fd); // root_fd == 3

    // args push args in ctx
    for arg in std::env::args() {
      let _ = ctx.push_arg(&arg);
    }

    // envs push envs in ctx
    for (var, value) in std::env::vars() {
      let _ = ctx.push_env(var.as_str(), value.as_str());
    }

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
      root_fd,
      socket_fd,
    })
  }

  #[tokio::main(flavor = "current_thread")]
  pub async fn args_get(&mut self, request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    use preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1;

    let (
      argv_ptr,
      argv_buf_ptr,
    ) = serde_json::from_str::<(i32, i32)>(std::str::from_utf8(request)?)?;
    dbg!(&argv_ptr, &argv_buf_ptr);

    // TODO argument error detect

    let argv = &wiggle::GuestPtr::new(&self.shared_memory, argv_ptr.try_into()?);
    let argv_buf = &wiggle::GuestPtr::new(&self.shared_memory, argv_buf_ptr.try_into()?);
    dbg!(&argv, &argv_buf);
    let _ = self.ctx.args_get(argv, argv_buf).await?;

    Ok(format!(r#"[{}]"#, Into::<u16>::into(Errno::Success)).as_bytes().to_owned())
  }

  #[tokio::main(flavor = "current_thread")]
  pub async fn args_sizes_get(&mut self, request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    use preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1;

    let (
      argc_ptr,
      argv_buf_size_ptr,
    ) = serde_json::from_str::<(i32, i32)>(std::str::from_utf8(request)?)?;
    dbg!(&argc_ptr, &argv_buf_size_ptr);

    // TODO argument error detect

    let (argc, argv_buf_size) = self.ctx.args_sizes_get().await?;
    dbg!(argc, argv_buf_size);
    let _ = wiggle::GuestPtr::new(&self.shared_memory, argc_ptr.try_into()?).write(argc)?;
    let _ = wiggle::GuestPtr::new(&self.shared_memory, argv_buf_size_ptr.try_into()?).write(argv_buf_size)?;

    Ok(format!(r#"[{}]"#, Into::<u16>::into(Errno::Success)).as_bytes().to_owned())
  }

  #[tokio::main(flavor = "current_thread")]
  pub async fn environ_get(&mut self, request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    use preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1;

    let (
      environ_ptr,
      environ_buf_ptr,
    ) = serde_json::from_str::<(i32, i32)>(std::str::from_utf8(request)?)?;
    dbg!(&environ_ptr, &environ_buf_ptr);

    // TODO argument error detect

    let environ = &wiggle::GuestPtr::new(&self.shared_memory, environ_ptr.try_into()?);
    let environ_buf = &wiggle::GuestPtr::new(&self.shared_memory, environ_buf_ptr.try_into()?);
    dbg!(&environ, &environ_buf);
    let _ = self.ctx.environ_get(environ, environ_buf).await?;

    Ok(format!(r#"[{}]"#, Into::<u16>::into(Errno::Success)).as_bytes().to_owned())
  }

  #[tokio::main(flavor = "current_thread")]
  pub async fn environ_sizes_get(&mut self, request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    use preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1;

    let (
      environ_count_ptr,
      environ_buf_size_ptr,
    ) = serde_json::from_str::<(i32, i32)>(std::str::from_utf8(request)?)?;
    dbg!(&environ_count_ptr, &environ_buf_size_ptr);

    // TODO argument error detect

    let (environ_count, environ_buf_size) = self.ctx.environ_sizes_get().await?;
    dbg!(environ_count, environ_buf_size);
    let _ = wiggle::GuestPtr::new(&self.shared_memory, environ_count_ptr.try_into()?).write(environ_count)?;
    let _ = wiggle::GuestPtr::new(&self.shared_memory, environ_buf_size_ptr.try_into()?).write(environ_buf_size)?;

    Ok(format!(r#"[{}]"#, Into::<u16>::into(Errno::Success)).as_bytes().to_owned())
  }

  #[tokio::main(flavor = "current_thread")]
  pub async fn clock_res_get(&mut self, request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    use preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1;

    let (
      clockid,
      resolution_ptr,
    ) = serde_json::from_str::<(i32, i32)>(std::str::from_utf8(request)?)?;
    dbg!(&clockid, &resolution_ptr);

    let resolution = self.ctx.clock_res_get(clockid.try_into()?).await?;
    dbg!(resolution);
    let _ = wiggle::GuestPtr::new(&self.shared_memory, resolution_ptr.try_into()?).write(resolution)?;
    dbg!("test");
    Ok(format!(r#"[{}]"#, Into::<u16>::into(Errno::Success)).as_bytes().to_owned())
  }

  #[tokio::main(flavor = "current_thread")]
  pub async fn clock_time_get(&mut self, request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    use preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1;

    let (
      id,
      precision,
      time_ptr,
    ) = serde_json::from_str::<(i32, i64, i32)>(std::str::from_utf8(request)?)?;
    dbg!(&id, &precision, &time_ptr);

    let time = self.ctx.clock_time_get(id.try_into()?, precision.try_into()?).await?;
    dbg!(time);
    let _ = wiggle::GuestPtr::new(&self.shared_memory, time_ptr.try_into()?).write(time)?;

    Ok(format!(r#"[{}]"#, Into::<u16>::into(Errno::Success)).as_bytes().to_owned())
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

    let fdstat = self.ctx.fd_fdstat_get(fd.try_into()?).await?;
    dbg!(&fdstat);
    let _ = wiggle::GuestPtr::new(&self.shared_memory, fdstat_ptr.try_into()?).write(fdstat)?;

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
    dbg!(
      fd, 
      format!("{:#b}", fs_rights_base),
      format!("{:#b}", fs_rights_inheriting),
    );

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
  pub async fn path_create_directory(&mut self, request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    use preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1;

    let (
      fd, // The file descriptor.
      path_ptr, // A memory location that holds the path name.
      path_len, // The length of the path
    ) = serde_json::from_str::<(i32, i32, i32)>(std::str::from_utf8(request)?)?;
    dbg!(fd, path_ptr, path_len);
    let path = &wiggle::GuestPtr::new(&self.shared_memory, (path_ptr.try_into()?, path_len.try_into()?));

    let _ = self.ctx.path_create_directory(fd.try_into()?, path).await?;

    Ok(format!(r#"[{}]"#, Into::<u16>::into(Errno::Success)).as_bytes().to_owned())
  }

  #[tokio::main(flavor = "current_thread")]
  pub async fn path_filestat_get(&mut self, request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    use preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1;

    let (
      fd, // The file descriptor.
      flags, // Flags determining the method of how the path is resolved.
      path_ptr, // A memory location that holds the path name.
      path_len, // The length of the path
      buf, // A memory location to store the file stat.
    ) = serde_json::from_str::<(i32, i32, i32, i32, i32)>(std::str::from_utf8(request)?)?;
    dbg!(fd, flags, path_ptr, path_len, buf);

    let path = &wiggle::GuestPtr::new(&self.shared_memory, (path_ptr.try_into()?, path_len.try_into()?));

    // fd need to be a file fd, not a folder.
    let filestat = self.ctx.path_filestat_get(
      fd.try_into()?,
      flags.try_into()?,
      path,
    ).await?;
    dbg!(&filestat);
    let _ = wiggle::GuestPtr::new(&self.shared_memory, buf.try_into()?).write(filestat)?;

    Ok(format!(r#"[{}]"#, Into::<u16>::into(Errno::Success)).as_bytes().to_owned())
  }

  #[tokio::main(flavor = "current_thread")]
  pub async fn path_filestat_set_times(&mut self, request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    use preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1;
    let (
      fd, // The file descriptor.
      flags, // Flags determining the method of how the path is resolved.
      path_ptr, // A memory location that holds the path name.
      path_len, // The length of the path
      atim, // The desired values of the data access timestamp.
      mtim, // The desired values of the data modification timestamp.
      fst_flags, // A bitmask indicating which timestamps to adjust.
    ) = serde_json::from_str::<(i32, i32, i32, i32, i64, i64, i32)>(std::str::from_utf8(request)?)?;
    dbg!(
      fd,
      format!("{:#b}", flags),
      path_ptr,
      path_len,
      atim,
      mtim,
      format!("{:#b}", fst_flags),
    );

    let path = &wiggle::GuestPtr::new(&self.shared_memory, (path_ptr.try_into()?, path_len.try_into()?));

    // fd need to be a file fd, not a folder.
    let _ = self.ctx.path_filestat_set_times(
      fd.try_into()?,
      flags.try_into()?,
      path,
      atim.try_into()?,
      mtim.try_into()?,
      fst_flags.try_into()?,
    ).await?;

    Ok(format!(r#"[{}]"#, Into::<u16>::into(Errno::Success)).as_bytes().to_owned())
  }

  #[tokio::main(flavor = "current_thread")]
  pub async fn path_link(&mut self, request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    let (
      old_fd, // The file descriptor.
      old_flags, // Flags determining the method of how the path is resolved.
      old_path_ptr, // A memory location that holds the contents of the symbolic link.
      old_path_len, // The length of the old path.
      new_fd, // The working directory at which the resolution of the new path starts.
      new_path_ptr, // A memory location that holds the destination path at which to create the symbolic link.
      new_path_len, // The length of the new path.
    ) = serde_json::from_str::<(i32, i32, i32, i32, i32, i32, i32)>(std::str::from_utf8(request)?)?;
    dbg!(old_fd, old_flags, old_path_ptr, old_path_len, new_fd, new_path_ptr, new_path_len);

    let old_path = &wiggle::GuestPtr::new(&self.shared_memory, (old_path_ptr.try_into()?, old_path_len.try_into()?));
    let new_path = &wiggle::GuestPtr::new(&self.shared_memory, (new_path_ptr.try_into()?, new_path_len.try_into()?));

    let _ = self.ctx.path_link(
      old_fd.try_into()?,
      old_flags.try_into()?,
      old_path.try_into()?,
      new_fd.try_into()?,
      new_path.try_into()?,
    ).await?;

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
    dbg!(
      fd,
      format!("{:#b}", dirflags),
      path_ptr,
      path_len,
      format!("{:#b}", oflags),
      format!("{:#b}", fs_rights_base),
      format!("{:#b}", fs_rights_inheriting),
      format!("{:#b}", fdflags),
      fd_ptr
    );

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
    dbg!(fd);
    let _ = wiggle::GuestPtr::new(&self.shared_memory, fd_ptr.try_into()?).write(fd)?;

    Ok(format!(r#"[{}]"#, Into::<u16>::into(Errno::Success)).as_bytes().to_owned())
  }

  #[tokio::main(flavor = "current_thread")]
  pub async fn path_readlink(&mut self, request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    use preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1;

    let (
      fd,
      path_ptr,
      path_len,
      buf,
      buf_len,
      bufused_ptr,
    ) = serde_json::from_str::<(i32, i32, i32, i32, i32, i32)>(std::str::from_utf8(request)?)?;
    dbg!(fd, path_ptr, path_len, buf, buf_len);
    let path = &wiggle::GuestPtr::new(&self.shared_memory, (path_ptr.try_into()?, path_len.try_into()?));
    let buf_ptr = &wiggle::GuestPtr::new(&self.shared_memory, buf.try_into()?);

    let bufused = self.ctx.path_readlink(
      fd.try_into()?,
      path,
      buf_ptr.try_into()?,
      buf_len.try_into()?
    ).await?;

    dbg!(bufused);
    let _ = wiggle::GuestPtr::new(&self.shared_memory, bufused_ptr.try_into()?).write(bufused)?;

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
  pub async fn path_rename(&mut self, request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    let (
      fd, // The file descriptor. old path should be relative to fd.
      old_path_ptr, // A memory location that holds the contents of the symbolic link.
      old_path_len, // The length of the old path.
      new_fd, // The working directory at which the resolution of the new path starts.
      new_path_ptr, // A memory location that holds the destination path at which to create the symbolic link.
      new_path_len, // The length of the new path.
    ) = serde_json::from_str::<(i32, i32, i32, i32, i32, i32)>(std::str::from_utf8(request)?)?;
    dbg!(fd, old_path_ptr, old_path_len, new_fd, new_path_ptr, new_path_len);

    let old_path = &wiggle::GuestPtr::new(&self.shared_memory, (old_path_ptr.try_into()?, old_path_len.try_into()?));
    let new_path = &wiggle::GuestPtr::new(&self.shared_memory, (new_path_ptr.try_into()?, new_path_len.try_into()?));

    let _ = self.ctx.path_rename(
      fd.try_into()?,
      old_path.try_into()?,
      new_fd.try_into()?,
      new_path.try_into()?,
    ).await?;

    Ok(format!(r#"[{}]"#, Into::<u16>::into(Errno::Success)).as_bytes().to_owned())
  }

  // Only NTFS supports symbolic links in Windows, and only with administrator privileges.
  #[tokio::main(flavor = "current_thread")]
  pub async fn path_symlink(&mut self, request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    let (
      old_path_ptr, // A memory location that holds the contents of the symbolic link.
      old_path_len, // The length of the old path.
      fd, // The file descriptor. old path should be relative to fd.
      new_path_ptr, // A memory location that holds the destination path at which to create the symbolic link.
      new_path_len, // The length of the new path.
    ) = serde_json::from_str::<(i32, i32, i32, i32, i32)>(std::str::from_utf8(request)?)?;
    dbg!(old_path_ptr, old_path_len, fd, new_path_ptr, new_path_len);

    let old_path = &wiggle::GuestPtr::new(&self.shared_memory, (old_path_ptr.try_into()?, old_path_len.try_into()?));
    let new_path = &wiggle::GuestPtr::new(&self.shared_memory, (new_path_ptr.try_into()?, new_path_len.try_into()?));

    let _ = self.ctx.path_symlink(
      old_path.try_into()?,
      fd.try_into()?,
      new_path.try_into()?,
    ).await?;

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

  #[tokio::main(flavor = "current_thread")]
  pub async fn poll_oneoff(&mut self, request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    use preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1;

    let (
      in_ptr,
      out_ptr,
      nsubscriptions,
      nevents_ptr,
    ) = serde_json::from_str::<(i32, i32, i32, i32)>(std::str::from_utf8(request)?)?;
    dbg!(in_ptr, out_ptr, nsubscriptions, nevents_ptr);
    let r#in = &wiggle::GuestPtr::new(&self.shared_memory, in_ptr.try_into()?);
    let out = &wiggle::GuestPtr::new(&self.shared_memory, out_ptr.try_into()?);

    let nevents = self.ctx.poll_oneoff(r#in.try_into()?, out.try_into()?, nsubscriptions.try_into()?).await?;

    let _ = wiggle::GuestPtr::new(&self.shared_memory, nevents_ptr.try_into()?).write(nevents)?;

    Ok(format!(r#"[{}]"#, Into::<u16>::into(Errno::Success)).as_bytes().to_owned())
  }

  #[tokio::main(flavor = "current_thread")]
  pub async fn proc_exit(&mut self, request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    use preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1;

    let (
      rval,
    ) = serde_json::from_str::<(i32,)>(std::str::from_utf8(request)?)?;
    dbg!(rval);

    let _ = self.ctx.proc_exit(rval as u32);

    Ok(format!(r#"[{}]"#, Into::<u16>::into(Errno::Success)).as_bytes().to_owned())
  }

  #[tokio::main(flavor = "current_thread")]
  pub async fn proc_raise(&mut self, request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    use preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1;

    let (
      sig,
    ) = serde_json::from_str::<(i32,)>(std::str::from_utf8(request)?)?;
    dbg!(sig);

    let _ = self.ctx.proc_raise(sig.try_into()?);

    Ok(format!(r#"[{}]"#, Into::<u16>::into(Errno::Success)).as_bytes().to_owned())
  }

  #[tokio::main(flavor = "current_thread")]
  pub async fn sched_yield(&mut self, _request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    use preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1;

    let _ = self.ctx.sched_yield();

    Ok(format!(r#"[{}]"#, Into::<u16>::into(Errno::Success)).as_bytes().to_owned())
  }

  #[tokio::main(flavor = "current_thread")]
  pub async fn random_get(&mut self, request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    use preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1;

    let (
      buf_ptr,
      buf_len,
    ) = serde_json::from_str::<(i32, i32)>(std::str::from_utf8(request)?)?;
    dbg!(buf_ptr, buf_len);

    let buf = &wiggle::GuestPtr::new(&self.shared_memory, buf_ptr.try_into()?);

    let _ = self.ctx.random_get(buf.try_into()?, buf_len.try_into()?);

    Ok(format!(r#"[{}]"#, Into::<u16>::into(Errno::Success)).as_bytes().to_owned())
  }

  #[tokio::main(flavor = "current_thread")]
  pub async fn sock_accept(&mut self, request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    use preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1;

    let (
      fd,
      flags,
      result_fd_ptr,
    ) = serde_json::from_str::<(i32, i32, i32)>(std::str::from_utf8(request)?)?;
    dbg!(fd, flags, result_fd_ptr);

    let result_fd = self.ctx.sock_accept(fd.try_into()?, flags.try_into()?).await?;
    dbg!(result_fd);

    let _ = wiggle::GuestPtr::new(&self.shared_memory, result_fd_ptr.try_into()?).write(result_fd)?;

    Ok(format!(r#"[{}]"#, Into::<u16>::into(Errno::Success)).as_bytes().to_owned())
  }

  #[tokio::main(flavor = "current_thread")]
  pub async fn sock_recv(&mut self, request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    use preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1;

    let (
      fd, // The file descriptor.
      ri_data_ptr, // List of scatter/gather vectors in which to store data.
      ri_data_len, // The length of the iovs.
      ri_flags, // The length of the iovs.
      size_ptr, // A memory location to store the bytes read.
      roflags_ptr, // A memory location to store the bytes read.
    ) = serde_json::from_str::<(i32, i32, i32, i32, i32, i32)>(std::str::from_utf8(request)?)?;
    dbg!(fd, ri_data_ptr, ri_data_len, ri_flags, size_ptr, roflags_ptr);

    // convert ivos_ptr and ivos_len to preview_1::types::IvosArray
    // iovs_ptr need to align to iovs_array's alignment 4.
    let ri_data = preview_1::types::IovecArray::new(&self.shared_memory, (ri_data_ptr.try_into()?, ri_data_len.try_into()?));
    dbg!(ri_data.get(0).unwrap().read()?);

    // fd need to be a file fd, not a folder.
    let (size, roflags) = self.ctx.sock_recv(fd.try_into()?, &ri_data, ri_flags.try_into()?).await?;
    dbg!(&size, &roflags);
    let _ = wiggle::GuestPtr::new(&self.shared_memory, size_ptr.try_into()?).write(size)?;
    let _ = wiggle::GuestPtr::new(&self.shared_memory, roflags_ptr.try_into()?).write(roflags)?;

    Ok(format!(r#"[{}]"#, Into::<u16>::into(Errno::Success)).as_bytes().to_owned())
  }

  #[tokio::main(flavor = "current_thread")]
  pub async fn sock_send(&mut self, request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    use preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1;

    let (
      fd, // The file descriptor.
      si_data_ptr, // List of scatter/gather vectors in which to store data.
      si_data_len, // The length of the iovs.
      si_flags, // The length of the iovs.
      size_ptr, // A memory location to store the bytes read.
    ) = serde_json::from_str::<(i32, i32, i32, i32, i32)>(std::str::from_utf8(request)?)?;
    dbg!(fd, si_data_ptr, si_data_len, si_flags, size_ptr);

    // convert ivos_ptr and ivos_len to preview_1::types::IvosArray
    // iovs_ptr need to align to iovs_array's alignment 4.
    let si_data = preview_1::types::CiovecArray::new(&self.shared_memory, (si_data_ptr.try_into()?, si_data_len.try_into()?));
    dbg!(si_data.get(0).unwrap().read()?);

    // fd need to be a file fd, not a folder.
    let size = self.ctx.sock_send(fd.try_into()?, &si_data, si_flags.try_into()?).await?;
    dbg!(&size);
    let _ = wiggle::GuestPtr::new(&self.shared_memory, size_ptr.try_into()?).write(size)?;

    Ok(format!(r#"[{}]"#, Into::<u16>::into(Errno::Success)).as_bytes().to_owned())
  }

  #[tokio::main(flavor = "current_thread")]
  pub async fn sock_shutdown(&mut self, request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    use preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1;
    // use wasi_common::file::TableFileExt;
    // use wasi_cap_std_sync::net::WasiFile;
    // let socket: wasi_cap_std_sync::net::Socket = socket.into();
    // socket TcpStream

    let (
      fd, // The file descriptor.
      how, // The length of the iovs.
    ) = serde_json::from_str::<(i32, i32)>(std::str::from_utf8(request)?)?;
    dbg!(fd, how);

    let _ = self.ctx.sock_shutdown(fd.try_into()?, how.try_into()?).await?;

    Ok(format!(r#"[{}]"#, Into::<u16>::into(Errno::Success)).as_bytes().to_owned())
  }
}

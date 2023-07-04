use webview2_com::Microsoft::Web::WebView2::Win32::{ ICoreWebView2Environment12, ICoreWebView2SharedBuffer, ICoreWebView2Environment };
use windows::core::ComInterface;
// use wry::{
//   application::window::Window,
//   Result,
//   http::{Request, Response, header}
// };
// use anyhow::Context;
use wasi_common::snapshots::preview_1;
use preview_1::error::Errno;

pub struct Wasi {
  pub ctx: wasi_common::WasiCtx,
  pub shared_buffer: ICoreWebView2SharedBuffer, // windows use ICoreWebView2SharedBuffer, others use a polyfill (host buffer with custom protocol access api)
  pub root_fd: u32,
}

impl Wasi {
  pub fn new(env: &ICoreWebView2Environment) -> anyhow::Result<Self> {
    let buf_len = 10240; // TODO config
    let ctx = wasi_cap_std_sync::WasiCtxBuilder::new().build();
    let temp_dir = std::env::temp_dir();
    let path = temp_dir.as_path(); // default root path is temp_dir TODO config
    // dbg!(&path);
    let dir = wasi_cap_std_sync::Dir::open_ambient_dir(
      path,
      wasi_cap_std_sync::ambient_authority()
    )?;
    let dir = wasi_cap_std_sync::dir::Dir::from_cap_std(dir);
    let root_fd = ctx.push_dir(
      Box::new(dir),
      std::path::PathBuf::new()
    )?;
    // let root_fd = types::Fd::from(root_fd); // root_fd == 3

    let environment = env.cast::<ICoreWebView2Environment12>()?;
    let shared_buffer = unsafe { environment.CreateSharedBuffer(buf_len.try_into()?)? };

    Ok(Wasi {
      ctx,
      shared_buffer,
      root_fd
    })
  }

  #[tokio::main(flavor = "current_thread")]
  pub async fn fd_readdir(&mut self, request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    use preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1;

    let args = std::str::from_utf8(request)?;
    let (fd, buf_ptr, buf_len, cookie, bufused_ptr) = serde_json::from_str::<(u32, u32, u32, u64, u32)>(args)?;

    let shared_buffer = &self.shared_buffer;
    let mut shared_buffer_ptr: *mut u8 = &mut 0u8;
    let _ = unsafe { shared_buffer.Buffer(&mut shared_buffer_ptr) };

    let slice: &mut [u8] = unsafe { std::slice::from_raw_parts_mut(shared_buffer_ptr, buf_len.try_into()?) };
    let guest_memory = wiggle::wasmtime::WasmtimeGuestMemory::new(slice);
    let buf= &wiggle::GuestPtr::new(&guest_memory, buf_ptr);

    let bufused_value = self.ctx.fd_readdir(fd.try_into()?, buf, buf_len.try_into()?, cookie).await?;

    unsafe { *(shared_buffer_ptr.offset(bufused_ptr.try_into()?) as *mut u32) = bufused_value };

    Ok(format!(r#"[{}]"#, Into::<u16>::into(Errno::Success)).as_bytes().to_owned())
  }

  #[tokio::main(flavor = "current_thread")]
  pub async fn path_remove_directory(&mut self, request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    use preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1;

    let args = std::str::from_utf8(request)?;
    let (fd, mut path) = serde_json::from_str::<(u32, String)>(args)?;
    let path_len = path.len();
    let path = unsafe { path.as_bytes_mut() };
    let guest_memory = wiggle::wasmtime::WasmtimeGuestMemory::new(path);
    let path = &wiggle::GuestPtr::new(&guest_memory, (0, path_len as u32));

    let _ = self.ctx.path_remove_directory(fd.try_into()?, path).await?;

    Ok(format!(r#"[{}]"#, Into::<u16>::into(Errno::Success)).as_bytes().to_owned())
  }

  #[tokio::main(flavor = "current_thread")]
  pub async fn path_unlink_file(&mut self, request: &Vec<u8>) -> anyhow::Result<Vec<u8>> {
    use preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1;

    let args = std::str::from_utf8(request)?;
    let (fd, mut path) = serde_json::from_str::<(u32, String)>(args)?;
    let path_len = path.len();
    let path = unsafe { path.as_bytes_mut() };
    let guest_memory = wiggle::wasmtime::WasmtimeGuestMemory::new(path);
    let path = &wiggle::GuestPtr::new(&guest_memory, (0, path_len as u32));

    let _ = self.ctx.path_unlink_file(fd.try_into()?, path).await?;

    Ok(format!(r#"[{}]"#, Into::<u16>::into(Errno::Success)).as_bytes().to_owned())
  }
}

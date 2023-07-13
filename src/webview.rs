pub use wvwasi_wry::webview::*;
use wvwasi_wry::{ webview, http, Error };
use crate::wasi::Wasi;

fn handle_wvwasi_protocol<'a>(
  wasis: &mut Vec<Wasi>,
  request: &http::Request<Vec<u8>>,
  webview: &webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2,
  env: &webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Environment,
  options: Option<WvWasiOptions<'a>>,
) -> std::result::Result<http::Response<std::borrow::Cow<'static, [u8]>>, Error> {
  use wasi_common::snapshots::preview_1::error::Errno;

  let request_path = request.uri().path();
  dbg!(&request_path);
  let request_body = request.body();
  // "[/wasi_index]/wasi_version/wasi_method"
  let response_body = if let Ok(re) = regex::Regex::new(r"(/(\d+))?/(wasi_snapshot_preview1)(/.+)") {
    let (wasi_index, wasi_version, path) = if let Some(caps) = re.captures(request_path) {
      let wasi_index = caps.get(2).map_or("0", |m| m.as_str());
      (
        match std::str::FromStr::from_str(wasi_index) {
          Ok(v) => { v },
          Err(_) => { 0 }
        },
        caps.get(3).map_or("", |m| m.as_str()),
        caps.get(4).map_or("", |m| m.as_str()),
      )
    } else {
      (0, "wasi_snapshot_preview1", "")
    };
    if wasi_version == "wasi_snapshot_preview1" {
      if path == "/init" {
        match Wasi::new(env, options) {
          Ok(wasi) => {
            use webview2_com::Microsoft::Web::WebView2::Win32;
            let wasi_index = wasis.len();
            let shared_buffer = wasi.shared_buffer.clone();
            let root_fd = wasi.root_fd;
            wasis.push(wasi);
            let errno = Into::<u16>::into(Errno::Success);

            let webview_17 = windows::core::ComInterface::cast::<Win32::ICoreWebView2_17>(webview).map_err(webview2_com::Error::WindowsError)?;
            let _ = unsafe {
              webview_17.PostSharedBufferToScript(
                &shared_buffer,
                Win32::COREWEBVIEW2_SHARED_BUFFER_ACCESS_READ_WRITE,
                windows::core::PCWSTR::from_raw(
                  ""
                    .encode_utf16()
                    .chain(Some(0))
                    .collect::<Vec<_>>()
                    .as_ptr(),
                ),
              )
            };
            Ok(format!(r#"[{},{},{}]"#, errno, wasi_index, root_fd).as_bytes().to_owned())
          },
          Err(err) => { Err(err) }
        }
      } else if let Some(wasi) = wasis.get_mut(wasi_index) {
        match path {
          "/fd_advise" => { wasi.fd_advise(request_body) },
          "/fd_allocate" => { wasi.fd_allocate(request_body) },
          "/fd_close" => { wasi.fd_close(request_body) },
          "/fd_datasync" => { wasi.fd_datasync(request_body) },
          "/fd_fdstat_get" => { wasi.fd_fdstat_get(request_body) },
          "/fd_fdstat_set_flags" => { wasi.fd_fdstat_set_flags(request_body) },
          "/fd_fdstat_set_rights" => { wasi.fd_fdstat_set_rights(request_body) },
          "/fd_filestat_get" => { wasi.fd_filestat_get(request_body) },
          "/fd_filestat_set_size" => { wasi.fd_filestat_set_size(request_body) },
          "/fd_filestat_set_times" => { wasi.fd_filestat_set_times(request_body) },
          "/fd_pread" => { wasi.fd_pread(request_body) },
          "/fd_prestat_get" => { wasi.fd_prestat_get(request_body) },
          "/fd_prestat_dir_name" => { wasi.fd_prestat_dir_name(request_body) },
          "/fd_pwrite" => { wasi.fd_pwrite(request_body) },
          "/fd_read" => { wasi.fd_read(request_body) },
          "/fd_readdir" => { wasi.fd_readdir(request_body) },
          "/fd_renumber" => { wasi.fd_renumber(request_body) },
          "/fd_seek" => { wasi.fd_seek(request_body) },
          "/fd_sync" => { wasi.fd_sync(request_body) },
          "/fd_tell" => { wasi.fd_tell(request_body) },
          "/fd_write" => { wasi.fd_write(request_body) },
          "/path_open" => { wasi.path_open(request_body) },
          "/path_remove_directory" => { wasi.path_remove_directory(request_body) },
          "/path_unlink_file" => { wasi.path_unlink_file(request_body) },
          _ => { 
            Ok("".as_bytes().to_owned())
          }
        }
      } else {
        Ok("".as_bytes().to_owned())
      }
    } else {
      Ok("".as_bytes().to_owned())
    }
  } else {
    Ok("".as_bytes().to_owned())
  };

  let response_body = match response_body {
    Ok(vec) => { vec },
    Err(err) => {
      dbg!(&err);
      format!(r#"[{},"{}"]"#, Into::<u16>::into(Errno::Io), err.to_string()).as_bytes().to_owned()
    }
  };

  http::Response::builder()
  .header(http::header::CONTENT_TYPE, "application/json")
  .header(http::header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
  .body(std::borrow::Cow::from(response_body))
  .map_err(Into::into)
}

pub struct WvWasiPreopen<'a> {
  pub path: &'a str, // The path argument here is a path name on the host filesystem
  pub guest_path: &'a str, // The guest_path argument is the name by which it will be known in wasm. https://docs.wasmtime.dev/c-api/wasi_8h.html#a6d738a3510c5f3aa4a6f49d7bb658cd1
}

pub struct WvWasiOptions<'a> {
  // fd_in: u32, // TODO
  // fd_out: u32, // TODO
  // fd_err: u32, // TODO
  // envp: String, // TODO
  // argc: u32; // TODO
  // argv: String; // TODO
  pub preopens: Vec<WvWasiPreopen<'a>>,
  // allocator: u32, // TODO
}
pub trait WebViewBuilderExtWvWasi {
  fn with_wvwasi(self, wv_wasi_options: Option<WvWasiOptions>) -> Self;
}

impl WebViewBuilderExtWvWasi for webview::WebViewBuilder<'_> {
  fn with_wvwasi(self, wv_wasi_options: Option<WvWasiOptions<'_>>) -> Self {
    let mut wasis: Vec<Wasi> = vec![];

    let wv_wasi_preopen = if let Some(
      WvWasiOptions { preopens }
    ) = wv_wasi_options {
      let wv_wasi_preopens: Vec<(String, String)> = preopens
      .into_iter()
      .map(|WvWasiPreopen { guest_path, path }| (
        guest_path.to_owned(),
        path.to_owned()
      ))
      .collect();

      wv_wasi_preopens
    } else {
      vec![]
    };

    self
    // Add wvwasi protocol handler. custom protocols are better than IPC because custom protocols can transmit binary data, while IPC can only encode binary data as strings to achieve transmission, and there may be some problems.https://github.com/webview/webview/issues/613#issuecomment-1080063115
    // for macOS and Linux, the request uri is wvwasi://localhost/fd_readdir
    // for windows it's https://wvwasi.localhost/fd_readdir
    .with_custom_protocol("wvwasi".into(), move |
      request,
      webview,
      env
    | {
      let wv_wasi_preopen: Vec<WvWasiPreopen> = wv_wasi_preopen
        .iter()
        .map(|(guest_path, path)| WvWasiPreopen { 
          guest_path: &guest_path,
          path: &path
        })
        .collect();

      let wv_wasi_options = WvWasiOptions {
        preopens: wv_wasi_preopen
      };

      handle_wvwasi_protocol(&mut wasis, request, webview, env, Some(wv_wasi_options)) 
    })

    // Add wvwasi initialization script
    .with_initialization_script(include_str!("initialization_script.js"))
  }
}
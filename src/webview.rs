pub use wry::webview::*;
use wry::{
  application::window::Window,
  Result,
  http::{Request, Response, header}
};

use crate::wasi::Wasi;

fn handle_wvwasi_protocol(
  wasis: &mut Vec<Wasi>,
  request: &Request<Vec<u8>>,
  webview: &webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2,
  env: &webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Environment
) -> std::result::Result<Response<std::borrow::Cow<'static, [u8]>>, wry::Error> {
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
        match Wasi::new(env) {
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
          "/fd_readdir" => { wasi.fd_readdir(request_body) },
          "/path_remove_directory" => { wasi.path_remove_directory(request_body) },
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

  Response::builder()
  .header(header::CONTENT_TYPE, "application/json")
  .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
  .body(std::borrow::Cow::from(response_body))
  .map_err(Into::into)
}


pub struct WebViewBuilder<'a> {
  inner_web_view_builder: wry::webview::WebViewBuilder<'a>,
  wasis: Vec<Wasi>
}

impl<'a> WebViewBuilder<'a> {
  /// Create [`WebViewBuilder`] from provided [`Window`].
  pub fn new(window: Window) -> Result<Self> {
    Ok(Self {
      inner_web_view_builder: wry::webview::WebViewBuilder::new(window)?,
      wasis: vec![]
    })
  }

  /// Indicates whether horizontal swipe gestures trigger backward and forward page navigation.
  ///
  /// ## Platform-specific:
  ///
  /// - **Android / iOS:** Unsupported.
  pub fn with_back_forward_navigation_gestures(mut self, gesture: bool) -> Self {
    self.inner_web_view_builder = self.inner_web_view_builder.with_back_forward_navigation_gestures(gesture);
    self
  }

  /// Sets whether the WebView should be transparent.
  ///
  /// ## Platform-specific:
  ///
  /// **Windows 7**: Not supported.
  pub fn with_transparent(mut self, transparent: bool) -> Self {
    self.inner_web_view_builder = self.inner_web_view_builder.with_transparent(transparent);
    self
  }

  /// Specify the webview background color. This will be ignored if `transparent` is set to `true`.
  ///
  /// The color uses the RGBA format.
  ///
  /// ## Platfrom-specific:
  ///
  /// - **macOS / iOS**: Not implemented.
  /// - **Windows**:
  ///   - on Windows 7, transparency is not supported and the alpha value will be ignored.
  ///   - on Windows higher than 7: translucent colors are not supported so any alpha value other than `0` will be replaced by `255`
  pub fn with_background_color(mut self, background_color: RGBA) -> Self {
    self.inner_web_view_builder = self.inner_web_view_builder.with_background_color(background_color);
    self
  }

  /// Sets whether the WebView should be transparent.
  pub fn with_visible(mut self, visible: bool) -> Self {
    self.inner_web_view_builder = self.inner_web_view_builder.with_visible(visible);
    self
  }

  /// Sets whether all media can be played without user interaction.
  pub fn with_autoplay(mut self, autoplay: bool) -> Self {
    self.inner_web_view_builder = self.inner_web_view_builder.with_autoplay(autoplay);
    self
  }

  /// Initialize javascript code when loading new pages. When webview load a new page, this
  /// initialization code will be executed. It is guaranteed that code is executed before
  /// `window.onload`.
  ///
  /// ## Platform-specific
  ///
  /// - **Android:** The Android WebView does not provide an API for initialization scripts,
  /// so we prepend them to each HTML head. They are only implemented on custom protocol URLs.
  pub fn with_initialization_script(mut self, js: &str) -> Self {
    self.inner_web_view_builder = self.inner_web_view_builder.with_initialization_script(js);
    self
  }

  /// Register custom file loading protocols with pairs of scheme uri string and a handling
  /// closure.
  ///
  /// The closure takes a [Request] and returns a [Response]
  ///
  /// # Warning
  /// Pages loaded from custom protocol will have different Origin on different platforms. And
  /// servers which enforce CORS will need to add exact same Origin header in `Access-Control-Allow-Origin`
  /// if you wish to send requests with native `fetch` and `XmlHttpRequest` APIs. Here are the
  /// different Origin headers across platforms:
  ///
  /// - macOS and Linux: `<scheme_name>://<path>` (so it will be `wry://examples` in `custom_protocol` example). On Linux, You need to enable `linux-headers` feature flag.
  /// - Windows: `https://<scheme_name>.<path>` (so it will be `https://wry.examples` in `custom_protocol` example)
  /// - Android: For loading content from the `assets` folder (which is copied to the Andorid apk) please
  /// use the function [`with_asset_loader`] from [`WebViewBuilderExtAndroid`] instead.
  /// This function on Android can only be used to serve assets you can embed in the binary or are
  /// elsewhere in Android (provided the app has appropriate access), but not from the `assets`
  /// folder which lives within the apk. For the cases where this can be used, it works the same as in macOS and Linux.
  /// - iOS: Same as macOS. To get the path of your assets, you can call [`CFBundle::resources_path`](https://docs.rs/core-foundation/latest/core_foundation/bundle/struct.CFBundle.html#method.resources_path). So url like `wry://assets/index.html` could get the html file in assets directory.
  ///
  /// [bug]: https://bugs.webkit.org/show_bug.cgi?id=229034
  // #[cfg(feature = "protocol")]
  pub fn with_custom_protocol<F>(mut self, name: String, mut handler: F) -> Self
  where
    F: FnMut(&Request<Vec<u8>>) -> Result<Response<std::borrow::Cow<'static, [u8]>>> + 'static,
  {
    self.inner_web_view_builder = self.inner_web_view_builder.with_custom_protocol(name, move |request, _webview, _env| {
      handler(request)
    });
    self
  }

  /// Set the IPC handler to receive the message from Javascript on webview to host Rust code.
  /// The message sent from webview should call `window.ipc.postMessage("insert_message_here");`.
  pub fn with_ipc_handler<F>(mut self, handler: F) -> Self
  where
    F: Fn(&Window, String) + 'static,
  {
    self.inner_web_view_builder = self.inner_web_view_builder.with_ipc_handler(handler);
    self
  }

  /// Set a handler closure to process incoming [`FileDropEvent`] of the webview.
  ///
  /// # Blocking OS Default Behavior
  /// Return `true` in the callback to block the OS' default behavior of handling a file drop.
  ///
  /// Note, that if you do block this behavior, it won't be possible to drop files on `<input type="file">` forms.
  /// Also note, that it's not possible to manually set the value of a `<input type="file">` via JavaScript for security reasons.
  // #[cfg(feature = "file-drop")]
  pub fn with_file_drop_handler<F>(mut self, handler: F) -> Self
  where
    F: Fn(&Window, FileDropEvent) -> bool + 'static,
  {
    self.inner_web_view_builder = self.inner_web_view_builder.with_file_drop_handler(handler);
    self
  }

  /// Load the provided URL with given headers when the builder calling [`WebViewBuilder::build`] to create the
  /// [`WebView`]. The provided URL must be valid.
  pub fn with_url_and_headers(mut self, url: &str, headers: wry::http::HeaderMap) -> Result<Self> {
    self.inner_web_view_builder = self.inner_web_view_builder.with_url_and_headers(url, headers).unwrap();
    Ok(self)
  }
  
  /// Load the provided URL when the builder calling [`WebViewBuilder::build`] to create the
  /// [`WebView`]. The provided URL must be valid.
  pub fn with_url(mut self, url: &str) -> Result<Self> {
    self.inner_web_view_builder = self.inner_web_view_builder.with_url(url).unwrap();
    Ok(self)
  }

  /// Load the provided HTML string when the builder calling [`WebViewBuilder::build`] to create the
  /// [`WebView`]. This will be ignored if `url` is provided.
  ///
  /// # Warning
  ///
  /// The Page loaded from html string will have `null` origin.
  ///
  /// ## PLatform-specific:
  ///
  /// - **Windows:** the string can not be larger than 2 MB (2 * 1024 * 1024 bytes) in total size
  pub fn with_html(mut self, html: impl Into<String>) -> Result<Self> {
    self.inner_web_view_builder = self.inner_web_view_builder.with_html(html).unwrap();
    Ok(self)
  }

  /// Set the web context that can share with multiple [`WebView`]s.
  pub fn with_web_context(mut self, web_context: &'a mut WebContext) -> Self {
    self.inner_web_view_builder = self.inner_web_view_builder.with_web_context(web_context);
    self
  }

  /// Set a custom [user-agent](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/User-Agent) for the WebView.
  pub fn with_user_agent(mut self, user_agent: &str) -> Self {
    self.inner_web_view_builder = self.inner_web_view_builder.with_user_agent(user_agent);
    self
  }

  /// Enable or disable web inspector which is usually called dev tool.
  ///
  /// Note this only enables dev tool to the webview. To open it, you can call
  /// [`WebView::open_devtools`], or right click the page and open it from the context menu.
  ///
  /// ## Platform-specific
  ///
  /// - macOS: This will call private functions on **macOS**. It's still enabled if set in **debug** build on mac,
  /// but requires `devtools` feature flag to actually enable it in **release** build.
  /// - Android: Open `chrome://inspect/#devices` in Chrome to get the devtools window. Wry's `WebView` devtools API isn't supported on Android.
  /// - iOS: Open Safari > Develop > [Your Device Name] > [Your WebView] to get the devtools window.
  pub fn with_devtools(mut self, devtools: bool) -> Self {
    self.inner_web_view_builder = self.inner_web_view_builder.with_devtools(devtools);
    self
  }

  /// Whether page zooming by hotkeys or gestures is enabled
  ///
  /// ## Platform-specific
  ///
  /// **macOS / Linux / Android / iOS**: Unsupported
  pub fn with_hotkeys_zoom(mut self, zoom: bool) -> Self {
    self.inner_web_view_builder = self.inner_web_view_builder.with_hotkeys_zoom(zoom);
    self
  }

  /// Set a navigation handler to decide if incoming url is allowed to navigate.
  ///
  /// The closure takes a `String` parameter as url and return `bool` to determine the url. True is
  /// allowed to navigate and false is not.
  pub fn with_navigation_handler(mut self, callback: impl Fn(String) -> bool + 'static) -> Self {
    self.inner_web_view_builder = self.inner_web_view_builder.with_navigation_handler(callback);
    self
  }

  /// Set a download started handler to manage incoming downloads.
  ///
  /// The closure takes two parameters - the first is a `String` representing the url being downloaded from and and the
  /// second is a mutable `PathBuf` reference that (possibly) represents where the file will be downloaded to. The latter
  /// parameter can be used to set the download location by assigning a new path to it - the assigned path _must_ be
  /// absolute. The closure returns a `bool` to allow or deny the download.
  pub fn with_download_started_handler(
    mut self,
    started_handler: impl FnMut(String, &mut std::path::PathBuf) -> bool + 'static,
  ) -> Self {
    self.inner_web_view_builder = self.inner_web_view_builder.with_download_started_handler(started_handler);
    self
  }

  /// Sets a download completion handler to manage downloads that have finished.
  ///
  /// The closure is fired when the download completes, whether it was successful or not.
  /// The closure takes a `String` representing the URL of the original download request, an `Option<PathBuf>`
  /// potentially representing the filesystem path the file was downloaded to, and a `bool` indicating if the download
  /// succeeded. A value of `None` being passed instead of a `PathBuf` does not necessarily indicate that the download
  /// did not succeed, and may instead indicate some other failure - always check the third parameter if you need to
  /// know if the download succeeded.
  ///
  /// ## Platform-specific:
  ///
  /// - **macOS**: The second parameter indicating the path the file was saved to is always empty, due to API
  /// limitations.
  pub fn with_download_completed_handler(
    mut self,
    download_completed_handler: impl Fn(String, Option<std::path::PathBuf>, bool) + 'static,
  ) -> Self {
    self.inner_web_view_builder = self.inner_web_view_builder.with_download_completed_handler(download_completed_handler);
    self
  }

  /// Enables clipboard access for the page rendered on **Linux** and **Windows**.
  ///
  /// macOS doesn't provide such method and is always enabled by default. But you still need to add menu
  /// item accelerators to use shortcuts.
  pub fn with_clipboard(mut self, clipboard: bool) -> Self {
    self.inner_web_view_builder = self.inner_web_view_builder.with_clipboard(clipboard);
    self
  }

  /// Set a new window request handler to decide if incoming url is allowed to be opened.
  ///
  /// The closure takes a `String` parameter as url and return `bool` to determine if the url can be
  /// opened in a new window. Returning true will open the url in a new window, whilst returning false
  /// will neither open a new window nor allow any navigation.
  pub fn with_new_window_req_handler(
    mut self,
    callback: impl Fn(String) -> bool + 'static,
  ) -> Self {
    self.inner_web_view_builder = self.inner_web_view_builder.with_new_window_req_handler(callback);
    self
  }

  /// Sets whether clicking an inactive window also clicks through to the webview. Default is `false`.
  ///
  /// ## Platform-specific
  ///
  /// This configuration only impacts macOS.
  pub fn with_accept_first_mouse(mut self, accept_first_mouse: bool) -> Self {
    self.inner_web_view_builder = self.inner_web_view_builder.with_accept_first_mouse(accept_first_mouse);
    self
  }

  /// Set a handler closure to process the change of the webview's document title.
  pub fn with_document_title_changed_handler(
    mut self,
    callback: impl Fn(&Window, String) + 'static,
  ) -> Self {
    self.inner_web_view_builder = self.inner_web_view_builder.with_document_title_changed_handler(callback);
    self
  }

  /// Run the WebView with incognito mode. Note that WebContext will be ingored if incognito is
  /// enabled.
  ///
  /// ## Platform-specific:
  ///
  /// - **Android:** Unsupported yet.
  pub fn with_incognito(mut self, incognito: bool) -> Self {
    self.inner_web_view_builder = self.inner_web_view_builder.with_incognito(incognito);
    self
  }

  /// Consume the builder and create the [`WebView`].
  ///
  /// Platform-specific behavior:
  ///
  /// - **Unix:** This method must be called in a gtk thread. Usually this means it should be
  /// called in the same thread with the [`EventLoop`] you create.
  ///
  /// [`EventLoop`]: crate::application::event_loop::EventLoop
  pub fn build(mut self) -> Result<WebView> {
    self.inner_web_view_builder

    // Add wvwasi protocol handler. custom protocols are better than IPC because custom protocols can transmit binary data, while IPC can only encode binary data as strings to achieve transmission, and there may be some problems.https://github.com/webview/webview/issues/613#issuecomment-1080063115
    // for macOS and Linux, the request uri is wvwasi://localhost/fd_readdir
    // for windows it's https://wvwasi.localhost/fd_readdir
    .with_custom_protocol("wvwasi".into(), move |request, webview, env| {
      handle_wvwasi_protocol(&mut self.wasis, request, webview, env)
    })

    // Add wvwasi initialization script
    .with_initialization_script(include_str!("initialization_script.js"))

    .build()
  }
}

#[cfg(windows)]
impl wry::webview::WebViewBuilderExtWindows for WebViewBuilder<'_> {
  fn with_additional_browser_args<S: Into<String>>(mut self, additional_args: S) -> Self {
    self.inner_web_view_builder = self.inner_web_view_builder.with_additional_browser_args(additional_args);
    self
  }

  fn with_browser_accelerator_keys(mut self, enabled: bool) -> Self {
    self.inner_web_view_builder = self.inner_web_view_builder.with_browser_accelerator_keys(enabled);
    self
  }

  fn with_theme(mut self, theme: Theme) -> Self {
    self.inner_web_view_builder = self.inner_web_view_builder.with_theme(theme);
    self
  }
}

#[cfg(target_os = "android")]
impl wry::webview::WebViewBuilderExtAndroid for WebViewBuilder<'_> {
  fn on_webview_created<
    F: Fn(
        wry::webview::prelude::Context<'_>,
      ) -> std::result::Result<(), tao::platform::android::ndk_glue::jni::errors::Error>
      + Send
      + 'static,
  >(
    mut self,
    f: F,
  ) -> Self {
    self.inner_web_view_builder = self.inner_web_view_builder.on_webview_created(f);
    self
  }

  #[cfg(feature = "protocol")]
  fn with_asset_loader(mut self, protocol: String) -> Self {
    self.inner_web_view_builder = self.inner_web_view_builder.with_asset_loader(protocol);
    self
  }
}

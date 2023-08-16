fn main() -> wvwasi::Result<()> {
  use wvwasi::{
    application::{
      event::{Event, StartCause, WindowEvent},
      event_loop::{ControlFlow, EventLoop},
      window::WindowBuilder,
    },
    webview::{ WebViewBuilder, WebViewBuilderExtWvWasi, WvWasiOptions, WvWasiPreopen },
  };

  let event_loop = EventLoop::new();
  let window = WindowBuilder::new()
    .with_title("Hello World")
    .build(&event_loop)?;
  let temp_dir = std::env::temp_dir();
  dbg!(&temp_dir);
  let _ = std::fs::create_dir(temp_dir.join("test"))?;
  let _ = std::fs::write(temp_dir.join("test.txt"), "Hi, I'm wvwasi")?;
  let _webview = WebViewBuilder::new(window)?
    .with_wvwasi(Some(WvWasiOptions {
      preopens: if let Some(temp_dir) = temp_dir.to_str() {
        vec![WvWasiPreopen { 
          guest_path: "/",
          path: temp_dir // test C:\Users\<username>\AppData\Local\Temp as root folder
        }]
      } else {
        vec![]
      }
    }))
    .with_html(include_str!("hello_world.html"))?
    .build()?;

  event_loop.run(move |event, _, control_flow| {
    *control_flow = ControlFlow::Wait;

    match event {
      Event::NewEvents(StartCause::Init) => println!("wvwasi webview has started!"),
      Event::WindowEvent {
        event: WindowEvent::CloseRequested,
        ..
      } => *control_flow = ControlFlow::Exit,
      _ => (),
    }
  });

}

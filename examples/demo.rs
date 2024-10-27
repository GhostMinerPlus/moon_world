use std::{
    sync::mpsc::{channel, Sender},
    thread,
};

use edge_lib::util::{
    data::{AsDataManager, MemDataManager},
    Path,
};
use moon_world::{err, util::engine::EngineBuilder};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

static mut WINDOW_OP: Option<Window> = None;

struct Application {
    tx_op: Option<Sender<json::JsonValue>>,
}

impl Application {
    fn new() -> Self {
        Self { tx_op: None }
    }

    fn run(mut self) -> err::Result<()> {
        log::info!("run");
        let event_loop = EventLoop::new().map_err(|e| {
            log::error!("{e}\nat EventLoop::new");

            moon_err::Error::new(
                err::ErrorKind::Other(format!("EventLoopError")),
                format!("failed to create EventLoop"),
                format!("at EventLoop::new"),
            )
        })?;

        event_loop.set_control_flow(ControlFlow::Poll);

        event_loop.run_app(&mut self).map_err(|e| {
            log::error!("{e}\nat EventLoop::run_app");

            moon_err::Error::new(
                err::ErrorKind::Other(format!("EventLoopError")),
                format!("failed to run app"),
                format!("at EventLoop::run_app"),
            )
        })
    }
}

impl ApplicationHandler for Application {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        unsafe {
            WINDOW_OP = Some(
                event_loop
                    .create_window(Window::default_attributes())
                    .unwrap(),
            )
        };

        let engine_builder =
            EngineBuilder::from_window(unsafe { WINDOW_OP.as_ref().unwrap() }).unwrap();
        let (tx, rx) = channel::<json::JsonValue>();
        self.tx_op = Some(tx.clone());
        thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async move {
                let mut dm: Box<MemDataManager> = Box::new(MemDataManager::new(None));

                dm.set(
                    &Path::from_str("Main->$w:view"),
                    vec![
                        format!("$->$:state->$:pos if $->$:state->$:pos $->$:props->$:pos"),
                        //
                        format!("$->$:root = ? _"),
                        format!("$->$:phy_ball = ? _"),
                        format!("$->$:vi_ball = ? _"),
                        //
                        format!("$->$:phy_ball->$:class = Physics:ball _"),
                        format!("$->$:phy_ball->$:props = ? _"),
                        format!("$->$:vi_ball->$:class = Vision:ball _"),
                        format!("$->$:vi_ball->$:props = ? _"),
                        format!("$->$:root->$:class = div _"),
                        format!("$->$:root->$:child = $->$:phy_ball _"),
                        format!("$->$:root->$:child += $->$:root->$:child $->$:vi_ball"),
                        //
                        format!("$->$:phy_ball->$:props->$:onstep = '$->$:state->$:pos\\s$world2_get_pos\\s$->$:vnode_id\\s_' _"),
                        format!("$->$:phy_ball->$:props->$:watcher = true _"),
                        format!("$->$:phy_ball->$:props->$:pos = $->$:state->$:pos _"),
                        format!("$->$:vi_ball->$:props->$:pos = $->$:state->$:pos _"),
                        //
                        format!("$->$:output dump $->$:root $"),
                    ],
                )
                .await
                .unwrap();

                let mut engine = engine_builder.build(dm).await.unwrap();
                loop {
                    while let Ok(event) = rx.try_recv() {
                        let entry_name = event["entry_name"].as_str().unwrap();
                        let event = &event["event"];
                        if let Err(e) = engine.event_handler(entry_name, event).await {
                            log::error!("{e:?}\nat event_handler");
                        }
                    }
                    if let Err(e) = engine.step().await {
                        log::error!("{e:?}\nat step");
                    }
                    if let Err(e) = engine.render() {
                        log::error!("{e:?}\nat render");
                    }
                }
            });
        });
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match &event {
            WindowEvent::CloseRequested => {
                log::info!("The close button was pressed; stopping");
                event_loop.exit();
            }
            WindowEvent::Resized(n_sz) => {
                let _ = self.tx_op.as_ref().unwrap().send(json::object! {
                    "entry_name": "onresize",
                    "event": {
                        "width": n_sz.width,
                        "height": n_sz.height,
                    }
                });
            }
            _ => (),
        }
    }
}

fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("warn,demo=debug,world2=debug"),
    )
    .init();
    Application::new().run().unwrap()
}

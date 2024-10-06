use std::{
    collections::HashMap,
    sync::mpsc::{channel, Sender},
    thread,
};

use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};
use world2::{err, util::engine::EngineBuilder};

static mut WINDOW_OP: Option<Window> = None;

pub struct Application {
    tx_op: Option<Sender<json::JsonValue>>,
}

impl Application {
    pub fn new() -> Self {
        Self { tx_op: None }
    }

    pub fn run(mut self) -> err::Result<()> {
        log::info!("run");
        let event_loop = EventLoop::new().map_err(err::map_append("\nat EventLoop::new"))?;
        event_loop.set_control_flow(ControlFlow::Poll);
        event_loop
            .run_app(&mut self)
            .map_err(err::map_append("\nat run_app"))
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

        let mut view_class = HashMap::new();

        view_class.insert(
            "Main".to_string(),
            vec![
                format!("$->$:onclick = '$->$:output\\s+\\s1\\s1','$->$:output\\s+\\s$->$:output\\s1' _"),
                format!("$->$:root = ? _"),
                format!("$->$:ball = ? _"),
                //
                format!("$->$:ball->$:class = ball _"),
                format!("$->$:ball->$:props = ? _"),
                format!("$->$:root->$:class = div _"),
                format!("$->$:root->$:child = $->$:ball _"),
                format!("$->$:root->$:props = ? _"),
                //
                format!("$->$:ball->$:props->$:watcher = true _"),
                format!("$->$:root->$:props->$:onclick = $->$:onclick _"),
                //
                format!("$->$:output dump $->$:root $"),
            ],
        );

        let engine_builder =
            EngineBuilder::from_window(unsafe { WINDOW_OP.as_ref().unwrap() }, view_class).unwrap();
        let (tx, rx) = channel::<json::JsonValue>();
        self.tx_op = Some(tx.clone());
        thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async move {
                let mut engine = engine_builder.build().await.unwrap();
                loop {
                    while let Ok(event) = rx.try_recv() {
                        let entry_name = event["entry_name"].as_str().unwrap();
                        let event = &event["event"];
                        if let Err(e) = engine.event_handler(entry_name, event).await {
                            log::error!("{e}\nat event_handler");
                        }
                    }
                    if let Err(e) = engine.step() {
                        log::error!("{e}\nat step");
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

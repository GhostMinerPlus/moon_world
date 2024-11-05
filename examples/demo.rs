use std::{
    sync::mpsc::{channel, Sender},
    thread,
};

use error_stack::ResultExt;
use moon_class::{
    util::{inc_v_from_str, str_of_value},
    AsClassManager, ClassExecutor, ClassManager,
};
use moon_world::{err, EngineBuilder};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
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
        let event_loop = EventLoop::new().change_context(err::Error::Other)?;

        event_loop.set_control_flow(ControlFlow::Poll);

        event_loop
            .run_app(&mut self)
            .change_context(err::Error::Other)
    }
}

impl ApplicationHandler for Application {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        unsafe {
            WINDOW_OP = Some(
                event_loop
                    .create_window(
                        Window::default_attributes().with_inner_size(PhysicalSize::new(1024, 1024)),
                    )
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
                let mut dm: Box<ClassManager> = Box::new(ClassManager::new());

                ClassExecutor::new(&mut *dm)
                    .execute(
                        &inc_v_from_str(&format!(
                            "{} = view[Main];",
                            str_of_value(&format!(
                                "div = $class[root];
                                Vision:light3 = $class[light3];
                                Vision:cube3 = $class[cube3];
                                Input:window = $class[window];
                                cube3 = $child[root];
                                light3 = $child[root];
                                window = $child[root];
                                ? = $props[window];
                                '$data[] = @new_size[@window];' = $onresize[$props[window]];

                                $class = $class[];
                                $props = $class[];
                                $onresize = $class[];
                                $child = $class[];
                                root = $source[];
                                dump[] = $result[];"
                            ))
                        ))
                        .unwrap(),
                    )
                    .await
                    .unwrap();

                let mut engine = engine_builder.build(dm).await.unwrap();
                loop {
                    while let Ok(event) = rx.try_recv() {
                        let entry_name = event["entry_name"].as_str().unwrap();
                        let data = &event["data"];

                        engine.event_handler(entry_name, data).await.unwrap();
                    }

                    engine.step().await.unwrap();

                    engine.render().unwrap();
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
                    "entry_name": "$onresize",
                    "data": {
                        "@width": n_sz.width,
                        "@height": n_sz.height,
                    }
                });
            }
            WindowEvent::CursorMoved {
                device_id: _,
                position,
            } => {
                let _ = self.tx_op.as_ref().unwrap().send(json::object! {
                    "entry_name": "$onmousemove",
                    "data": {
                        "@x": position.x,
                        "@y": position.y,
                    }
                });
            }
            _ => (),
        }
    }
}

fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info,wgpu=error,demo=debug,moon_world=debug,view_manager=debug"),
    )
    .init();

    Application::new().run().unwrap()
}

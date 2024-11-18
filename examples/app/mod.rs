use std::{
    sync::mpsc::{channel, Sender},
    thread,
    time::Duration,
};

use error_stack::ResultExt;
use moon_world::{err, EngineBuilder};
use tokio::time::sleep;
use view_manager::ViewProps;
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

mod state {
    use winit::window::Window;

    pub static mut WINDOW_OP: Option<Window> = None;
    pub static mut IS_SAVED: bool = false;
    pub static mut IS_VISIBLE: bool = true;
}
mod inner {
    use moon_class::{util::executor::ClassExecutor, ClassManager};

    use super::state;

    pub async fn mock_data() -> ClassManager {
        let mut cm = ClassManager::new();

        let mut ce = ClassExecutor::new(&mut cm);

        ce.execute_script(include_str!("../class/demo.class"))
            .await
            .unwrap();

        cm
    }

    pub fn set_mouse_visible(is_visible: bool) {
        unsafe {
            state::IS_VISIBLE = is_visible;

            state::WINDOW_OP
                .as_ref()
                .unwrap()
                .set_cursor_visible(is_visible);
        };
    }

    pub fn mouse_is_visible() -> bool {
        unsafe { state::IS_VISIBLE }
    }
}

pub struct Application {
    tx_op: Option<Sender<json::JsonValue>>,
}

impl Application {
    pub fn new() -> Self {
        Self { tx_op: None }
    }

    pub fn run(mut self) -> err::Result<()> {
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
            state::WINDOW_OP = Some(
                event_loop
                    .create_window(Window::default_attributes())
                    .unwrap(),
            )
        };

        let engine_builder =
            EngineBuilder::from_window(unsafe { state::WINDOW_OP.as_ref().unwrap() }).unwrap();
        let (tx, rx) = channel::<json::JsonValue>();
        self.tx_op = Some(tx.clone());
        thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async move {
                let mut engine = engine_builder
                    .build(Box::new(inner::mock_data().await))
                    .await
                    .unwrap();

                engine
                    .init(ViewProps {
                        class: "Main".to_string(),
                        props: json::Null,
                    })
                    .await;

                let mut alive = true;

                loop {
                    while let Ok(event) = rx.try_recv() {
                        let entry_name = event["entry_name"].as_str().unwrap();

                        let data = &event["data"];

                        engine.event_handler(entry_name, data).await.unwrap();

                        if entry_name == "$onclose" {
                            alive = false;

                            break;
                        }
                    }

                    if !alive {
                        break;
                    }

                    engine.step().await.unwrap();

                    engine.render().unwrap();

                    sleep(Duration::from_millis(10)).await;
                }

                unsafe {
                    state::IS_SAVED = true;
                }
            });
        });
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        match event {
            DeviceEvent::MouseMotion { delta: (dx, dy) } => {
                if !inner::mouse_is_visible() {
                    let window = unsafe { state::WINDOW_OP.as_ref().unwrap() };

                    let unit = window.inner_size().height as f64;

                    let _ = self.tx_op.as_ref().unwrap().send(json::object! {
                        "entry_name": "$cursormoved",
                        "data": {
                            "$x": dx / unit,
                            "$y": dy / unit
                        }
                    });
                }
            }
            _ => (),
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match &event {
            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => {
                if event.state.is_pressed() {
                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::Escape) => {
                            inner::set_mouse_visible(!inner::mouse_is_visible());
                        }
                        _ => {
                            let _ = self.tx_op.as_ref().unwrap().send(json::object! {
                                "entry_name": "$onkeydown",
                                "data": {
                                    "$key": event.logical_key.to_text(),
                                }
                            });
                        }
                    }
                } else {
                    let _ = self.tx_op.as_ref().unwrap().send(json::object! {
                        "entry_name": "$onkeyup",
                        "data": {
                            "$key": event.logical_key.to_text(),
                        }
                    });
                }
            }
            WindowEvent::CloseRequested => {
                log::info!("The close button was pressed; stopping");
                let _ = self.tx_op.as_ref().unwrap().send(json::object! {
                    "entry_name": "$onclose",
                    "data": {}
                });

                while !unsafe { state::IS_SAVED } {
                    std::thread::sleep(Duration::from_millis(10));
                }

                event_loop.exit();
            }
            WindowEvent::Resized(n_sz) => {
                let _ = self.tx_op.as_ref().unwrap().send(json::object! {
                    "entry_name": "$onresize",
                    "data": {
                        "$width": n_sz.width,
                        "$height": n_sz.height,
                    }
                });
            }
            _ => (),
        }
    }
}

use std::{
    sync::mpsc::{channel, Sender},
    thread,
    time::Duration,
};

use deno_cm::CmRuntime;
use error_stack::ResultExt;
use moon_class::{util::str_of_value, ClassManager};
use moon_world::{err, EngineBuilder};
use tokio::time::sleep;
use view_manager::ViewProps;
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
                let mut engine = engine_builder.build(Box::new(ClassManager::new())).await.unwrap();

                let mut cm_runtime = CmRuntime::new(engine.clone());

                cm_runtime
                    .execute_script_local(format!(
                        r#"
    await Deno.core.ops.cm_append("view", "Main", [{}]);
            "#,
                        str_of_value(
                            r#"const root = {};

const pos = context.state.pos? context.state.pos: [0.0, 0.0, 0.0];
const pos1 = context.state.pos1? context.state.pos1: [0.0, 0.0, 0.0];

root.$class = 'div';
root.$child = [
  {$class: 'Vision:light3', $props: {position: [0.0, 5.0, 0.0]} },
  {$class: 'Vision:cube3', $props: {position: pos, color: [0.2, 0.4, 1.0]} },
  {$class: 'Vision:cube3', $props: {position: pos1, color: [0.6, 1.0, 0.5]} },
  {$class: 'Physics:cube3', $props: {body_type: 'dynamic', position: [-1.0, 2.0, -3.0], $onstep: 'context.state.pos = (await Deno.core.ops.cm_get("@moon_world_pos", context.vnode_id.toString())).map(s => Number(s));return context.state;'} },
  {$class: 'Physics:cube3', $props: {position: [-1.0, 0.0, -3.0], $onstep: 'context.state.pos1 = (await Deno.core.ops.cm_get("@moon_world_pos", context.vnode_id.toString())).map(s => Number(s));return context.state;'} },
  {
    $class: 'Input:window',
    $props: {
      $onresize: 'await Deno.core.ops.cm_append("@new_size", "@window", [JSON.stringify(context.data)]);',
      $onkeydown: 'const step = {x: 0, y: 0, z: 0};if (context.data.key == "w") { step.y += 0.1; } else if (context.data.key == "s") { step.y -= 0.1; }await Deno.core.ops.cm_append("@new_step", "@camera", [JSON.stringify(step)]);'
    }
  }
];

return root;"#
                        )
                    ))
                    .await
                    .unwrap();

                engine
                    .init(
                        &mut cm_runtime,
                        ViewProps {
                            class: "Main".to_string(),
                            props: json::Null,
                        },
                    )
                    .await;

                loop {
                    while let Ok(event) = rx.try_recv() {
                        let entry_name = event["entry_name"].as_str().unwrap();
                        let data = &event["data"];

                        engine
                            .event_handler(&mut cm_runtime, entry_name, data)
                            .await
                            .unwrap();
                    }

                    engine.step(&mut cm_runtime).await.unwrap();

                    engine.render().unwrap();

                    sleep(Duration::from_millis(10)).await;
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
                        "width": n_sz.width,
                        "height": n_sz.height,
                    }
                });
            }
            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => {
                if event.state.is_pressed() {
                    let _ = self.tx_op.as_ref().unwrap().send(json::object! {
                        "entry_name": "$onkeydown",
                        "data": {
                            "key": event.logical_key.to_text(),
                        }
                    });
                }
            }
            _ => (),
        }
    }
}

fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::default()
            .default_filter_or("info,wgpu=warn,demo=debug,moon_world=debug"),
    )
    .init();

    Application::new().run().unwrap()
}

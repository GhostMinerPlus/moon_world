use std::{
    sync::mpsc::{channel, Sender},
    thread,
    time::Duration,
};

use error_stack::ResultExt;
use moon_class::{util::executor::ClassExecutor, ClassManager};
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
                let mut engine = engine_builder
                    .build(Box::new(ClassManager::new()))
                    .await
                    .unwrap();

                let mut cm_runtime = ClassExecutor::new(&mut engine);

                cm_runtime
                    .execute_script(
                        r#"
<
    #if({
        $left: $position($state()),
        $right: $position($props())
    }) = $position();
    #if({
        $left: $color($props()),
        $right: [0.2, 0.4, 1.0]
    }) = $color();

    {
        $class: div,
        $child: [
            {
                $class: Vision:cube3,
                $props: {
                    $position: $position(),
                    $color: $color()
                }
            },
            {
                $class: Physics:cube3,
                $props: {
                    $position: $position($props()),
                    $body_type: $body_type($props()),
                    $onstep: <
                        @moon_world_pos($vnode_id()) := $position($state());

                        $state() = $result();
                    >
                }
            }
        ]
    } = $result();
> = view(Box);

<
    #if({
        $left: $pos($state()),
        $right: [0.0, 0.0, 0.0]
    }) = $pos();
    #if({
        $left: $pos1($state()),
        $right: [0.0, 0.0, 0.0]
    }) = $pos1();

    {
        $class: div,
        $child: [
            {$class: Vision:light3, $props: {$position: [0.0, 5.0, 0.0]} },
            {$class: Box, $props: {$position: [-1.0, 2.0, -3.0], $color: [0.2, 0.4, 1.0], $body_type: dynamic} },
            {$class: Box, $props: {$position: [-1.0, 0.0, -3.0], $color: [0.6, 1.0, 0.5]} },
            {
                $class: Input:window,
                $props: {
                    $onresize: <#dump($data()) = @new_size(@window);>,
                    $onkeydown: <
                        0.0 = $x($step);
                        0.0 = $y($step);
                        0.0 = $z($step);

                        [
                            {
                                $case: <#inner({ $left: w, $right: $key($data())}) := $result();>,
                                $then: <-0.1 := $z($step);>
                            },
                            {
                                $case: <#inner({ $left: s, $right: $key($data())}) := $result();>,
                                $then: <0.1 := $z($step);>
                            },
                            {
                                $case: <#inner({ $left: a, $right: $key($data())}) := $result();>,
                                $then: <-0.1 := $x($step);>
                            },
                            {
                                $case: <#inner({ $left: d, $right: $key($data())}) := $result();>,
                                $then: <0.1 := $x($step);>
                            },
                            {
                                $case: <#inner({ $left: c, $right: $key($data())}) := $result();>,
                                $then: <-0.1 := $y($step);>
                            },
                            {
                                $case: <#inner({ $left: " ", $right: $key($data())}) := $result();>,
                                $then: <0.1 := $y($step);>
                            }
                        ] = $switch();

                        #dump($step) = @new_step(@camera);

                        [] := $result();
                    >
                }
            }
        ]
    } = $result();
> := view(Main);
            "#,
                    )
                    .await
                    .unwrap();

                engine
                    .init(ViewProps {
                        class: "Main".to_string(),
                        props: json::Null,
                    })
                    .await;

                loop {
                    while let Ok(event) = rx.try_recv() {
                        let entry_name = event["entry_name"].as_str().unwrap();
                        let data = &event["data"];

                        engine.event_handler(entry_name, data).await.unwrap();
                    }

                    engine.step().await.unwrap();

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
                        "$width": n_sz.width,
                        "$height": n_sz.height,
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
                            "$key": event.logical_key.to_text(),
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
        env_logger::Env::default().default_filter_or("info,wgpu=warn,demo=debug,moon_world=debug"),
    )
    .init();

    Application::new().run().unwrap()
}

use super::Engine;

pub fn step<D, E>(engine: &mut Engine<D, E>) {
    inner::clean_dead(engine);

    engine
        .scene_mp
        .get_mut(&engine.cur_scene_id)
        .unwrap()
        .step();

    inner::pull_collision_event(engine);
    inner::pull_force_event(engine);
    inner::on_step(engine);

    engine.time_stamp += 1;
}

mod inner {
    use crate::util::engine::{handle::SceneHandle, Engine};

    pub fn clean_dead<D, E>(engine: &mut Engine<D, E>) {
        let scene = engine.scene_mp.get_mut(&engine.cur_scene_id).unwrap();
        let dead_id_v = scene
            .body_mp
            .iter_mut()
            .map(|(id, body)| {
                if let Some(life_step) = body.life_step_op.as_mut() {
                    if *life_step == 0 {
                        return Some(*id);
                    }
                    *life_step -= 1;
                }
                None
            })
            .filter(|op| op.is_some())
            .map(|op| op.unwrap())
            .collect::<Vec<u64>>();
        for id in &dead_id_v {
            scene.remove_body(id);
        }
    }

    pub fn pull_collision_event<D, E>(engine: &mut Engine<D, E>) {
        let scene_id = engine.cur_scene_id;
        loop {
            let scene = engine.scene_mp.get(&scene_id).unwrap();
            let event_op = scene.collision_event_rx.try_recv();
            if event_op.is_err() {
                break;
            }
            if let Some(on_collision_event) = &scene.on_collision_event {
                (*on_collision_event.clone())(SceneHandle { engine, scene_id }, event_op.unwrap());
            }
        }
    }

    pub fn pull_force_event<D, E>(engine: &mut Engine<D, E>) {
        let scene_id = engine.cur_scene_id;
        loop {
            let scene = engine.scene_mp.get(&scene_id).unwrap();
            let event_op = scene.force_event_rx.try_recv();
            if event_op.is_err() {
                break;
            }
            if scene.on_force_event.is_none() {
                continue;
            }
            let on_force_event_op = scene.on_force_event.clone();
            (*on_force_event_op.as_ref().unwrap())(
                SceneHandle { engine, scene_id },
                event_op.unwrap(),
            );
        }
    }

    pub fn on_step<D, E>(engine: &mut Engine<D, E>) {
        let scene_id = engine.cur_scene_id;
        let time_stamp = engine.time_stamp;
        let scene = engine.scene_mp.get(&scene_id).unwrap();
        if scene.on_step.is_some() {
            let listener = scene.on_step.as_ref().unwrap().clone();
            (*listener)(SceneHandle { engine, scene_id }, time_stamp);
        }
    }
}

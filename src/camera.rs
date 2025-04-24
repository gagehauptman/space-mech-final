use std::slice::Windows;
use bevy::input::keyboard::KeyboardInput;
use bevy::input::mouse::{MouseButtonInput, MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use bevy::transform;
use bevy::window::PrimaryWindow;
use crate::*;
use chrono::{DateTime, Duration, Local};

#[derive(Component)]
pub struct CameraState {
    pub pan: f64,
    pub tilt: f64,
    pub dist: f64,
    pub focused: u32,
}

pub fn camera_controller(
    state_keeper: Res<StateKeeper>,
    camera: Single<(&Camera, &GlobalTransform, &mut CameraState)>,
    camera_grid: Single<(&mut GridCell<i64>, &mut Transform), With<FloatingOrigin>>,
    root_grid: Single<&Grid<i64>, With<BigSpace>>,

    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut evr_scroll: EventReader<MouseWheel>,
    mouse_input: Res<ButtonInput<MouseButton>>,

    mut query1: Query<(&BodyOverlayDisplay, &mut Node, &Text), Without<TextOverlay>>,
    query2: Query<(&BodyDisplay, &ObjectID, &GlobalTransform)>,
    mut query3: Query<(&Node, &mut Text, &TextOverlay), (With<TextOverlay>, Without<BodyOverlayDisplay>)>,
    q_windows: Query<&Window, With<PrimaryWindow>>,
) {
    let (camera, camera_global_transform, mut camera_state) = camera.into_inner();
    let root_grid = root_grid.into_inner();

    let mut abs_offset = state_keeper.state.get(&state_keeper.current_step).unwrap().get(&camera_state.focused).unwrap()[0];

    let speed = 1.0;

    if mouse_input.just_pressed(MouseButton::Left) {
        if let Some(position) = q_windows.single().cursor_position() {
            let pos_x = q_windows.single().width() - position.x;
            let pos_y = position.y;
            if pos_x < 1105.0 && pos_x > 5.0 {
                info!("{}", pos_x);
            }
        }
    }

    if keys.pressed(KeyCode::KeyW) {
        camera_state.tilt = camera_state.tilt + speed * time.delta_secs_f64();
        if camera_state.tilt > 89.9f64.to_radians() {
            camera_state.tilt = 89.9f64.to_radians();
        }
    }
    if keys.pressed(KeyCode::KeyS) {
        camera_state.tilt = camera_state.tilt - speed * time.delta_secs_f64();
        if camera_state.tilt < -89.9f64.to_radians() {
            camera_state.tilt = -89.9f64.to_radians();
        }
    }
    if keys.pressed(KeyCode::KeyA) {
        camera_state.pan = camera_state.pan - speed * time.delta_secs_f64();
    }
    if keys.pressed(KeyCode::KeyD) {
        camera_state.pan = camera_state.pan + speed * time.delta_secs_f64();
    }

    for ev in evr_scroll.read() {
        match ev.unit {
            MouseScrollUnit::Line => {
                camera_state.dist = camera_state.dist + (camera_state.dist * 0.06 * ev.y as f64);
            }
            _ => {}
        }
    }

    // Ensure that the camera doesn't end up at absurd distances (too close to the focus, or too far)
    if camera_state.dist < state_keeper.info.get(&camera_state.focused).unwrap().radius * 1.8 {
        camera_state.dist = state_keeper.info.get(&camera_state.focused).unwrap().radius * 1.8
    } else if camera_state.dist > state_keeper.info.get(&camera_state.focused).unwrap().radius.powf(1.5) {
        camera_state.dist = state_keeper.info.get(&camera_state.focused).unwrap().radius.powf(1.5)
    }

    // Camera Pan/Tilt
    let provided_up: Vec3 = state_keeper.info.get(&camera_state.focused).unwrap().tilt.as_vec3();
    let reference: Vec3 = if provided_up.abs_diff_eq(Vec3::X, 1e-6) {
        Vec3::Y
    } else {
        Vec3::X
    };

    let horizontal = (reference - provided_up * reference.dot(provided_up)).normalize();
    let perpendicular = provided_up.cross(horizontal);

    let forward = (horizontal * (camera_state.tilt.cos() * camera_state.pan.cos()) as f32)
        + (perpendicular * (camera_state.tilt.cos() * camera_state.pan.sin()) as f32)
        + (provided_up * camera_state.tilt.sin() as f32);
    let forward = forward.normalize();

    let offset = forward * camera_state.dist as f32;
    let camera_x = offset.x;
    let camera_y = offset.y;
    let camera_z = offset.z;

    let (new_camera_grid_gridcell, new_camera_grid_transform) = root_grid.translation_to_grid(
        DVec3::new((camera_x + abs_offset.x as f32) as f64, (camera_y + abs_offset.y as f32) as f64, (camera_z + abs_offset.z as f32) as f64)
    );

    let (mut camera_grid_gridcell, mut camera_grid_transform) = camera_grid.into_inner();
    camera_grid_gridcell.x = new_camera_grid_gridcell.x;
    camera_grid_gridcell.y = new_camera_grid_gridcell.y;
    camera_grid_gridcell.z = new_camera_grid_gridcell.z;
    camera_grid_transform.translation = new_camera_grid_transform;

    let right = provided_up.cross(forward).normalize();
    let up = forward.cross(right);
    let rotation_matrix = Mat3::from_cols(right, up, forward);
    camera_grid_transform.rotation = Quat::from_mat3(&rotation_matrix);
    // End Camera Pan/Tilt

    // // Update planet labels to match the position of the planets
    for (_body_display, body_object_id, body_global_transform) in query2.iter() {
        let (_body_overlay_display, mut node, mut _text) = query1.get_mut(state_keeper.info.get(&body_object_id.id).unwrap().body_overlay_display_id.unwrap()).unwrap();
        let world_position = body_global_transform.translation();
        if body_object_id.id == state_keeper.inertial || state_keeper.info.get(&body_object_id.id).unwrap().kepler_parent == state_keeper.inertial {
            if let Ok(viewport_pos) = camera.world_to_viewport(camera_global_transform, world_position) {
                node.display = Display::DEFAULT;
                node.top = Val::Px(viewport_pos.y);
                node.left = Val::Px(viewport_pos.x);
            } else {
                node.display = Display::None;
            }
        } else {
            node.display = Display::None;
        }
    }

    // Display OE
    for (node, mut text, text_overlay) in query3.iter_mut() {
        if text_overlay.id == 0 {
            let t0 = DateTime::parse_from_str("2025 Apr 01 12:00:00 +0000", "%Y %b %d %H:%M:%S %z").unwrap();
            text.0 = (t0 + Duration::seconds((state_keeper.current_step as f64 * state_keeper.dt) as i64)).to_string();
        } else if text_overlay.id == 1 {
            let parent_id = state_keeper.info.get(&camera_state.focused).unwrap().kepler_parent;
            let parent_state = state_keeper.state.get(&state_keeper.current_step).unwrap().get(&parent_id).unwrap();
            let state = state_keeper.state.get(&state_keeper.current_step).unwrap().get(&camera_state.focused).unwrap();
            let current_oe = oe_from_rv(state_keeper.info.get(&parent_id).unwrap().mu, &sub_body_state(state, parent_state));
            text.0 = current_oe.to_string();
        } else if text_overlay.id == 2 {
            if let Some(f) = &state_keeper.interplanetary {
                text.0 = format!("dv1: {}, dv2: {}", f.dv1 as i32, f.dv2 as i32);
            }
        }
    }
}
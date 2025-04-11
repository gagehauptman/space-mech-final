mod nbody;
mod bodies_init;
mod camera;
mod keplerian;
mod ui;

use std::time::Instant;

use std::collections::HashMap;
use std::f64::consts::PI;
use std::os::linux::raw::stat;
use bevy::core_pipeline::bloom::Bloom;
use bevy::core_pipeline::Skybox;
use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::image::ImageSampler;
use bevy::pbr::NotShadowCaster;
use bevy::prelude::*;
use bevy::render::camera::Exposure;
use bevy::render::mesh::PrimitiveTopology;
use bevy::render::render_resource::{AddressMode, Extent3d, FilterMode, SamplerDescriptor, TextureDimension, TextureFormat};
use bevy_math::DVec3;
use big_space::prelude::*;
use crate::camera::CameraState;
use crate::keplerian::*;
use crate::ui::*;

type BodyState = [DVec3;2]; // r, v
fn add_body_state(a: &BodyState, b: &BodyState) -> BodyState {
    [a[0] + b[0], a[1] + b[1]]
}

fn sub_body_state(a: &BodyState, b: &BodyState) -> BodyState {
    [a[0] - b[0], a[1] - b[1]]
}
type BodyStates = HashMap<u32, BodyState>; // BodyID -> BodyStates
type TimeStates = HashMap<u32, BodyStates>; // Time -> BodyIDs
type BodyInfos = HashMap<u32, BodyInfo>;


struct BodyInfo {
    name: String,
    mu: f64,
    radius: f64,
    j2: f64,
    rotational_rate: f64,
    tilt: DVec3,
    affected: bool,
    affects: bool,
    kepler_parent: u32,
    display_as_keplerian: bool,
    orbit_display_id: Option<Entity>,
    body_display_id: Option<Entity>,
    body_display_grid_id: Option<Entity>,
    body_overlay_display_id: Option<Entity>,
    texture: String,
}

#[derive(Resource)]
struct StateKeeper {
    paused: bool,
    current_step: u32,
    time: f64,
    dt: f64,
    step_limit: u32,
    last_step_computed: u32,
    state: TimeStates,
    info: BodyInfos,
    inertial: u32,
    hypothetical: Option<Entity>,
    interplanetary: Interplanetary,
}

#[derive(Component)]
struct RootGrid {}

#[derive(Component)]
struct TextOverlay {
    id: u32,
}

#[derive(Component)]
struct ObjectID {
    id: u32,
}

#[derive(Component)]
struct OrbitDisplay {}

#[derive(Component)]
struct Hypothetical {}

#[derive(Component)]
struct BodyDisplay {}

#[derive(Component)]
struct BodyOverlayDisplay {
    selected: bool,
    focused: bool,
}

#[derive(Component)]
struct BodyDisplayGrid {}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            BigSpacePlugin::<i64>::default(),
            FrameTimeDiagnosticsPlugin::default()
            // big_space::camera::CameraControllerPlugin::<i64>::default(),
        ))
        .add_systems(Startup, setup)
        .add_systems(Startup, populate_state.after(setup))
        .add_systems(Startup, ui::setup_ui.after(populate_state))
        .add_systems(Update, display_state.after(main_tick))
        .add_systems(Update, camera::camera_controller.after(display_state))
        .add_systems(Update, main_tick.after(populate_state))
        .add_systems(Update, populate_state)
        .add_systems(Update, ui::button_interaction)
        .run();
}


// Spawn a StateKeeper, add in every planet/moon with their initial states for T0 (April 1, 2025) in HCI
fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>, asset_server: Res<AssetServer>, mut images: ResMut<Assets<Image>>) {
    let mut body_states: BodyStates = HashMap::new();
    let mut body_infos: BodyInfos = HashMap::new();
    let mut time_states: TimeStates = HashMap::new();


    let mut id_count = 0;
    commands.spawn((
        Text::new("AE313 Space Mechanics Final Project\nWASD to pan/tilt"),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        })
    );

    commands.spawn((
        Text::new("Time: T0"),
        TextOverlay { id: 0 },
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            right: Val::Px(12.0),
            ..default()
        }),
    );
    commands.spawn((
        Text::new("Current OE"),
        TextOverlay { id: 1 },
        Node {
           position_type: PositionType::Absolute,
           bottom: Val::Px(12.0),
           right: Val::Px(12.0),
           ..default()
        }),
    );

    let mut hypothetical_display: Option<Entity> = None;

    commands.spawn_big_space_default(|root_grid: &mut GridCommands<i64>| {
        root_grid.insert(RootGrid {});
        root_grid.with_grid_default(|camera_grid: &mut GridCommands<i64> | {
            camera_grid.insert((
                FloatingOrigin,
                Transform::from_translation(Vec3::new(1.5e11,0.0,0.0)).looking_at(Vec3::ZERO, Vec3::Z),
                ));

            camera_grid.spawn_spatial((
                Camera3d::default(),
                Transform::from_xyz(0.0,0.0,0.0),
                Camera {
                    hdr: true,
                    ..default()
                },
                CameraState {
                    pan: 0.0,
                    tilt: 0.0,
                    dist: 1.5e9,
                    focused: 0,
                },
                Exposure::SUNLIGHT,
                Bloom::NATURAL,
                Skybox {
                    image: asset_server.load("textures/hdr-cubemap-4096x4096.ktx2"), // Pretty skybox! What's the point of anything if you don't have ✨aesthetic✨. FPS is lame anyways.
                    brightness: 75000.,
                    ..default()
                }));
        });

        // Spawn the 'display' orbit for showing transfers, etc etc. only need one i think ??
        let mut mesh = Mesh::new(PrimitiveTopology::LineStrip, Default::default());
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<Vec3>::new());
        hypothetical_display = Some(root_grid.spawn_spatial((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::WHITE,
                emissive: LinearRgba::new(0., 0.5, 0., 0.5),
                unlit: true,
                ..default()
            })),
            OrbitDisplay {},
            Hypothetical {},
        )).id());

        // Spawn orbit displays, grids, and spheres for each body to display (planets, moons, etc)
        for body in bodies_init::planets_info() {
            body_states.insert(id_count, body.0);
            body_infos.insert(id_count, body.1);

            let mut mesh = Mesh::new(PrimitiveTopology::LineStrip, Default::default());
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<Vec3>::new());
            let orbit_display_id = root_grid.spawn_spatial((
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    emissive: LinearRgba::new(0., 0.5, 0., 0.5),
                    unlit: true,
                    ..default()
                })),
                OrbitDisplay {},
                ObjectID { id: id_count }
            )).id();
            body_infos.get_mut(&id_count).unwrap().orbit_display_id = Some(orbit_display_id);

            root_grid.with_grid_default(|this_grid| {
                this_grid.insert((BodyDisplayGrid{}, ObjectID{id: id_count}) );
                body_infos.get_mut(&id_count).unwrap().body_display_grid_id = Some(this_grid.id());

                let mut material = materials.add(StandardMaterial {
                    base_color: Color::linear_rgba(0.5,0.5,0.5,1.0),
                    ..default()
                });
                if body_infos.get(&id_count).unwrap().name == "Sun" {
                    material = materials.add(StandardMaterial {
                        base_color: Color::WHITE,
                        emissive: LinearRgba::rgb(120.,100.,100.),
                        ..default()
                    });
                } else {
                    let texture_handle = asset_server.load(format!("textures/{}", body_infos.get(&id_count).unwrap().texture));

                    material = materials.add(StandardMaterial {
                        base_color: Color::srgba(1.0, 1.0, 1.0, 1.0),
                        base_color_texture: Some(texture_handle.clone()),
                        alpha_mode: AlphaMode::Blend,
                        unlit: true,
                        ..default()
                    });
                }

                let mut transform = Transform::from_xyz(0.0,0.0,0.0);
                transform.rotation = Quat::from_rotation_arc(Vec3::Z, body_infos.get(&id_count).unwrap().tilt.as_vec3());

                let body_display_id = this_grid.spawn_spatial((
                    Mesh3d(meshes.add(Sphere::new(body_infos.get(&id_count).unwrap().radius as f32 ).mesh().uv(32,18))),
                    MeshMaterial3d(material),
                    transform,
                    NotShadowCaster,
                    BodyDisplay {},
                    ObjectID { id: id_count },
                )).id();

                body_infos.get_mut(&id_count).unwrap().body_display_id = Some(body_display_id);
            });
            id_count += 1;
        }
    });

    for (_body_id, body_info) in body_infos.iter_mut() {
        let body_overlay_display_id = commands.spawn((
            Node {
                position_type: PositionType::Absolute,
                ..default()
            },
            Text(body_info.name.clone()),
            BodyOverlayDisplay {selected: false, focused: false},
            ObjectID { id: id_count },
        )).id();

        body_info.body_overlay_display_id = Some(body_overlay_display_id);
    }

    time_states.insert(0, body_states);

    commands.insert_resource(StateKeeper {paused: true, current_step: 0, time: 0.0, dt: 100.0, step_limit: 20000, last_step_computed: 0, state: time_states, info: body_infos, inertial: 0, hypothetical: hypothetical_display, interplanetary: Interplanetary {body0: 0, body1: 3, body2: 4, oe0: OE::empty(), oe1: OE::empty(), oe2: OE::empty() } });
}

// Populate all empty states from t(1) to t(end) where end is how many steps ahead of t0 to predict
fn populate_state(mut state_keeper: ResMut<StateKeeper>) {
    let start = Instant::now();
    let mut time_limit = true;
    if state_keeper.last_step_computed == 0 {
        time_limit = false;
    }
    let mut steps = 0;
    for i in state_keeper.last_step_computed+1..state_keeper.step_limit {
        let last_state = state_keeper.state.get(&(i - 1)).unwrap();
        let new_state = nbody::rk4_step(&state_keeper.info, &last_state, state_keeper.dt);
        state_keeper.state.insert(i, new_state);
        state_keeper.last_step_computed=i;
        steps += 1;

        if start.elapsed().as_millis() > 16 && time_limit {
            break
        }
    }
}

fn display_state(
    mut meshes: ResMut<Assets<Mesh>>,
    mut state_keeper: ResMut<StateKeeper>,
    mut orbit_display_query: Query<(&mut Mesh3d, &mut GridCell<i64>, &mut Transform), (With<OrbitDisplay>, Without<BodyDisplayGrid>, Without<Hypothetical>)>,
    mut body_display_query: Query<(&mut GridCell<i64>, &mut Transform), (With<BodyDisplayGrid>, Without<OrbitDisplay>)>,
    mut root_grid: Single<&mut Grid<i64>, With<RootGrid>>,
    mut hypothetical_query: Single<(&mut Mesh3d, &mut GridCell<i64>, &mut Transform), (With<OrbitDisplay>, With<Hypothetical>)>,
    mut porkchop_image: Single<(&mut PorkchopImage)>,
    camera: Single<(&Camera, &GlobalTransform, &mut CameraState)>,
    keys: Res<ButtonInput<KeyCode>>,
    mut images: ResMut<Assets<Image>>,
) {
    let (camera, camera_global_transform, mut camera_state) = camera.into_inner();
    let start = Instant::now();
    // Collect the keys to avoid holding an immutable borrow of state_keeper.state
    let body_ids: Vec<_> = state_keeper.state.get(&state_keeper.current_step)
        .unwrap()
        .keys()
        .cloned()
        .collect();

    let (porkchop_image_ref) = &porkchop_image.into_inner().handle;

    if keys.just_pressed(KeyCode::KeyP) {
        if let Some(porkchop_image) = images.get_mut(porkchop_image_ref) {
            let delta_step = 864;
            for i in 0..porkchop_image.width() {
                let departure_step = state_keeper.current_step + delta_step * i;
                for j in 0..porkchop_image.height() {
                    let arrival_step = state_keeper.current_step + (delta_step * (porkchop_image.height() - j));
                    if departure_step < state_keeper.last_step_computed && arrival_step < state_keeper.last_step_computed && arrival_step > departure_step {
                        let d_step = (arrival_step - departure_step);
                        let dt = d_step as f64 * state_keeper.dt;
                        let mu = state_keeper.info.get(&0).unwrap().mu;
                        let r1 = state_keeper.state.get(&departure_step).unwrap().get(&3).unwrap()[0];
                        let v1 = state_keeper.state.get(&departure_step).unwrap().get(&3).unwrap()[1];
                        let r2 = state_keeper.state.get(&arrival_step).unwrap().get(&4).unwrap()[0];
                        let v2 = state_keeper.state.get(&arrival_step).unwrap().get(&4).unwrap()[1];
                        if let Ok((v1_new_1, v2_new_1)) = lambert_bate::get_velocities(r1.to_array(), r2.to_array(), dt, mu, true, 1e-7, 100) {
                            if let Ok((v1_new_2, v2_new_2)) = lambert_bate::get_velocities(r1.to_array(), r2.to_array(), dt, mu, false, 1e-7, 100) {
                                let delta_v_1 = (DVec3::from_array(v1_new_1) - v1).length().abs() + (DVec3::from_array(v2_new_1) - v2).length().abs();
                                let delta_v_2 = (DVec3::from_array(v1_new_2) - v1).length().abs() + (DVec3::from_array(v2_new_2) - v2).length().abs();
                                let lowest = delta_v_1.min(delta_v_2);
                                let width = porkchop_image.width() as usize;
                                let idx = ((j as usize) * width + (i as usize)) * 4;

                                let lower = 7000.0;
                                let upper = 20000.0;
                                let ratio = ((lowest - lower).min(upper).max(lower) - lower) / lower;
                                if lowest < upper {
                                    porkchop_image.data[idx] = (255.0 * ratio) as u8;       // Red
                                    porkchop_image.data[idx + 1] = 0;   // Green
                                    porkchop_image.data[idx + 2] = (255.0 * (1.0 - ratio)) as u8;   // Blue
                                    porkchop_image.data[idx + 3] = 255;          // Alpha (fully opaque)
                                } else {
                                    porkchop_image.data[idx] = 255;       // Red
                                    porkchop_image.data[idx + 1] = 255;   // Green
                                    porkchop_image.data[idx + 2] = 255;   // Blue
                                    porkchop_image.data[idx + 3] = 255;          // Alpha (fully opaque)
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // // Calculate, display the hypothetical orbit
    // let (mut hypothetical_mesh3d, mut hypothetical_gridcell, mut hypothetical_transform) = hypothetical_query.into_inner();
    // let d_step = 207360;
    // if true && state_keeper.last_step_computed > state_keeper.current_step + d_step  { // If displaying an interplanetary trajectory
    //     let dt = d_step as f64 * state_keeper.dt;
    //     state_keeper.interplanetary = solve_interplanetary(0, 3, 4, &state_keeper.state.get(&state_keeper.current_step).unwrap().get(&3).unwrap()[0], &state_keeper.state.get(&(state_keeper.current_step+d_step)).unwrap().get(&4).unwrap()[0], state_keeper.info.get(&0).unwrap().mu, dt);
    //     let mut oe = &state_keeper.interplanetary.oe0;
    //
    //     if state_keeper.inertial == state_keeper.interplanetary.body0 {
    //         oe = &state_keeper.interplanetary.oe0;
    //     } else if state_keeper.inertial == state_keeper.interplanetary.body1 {
    //         oe = &state_keeper.interplanetary.oe1;
    //     } else if state_keeper.inertial == state_keeper.interplanetary.body2 {
    //         oe = &state_keeper.interplanetary.oe2;
    //     }
    //
    //     let mut mesh = Mesh::new(PrimitiveTopology::LineStrip, Default::default());
    //     let p0 = position_from_true_anomoly(&oe, 0.0);
    //     let sub = p0;
    //     let positions: Vec<Vec3> = oe_to_vec(&oe, &sub);
    //
    //     let (new_grid_cell, new_translation) = root_grid.translation_to_grid(sub);
    //     *hypothetical_gridcell = new_grid_cell;
    //     hypothetical_transform.translation = new_translation;
    //
    //     mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    //     hypothetical_mesh3d.0 = meshes.add(mesh);
    // }

    // Display the body and orbit of each body
    for id in body_ids {
        let body_display_grid_id = state_keeper.info.get(&id).unwrap().body_display_grid_id;
        // let body_display_id = state_keeper.info.get(&id).unwrap().body_display_id;
        let orbit_display_id = state_keeper.info.get(&id).unwrap().orbit_display_id;
        if let Some(body_display_grid_id) = body_display_grid_id {
            let (mut body_display_grid_gridcell, mut body_display_grid_transform) = body_display_query.get_mut(body_display_grid_id).unwrap();
            let pos = state_keeper.state.get(&state_keeper.current_step).unwrap().get(&id).unwrap()[0];
            let (new_grid_cell, new_translation) = root_grid.translation_to_grid(pos);
            *body_display_grid_gridcell = new_grid_cell;
            body_display_grid_transform.translation = new_translation;
        }

        if let Some(orbit_display_id) = orbit_display_id {
            let parent_id = state_keeper.info.get(&id).unwrap().kepler_parent;
            let parent_state = state_keeper.state.get(&state_keeper.current_step).unwrap().get(&parent_id).unwrap();
            let state = state_keeper.state.get(&state_keeper.current_step).unwrap().get(&id).unwrap();
            // Adjust the origin of the mesh so that it is at the first point in the mesh
            // TODO: Maybe position origin in the middle (average position of vec) of mesh? idk
            let (mut orbit_display_mesh3d, mut orbit_display_gridcell, mut orbit_display_transform) = orbit_display_query.get_mut(orbit_display_id).unwrap();

            if state_keeper.info.get(&id).unwrap().display_as_keplerian { // If we should display the orbit as keplerian, we calculate one full orbit (360deg), and then adjust each position for the origin of the mesh, parent body, and the inertial reference frame
                if (state_keeper.info.get(&id).unwrap().kepler_parent == state_keeper.inertial) || (state_keeper.info.get(&id).unwrap().kepler_parent == 0) || (camera_state.focused == id) { // Only display keplerian orbits if the parent body is the inertial reference
                    let oe = oe_from_rv(state_keeper.info.get(&parent_id).unwrap().mu, &sub_body_state(state, parent_state));

                    let p0 = position_from_true_anomoly(&oe, 0.0);
                    let sub = parent_state[0] + p0;

                    let (new_grid_cell, new_translation) = root_grid.translation_to_grid(sub);
                    *orbit_display_gridcell = new_grid_cell;
                    orbit_display_transform.translation = new_translation;

                    let mut mesh = Mesh::new(PrimitiveTopology::LineStrip, Default::default());
                    let positions: Vec<Vec3> = oe_to_vec(&oe, &sub);
                    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
                    orbit_display_mesh3d.0 = meshes.add(mesh);
                } else {
                    let mut mesh = Mesh::new(PrimitiveTopology::LineStrip, Default::default());
                    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<Vec3>::new());
                    orbit_display_mesh3d.0 = meshes.add(mesh);
                }
            } else {

            }
        }
    }

    let duration = start.elapsed();
    // info!("Display Oribs {:?}", duration);
}

fn main_tick(mut state_keeper: ResMut<StateKeeper>, time: Res<Time>, keys: Res<ButtonInput<KeyCode>>,) {
    state_keeper.step_limit = 3153600;
    if keys.just_pressed(KeyCode::Space) {
        state_keeper.paused = !state_keeper.paused;
    }

    let speed = 500;

    if keys.pressed(KeyCode::Equal) {
        let mut new: i32 = state_keeper.current_step as i32 + speed;
        if new > state_keeper.last_step_computed as i32 - 1001 {
            new = state_keeper.last_step_computed as i32 - 1001;
            if new < 1 {
                new = 0
            }
        }
        state_keeper.current_step = new as u32;
    }
    if keys.pressed(KeyCode::Minus) {
        let mut new = state_keeper.current_step as i32 - speed;
        if new < 1 {
            new = 0
        }
        state_keeper.current_step = new as u32;
    }

    if !state_keeper.paused {
        state_keeper.current_step = state_keeper.last_step_computed - 1000;
    }
}
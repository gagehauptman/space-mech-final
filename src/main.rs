mod nbody;
mod bodies_init;
mod camera;
mod keplerian;
mod ui;
mod interplanetary;
mod porkchop;

use std::time::Instant;

use std::collections::HashMap;
use std::f64::consts::PI;
use std::f64::INFINITY;
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
use crate::interplanetary::*;
use crate::porkchop::*;

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
    interplanetary: Option<Interplanetary>,
    interplanetary_selection: (u32,u32,u32,u32,bool),
    interplanetaries: HashMap<u32, Vec<(u32, Interplanetary)>>
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
        .add_systems(Update, main_tick)
        .add_systems(Update, button_interaction)
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
    commands.spawn((
           Text::new("IP"),
           TextOverlay { id: 2 },
           Node {
               position_type: PositionType::Absolute,
               bottom: Val::Px(12.0),
               left: Val::Px(12.0),
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
                    image: asset_server.load("textures/hdr-cubemap-1024x1024.ktx2"), // Pretty skybox! What's the point of anything if you don't have ✨aesthetic✨. FPS is lame anyways.
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

    commands.insert_resource(StateKeeper {paused: true, current_step: 0, time: 0.0, dt: 100.0, step_limit: 864*365*4, last_step_computed: 0, state: time_states, info: body_infos, inertial: 0, hypothetical: hypothetical_display, interplanetary: None, interplanetary_selection: (3,4,0,864*120,true), interplanetaries: HashMap::new() });
}

fn populate_state(mut state_keeper: ResMut<StateKeeper>) {
    // Populate the state
    for i in 1..state_keeper.step_limit {
        let last_state = state_keeper.state.get(&(i - 1)).unwrap();
        let new_state = nbody::rk4_step(&state_keeper.info, &last_state, state_keeper.dt);
        state_keeper.state.insert(i, new_state);
        state_keeper.last_step_computed=i;
    }

    // Porkchop plot shenanigans
    let mut dv_grid: Vec<Vec<f32>> = Vec::with_capacity(365 * 2);
    let step_day = 864;
    let max_travel = 12 * 30 * step_day;
    let depart_end = (state_keeper.step_limit - max_travel)
        .min(2 * 365 * step_day);

    let mut lowest_dv = f64::INFINITY;
    let mut lowest_dv_pos: [u32;2] = [0,0];
    let mut lowest_dv_short: bool = true;
    for travel in (30*3*step_day .. 30*12*step_day).step_by(step_day as usize) {
        let mut row = Vec::with_capacity(depart_end as usize / step_day as usize);
        for depart in (0..depart_end).step_by(step_day as usize) {
            let ip1 = interplanetary(&state_keeper, depart, depart + travel, 3, 4, true);
            let ip2 = interplanetary(&state_keeper, depart, depart + travel, 3, 4, false);
            let mut is_ip1_lowest = true;
            let mut dv = f64::INFINITY;
            if ip1.dv1 + ip1.dv2 < ip2.dv1 + ip2.dv2 {
                dv = ip1.dv1 + ip1.dv2;
            } else {
                dv = ip2.dv1 + ip2.dv2;
                is_ip1_lowest = false;
            }
            row.push(dv as f32);

            if dv < lowest_dv {
                lowest_dv = dv;
                lowest_dv_pos = [depart,travel];
                lowest_dv_short = is_ip1_lowest;
            }
        }
        dv_grid.push(row);
    }

    state_keeper.interplanetary_selection.2 = lowest_dv_pos[0];
    state_keeper.interplanetary_selection.3 = lowest_dv_pos[0]+lowest_dv_pos[1];
    state_keeper.interplanetary_selection.4 = lowest_dv_short;
    state_keeper.interplanetary = Some(interplanetary(&state_keeper, state_keeper.interplanetary_selection.2, state_keeper.interplanetary_selection.3, state_keeper.interplanetary_selection.0, state_keeper.interplanetary_selection.1, state_keeper.interplanetary_selection.4));

    let height = dv_grid.len() as u32;
    let width  = dv_grid.get(0).map_or(0, |r| r.len()) as u32;
    make_porkchop_plot(&dv_grid, width, height).unwrap();
}

fn display_state(
    mut meshes: ResMut<Assets<Mesh>>,
    mut state_keeper: ResMut<StateKeeper>,
    mut orbit_display_query: Query<(&mut Mesh3d, &mut GridCell<i64>, &mut Transform), (With<OrbitDisplay>, Without<BodyDisplayGrid>, Without<Hypothetical>)>,
    mut body_display_query: Query<(&mut GridCell<i64>, &mut Transform), (With<BodyDisplayGrid>, Without<OrbitDisplay>)>,
    mut root_grid: Single<&mut Grid<i64>, With<RootGrid>>,
    mut hypothetical_query: Single<(&mut Mesh3d, &mut GridCell<i64>, &mut Transform), (With<OrbitDisplay>, With<Hypothetical>)>,
    camera: Single<(&Camera, &GlobalTransform, &mut CameraState)>,
) {
    let (camera, camera_global_transform, mut camera_state) = camera.into_inner();
    let start = Instant::now();
    // Collect the keys to avoid holding an immutable borrow of state_keeper.state
    let body_ids: Vec<_> = state_keeper.state.get(&state_keeper.current_step)
        .unwrap()
        .keys()
        .cloned()
        .collect();

    // Display the interplanetary "hypothetical" orbit, if it exists
    let (mut hypothetical_mesh3d, mut hypothetical_gridcell, mut hypothetical_transform) = hypothetical_query.into_inner();
    if let Some(f) = &state_keeper.interplanetary {
        let mut oe = &f.oe0;
        let mut show = false;
        let mut id = 0;
        if state_keeper.inertial == f.body0 {
            oe = &f.oe0;
            show = true;
            id = f.body0;
        } else if state_keeper.inertial == f.body1 {
            oe = &f.oe1;
            show = true;
            id = f.body1;
        } else if state_keeper.inertial == f.body2 {
            oe = &f.oe2;
            show = true;
            id = f.body2;
        }

        if show {
            let mut mesh = Mesh::new(PrimitiveTopology::LineStrip, Default::default());
            let mut positions: Vec<Vec3> = oe_to_vec(&oe);
            let p0= positions[0];

            let (new_grid_cell, new_translation) = root_grid.translation_to_grid(p0.as_dvec3() + state_keeper.state.get(&state_keeper.current_step).unwrap().get(&id).unwrap()[0]);
            *hypothetical_gridcell = new_grid_cell;
            hypothetical_transform.translation = new_translation;

            for i in 0..positions.len() {
                positions[i] = positions[i] - p0;
            }

            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
            hypothetical_mesh3d.0 = meshes.add(mesh);
        } else {
            let mut mesh = Mesh::new(PrimitiveTopology::LineStrip, Default::default());
            let positions: Vec<Vec3> = Vec::new();

            let (new_grid_cell, new_translation) = root_grid.translation_to_grid(DVec3::ZERO);
            *hypothetical_gridcell = new_grid_cell;
            hypothetical_transform.translation = new_translation;

            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
            hypothetical_mesh3d.0 = meshes.add(mesh);
        }
    } else {
        let mut mesh = Mesh::new(PrimitiveTopology::LineStrip, Default::default());
        let positions: Vec<Vec3> = Vec::new();

        let (new_grid_cell, new_translation) = root_grid.translation_to_grid(DVec3::ZERO);
        *hypothetical_gridcell = new_grid_cell;
        hypothetical_transform.translation = new_translation;

        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        hypothetical_mesh3d.0 = meshes.add(mesh);
    }

    // Display the body and orbit of each body at the current step
    for id in body_ids {
        let body_display_grid_id = state_keeper.info.get(&id).unwrap().body_display_grid_id;
        let orbit_display_id = state_keeper.info.get(&id).unwrap().orbit_display_id;

        // Update the grid the body mesh is in to the correct position
        if let Some(body_display_grid_id) = body_display_grid_id {
            let (mut body_display_grid_gridcell, mut body_display_grid_transform) = body_display_query.get_mut(body_display_grid_id).unwrap();
            let pos = state_keeper.state.get(&state_keeper.current_step).unwrap().get(&id).unwrap()[0];
            let (new_grid_cell, new_translation) = root_grid.translation_to_grid(pos);
            *body_display_grid_gridcell = new_grid_cell;
            body_display_grid_transform.translation = new_translation;
        }

        // Update the grid the orbit mesh is in to the correct position, and then update the mesh vertices accordingly
        if let Some(orbit_display_id) = orbit_display_id {
            let parent_id = state_keeper.info.get(&id).unwrap().kepler_parent;
            let parent_state = state_keeper.state.get(&state_keeper.current_step).unwrap().get(&parent_id).unwrap();
            let state = state_keeper.state.get(&state_keeper.current_step).unwrap().get(&id).unwrap();
            let (mut orbit_display_mesh3d, mut orbit_display_gridcell, mut orbit_display_transform) = orbit_display_query.get_mut(orbit_display_id).unwrap();

            if state_keeper.info.get(&id).unwrap().display_as_keplerian { // If we should display the orbit as keplerian, we calculate one full orbit (360deg), and then adjust each position for the origin of the mesh, parent body, and the inertial reference frame
                if id == state_keeper.inertial || parent_id == state_keeper.inertial {
                    let oe = oe_from_rv(state_keeper.info.get(&parent_id).unwrap().mu, &sub_body_state(state, parent_state));
                    let positions: Vec<Vec3> = oe_to_vec(&oe);
                    let p0= positions[0];
                    let (new_grid_cell, new_translation) = root_grid.translation_to_grid(p0.as_dvec3() + parent_state[0]);
                    *orbit_display_gridcell = new_grid_cell;
                    orbit_display_transform.translation = new_translation;

                    let mut mesh = Mesh::new(PrimitiveTopology::LineStrip, Default::default());
                    let mut positions: Vec<Vec3> = positions;
                    for i in 0..positions.len() {
                        positions[i] = positions[i] - p0;
                    }
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
    // info!("Display Orbits {:?}", duration);
}

fn main_tick(mut state_keeper: ResMut<StateKeeper>) {

}
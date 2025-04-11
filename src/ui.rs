use bevy::prelude::*;
use crate::*;
use crate::camera::CameraState;

#[derive(Component)]
pub struct UICamera {}

#[derive(Component)]
pub struct BodySelectButton { id: u32 }

#[derive(Component)]
pub struct PorkchopPlot {}

#[derive(Component)]
pub struct PorkchopImage {
    pub handle: Handle<Image>
}

pub fn setup_ui(mut commands: Commands, state_keeper: Res<StateKeeper>, mut images: ResMut<Assets<Image>>) {
    let menu_root = commands.spawn((
        Node {
            bottom: Val::Px(20.0),
            position_type: PositionType::Absolute,
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(5.0)),
            ..default()
        }
        )).id();

    for (id, body_info) in state_keeper.info.iter() {
        if id.clone() == 0 || id.clone() == 3 || id.clone() == 4 || id.clone() == 9 {
            let entity = commands.spawn((
                Button{},
                Text::new(body_info.name.clone()),
                BodySelectButton { id: id.clone() }
            )).id();

            commands.entity(menu_root).add_child(entity);
        }
    }

    let porkchop_root = commands.spawn((
        Node {
            top: Val::Px(50.0),
            right: Val::Px(5.0),
            position_type: PositionType::Absolute,
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(5.0)),
            width: Val::Px(500.0),
            ..default()
        },
        )).id();

    let image_handle = images.add(Image::new_fill(
        Extent3d { width: 2500, height: 2500, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[255u8, 255u8, 255u8, 255u8],
        TextureFormat::Rgba8UnormSrgb,
        Default::default()
    ));


    let entity = commands.spawn((
        ImageNode::new(image_handle.clone()),
        PorkchopImage{handle: image_handle.clone()}
        )).id();

    commands.entity(porkchop_root).add_child(entity);
}

pub fn button_interaction(mut state_keeper: ResMut<StateKeeper>, mut button_query: Query<(&Interaction, &BodySelectButton)>, mut camera: Single<(&Camera, &GlobalTransform, &mut CameraState)>) {
    let (_camera, _camera_global_transform, mut camera_state) = camera.into_inner();
    for (interaction, button) in button_query.iter_mut() {
        if *interaction == Interaction::Pressed {
            state_keeper.inertial = button.id;
            camera_state.focused = button.id;
            info!("Selected {}", button.id);
        }
    }
}
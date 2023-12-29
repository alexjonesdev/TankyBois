//Includes
use bevy::{prelude::*, render::camera::ScalingMode, sprite::MaterialMesh2dBundle, window::{WindowResolution, PrimaryWindow},};

//Constants
const BOUNDS: Vec2 = Vec2::new(1200.0, 640.0);

//Types

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                // fill the entire browser window
                fit_canvas_to_parent: true,
                resolution: WindowResolution::new(1920., 1080.),
                resizable: true,
                // don't hijack keyboard shortcuts like F5, F6, F12, Ctrl+R etc.
                //prevent_default_event_handling: false,
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::rgb(0.53, 0.53, 0.53)))
        .init_resource::<MyWorldCoords>()
        .add_systems(Startup, setup)
        .add_systems(Update, (my_cursor_system, player_movement_system))
        .add_systems(Update, bevy::window::close_on_esc)
        .run();
}

/// We will store the world position of the mouse cursor here.
#[derive(Resource, Default)]
struct MyWorldCoords(Vec2);

/// Used to help identify our main camera
#[derive(Component)]
struct MainCamera;

/// player component
#[derive(Component)]
struct Player {
    /// linear speed in meters per second
    movement_speed: f32,
    /// rotation speed in radians per second
    rotation_speed: f32,
}

/// player component
#[derive(Component)]
struct Turret {
    /// rotation speed in radians per second
    rotation_speed: f32,
}

#[derive(Component)]
struct Target;

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<ColorMaterial>>) {
    let mut camera_bundle = Camera2dBundle::default();
    camera_bundle.projection.scaling_mode = ScalingMode::FixedVertical(30.);
    commands.spawn((camera_bundle, MainCamera));

    // Rectangle
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.25, 0.25, 0.75),
                custom_size: Some(Vec2::new(2.0, 4.0)),
                ..default()
            },
        transform: Transform::from_translation(Vec3::new(0., 0., 1.)),
        ..default()
        },
        Player {
            movement_speed: 10.0,                  // meters per second
            rotation_speed: f32::to_radians(180.0), // degrees per second
        },
    ));

    // Triangle
    commands.spawn((
        MaterialMesh2dBundle {
        mesh: meshes.add(shape::RegularPolygon::new(1., 3).into()).into(),
        material: materials.add(ColorMaterial::from(Color::TURQUOISE)),
        transform: Transform::from_translation(Vec3::new(0., 0., 2.)),
        ..default()
        },
        Turret {
            rotation_speed: f32::to_radians(180.0), // degrees per second
        },
    ));

    // Circle
    commands.spawn((
        MaterialMesh2dBundle {
        mesh: meshes.add(shape::Circle::new(0.1).into()).into(),
        material: materials.add(ColorMaterial::from(Color::PURPLE)),
        transform: Transform::from_translation(Vec3::new(0., 0., 3.)),
        ..default()
    },
        Target,
    ));
}

/// Demonstrates applying rotation and movement based on keyboard input.
fn player_movement_system(
    time: Res<Time>,
    keys: Res<Input<KeyCode>>,
    mut player_query: Query<(&Player, &mut Transform), With<Player>>,
    mut target_query: Query<(&Target, &mut Transform), Without<Player>>,
    mut turret_query: Query<(&Turret, &mut Transform), (Without<Player>, Without<Target>)>,
    mouse_cords: Res<MyWorldCoords>,
) {
    let (ship, mut ship_transform) = player_query.single_mut();
    let (_target, mut tar_transform) = target_query.single_mut();
    let (turret, mut tur_transform) = turret_query.single_mut();
    
    let target_translation = tar_transform.translation.xy();

    let mut rotation_factor = 0.0;
    let mut movement_factor = 0.0;

    if keys.any_pressed([KeyCode::Left, KeyCode::A]) {
        rotation_factor += 1.0;
    }

    if keys.any_pressed([KeyCode::Right, KeyCode::D]) {
        rotation_factor -= 1.0;
    }

    if keys.any_pressed([KeyCode::Up, KeyCode::W]) {
        movement_factor += 1.0;
    }

    if keys.any_pressed([KeyCode::Down, KeyCode::S]) {
        movement_factor -= 1.0;
    }

    // update the ship rotation around the Z axis (perpendicular to the 2D plane of the screen)
    ship_transform.rotate_z(rotation_factor * ship.rotation_speed * time.delta_seconds());

    // get the ship's forward vector by applying the current rotation to the ships initial facing vector
    let movement_direction = ship_transform.rotation * Vec3::Y;
    // get the distance the ship will move based on direction, the ship's movement speed and delta time
    let movement_distance = movement_factor * ship.movement_speed * time.delta_seconds();
    // create the change in translation using the new movement direction and distance
    let translation_delta = movement_direction * movement_distance;
    // update the ship translation with our new translation delta
    ship_transform.translation += translation_delta;

    // bound the ship within the invisible level bounds
    let extents = Vec3::from((BOUNDS / 2.0, 0.0));
    ship_transform.translation = ship_transform.translation.min(extents).max(-extents);

    // Target Handling
    tar_transform.translation = Vec3::from((mouse_cords.0, 3.));

    // Turret Handling
    tur_transform.translation = ship_transform.translation;
    // get the enemy ship forward vector in 2D (already unit length)
    let turret_forward = (tur_transform.rotation * Vec3::Y).xy();

    // get the vector from the enemy ship to the player ship in 2D and normalize it.
    let to_target = (target_translation - tur_transform.translation.xy()).normalize();

    // get the dot product between the enemy forward vector and the direction to the player.
    let forward_dot_target = turret_forward.dot(to_target);

    // if the dot product is approximately 1.0 then the enemy is already facing the player and
    // we can early out.
    if !((forward_dot_target - 1.0).abs() < f32::EPSILON) {
        // get the right vector of the enemy ship in 2D (already unit length)
        let tur_right = (tur_transform.rotation * Vec3::X).xy();

        // get the dot product of the enemy right vector and the direction to the player ship.
        // if the dot product is negative them we need to rotate counter clockwise, if it is
        // positive we need to rotate clockwise. Note that `copysign` will still return 1.0 if the
        // dot product is 0.0 (because the player is directly behind the enemy, so perpendicular
        // with the right vector).
        let right_dot_target = tur_right.dot(to_target);

        // determine the sign of rotation from the right dot player. We need to negate the sign
        // here as the 2D bevy co-ordinate system rotates around +Z, which is pointing out of the
        // screen. Due to the right hand rule, positive rotation around +Z is counter clockwise and
        // negative is clockwise.
        let rotation_sign = -f32::copysign(1.0, right_dot_target);

        // limit rotation so we don't overshoot the target. We need to convert our dot product to
        // an angle here so we can get an angle of rotation to clamp against.
        let max_angle = forward_dot_target.clamp(-1.0, 1.0).acos(); // clamp acos for safety

        // calculate angle of rotation with limit
        let rotation_angle =
            rotation_sign * (turret.rotation_speed * time.delta_seconds()).min(max_angle);

        // rotate the enemy to face the player
        tur_transform.rotate_z(rotation_angle);
    }
}

// fn cursor_position(
//     q_windows: Query<&Window, With<PrimaryWindow>>,
//     mut target_query: Query<&mut Target>,
// ) {
//     // Games typically only have one window (the primary window)
//     let mut target = target_query.single_mut();
//     if let Some(position) = q_windows.single().cursor_position() {
//         println!("Cursor is inside the primary window, at {:?}", position);
//         target.location = position;
//     } else {
//         println!("Cursor is not in the game window.");
//         target.location = Vec2::new(0., 0.)
//     }
// }

// fn cursor_position_debug(
//     q_windows: Query<&Window, With<PrimaryWindow>>,
// ) {
//     // Games typically only have one window (the primary window)
//     if let Some(position) = q_windows.single().cursor_position() {
//         println!("Cursor is inside the primary window, at {:?}", position);
//     } else {
//         println!("Cursor is not in the game window.");
//     }
// }

fn my_cursor_system(
    mut mycoords: ResMut<MyWorldCoords>,
    // query to get the window (so we can read the current cursor position)
    q_window: Query<&Window, With<PrimaryWindow>>,
    // query to get camera transform
    q_camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
) {
    // get the camera info and transform
    // assuming there is exactly one main camera entity, so Query::single() is OK
    let (camera, camera_transform) = q_camera.single();

    // There is only one primary window, so we can similarly get it from the query:
    let window = q_window.single();

    // check if the cursor is inside the window and get its position
    // then, ask bevy to convert into world coordinates, and truncate to discard Z
    if let Some(world_position) = window.cursor_position()
        .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor))
        .map(|ray| ray.origin.truncate())
    {
        mycoords.0 = world_position;
        eprintln!("World coords: {}/{}", world_position.x, world_position.y);
    }
}
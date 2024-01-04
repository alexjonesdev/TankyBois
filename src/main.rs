// Includes
use bevy::{prelude::*,
    render::camera::ScalingMode,
    sprite::MaterialMesh2dBundle,
    window::{WindowResolution, PrimaryWindow},
    input::mouse::MouseWheel,
    utils::HashMap};
use bevy_ggrs::*;
use bevy_matchbox::prelude::*;

// Constants
const BOUNDS: Vec2 = Vec2::new(1200.0, 640.0);
const SCALE_STEP: f32 = 5.;
const MAX_SCALE: f32 = 100.;
const MAP_SIZE: u32 = 1000;
const GRID_WIDTH: f32 = 0.05;

const INPUT_FORWARD: u8 = 1 << 0;
const INPUT_REVERSE: u8 = 1 << 1;
const INPUT_LEFT: u8 = 1 << 2;
const INPUT_RIGHT: u8 = 1 << 3;
const INPUT_FIRE: u8 = 1 << 4;

//Types
// The first generic parameter, u8, is the input type: 4-directions + fire fits easily in a single byte
// The second parameter is the address type of peers: Matchbox' WebRtcSocket addresses are called `PeerId`s
type Config = bevy_ggrs::GgrsConfig<u8, PeerId>;

// Main
fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
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
            }),
            GgrsPlugin::<Config>::default(),
        ))
        .rollback_component_with_clone::<Transform>()
        .insert_resource(ClearColor(Color::rgb(0.53, 0.53, 0.53)))
        .init_resource::<MyWorldCoords>()
        .init_resource::<MyScale>()
        .init_resource::<MyNumPlayers>()
        .add_systems(Startup, (
            setup,
            spawn_players,
            start_matchbox_socket,))
        .add_systems(Update, (
            // my_cursor_system, 
            // player_movement_system,
            zoom_scalingmode,
            wait_for_players,
            bevy::window::close_on_esc))
        .add_systems(ReadInputs, (
            my_cursor_system,
            read_local_inputs,))
        .add_systems(GgrsSchedule, move_players2)
        .run();
}

/// We will store the world position of the mouse cursor here.
#[derive(Resource, Default)]
struct MyWorldCoords(Vec2);

/// Keeps track of current Zoom level
#[derive(Resource, Default)]
struct MyScale(f32);

/// We will store the number of players here
#[derive(Resource, Default)]
struct MyNumPlayers(u16);

/// Used to help identify our main camera
#[derive(Component)]
struct MainCamera;

/// player component
#[derive(Component)]
struct Player {
    handle: usize,
    /// linear speed in meters per second
    movement_speed: f32,
    /// rotation speed in radians per second
    rotation_speed: f32,
}

/// player component
#[derive(Component)]
struct Turret {
    handle: usize,
    /// rotation speed in radians per second
    rotation_speed: f32,
}

/// Target Reticle Component
#[derive(Component)]
struct Target;

/// Initializes the player shapes and camera
fn setup(
    mut commands: Commands,
    mut my_scale: ResMut<MyScale>,
    mut my_num_players: ResMut<MyNumPlayers>,
) {
    //Default player number
    my_num_players.0 = 2;
    // Default camera scale
    my_scale.0 = 30.;
    // Camera
    let mut camera_bundle = Camera2dBundle::default();
    // camera_bundle.projection.scaling_mode = ScalingMode::FixedVertical(100.);
    camera_bundle.projection.scaling_mode = ScalingMode::WindowSize(my_scale.0);
    commands.spawn((camera_bundle, MainCamera));


    // MAP SETUP
    // Horizontal lines
    for i in 0..=MAP_SIZE {
        commands.spawn(SpriteBundle {
            transform: Transform::from_translation(Vec3::new(
                0.,
                i as f32 - MAP_SIZE as f32 / 2.,
                -1.,
            )),
            sprite: Sprite {
                color: Color::rgb(0.27, 0.27, 0.27),
                custom_size: Some(Vec2::new(MAP_SIZE as f32, GRID_WIDTH)),
                ..default()
            },
            ..default()
        });
    }

    // Vertical lines
    for i in 0..=MAP_SIZE {
        commands.spawn(SpriteBundle {
            transform: Transform::from_translation(Vec3::new(
                i as f32 - MAP_SIZE as f32 / 2.,
                0.,
                -1.,
            )),
            sprite: Sprite {
                color: Color::rgb(0.27, 0.27, 0.27),
                custom_size: Some(Vec2::new(GRID_WIDTH, MAP_SIZE as f32)),
                ..default()
            },
            ..default()
        });
    }
}

/// Spawns the player sprite(s)
fn spawn_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // Rectangle
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.25, 0.25, 0.75),
                custom_size: Some(Vec2::new(2.0, 4.0)),
                ..default()
            },
        transform: Transform::from_translation(Vec3::new(0., 0., 100.)),
        ..default()
        },
        Player {
            handle: 0,
            movement_speed: 10.0,                  // meters per second
            rotation_speed: f32::to_radians(180.0), // degrees per second
        },
    ));

    // Triangle
    commands.spawn((
        MaterialMesh2dBundle {
        mesh: meshes.add(shape::RegularPolygon::new(1., 3).into()).into(),
        material: materials.add(ColorMaterial::from(Color::TURQUOISE)),
        transform: Transform::from_translation(Vec3::new(0., 0., 101.)),
        ..default()
        },
        Turret {
            handle: 0,
            rotation_speed: f32::to_radians(180.0), // degrees per second
        },
    ));

    // Circle
    commands.spawn((
        MaterialMesh2dBundle {
        mesh: meshes.add(shape::Circle::new(0.1).into()).into(),
        material: materials.add(ColorMaterial::from(Color::PURPLE)),
        transform: Transform::from_translation(Vec3::new(0., 0., 102.)),
        ..default()
    },
        Target,
    ));
}

/// Starts the matchbox socket to connect to the matchmaking server
fn start_matchbox_socket(mut commands: Commands) {
    let room_url = "ws://127.0.0.1:3536/extreme_bevy?next=2";
    info!("connecting to matchbox server: {room_url}");
    commands.insert_resource(MatchboxSocket::new_ggrs(room_url));
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
    tar_transform.translation = Vec3::from((mouse_cords.0, 102.));

    // Turret Handling
    tur_transform.translation = Vec3::from((ship_transform.translation.truncate(), 101.));
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

fn zoom_scalingmode(
    mut query_camera: Query<&mut OrthographicProjection, With<MainCamera>>,
    mut scroll_evr: EventReader<MouseWheel>,
    mut my_scale: ResMut<MyScale>,
) {
    let mut projection = query_camera.single_mut();

    for ev in scroll_evr.read() {
        if ev.y < 0. {
            if my_scale.0 > SCALE_STEP {
                my_scale.0 -= SCALE_STEP;
                projection.scaling_mode = ScalingMode::WindowSize(my_scale.0);
            } else {
                continue;
            }
            // projection.scale += 1.;
        } else if ev.y > 0. {
            if my_scale.0 < MAX_SCALE - SCALE_STEP {
                my_scale.0 += SCALE_STEP;
                projection.scaling_mode = ScalingMode::WindowSize(my_scale.0);
            } else {
                continue;
            }
            // projection.scale -= 1.;
        } else {
            continue;
        }

        println!("Current scale: {}", my_scale.0);
        println!("Scroll (line units): vertical: {}, horizontal: {}", ev.y, ev.x);
    }
}

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
        // eprintln!("World coords: {}/{}", world_position.x, world_position.y);
    }
}

fn wait_for_players(
    mut commands: Commands,
    mut socket: ResMut<MatchboxSocket<SingleChannel>>,
    my_num_players: ResMut<MyNumPlayers>,
){
    if socket.get_channel(0).is_err() {
        return; // we've already started
    }
    
    // Check for new connections
    socket.update_peers();
    let players = socket.players();

    let num_players = usize::from(my_num_players.0);
    if players.len() < num_players {
        return; // wait for more players
    }

    info!("All peers have joined, going in-game");

    // create a GGRS P2P session
    let mut session_builder = ggrs::SessionBuilder::<Config>::new()
        .with_num_players(num_players)
        .with_input_delay(1);

    for (i, player) in players.into_iter().enumerate() {
        session_builder = session_builder
            .add_player(player, i)
            .expect("failed to add player");
    }

    // move the channel out of the socket (required because GGRS takes ownership of it)
    let channel = socket.take_channel(0).unwrap();

    // start the GGRS session
    let ggrs_session = session_builder
        .start_p2p_session(channel)
        .expect("failed to start session");

    commands.insert_resource(bevy_ggrs::Session::P2P(ggrs_session));
}

fn read_local_inputs(
    mut commands: Commands,
    keys: Res<Input<KeyCode>>,
    local_players: Res<LocalPlayers>,
) {
    let mut local_inputs = HashMap::new();

    for handle in &local_players.0 {
        let mut input = 0u8;

        if keys.any_pressed([KeyCode::Up, KeyCode::W]) {
            input |= INPUT_FORWARD;
        }
        if keys.any_pressed([KeyCode::Down, KeyCode::S]) {
            input |= INPUT_REVERSE;
        }
        if keys.any_pressed([KeyCode::Left, KeyCode::A]) {
            input |= INPUT_LEFT
        }
        if keys.any_pressed([KeyCode::Right, KeyCode::D]) {
            input |= INPUT_RIGHT;
        }
        if keys.any_pressed([KeyCode::Space, KeyCode::Return]) {
            input |= INPUT_FIRE;
        }

        local_inputs.insert(*handle, input);
    }

    commands.insert_resource(LocalInputs::<Config>(local_inputs));
}

fn move_players(
    mut players: Query<(&mut Transform, &Player)>,
    inputs: Res<PlayerInputs<Config>>,
    time: Res<Time>,
) {
    for (mut transform, player) in &mut players {
        let (input, _) = inputs[player.handle];

        let mut direction = Vec2::ZERO;

        if input & INPUT_FORWARD != 0 {
            direction.y += 1.;
        }
        if input & INPUT_REVERSE != 0 {
            direction.y -= 1.;
        }
        if input & INPUT_RIGHT != 0 {
            direction.x += 1.;
        }
        if input & INPUT_LEFT != 0 {
            direction.x -= 1.;
        }
        if direction == Vec2::ZERO {
            continue;
        }

        let move_speed = 7.;
        let move_delta = direction * move_speed * time.delta_seconds();
        transform.translation += move_delta.extend(0.);
    }
}

/// Spawns the player sprite(s)
fn spawn_players(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    my_num_players: ResMut<MyNumPlayers>,
) {
    for i in 0..my_num_players.0 {
        // Rectangle
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: Color::rgb(0.25, 0.25, 0.75),
                    custom_size: Some(Vec2::new(2.0, 4.0)),
                    ..default()
                },
            transform: Transform::from_translation(Vec3::new(0. + f32::from(i) * 5., 0., 100.)),
            ..default()
            },
            Player {
                handle: usize::from(i),
                movement_speed: 10.0,                  // meters per second
                rotation_speed: f32::to_radians(180.0), // degrees per second
            },
        ))
        .add_rollback();

        // Triangle
        commands.spawn((
            MaterialMesh2dBundle {
            mesh: meshes.add(shape::RegularPolygon::new(1., 3).into()).into(),
            material: materials.add(ColorMaterial::from(Color::TURQUOISE)),
            transform: Transform::from_translation(Vec3::new(0. + f32::from(i) * 5., 0., 101.)),
            ..default()
            },
            Turret {
                handle: usize::from(i),
                rotation_speed: f32::to_radians(180.0), // degrees per second
            },
        ))
        .add_rollback();

        // Circle
        commands.spawn((
            MaterialMesh2dBundle {
            mesh: meshes.add(shape::Circle::new(0.1).into()).into(),
            material: materials.add(ColorMaterial::from(Color::PURPLE)),
            transform: Transform::from_translation(Vec3::new(0. + f32::from(i) *5., 0., 102.)),
            ..default()
        },
            Target,
        ));
        // .add_rollback();
    }
}

fn move_players2(
    inputs: Res<PlayerInputs<Config>>,
    time: Res<Time>,
    mut player_query: Query<(&Player, &mut Transform), With<Player>>,
    mut target_query: Query<(&Target, &mut Transform), Without<Player>>,
    // mut turret_query: Query<(&Turret, &mut Transform), (Without<Player>, Without<Target>)>,
    mouse_cords: Res<MyWorldCoords>,
) {
    // Body handling
    for (ship, mut ship_transform) in &mut player_query {
        let (input, _) = inputs[ship.handle];

        let mut rotation_factor = 0.0;
        let mut movement_factor = 0.0;

        if input & INPUT_FORWARD != 0 {
            movement_factor += 1.0
        }
        if input & INPUT_REVERSE != 0 {
            movement_factor -= 1.0
        }
        if input & INPUT_RIGHT != 0 {
            rotation_factor -= 1.0;
        }
        if input & INPUT_LEFT != 0 {
            rotation_factor += 1.0;
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
        for (_target, mut tar_transform) in &mut target_query{
            // let target_translation = tar_transform.translation.xy();
            tar_transform.translation = Vec3::from((mouse_cords.0, 102.));
        }
    }
}

// fn camera_follow(
//     local_players: Res<LocalPlayers>,
//     players: Query<(&Player, &Transform)>,
//     mut cameras: Query<&mut Transform, (With<Camera>, Without<Player>)>,
// ) {
//     for (player, player_transform) in &players {
//         // only follow the local player
//         if !local_players.0.contains(&player.handle) {
//             continue;
//         }

//         let pos = player_transform.translation;

//         for mut transform in &mut cameras {
//             transform.translation.x = pos.x;
//             transform.translation.y = pos.y;
//         }
//     }
// }
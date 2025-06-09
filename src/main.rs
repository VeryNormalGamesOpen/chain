use avian3d::prelude::*;
use bevy::asset::AssetMetaCheck;
use bevy::render::view::RenderLayers;
use bevy::{
    color::palettes::css,
    prelude::*,
    window::{WindowMode, WindowResolution},
};
use bevy_asset_loader::asset_collection::AssetCollection;
use bevy_asset_loader::prelude::*;
use bevy_seedling::prelude::*;
use bevy_seedling::sample::Sample;
use bevy_third_person_camera::*;

use bevy_tnua::{TnuaProximitySensor, prelude::*};
use bevy_tnua_avian3d::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins
                .set(AssetPlugin {
                    // Wasm builds will check for meta files (that don't exist) if this isn't set.
                    // This causes errors and even panics in web builds on itch.
                    // See https://github.com/bevyengine/bevy_github_ci_template/issues/48.
                    meta_check: AssetMetaCheck::Never,
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        resizable: false,
                        position: WindowPosition::Automatic,
                        mode: WindowMode::BorderlessFullscreen(MonitorSelection::Current),
                        resolution: WindowResolution::default(),
                        visible: true,
                        ..default()
                    }),
                    ..default()
                }),
            PhysicsPlugins::default(),
            TnuaControllerPlugin::new(FixedUpdate),
            TnuaAvian3dPlugin::new(FixedUpdate),
            SeedlingPlugin::default(),
            ThirdPersonCameraPlugin,
        ))
        .init_state::<GameState>()
        .add_loading_state(
            LoadingState::new(GameState::Loading)
                .continue_to_state(GameState::Menu)
                .load_collection::<AtomAssets>()
                .load_collection::<FontAssets>()
                .load_collection::<SoundAssets>(),
        )
        .add_systems(
            Update,
            (
                (game_camera, show_menu).run_if(state_changed::<GameState>),
                (setup_menu).run_if(in_state(GameState::Menu).and(run_once)),
                (start_button_system, exit_button_system)
                    .run_if(in_state(GameState::Menu).or(in_state(GameState::Win)).or(in_state(GameState::Pause))),
                key_pause.run_if(in_state(GameState::Game)),
                key_unpause.run_if(in_state(GameState::Pause)),
                (setup_camera_and_lights, setup_level)
                    .run_if(in_state(GameState::Game).and(run_once)),
                setup_player.run_if(
                    in_state(GameState::Game)
                        .and(not(any_with_component::<ThirdPersonCameraTarget>)),
                ),
                collision_response.run_if(on_event::<CollisionWith>),
                end_game.run_if(on_event::<GameOver>),
                (detect_atom).run_if(in_state(GameState::Game)),
            ),
        )
        .add_systems(
            FixedUpdate,
            apply_controls
                .in_set(TnuaUserControlsSystemSet)
                .run_if(in_state(GameState::Game)),
        )
        .add_event::<CollisionWith>()
        .add_event::<GameOver>()
        .run();
}

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);

#[derive(AssetCollection, Resource)]
pub struct AtomAssets {
    #[asset(path = "Atom.glb#Scene0")]
    u_atom: Handle<Scene>,
}

#[derive(AssetCollection, Resource)]
pub struct FontAssets {
    #[asset(path = "NotoSerif-Medium.ttf")]
    u_atom: Handle<Font>,
}

#[derive(AssetCollection, Resource)]
pub struct SoundAssets {
    #[asset(path = "HugeExplosion2.wav")]
    u_atom: Handle<Sample>,
}

#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
pub enum GameState {
    #[default]
    Loading,
    Menu,
    Game,
    Pause,
    Win,
}

#[derive(Event)]
struct CollisionWith(Entity);

#[derive(Event)]
struct GameOver(GameState);

#[derive(Component)]
struct WinGame;

#[derive(Component)]
struct MenuCamera;

#[derive(Component)]
struct StartButton;

#[derive(Component)]
struct QuitButton;

#[derive(Component)]
struct Menu {
    show_state: GameState,
}

#[derive(Component)]
struct DeathCountText;

fn game_camera(
    mut menu_cam_query: Query<&mut Camera, (With<MenuCamera>, Without<ThirdPersonCameraTarget>)>,
    mut game_cam_query: Query<(&mut Camera, &mut ThirdPersonCamera), Without<MenuCamera>>,
    state: Res<State<GameState>>,
) {
    let game_cam: bool = match state.get() {
        GameState::Loading => false,
        GameState::Menu => false,
        GameState::Game => true,
        GameState::Win => false,
        GameState::Pause => false,
    };

    if let Ok(mut menu_cam) = menu_cam_query.single_mut() {
        menu_cam.is_active = !game_cam;
    };
    if let Ok((mut cam, mut t_cam)) = game_cam_query.single_mut() {
        cam.is_active = game_cam;
        t_cam.cursor_lock_active = game_cam;
    };
}

fn show_menu(mut menu: Query<(&mut Visibility, &Menu)>, state: Res<State<GameState>>) {
    for (mut menu_viz, menu_type) in menu.iter_mut() {
        if menu_type.show_state == *state.get() {
            *menu_viz = Visibility::Visible;
        } else {
            *menu_viz = Visibility::Hidden;
        }
    }
}

fn setup_camera_and_lights(mut commands: Commands) {
    commands.spawn((
        Camera {
            clear_color: ClearColorConfig::Custom(Color::from(css::DARK_GRAY)),
            ..default()
        },
        Camera3d::default(),
        RenderLayers::layer(0),
        ThirdPersonCamera {
            offset: Offset::new(2.0, 0.0),
            cursor_lock_toggle_enabled: true,
            cursor_lock_key: KeyCode::KeyC,
            ..default()
        },
    ));

    commands.spawn((PointLight::default(), Transform::from_xyz(5.0, 5.0, 5.0)));

    commands.spawn((
        DirectionalLight {
            illuminance: 4000.0,
            shadows_enabled: true,
            ..Default::default()
        },
        Transform::default().looking_to(-Vec3::Y, Vec3::Z),
    ));
}

fn start_button_system(
    mut interaction_query: Query<
        (
            &Interaction,
            &mut BackgroundColor,
            &mut BorderColor,
            &Children,
        ),
        (Changed<Interaction>, With<Button>, With<StartButton>),
    >,
    mut text_query: Query<&mut Text>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for (interaction, mut color, mut border_color, children) in &mut interaction_query {
        let text = text_query.get_mut(children[0]).unwrap();
        match *interaction {
            Interaction::Pressed => {
                *color = PRESSED_BUTTON.into();
                border_color.0 = css::RED.into();
                next_state.set(GameState::Game);
            }
            Interaction::Hovered => {
                *color = HOVERED_BUTTON.into();
                border_color.0 = Color::WHITE;
            }
            Interaction::None => {
                *color = NORMAL_BUTTON.into();
                border_color.0 = Color::BLACK;
            }
        }
    }
}

fn exit_button_system(
    mut interaction_query: Query<
        (
            &Interaction,
            &mut BackgroundColor,
            &mut BorderColor,
            &Children,
        ),
        (Changed<Interaction>, With<Button>, With<QuitButton>),
    >,
    mut text_query: Query<&mut Text>,
    mut exit: EventWriter<AppExit>,
) {
    for (interaction, mut color, mut border_color, children) in &mut interaction_query {
        let text = text_query.get_mut(children[0]).unwrap();
        match *interaction {
            Interaction::Pressed => {
                *color = PRESSED_BUTTON.into();
                border_color.0 = css::RED.into();
                exit.write(AppExit::Success);
            }
            Interaction::Hovered => {
                *color = HOVERED_BUTTON.into();
                border_color.0 = Color::WHITE;
            }
            Interaction::None => {
                *color = NORMAL_BUTTON.into();
                border_color.0 = Color::BLACK;
            }
        }
    }
}

fn setup_menu(mut commands: Commands, font_assets: Res<FontAssets>) {
    commands.spawn((
        Camera2d,
        RenderLayers::layer(1),
        MenuCamera,
        IsDefaultUiCamera,
    ));

    commands.spawn((main_menu(&font_assets), RenderLayers::layer(1)));

    commands.spawn((
        win_menu(&font_assets),
        RenderLayers::layer(1),
        Visibility::Hidden,
    ));

    commands.spawn((
        pause_menu(&font_assets),
        RenderLayers::layer(1),
        Visibility::Hidden,
    ));
}

fn setup_level(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    atom_assets: Res<AtomAssets>,
) {
    // Spawn the ground.
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(1024.0, 1024.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        RigidBody::Static,
        Collider::half_space(Vec3::Y),
    ));

    for n in 1..10 {
        commands.spawn((
            SceneRoot(atom_assets.u_atom.clone()),
            Transform::from_xyz(10.0, 4.0, -20.0 + 9.0 * n as f32).looking_to(Vec3::Z, Vec3::Y),
            RigidBody::Static,
            Collider::sphere(4.0),
            WinGame,
        ));
    }
}

fn setup_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Sphere { radius: 0.5 })),
        MeshMaterial3d(materials.add(Color::from(css::DARK_CYAN))),
        Transform::from_xyz(0.0, 4.0, 0.0),
        RigidBody::Dynamic,
        Collider::sphere(0.5),
        TnuaController::default(),
        TnuaAvian3dSensorShape(Collider::cylinder(0.7, 0.0)),
        ThirdPersonCameraTarget,
    ));
}

fn detect_atom(
    query: Query<&TnuaProximitySensor>,
    mut event_collision: EventWriter<CollisionWith>,
) {
    let Ok(sensor) = query.single() else {
        return;
    };

    let Some(output) = &sensor.output else {
        return;
    };

    let entity2 = output.entity;

    event_collision.write(CollisionWith(entity2));

    println!("Player and {entity2} colliding");
}

fn collision_response(
    mut event_collision: EventReader<CollisionWith>,
    mut event_game_over: EventWriter<GameOver>,
    query: Query<&WinGame>,
) {
    for ev in event_collision.read() {
        eprintln!("Entity {:?} Collide!", &ev.0);
        if query.contains(ev.0) {
            event_game_over.write(GameOver(GameState::Win));
        }
    }
}

fn end_game(
    player: Single<Entity, With<ThirdPersonCameraTarget>>,
    mut commands: Commands,
    mut event_game_over: EventReader<GameOver>,
    _state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
    sound_assets: Res<SoundAssets>,
) {
    let Some(ev) = event_game_over.read().last() else {
        return;
    };
    commands.spawn(SamplePlayer::new(sound_assets.u_atom.clone()));

    commands.entity(*player).despawn();
    next_state.set(ev.0.clone());

    event_game_over.clear();
}

fn key_pause(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        next_state.set(GameState::Pause);
    }
}

fn key_unpause(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if  keyboard.just_pressed(KeyCode::Escape) {
        next_state.set(GameState::Game);
    }
}

fn apply_controls(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut TnuaController, &GlobalTransform)>,
    camera_query: Query<&GlobalTransform, With<ThirdPersonCamera>>,
) {
    let Ok((mut controller, player_transform)) = query.single_mut() else {
        return;
    };

    let Ok(camera) = camera_query.single() else {
        return;
    };

    let mut direction = Vec3::ZERO;

    if keyboard.pressed(KeyCode::KeyW) {
        direction += player_transform.forward().as_vec3();
    }
    if keyboard.pressed(KeyCode::KeyS) {
        direction += player_transform.back().as_vec3();
    }
    if keyboard.pressed(KeyCode::KeyA) {
        direction += player_transform.left().as_vec3();
    }
    if keyboard.pressed(KeyCode::KeyD) {
        direction += player_transform.right().as_vec3();
    }

    controller.basis(TnuaBuiltinWalk {
        desired_velocity: direction.normalize_or_zero() * 20.0,
        desired_forward: Dir3::new(
            camera.forward().as_vec3() - camera.forward().as_vec3().project_onto(Vec3::Y),
        )
        .ok(),

        float_height: 4.0,
        ..Default::default()
    });

    if keyboard.pressed(KeyCode::Space) {
        controller.action(TnuaBuiltinJump {
            height: 2.5,
            ..Default::default()
        });
    }
}

fn main_menu(assets: &FontAssets) -> impl Bundle + use<> {
    (
        Menu {
            show_state: GameState::Menu,
        },
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            row_gap: Val::Px(10.0),
            ..default()
        },
        children![
            (
                Text::new("Fissile Material"),
                TextFont {
                    font: assets.u_atom.clone(),
                    font_size: 100.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                TextShadow::default(),
            ),
            (
                Button,
                StartButton,
                Node {
                    width: Val::Px(300.0),
                    height: Val::Px(80.0),
                    border: UiRect::all(Val::Px(5.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BorderColor(Color::BLACK),
                BorderRadius::MAX,
                BackgroundColor(NORMAL_BUTTON),
                children![(
                    Text::new("Start Game"),
                    TextFont {
                        font: assets.u_atom.clone(),
                        font_size: 38.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.9, 0.9)),
                    TextShadow::default(),
                )]
            ),
            (
                Button,
                QuitButton,
                Node {
                    width: Val::Px(300.0),
                    height: Val::Px(80.0),
                    border: UiRect::all(Val::Px(5.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BorderColor(Color::BLACK),
                BorderRadius::MAX,
                BackgroundColor(NORMAL_BUTTON),
                children![(
                    Text::new("Quit"),
                    TextFont {
                        font: assets.u_atom.clone(),
                        font_size: 38.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.9, 0.9)),
                    TextShadow::default(),
                )]
            )
        ],
    )
}

fn win_menu(assets: &FontAssets) -> impl Bundle + use<> {
    (
        Menu {
            show_state: GameState::Win,
        },
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            row_gap: Val::Px(10.0),
            ..default()
        },
        children![
            (
                Text::new("Congratulations!"),
                TextFont {
                    font: assets.u_atom.clone(),
                    font_size: 100.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                TextShadow::default(),
            ),
            (
                Text::new("200000 Civilian Deaths!"),
                DeathCountText,
                TextFont {
                    font: assets.u_atom.clone(),
                    font_size: 80.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                TextShadow::default(),
            ),
            (
                Button,
                StartButton,
                Node {
                    width: Val::Px(300.0),
                    height: Val::Px(80.0),
                    border: UiRect::all(Val::Px(5.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BorderColor(Color::BLACK),
                BorderRadius::MAX,
                BackgroundColor(NORMAL_BUTTON),
                children![(
                    Text::new("Play Again"),
                    TextFont {
                        font: assets.u_atom.clone(),
                        font_size: 38.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.9, 0.9)),
                    TextShadow::default(),
                )]
            ),
            (
                Button,
                QuitButton,
                Node {
                    width: Val::Px(300.0),
                    height: Val::Px(80.0),
                    border: UiRect::all(Val::Px(5.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BorderColor(Color::BLACK),
                BorderRadius::MAX,
                BackgroundColor(NORMAL_BUTTON),
                children![(
                    Text::new("Quit"),
                    TextFont {
                        font: assets.u_atom.clone(),
                        font_size: 38.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.9, 0.9)),
                    TextShadow::default(),
                )]
            )
        ],
    )
}

fn pause_menu(assets: &FontAssets) -> impl Bundle + use<> {
    (
        Menu {
            show_state: GameState::Pause,
        },
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            row_gap: Val::Px(10.0),
            ..default()
        },
        children![
            (
                Button,
                StartButton,
                Node {
                    width: Val::Px(300.0),
                    height: Val::Px(80.0),
                    border: UiRect::all(Val::Px(5.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BorderColor(Color::BLACK),
                BorderRadius::MAX,
                BackgroundColor(NORMAL_BUTTON),
                children![(
                    Text::new("Resume"),
                    TextFont {
                        font: assets.u_atom.clone(),
                        font_size: 38.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.9, 0.9)),
                    TextShadow::default(),
                )]
            ),
            (
                Button,
                QuitButton,
                Node {
                    width: Val::Px(300.0),
                    height: Val::Px(80.0),
                    border: UiRect::all(Val::Px(5.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BorderColor(Color::BLACK),
                BorderRadius::MAX,
                BackgroundColor(NORMAL_BUTTON),
                children![(
                    Text::new("Quit"),
                    TextFont {
                        font: assets.u_atom.clone(),
                        font_size: 38.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.9, 0.9)),
                    TextShadow::default(),
                )]
            )
        ],
    )
}
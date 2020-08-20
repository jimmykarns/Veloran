pub mod camera;
pub mod figure;
pub mod particle;
pub mod simple;
pub mod terrain;

pub use self::{
    camera::{Camera, CameraMode},
    figure::FigureMgr,
    particle::ParticleMgr,
    terrain::Terrain,
};
use crate::{
    audio::{music::MusicMgr, sfx::SfxMgr, AudioFrontend},
    render::{
        create_pp_mesh, create_skybox_mesh, Consts, Globals, Light, Model, PostProcessLocals,
        PostProcessPipeline, Renderer, Shadow, SkyboxLocals, SkyboxPipeline,
    },
    window::{AnalogGameInput, Event},
};
use anim::character::SkeletonAttr;
use common::{
    comp,
    outcome::Outcome,
    state::{DeltaTime, State},
    terrain::{BlockKind, TerrainChunk},
    vol::ReadVol,
};
use comp::item::Reagent;
use specs::{Entity as EcsEntity, Join, WorldExt};
use vek::*;

// TODO: Don't hard-code this.
const CURSOR_PAN_SCALE: f32 = 0.005;

const MAX_LIGHT_COUNT: usize = 32;
const MAX_SHADOW_COUNT: usize = 24;
const LIGHT_DIST_RADIUS: f32 = 64.0; // The distance beyond which lights may not emit light from their origin
const SHADOW_DIST_RADIUS: f32 = 8.0;
const SHADOW_MAX_DIST: f32 = 96.0; // The distance beyond which shadows may not be visible

/// Above this speed is considered running
/// Used for first person camera effects
const RUNNING_THRESHOLD: f32 = 0.7;

struct EventLight {
    light: Light,
    timeout: f32,
    fadeout: fn(f32) -> f32,
}

struct Skybox {
    model: Model<SkyboxPipeline>,
    locals: Consts<SkyboxLocals>,
}

struct PostProcess {
    model: Model<PostProcessPipeline>,
    locals: Consts<PostProcessLocals>,
}

pub struct Scene {
    globals: Consts<Globals>,
    lights: Consts<Light>,
    shadows: Consts<Shadow>,
    camera: Camera,
    camera_input_state: Vec2<f32>,
    event_lights: Vec<EventLight>,

    skybox: Skybox,
    postprocess: PostProcess,
    terrain: Terrain<TerrainChunk>,
    loaded_distance: f32,
    select_pos: Option<Vec3<i32>>,

    particle_mgr: ParticleMgr,
    figure_mgr: FigureMgr,
    sfx_mgr: SfxMgr,
    music_mgr: MusicMgr,
}

pub struct SceneData<'a> {
    pub state: &'a State,
    pub player_entity: specs::Entity,
    pub target_entity: Option<specs::Entity>,
    pub loaded_distance: f32,
    pub view_distance: u32,
    pub tick: u64,
    pub thread_pool: &'a uvth::ThreadPool,
    pub gamma: f32,
    pub mouse_smoothing: bool,
    pub sprite_render_distance: f32,
    pub particles_enabled: bool,
    pub figure_lod_render_distance: f32,
    pub is_aiming: bool,
}

impl Scene {
    /// Create a new `Scene` with default parameters.
    pub fn new(renderer: &mut Renderer) -> Self {
        let resolution = renderer.get_resolution().map(|e| e as f32);

        Self {
            globals: renderer.create_consts(&[Globals::default()]).unwrap(),
            lights: renderer
                .create_consts(&[Light::default(); MAX_LIGHT_COUNT])
                .unwrap(),
            shadows: renderer
                .create_consts(&[Shadow::default(); MAX_SHADOW_COUNT])
                .unwrap(),
            camera: Camera::new(resolution.x / resolution.y, CameraMode::ThirdPerson),
            camera_input_state: Vec2::zero(),
            event_lights: Vec::new(),

            skybox: Skybox {
                model: renderer.create_model(&create_skybox_mesh()).unwrap(),
                locals: renderer.create_consts(&[SkyboxLocals::default()]).unwrap(),
            },
            postprocess: PostProcess {
                model: renderer.create_model(&create_pp_mesh()).unwrap(),
                locals: renderer
                    .create_consts(&[PostProcessLocals::default()])
                    .unwrap(),
            },
            terrain: Terrain::new(renderer),
            loaded_distance: 0.0,
            select_pos: None,

            particle_mgr: ParticleMgr::new(renderer),
            figure_mgr: FigureMgr::new(),
            sfx_mgr: SfxMgr::new(),
            music_mgr: MusicMgr::new(),
        }
    }

    /// Get a reference to the scene's globals.
    pub fn globals(&self) -> &Consts<Globals> { &self.globals }

    /// Get a reference to the scene's camera.
    pub fn camera(&self) -> &Camera { &self.camera }

    /// Get a reference to the scene's terrain.
    pub fn terrain(&self) -> &Terrain<TerrainChunk> { &self.terrain }

    /// Get a reference to the scene's particle manager.
    pub fn particle_mgr(&self) -> &ParticleMgr { &self.particle_mgr }

    /// Get a reference to the scene's figure manager.
    pub fn figure_mgr(&self) -> &FigureMgr { &self.figure_mgr }

    /// Get a mutable reference to the scene's camera.
    pub fn camera_mut(&mut self) -> &mut Camera { &mut self.camera }

    /// Set the block position that the player is interacting with
    pub fn set_select_pos(&mut self, pos: Option<Vec3<i32>>) { self.select_pos = pos; }

    pub fn select_pos(&self) -> Option<Vec3<i32>> { self.select_pos }

    /// Handle an incoming user input event (e.g.: cursor moved, key pressed,
    /// window closed).
    ///
    /// If the event is handled, return true.
    pub fn handle_input_event(&mut self, event: Event) -> bool {
        match event {
            // When the window is resized, change the camera's aspect ratio
            Event::Resize(dims) => {
                self.camera.set_aspect_ratio(dims.x as f32 / dims.y as f32);
                true
            },
            // Panning the cursor makes the camera rotate
            Event::CursorPan(delta) => {
                self.camera.rotate_by(Vec3::from(delta) * CURSOR_PAN_SCALE);
                true
            },
            // Zoom the camera when a zoom event occurs
            Event::Zoom(delta) => {
                self.camera
                    .zoom_switch(delta * (0.05 + self.camera.get_distance() * 0.01));
                true
            },
            Event::AnalogGameInput(input) => match input {
                AnalogGameInput::CameraX(d) => {
                    self.camera_input_state.x = d;
                    true
                },
                AnalogGameInput::CameraY(d) => {
                    self.camera_input_state.y = d;
                    true
                },
                _ => false,
            },
            // All other events are unhandled
            _ => false,
        }
    }

    pub fn handle_outcome(
        &mut self,
        outcome: &Outcome,
        scene_data: &SceneData,
        audio: &mut AudioFrontend,
    ) {
        self.particle_mgr.handle_outcome(&outcome, &scene_data);
        self.sfx_mgr.handle_outcome(&outcome, audio);

        match outcome {
            Outcome::Explosion {
                pos,
                power,
                reagent,
            } => self.event_lights.push(EventLight {
                light: Light::new(
                    *pos,
                    match reagent {
                        Some(Reagent::Blue) => Rgb::new(0.0, 0.0, 1.0),
                        Some(Reagent::Green) => Rgb::new(0.0, 1.0, 0.0),
                        Some(Reagent::Purple) => Rgb::new(1.0, 0.0, 1.0),
                        Some(Reagent::Red) => Rgb::new(1.0, 0.0, 0.0),
                        Some(Reagent::Yellow) => Rgb::new(1.0, 1.0, 0.0),
                        None => Rgb::new(1.0, 0.5, 0.0),
                    },
                    *power
                        * match reagent {
                            Some(_) => 5.0,
                            None => 2.5,
                        },
                ),
                timeout: match reagent {
                    Some(_) => 1.0,
                    None => 0.5,
                },
                fadeout: |timeout| timeout * 2.0,
            }),
            Outcome::ProjectileShot { .. } => {},
        }
    }

    /// Maintain data such as GPU constant buffers, models, etc. To be called
    /// once per tick.
    pub fn maintain(
        &mut self,
        renderer: &mut Renderer,
        audio: &mut AudioFrontend,
        scene_data: &SceneData,
    ) {
        // Get player position.
        let ecs = scene_data.state.ecs();

        let player_pos = ecs
            .read_storage::<comp::Pos>()
            .get(scene_data.player_entity)
            .map_or(Vec3::zero(), |pos| pos.0);

        let player_rolling = ecs
            .read_storage::<comp::CharacterState>()
            .get(scene_data.player_entity)
            .map_or(false, |cs| cs.is_dodge());

        let is_running = ecs
            .read_storage::<comp::Vel>()
            .get(scene_data.player_entity)
            .map(|v| v.0.magnitude_squared() > RUNNING_THRESHOLD.powi(2))
            .unwrap_or(false);

        let on_ground = ecs
            .read_storage::<comp::PhysicsState>()
            .get(scene_data.player_entity)
            .map(|p| p.on_ground);

        let player_scale = match scene_data
            .state
            .ecs()
            .read_storage::<comp::Body>()
            .get(scene_data.player_entity)
        {
            Some(comp::Body::Humanoid(body)) => SkeletonAttr::calculate_scale(body),
            _ => 1_f32,
        };

        // Add the analog input to camera
        self.camera
            .rotate_by(Vec3::from([self.camera_input_state.x, 0.0, 0.0]));
        self.camera
            .rotate_by(Vec3::from([0.0, self.camera_input_state.y, 0.0]));

        // Alter camera position to match player.
        let tilt = self.camera.get_orientation().y;
        let dist = self.camera.get_distance();

        let up = match self.camera.get_mode() {
            CameraMode::FirstPerson => {
                if player_rolling {
                    player_scale * 0.8
                } else if is_running && on_ground.unwrap_or(false) {
                    player_scale * 1.65 + (scene_data.state.get_time() as f32 * 17.0).sin() * 0.05
                } else {
                    player_scale * 1.65
                }
            },
            CameraMode::ThirdPerson if scene_data.is_aiming => player_scale * 2.1,
            CameraMode::ThirdPerson => player_scale * 1.65,
            CameraMode::Freefly => 0.0,
        };

        match self.camera.get_mode() {
            CameraMode::FirstPerson | CameraMode::ThirdPerson => {
                self.camera.set_focus_pos(
                    player_pos + Vec3::unit_z() * (up - tilt.min(0.0).sin() * dist * 0.6),
                );
            },
            CameraMode::Freefly => {},
        };

        // Tick camera for interpolation.
        self.camera.update(
            scene_data.state.get_time(),
            scene_data.state.get_delta_time(),
            scene_data.mouse_smoothing,
            Some(&*scene_data.state.terrain()),
        );

        // Compute camera matrices.
        self.camera.compute_dependents(&*scene_data.state.terrain());
        let camera::Dependents {
            view_mat,
            proj_mat,
            cam_pos,
            ..
        } = self.camera.dependents();

        // Update chunk loaded distance smoothly for nice shader fog
        self.loaded_distance =
            (0.98 * self.loaded_distance + 0.02 * scene_data.loaded_distance).max(0.01);

        // Update light constants
        let mut lights = (
            &scene_data.state.ecs().read_storage::<comp::Pos>(),
            scene_data.state.ecs().read_storage::<comp::Ori>().maybe(),
            scene_data
                .state
                .ecs()
                .read_storage::<crate::ecs::comp::Interpolated>()
                .maybe(),
            &scene_data
                .state
                .ecs()
                .read_storage::<comp::LightAnimation>(),
        )
            .join()
            .filter(|(pos, _, _, _)| {
                (pos.0.distance_squared(player_pos) as f32)
                    < self.loaded_distance.powf(2.0) + LIGHT_DIST_RADIUS
            })
            .map(|(pos, ori, interpolated, light_anim)| {
                // Use interpolated values if they are available
                let (pos, ori) =
                    interpolated.map_or((pos.0, ori.map(|o| o.0)), |i| (i.pos, Some(i.ori)));
                let rot = {
                    if let Some(o) = ori {
                        Mat3::rotation_z(-o.x.atan2(o.y))
                    } else {
                        Mat3::identity()
                    }
                };
                Light::new(
                    pos + (rot * light_anim.offset),
                    light_anim.col,
                    light_anim.strength,
                )
            })
            .chain(
                self.event_lights
                    .iter()
                    .map(|el| el.light.with_strength((el.fadeout)(el.timeout))),
            )
            .collect::<Vec<_>>();
        lights.sort_by_key(|light| light.get_pos().distance_squared(player_pos) as i32);
        lights.truncate(MAX_LIGHT_COUNT);
        renderer
            .update_consts(&mut self.lights, &lights)
            .expect("Failed to update light constants");

        // Update event lights
        let dt = ecs.fetch::<DeltaTime>().0;
        self.event_lights.drain_filter(|el| {
            el.timeout -= dt;
            el.timeout <= 0.0
        });

        // Update shadow constants
        let mut shadows = (
            &scene_data.state.ecs().read_storage::<comp::Pos>(),
            scene_data
                .state
                .ecs()
                .read_storage::<crate::ecs::comp::Interpolated>()
                .maybe(),
            scene_data.state.ecs().read_storage::<comp::Scale>().maybe(),
            &scene_data.state.ecs().read_storage::<comp::Body>(),
            &scene_data.state.ecs().read_storage::<comp::Stats>(),
        )
            .join()
            .filter(|(_, _, _, _, stats)| !stats.is_dead)
            .filter(|(pos, _, _, _, _)| {
                (pos.0.distance_squared(player_pos) as f32)
                    < (self.loaded_distance.min(SHADOW_MAX_DIST) + SHADOW_DIST_RADIUS).powf(2.0)
            })
            .map(|(pos, interpolated, scale, _, _)| {
                Shadow::new(
                    // Use interpolated values pos if it is available
                    interpolated.map_or(pos.0, |i| i.pos),
                    scale.map_or(1.0, |s| s.0),
                )
            })
            .collect::<Vec<_>>();
        shadows.sort_by_key(|shadow| shadow.get_pos().distance_squared(player_pos) as i32);
        shadows.truncate(MAX_SHADOW_COUNT);
        renderer
            .update_consts(&mut self.shadows, &shadows)
            .expect("Failed to update light constants");

        // Update global constants.
        renderer
            .update_consts(&mut self.globals, &[Globals::new(
                view_mat,
                proj_mat,
                cam_pos,
                self.camera.get_focus_pos(),
                self.loaded_distance,
                scene_data.state.get_time_of_day(),
                scene_data.state.get_time(),
                renderer.get_resolution(),
                lights.len(),
                shadows.len(),
                scene_data
                    .state
                    .terrain()
                    .get(cam_pos.map(|e| e.floor() as i32))
                    .map(|b| b.kind())
                    .unwrap_or(BlockKind::Air),
                self.select_pos,
                scene_data.gamma,
                self.camera.get_mode(),
                scene_data.sprite_render_distance as f32 - 20.0,
            )])
            .expect("Failed to update global constants");

        // Maintain the terrain.
        self.terrain.maintain(
            renderer,
            &scene_data,
            self.camera.get_focus_pos(),
            self.loaded_distance,
            view_mat,
            proj_mat,
        );

        // Maintain the figures.
        self.figure_mgr.maintain(renderer, scene_data, &self.camera);

        // Remove unused figures.
        self.figure_mgr.clean(scene_data.tick);

        // Maintain the particles.
        self.particle_mgr.maintain(renderer, &scene_data);

        // Maintain audio
        self.sfx_mgr.maintain(
            audio,
            scene_data.state,
            scene_data.player_entity,
            &self.camera,
        );
        self.music_mgr.maintain(audio, scene_data.state);
    }

    /// Render the scene using the provided `Renderer`.
    pub fn render(
        &mut self,
        renderer: &mut Renderer,
        state: &State,
        player_entity: EcsEntity,
        tick: u64,
        scene_data: &SceneData,
    ) {
        // Render terrain and figures.
        self.terrain.render(
            renderer,
            &self.globals,
            &self.lights,
            &self.shadows,
            self.camera.get_focus_pos(),
        );
        self.figure_mgr.render(
            renderer,
            state,
            player_entity,
            tick,
            &self.globals,
            &self.lights,
            &self.shadows,
            &self.camera,
            scene_data.figure_lod_render_distance,
        );

        self.particle_mgr.render(
            renderer,
            scene_data,
            &self.globals,
            &self.lights,
            &self.shadows,
        );

        // Render the skybox.
        renderer.render_skybox(&self.skybox.model, &self.globals, &self.skybox.locals);

        self.figure_mgr.render_player(
            renderer,
            state,
            player_entity,
            tick,
            &self.globals,
            &self.lights,
            &self.shadows,
            &self.camera,
            scene_data.figure_lod_render_distance,
        );

        self.terrain.render_translucent(
            renderer,
            &self.globals,
            &self.lights,
            &self.shadows,
            self.camera.get_focus_pos(),
            scene_data.sprite_render_distance,
        );

        renderer.render_post_process(
            &self.postprocess.model,
            &self.globals,
            &self.postprocess.locals,
        );
    }
}

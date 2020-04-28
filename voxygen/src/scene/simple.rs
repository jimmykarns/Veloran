use crate::{
    anim::{
        character::{CharacterSkeleton, IdleAnimation, SkeletonAttr},
        fixture::FixtureSkeleton,
        Animation, Skeleton,
    },
    render::{
        create_pp_mesh, create_skybox_mesh, Consts, FigurePipeline, FirstDrawer, Globals, Light,
        Model, PostProcessLocals, PostProcessPipeline, Renderer, SecondDrawer, Shadow,
        SkyboxLocals, SkyboxPipeline,
    },
    scene::{
        camera::{self, Camera, CameraMode},
        figure::{load_mesh, FigureModelCache, FigureState},
    },
    window::{Event, PressState},
};
use common::{
    comp::{humanoid, Body, Equipment},
    terrain::BlockKind,
    vol::{BaseVol, ReadVol, Vox},
};
use log::error;
use vek::*;

#[derive(PartialEq, Eq, Copy, Clone)]
struct VoidVox;
impl Vox for VoidVox {
    fn empty() -> Self { VoidVox }

    fn is_empty(&self) -> bool { true }

    fn or(self, _other: Self) -> Self { VoidVox }
}
struct VoidVol;
impl BaseVol for VoidVol {
    type Error = ();
    type Vox = VoidVox;
}
impl ReadVol for VoidVol {
    fn get<'a>(&'a self, _pos: Vec3<i32>) -> Result<&'a Self::Vox, Self::Error> { Ok(&VoidVox) }
}

struct Skybox {
    model: Model,
    locals: Consts<SkyboxLocals>,
}

struct PostProcess {
    model: Model,
    locals: Consts<PostProcessLocals>,
}

pub struct Scene {
    globals: Consts<Globals>,
    lights: Consts<Light>,
    shadows: Consts<Shadow>,
    camera: Camera,

    skybox: Skybox,
    postprocess: PostProcess,
    backdrop: Option<(Model, FigureState<FixtureSkeleton>)>,

    figure_model_cache: FigureModelCache,
    figure_state: FigureState<CharacterSkeleton>,

    turning: bool,
    char_ori: f32,
}

pub struct SceneData {
    pub time: f64,
    pub delta_time: f32,
    pub tick: u64,
    pub body: Option<humanoid::Body>,
    pub gamma: f32,
}

impl Scene {
    pub fn new(renderer: &mut Renderer, backdrop: Option<&str>) -> Self {
        let resolution = renderer.get_resolution().map(|e| e as f32);

        let mut camera = Camera::new(resolution.x / resolution.y, CameraMode::ThirdPerson);
        camera.set_focus_pos(Vec3::unit_z() * 1.5);
        camera.set_distance(3.0); // 4.2
        camera.set_orientation(Vec3::new(0.0, 0.0, 0.0));

        Self {
            globals: renderer.create_consts(&[Globals::default()]),
            lights: renderer.create_consts(&[Light::default(); 32]),
            shadows: renderer.create_consts(&[Shadow::default(); 32]),
            camera,

            skybox: Skybox {
                model: renderer.create_model(&create_skybox_mesh()),
                locals: renderer.create_consts(&[SkyboxLocals::default()]),
            },
            postprocess: PostProcess {
                model: renderer.create_model(&create_pp_mesh()),
                locals: renderer.create_consts(&[PostProcessLocals::default()]),
            },
            figure_model_cache: FigureModelCache::new(),
            figure_state: FigureState::new(renderer, CharacterSkeleton::new()),

            backdrop: backdrop.map(|specifier| {
                (
                    renderer.create_model(&load_mesh(specifier, Vec3::new(-55.0, -49.5, -2.0))),
                    FigureState::new(renderer, FixtureSkeleton::new()),
                )
            }),

            turning: false,
            char_ori: 0.0,
        }
    }

    pub fn globals(&self) -> &Consts<Globals> { &self.globals }

    pub fn camera_mut(&mut self) -> &mut Camera { &mut self.camera }

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
            Event::MouseButton(_, state) => {
                self.turning = state == PressState::Pressed;
                true
            },
            Event::CursorMove(delta) if self.turning => {
                self.char_ori += delta.x * 0.01;
                true
            },
            // All other events are unhandled
            _ => false,
        }
    }

    pub fn maintain(&mut self, renderer: &mut Renderer, scene_data: SceneData) {
        self.camera.update(scene_data.time);

        self.camera.compute_dependents(&VoidVol);
        let camera::Dependents {
            view_mat,
            proj_mat,
            cam_pos,
        } = self.camera.dependents();
        const VD: f32 = 115.0; // View Distance
        const TIME: f64 = 43200.0; // 12 hours*3600 seconds
        renderer.update_consts(&mut self.globals, &[Globals::new(
            view_mat,
            proj_mat,
            cam_pos,
            self.camera.get_focus_pos(),
            VD,
            TIME,
            scene_data.time,
            renderer.get_resolution(),
            0,
            0,
            BlockKind::Air,
            None,
            scene_data.gamma,
        )]);

        self.figure_model_cache.clean(scene_data.tick);

        if let Some(body) = scene_data.body {
            let tgt_skeleton = IdleAnimation::update_skeleton(
                self.figure_state.skeleton_mut(),
                scene_data.time,
                scene_data.time,
                &mut 0.0,
                &SkeletonAttr::from(&body),
            );
            self.figure_state
                .skeleton_mut()
                .interpolate(&tgt_skeleton, scene_data.delta_time);
        }

        self.figure_state.update(
            renderer,
            Vec3::zero(),
            Vec3::zero(),
            Vec3::new(self.char_ori.sin(), -self.char_ori.cos(), 0.0),
            1.0,
            Rgba::broadcast(1.0),
            1.0 / 60.0, // TODO: Use actual deltatime here?
            1.0,
            1.0,
            0,
            true,
        );
    }

    pub fn first_render<'b>(
        &'b mut self,
        drawer: &'b mut FirstDrawer<'b>,
        tick: u64,
        body: Option<humanoid::Body>,
        equipment: &Equipment,
    ) {
        drawer.draw_skybox(&self.skybox.model, &self.skybox.locals, &self.globals);

        if let Some(body) = body {
            let model = &self
                .figure_model_cache
                .get_or_create_model(
                    drawer.renderer,
                    Body::Humanoid(body),
                    Some(equipment),
                    tick,
                    CameraMode::default(),
                    None,
                )
                .0;

            drawer.draw_figure(
                model,
                self.figure_state.locals(),
                self.figure_state.bone_consts(),
                &self.globals,
                &self.lights,
                &self.shadows,
            );
        }

        if let Some((model, state)) = &self.backdrop {
            drawer.draw_figure(
                model,
                state.locals(),
                state.bone_consts(),
                &self.globals,
                &self.lights,
                &self.shadows,
            );
        }
    }

    pub fn second_render<'b>(&'b mut self, drawer: &'b mut SecondDrawer<'b>) {
        drawer.draw_post_process(
            &self.postprocess.model,
            &self.postprocess.locals,
            &self.globals,
        );
    }
}

pub mod consts;
pub mod instances;
pub mod mesh;
pub mod model;
pub mod pipelines;
pub mod renderer;
pub mod texture;
mod util;

// Reexports
pub use self::{
    consts::Consts,
    instances::Instances,
    mesh::{Mesh, Quad, Tri},
    model::{DynamicModel, Model},
    pipelines::{
        figure::{BoneData as FigureBoneData, FigurePipeline, Locals as FigureLocals},
        fluid::FluidPipeline,
        postprocess::{
            create_mesh as create_pp_mesh, Locals as PostProcessLocals, PostProcessPipeline,
        },
        skybox::{create_mesh as create_skybox_mesh, Locals as SkyboxLocals, SkyboxPipeline},
        sprite::{Instance as SpriteInstance, SpritePipeline},
        terrain::{Locals as TerrainLocals, TerrainPipeline},
        ui::{
            create_quad as create_ui_quad, create_tri as create_ui_tri, Locals as UiLocals,
            Mode as UiMode, UiPipeline,
        },
        Globals, Light, Shadow,
    },
    renderer::Renderer,
    texture::Texture,
};
use zerocopy::AsBytes;

pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24PlusStencil8;

/// Used to represent a specific rendering configuration.
///
/// Note that pipelines are tied to the
/// rendering backend, and as such it is necessary to modify the rendering
/// subsystem when adding new pipelines - custom pipelines are not currently an
/// objective of the rendering subsystem.
///
/// # Examples
///
/// - `SkyboxPipeline`
/// - `FigurePipeline`
pub trait Pipeline {
    type Vertex: Clone + AsBytes;
}

use serde_derive::{Deserialize, Serialize};
/// Anti-aliasing modes
#[derive(PartialEq, Eq, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum AaMode {
    None,
    Fxaa,
    MsaaX4,
    MsaaX8,
    MsaaX16,
    SsaaX4,
}

/// Cloud modes
#[derive(PartialEq, Eq, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum CloudMode {
    None,
    Regular,
}

/// Fluid modes
#[derive(PartialEq, Eq, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum FluidMode {
    Cheap,
    Shiny,
}

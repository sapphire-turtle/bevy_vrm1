mod material;
mod outline_pass;
mod setup;

use crate::error::vrm_error;
use crate::prelude::*;
use crate::vrm::gltf::materials::VrmcMaterialsExtensitions;
use crate::vrm::mtoon::outline_pass::MToonOutlinePlugin;
use crate::vrm::mtoon::setup::MToonMaterialSetupPlugin;
use bevy::asset::{AssetId, AssetPath, embedded_asset, embedded_path};
use bevy::prelude::*;
use std::collections::HashMap;

pub mod prelude {
    pub use crate::vrm::mtoon::{MtoonMaterialPlugin, VrmcMaterialRegistry, material::prelude::*};
}

/// The embedded asset path of the `MToon` vertex shader.
fn mtoon_vertex_shader_path() -> AssetPath<'static> {
    AssetPath::from_path_buf(embedded_path!("mtoon_vertex.wesl")).with_source("embedded")
}

/// The embedded asset path of the `MToon` fragment shader.
fn mtoon_fragment_shader_path() -> AssetPath<'static> {
    AssetPath::from_path_buf(embedded_path!("mtoon_fragment.wesl")).with_source("embedded")
}

pub struct MtoonMaterialPlugin;

impl Plugin for MtoonMaterialPlugin {
    fn build(
        &self,
        app: &mut App,
    ) {
        // Register the shaders before `MaterialPlugin`: it starts loading the
        // material's vertex and fragment shaders while it is being built.
        // `mtoon_types` is imported by the other two, so it stays loaded
        // as a shader library (imported as `bevy_vrm1::vrm::mtoon_types`).
        bevy::shader::load_shader_library!(app, "mtoon_types.wesl");
        embedded_asset!(app, "mtoon_vertex.wesl");
        embedded_asset!(app, "mtoon_fragment.wesl");
        app.register_type::<MToonMaterial>()
            .register_type::<MToonOutline>()
            .register_type::<VrmcMaterialRegistry>()
            .register_type::<RimLighting>()
            .register_type::<UVAnimation>()
            .register_type::<Shade>()
            .add_plugins(MaterialPlugin::<MToonMaterial>::default())
            .add_plugins((MToonMaterialSetupPlugin, MToonOutlinePlugin));
    }
}

#[derive(Component, Default, Debug, Reflect)]
#[reflect(Component)]
pub struct VrmcMaterialRegistry {
    pub images: Vec<Handle<Image>>,
    pub materials: HashMap<AssetId<StandardMaterial>, VrmcMaterialsExtensitions>,
}

impl VrmcMaterialRegistry {
    pub fn new(
        gltf: &Gltf,
        images: Vec<Handle<Image>>,
        asset_server: &AssetServer,
    ) -> Self {
        Self::try_new(gltf, images, asset_server).unwrap_or_default()
    }

    fn try_new(
        gltf: &Gltf,
        images: Vec<Handle<Image>>,
        asset_server: &AssetServer,
    ) -> Option<Self> {
        // Match glTF materials to Bevy `StandardMaterial` handles by index,
        // not by name. The glTF spec does not require material names to be
        // unique, and some exporters (e.g. VRoid) produce multiple materials
        // that share a name. `Gltf::named_materials` is a `HashMap` keyed by
        // name, so duplicates collapse to a single entry and any meshes bound
        // to the overwritten materials skip the MToon conversion entirely,
        // rendering with the default `StandardMaterial` instead.
        let materials = gltf
            .source
            .as_ref()?
            .materials()
            .flat_map(|m| {
                let index = m.index()?;
                let gltf_material_path = gltf.materials.get(index)?.path()?;
                let std_label = format!("{}/std", gltf_material_path.label()?);
                let std_path = gltf_material_path.clone().with_label(std_label);
                let asset_id = asset_server.load::<StandardMaterial>(std_path).id();
                let extensions = m.extensions()?;
                match serde_json::from_value(extensions.get("VRMC_materials_mtoon")?.clone()) {
                    Ok(properties) => Some((asset_id, properties)),
                    Err(e) => {
                        vrm_error!("Failed to parse VRMC_materials_mtoon", e);
                        None
                    }
                }
            })
            .collect();
        Some(Self { materials, images })
    }
}

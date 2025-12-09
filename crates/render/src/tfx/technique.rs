use std::{ops::Deref, sync::Arc};

use alkahest_data::tfx::{STechnique, STechniqueShader, ShaderStage, TechniqueBindMode};
use anyhow::{ensure, Context};
use d3d11::DeviceChild;
use tiger_parse::PackageManagerExt;
use tiger_pkg::{package_manager, TagHash};

use super::dynamic_constants::DynamicConstants;
use crate::{
    asset::Handle, gpu::command_list::CommandList,
    tfx::expression_vm::interpreter::TempObjectChannels, Gpu,
};

pub struct Technique {
    pub tech: STechnique,
    pub hash: TagHash,

    pub stage_vertex: Option<TechniqueStage>,
    pub stage_hull: Option<TechniqueStage>,
    pub stage_domain: Option<TechniqueStage>,
    pub stage_geometry: Option<TechniqueStage>,
    pub stage_pixel: Option<TechniqueStage>,
    pub stage_compute: Option<TechniqueStage>,
}

impl Technique {
    pub fn all_stages(&self) -> [(&STechniqueShader, Option<&TechniqueStage>); 6] {
        [
            (&self.tech.shader_pixel, self.stage_pixel.as_ref()),
            (&self.tech.shader_hull, self.stage_hull.as_ref()),
            (&self.tech.shader_domain, self.stage_domain.as_ref()),
            (&self.tech.shader_geometry, self.stage_geometry.as_ref()),
            (&self.tech.shader_vertex, self.stage_vertex.as_ref()),
            (&self.tech.shader_compute, self.stage_compute.as_ref()),
        ]
    }

    pub fn all_stages_mut(&mut self) -> [(&STechniqueShader, Option<&mut TechniqueStage>); 6] {
        [
            (&self.tech.shader_pixel, self.stage_pixel.as_mut()),
            (&self.tech.shader_hull, self.stage_hull.as_mut()),
            (&self.tech.shader_domain, self.stage_domain.as_mut()),
            (&self.tech.shader_geometry, self.stage_geometry.as_mut()),
            (&self.tech.shader_vertex, self.stage_vertex.as_mut()),
            (&self.tech.shader_compute, self.stage_compute.as_mut()),
        ]
    }

    pub fn is_loaded(&self) -> bool {
        for (_shader, stage) in self.all_stages() {
            if let Some(stage) = stage {
                if !stage.is_loaded() {
                    return false;
                }
            }
        }

        true
    }
}

impl Technique {
    #[profiling::function]
    #[tracing::instrument(skip(gpu, hash), fields(technique = %hash))]
    pub fn load(gpu: &Arc<Gpu>, hash: TagHash) -> anyhow::Result<Self> {
        let tech = package_manager()
            .read_tag_struct::<STechnique>(hash)
            .context("Failed to read technique tag")?;
        Ok(Self {
            stage_vertex: TechniqueStage::load(gpu, &tech.shader_vertex, ShaderStage::Vertex, hash)
                .context("Failed to load vertex stage")?,
            stage_hull: TechniqueStage::load(gpu, &tech.shader_hull, ShaderStage::Hull, hash)
                .context("Failed to load hull stage")?,
            stage_domain: TechniqueStage::load(gpu, &tech.shader_domain, ShaderStage::Domain, hash)
                .context("Failed to load domain stage")?,
            stage_geometry: TechniqueStage::load(
                gpu,
                &tech.shader_geometry,
                ShaderStage::Geometry,
                hash,
            )
            .context("Failed to load geometry stage")?,
            stage_pixel: TechniqueStage::load(gpu, &tech.shader_pixel, ShaderStage::Pixel, hash)
                .context("Failed to load pixel stage")?,
            stage_compute: TechniqueStage::load(
                gpu,
                &tech.shader_compute,
                ShaderStage::Compute,
                hash,
            )
            .context("Failed to load compute stage")?,
            tech,
            hash,
        })
    }

    pub fn bind(&self, cmd: &mut CommandList) -> anyhow::Result<()> {
        self.bind_with_channels(cmd, None)
    }

    // #[profiling::function]
    // #[tracing::instrument(skip(self, cmd, channels), fields(technique = %self.hash))]
    pub fn bind_with_channels(
        &self,
        cmd: &mut CommandList,
        channels: Option<&TempObjectChannels>,
    ) -> anyhow::Result<()> {
        profiling::scope!("Technique::bind", &format!("hash={}", self.hash));
        // TODO(cohae): This might break (it probably will, it just wont have that big of an impact)
        if cmd.set_bound_technique(self.hash) {
            return Ok(());
        }

        // let _s = info_span!("Bind technique", technique = %self.hash).entered();
        // let states = renderer.gpu.current_states.load().select(&self.tech.states);
        let states = cmd
            .state
            .select(&self.tech.states)
            .select(&cmd.state_override);
        cmd.apply_state(&states);

        match self.bind_mode {
            TechniqueBindMode::VertexPixel => {
                cmd.geometry_set_shader(None);
                cmd.hull_set_shader(None);
                cmd.domain_set_shader(None);
                cmd.compute_set_shader(None);

                self.stage_vertex.as_ref().unwrap().bind(cmd, channels)?;
                self.stage_pixel.as_ref().unwrap().bind(cmd, channels)?;
            }
            TechniqueBindMode::VertexOnly => {
                cmd.pixel_set_shader(None);
                cmd.geometry_set_shader(None);
                cmd.hull_set_shader(None);
                cmd.domain_set_shader(None);
                cmd.compute_set_shader(None);

                self.stage_vertex.as_ref().unwrap().bind(cmd, channels)?;
            }
            TechniqueBindMode::VertexGeometryPixel => {
                cmd.hull_set_shader(None);
                cmd.domain_set_shader(None);
                cmd.compute_set_shader(None);

                self.stage_vertex.as_ref().unwrap().bind(cmd, channels)?;
                self.stage_geometry.as_ref().unwrap().bind(cmd, channels)?;
                self.stage_pixel.as_ref().unwrap().bind(cmd, channels)?;
            }
            TechniqueBindMode::VertexPixelTesselated => {
                cmd.geometry_set_shader(None);
                cmd.compute_set_shader(None);

                self.stage_vertex.as_ref().unwrap().bind(cmd, channels)?;
                self.stage_hull.as_ref().unwrap().bind(cmd, channels)?;
                self.stage_domain.as_ref().unwrap().bind(cmd, channels)?;
                self.stage_pixel.as_ref().unwrap().bind(cmd, channels)?;
            }
            TechniqueBindMode::VertexOnlyTesselated => {
                cmd.pixel_set_shader(None);
                cmd.geometry_set_shader(None);
                cmd.compute_set_shader(None);

                self.stage_vertex.as_ref().unwrap().bind(cmd, channels)?;
                self.stage_hull.as_ref().unwrap().bind(cmd, channels)?;
                self.stage_domain.as_ref().unwrap().bind(cmd, channels)?;
            }
            TechniqueBindMode::Compute => {
                cmd.vertex_set_shader(None);
                cmd.pixel_set_shader(None);
                cmd.geometry_set_shader(None);
                cmd.hull_set_shader(None);
                cmd.domain_set_shader(None);

                self.stage_compute.as_ref().unwrap().bind(cmd, channels)?;
            }
        }

        Ok(())
    }
}

impl Deref for Technique {
    type Target = STechnique;

    fn deref(&self) -> &Self::Target {
        &self.tech
    }
}

pub struct TechniqueStage {
    pub shader: STechniqueShader,
    pub stage: ShaderStage,

    pub shader_module: ShaderModule,
    pub dynamic_constants: DynamicConstants,
}

impl TechniqueStage {
    pub fn load(
        gpu: &Arc<Gpu>,
        shader: &STechniqueShader,
        stage: ShaderStage,
        technique_hash: TagHash,
    ) -> anyhow::Result<Option<Self>> {
        if shader.shader.is_none() {
            return Ok(None);
        }

        // if stage == ShaderStage::Vertex {
        //     println!("{technique_hash}");
        //     let mut decompiler = DecompilerState::new(&shader.constants.bytecode);
        //     let mut output = vec![String::new(); 128];
        //     if decompiler
        //         .evaluate(&shader.constants.bytecode_constants, &mut output)
        //         .is_ok()
        //     {
        //         for (index, line) in output.iter().enumerate() {
        //             if line.is_empty() {
        //                 continue;
        //             }
        //             println!("\tcb0[{}]: {}", index, line);
        //         }
        //     }
        // }

        let dynamic_constants = DynamicConstants::load(gpu, &shader.constants)?;

        // let recompile = false;
        // let recompile = is_renderdoc_connected()
        //     && !Renderer::is_initialized() // Only recompile from shaders that are loaded before the renderer is initialized (mainly globals)
        //     && !matches!(stage, ShaderStage::Geometry | ShaderStage::Compute); // Geometry shaders are broken, skip em
        let shader_module =
        //     = if recompile {
        //     ShaderModule::load_recompile(gpu, shader.shader, &dynamic_constants)
        // } else {
            ShaderModule::load(gpu, shader.shader)
        // }
        .with_context(|| format!("Failed to load shader module {}", shader.shader))?
        .with_name(&format!(
            "{} {} (Technique {})",
            stage.short_name(),
            shader.shader,
            technique_hash
        ));

        Ok(Some(Self {
            shader: shader.clone(),
            stage,

            dynamic_constants,
            shader_module,
        }))
    }

    #[profiling::function]
    pub fn bind(
        &self,
        cmd: &mut CommandList,
        channels: Option<&TempObjectChannels>,
    ) -> anyhow::Result<()> {
        self.shader_module.bind(cmd);
        self.dynamic_constants.bind(cmd, self.stage, channels)?;

        Ok(())
    }

    pub fn is_loaded(&self) -> bool {
        self.dynamic_constants
            .textures
            .iter()
            .all(|(_, tex)| tex.as_ref().map(Handle::is_loaded).unwrap_or(true))
    }
}

pub enum ShaderModule {
    Vertex(d3d11::VertexShader),
    Pixel(d3d11::PixelShader),
    Geometry(d3d11::GeometryShader),
    Hull(d3d11::HullShader),
    Domain(d3d11::DomainShader),
    Compute(d3d11::ComputeShader),
}

impl ShaderModule {
    pub fn bind(&self, cmd: &mut CommandList) {
        match self {
            ShaderModule::Vertex(shader) => cmd.vertex_set_shader(shader),
            ShaderModule::Pixel(shader) => cmd.pixel_set_shader(shader),
            ShaderModule::Geometry(shader) => cmd.geometry_set_shader(shader),
            ShaderModule::Hull(shader) => cmd.hull_set_shader(shader),
            ShaderModule::Domain(shader) => cmd.domain_set_shader(shader),
            ShaderModule::Compute(shader) => cmd.compute_set_shader(shader),
        }
    }

    pub fn with_name(self, name: &str) -> Self {
        self.set_name(name);
        self
    }

    pub fn set_name(&self, name: &str) {
        match self {
            ShaderModule::Vertex(shader) => shader.set_debug_name(name),
            ShaderModule::Pixel(shader) => shader.set_debug_name(name),
            ShaderModule::Geometry(shader) => shader.set_debug_name(name),
            ShaderModule::Hull(shader) => shader.set_debug_name(name),
            ShaderModule::Domain(shader) => shader.set_debug_name(name),
            ShaderModule::Compute(shader) => shader.set_debug_name(name),
        };
    }

    pub fn load(gpu: &Gpu, hash: TagHash) -> anyhow::Result<Self> {
        let entry = package_manager()
            .get_entry(hash)
            .context("Entry not found")?;
        ensure!(
            entry.file_type == 33 && entry.file_subtype <= 6,
            "Shader header type mismatch"
        );

        let data = package_manager()
            .read_tag(entry.reference)
            .context("Failed to read shader data")?;

        match entry.file_subtype {
            0 => Ok(ShaderModule::Pixel(gpu.create_pixel_shader(&data)?)),
            1 => Ok(ShaderModule::Vertex(gpu.create_vertex_shader(&data)?)),
            2 => Ok(ShaderModule::Geometry(gpu.create_geometry_shader(&data)?)),
            3..=5 => {
                anyhow::bail!("Unsupported shader type: {}", entry.file_subtype);
            }
            6 => Ok(ShaderModule::Compute(gpu.create_compute_shader(&data)?)),
            _ => unreachable!(),
        }
    }

    pub fn load_raw(gpu: &Gpu, data: &[u8], stage: ShaderStage) -> anyhow::Result<Self> {
        match stage {
            ShaderStage::Vertex => Ok(ShaderModule::Vertex(gpu.create_vertex_shader(data)?)),
            ShaderStage::Pixel => Ok(ShaderModule::Pixel(gpu.create_pixel_shader(data)?)),
            ShaderStage::Geometry => Ok(ShaderModule::Geometry(gpu.create_geometry_shader(data)?)),
            ShaderStage::Hull => Ok(ShaderModule::Hull(gpu.create_hull_shader(data)?)),
            ShaderStage::Domain => Ok(ShaderModule::Domain(gpu.create_domain_shader(data)?)),
            ShaderStage::Compute => Ok(ShaderModule::Compute(gpu.create_compute_shader(data)?)),
        }
    }

    pub fn compile(gpu: &Gpu, source: &str, stage: ShaderStage) -> anyhow::Result<Self> {
        let stage_d3d = match stage {
            ShaderStage::Pixel => d3d11::fxc::ShaderTarget::Pixel,
            ShaderStage::Vertex => d3d11::fxc::ShaderTarget::Vertex,
            ShaderStage::Geometry => d3d11::fxc::ShaderTarget::Geometry,
            ShaderStage::Hull => d3d11::fxc::ShaderTarget::Hull,
            ShaderStage::Compute => d3d11::fxc::ShaderTarget::Compute,
            ShaderStage::Domain => d3d11::fxc::ShaderTarget::Domain,
        };

        let bytecode = d3d11::fxc::compile(source.as_bytes(), None, &[], "main", stage_d3d)
            .context("Failed to compile shader")?;

        Self::load_raw(gpu, &bytecode, stage)
    }

    // pub fn load_recompile(
    //     gpu: &Gpu,
    //     hash: TagHash,
    //     dynamic_constants: &DynamicConstants,
    // ) -> anyhow::Result<Self> {
    //     if matches!(hash.0, 0x80B35874) {
    //         warn!("Shader {hash} can't be decompiled correctly, skipping");
    //         return Self::load(gpu, hash);
    //     }

    //     info!("Recompiling shader {hash} to include TFX information");

    //     let entry = package_manager()
    //         .get_entry(hash)
    //         .context("Entry not found")?;
    //     ensure!(
    //         entry.file_type == 33 && entry.file_subtype <= 6,
    //         "Shader header type mismatch"
    //     );

    //     let data_original = package_manager()
    //         .read_tag(entry.reference)
    //         .context("Failed to read shader data")?;

    //     let Ok(mut decompiled) = hlsldecompiler::decompile(&data_original) else {
    //         return Self::load(gpu, hash);
    //     };

    //     let mut texture_slots = vec![];

    //     for (t, _) in &dynamic_constants.textures {
    //         texture_slots.push((*t, format!("fixed_{t}")));
    //     }

    //     if let Ok(externs) = get_texture_externs_from_bytecode(&dynamic_constants.bytecode) {
    //         for (slot, extern_, offset) in externs {
    //             if let Some(fieldname) = Externs::get_extern_field_name(extern_, offset as usize) {
    //                 texture_slots.push((slot, format!("extern_{extern_:?}_{fieldname}")));
    //             } else {
    //                 texture_slots.push((slot, format!("extern_{extern_:?}_0x{offset:02X}")));
    //             }
    //         }
    //     }

    //     let mut pre_comments = String::new();

    //     let mut cb0 = vec![];
    //     for v in &dynamic_constants.initial_constants {
    //         cb0.push(format!("float4({}, {}, {}, {})", v.x, v.y, v.z, v.w));
    //     }

    //     if let Err(e) = expression_vm::decompiler::DecompilerState::new(&dynamic_constants.bytecode)
    //         .evaluate(&dynamic_constants.bytecode_constants, &mut cb0)
    //     {
    //         writeln!(&mut pre_comments, "// Error decompiling bytecode: {e}")?;
    //     }

    //     writeln!(&mut pre_comments, "// TFX data:")?;
    //     for (slot, val) in cb0.iter().enumerate() {
    //         writeln!(&mut pre_comments, "//     cb0[{slot}]: {val}")?;
    //     }
    //     writeln!(&mut pre_comments)?;

    //     decompiled.insert_str(0, &pre_comments);

    //     for (slot, name) in texture_slots {
    //         // Regex should match the slot number + whitespace or period
    //         let regex = Regex::new(&format!(r"\bt{slot}\s")).unwrap();
    //         decompiled = regex
    //             .replace_all(&decompiled, format!("{name} "))
    //             .to_string();

    //         let regex = Regex::new(&format!(r"\bt{slot}\.")).unwrap();
    //         decompiled = regex
    //             .replace_all(&decompiled, format!("{name}."))
    //             .to_string();
    //     }

    //     let stage = match entry.file_subtype {
    //         0 => ShaderStage::Pixel,
    //         1 => ShaderStage::Vertex,
    //         2 => ShaderStage::Geometry,
    //         3..=5 => {
    //             anyhow::bail!("Unsupported shader type: {}", entry.file_subtype);
    //         }
    //         6 => ShaderStage::Compute,
    //         _ => unreachable!(),
    //     };

    //     let compiled = match Self::compile(gpu, &decompiled, stage) {
    //         Ok(compiled) => compiled,
    //         Err(e) => {
    //             println!("Failed to compile shader: {e:?}");
    //             // Compile failed, load original shader from disk (this happens often with compute/geometry shaders)
    //             return Self::load(gpu, hash);
    //         }
    //     };

    //     Ok(compiled)
    // }
}

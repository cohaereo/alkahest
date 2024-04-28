//! ## Shader pre-compiler
//! This build script compiles all the shaders in the `shaders` directory into DXBC binaries.
//! Shaders are discovered by recursively searching the `shaders` directory for `.hlsl` files

use std::{
    alloc::{alloc_zeroed, dealloc, Layout},
    ffi::{c_void, CStr, CString},
    fs::File,
    io::Read,
    path::Path,
};

use windows::{
    core::{s, Error, PCSTR},
    Win32::{
        Foundation::E_FAIL,
        Graphics::Direct3D::{
            Fxc::D3DCompile, ID3DBlob, ID3DInclude, ID3DInclude_Impl, D3D_INCLUDE_TYPE,
            D3D_SHADER_MACRO,
        },
    },
};

#[derive(Clone, Copy)]
enum ShaderStage {
    Vertex,
    Pixel,
    Geometry,
    Compute,
}

impl ShaderStage {
    pub fn short(&self) -> &str {
        match self {
            ShaderStage::Vertex => "vs",
            ShaderStage::Pixel => "ps",
            ShaderStage::Geometry => "gs",
            ShaderStage::Compute => "cs",
        }
    }

    pub fn define(&self) -> PCSTR {
        match self {
            ShaderStage::Vertex => s!("STAGE_VS"),
            ShaderStage::Pixel => s!("STAGE_PS"),
            ShaderStage::Geometry => s!("STAGE_GS"),
            ShaderStage::Compute => s!("STAGE_CS"),
        }
    }

    pub fn target(&self) -> PCSTR {
        match self {
            ShaderStage::Vertex => s!("vs_5_0"),
            ShaderStage::Pixel => s!("ps_5_0"),
            ShaderStage::Geometry => s!("gs_5_0"),
            ShaderStage::Compute => s!("cs_5_0"),
        }
    }

    pub fn entry(&self) -> PCSTR {
        match self {
            ShaderStage::Vertex => s!("VSMain"),
            ShaderStage::Pixel => s!("PSMain"),
            ShaderStage::Geometry => s!("GSMain"),
            ShaderStage::Compute => s!("CSMain"),
        }
    }
}

fn compile_blob(filename: &str, source: &str, stage: ShaderStage) -> ID3DBlob {
    let mut shader_blob: Option<ID3DBlob> = None;
    let mut errors = None;

    let si = ShaderIncluder;
    let includer = ID3DInclude::new(&si);
    let result = unsafe {
        D3DCompile(
            source.as_ptr() as _,
            source.len(),
            None,
            Some(
                [
                    D3D_SHADER_MACRO {
                        Name: stage.define(),
                        Definition: PCSTR::null(),
                    },
                    D3D_SHADER_MACRO {
                        Name: PCSTR::null(),
                        Definition: PCSTR::null(),
                    },
                ]
                .as_ptr() as _,
            ),
            Some(&*includer),
            stage.entry(),
            stage.target(),
            0,
            0,
            &mut shader_blob,
            Some(&mut errors),
        )
    };

    let mut error_string = String::new();
    if let Some(errors) = errors {
        let estr = unsafe {
            let eptr = errors.GetBufferPointer();
            std::slice::from_raw_parts(eptr.cast(), errors.GetBufferSize())
        };
        let errors = String::from_utf8_lossy(estr);
        error_string = errors.to_string();
    }

    if result.is_err() {
        panic!("Failed to compile shader '{filename}': {error_string}");
    }

    if !error_string.is_empty() {
        eprintln!("Warnings: {error_string}");
    }

    shader_blob.unwrap()
}

fn build_stage(out_dir: &Path, filename: &Path, stage: ShaderStage) {
    let source = std::fs::read_to_string(filename).unwrap();
    if !source.contains(&unsafe { stage.entry().to_string().unwrap() }) {
        return;
    }

    let blob = compile_blob(filename.to_string_lossy().as_ref(), &source, stage);

    let output = out_dir.join(filename.with_extension(format!("hlsl.{}.dxbc", stage.short())));
    let vs_blob = unsafe {
        std::slice::from_raw_parts(blob.GetBufferPointer() as *const u8, blob.GetBufferSize())
    };

    println!("Writing to {:?}", output);
    let directory = output.parent().unwrap();
    std::fs::create_dir_all(directory).unwrap();

    std::fs::write(&output, vs_blob).unwrap();
}

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_dir = std::path::Path::new(&out_dir);

    let mut shader_files = vec![];

    println!("cargo:rerun-if-changed=assets/shaders");

    for entry in glob::glob("assets/shaders/**/*.hlsl").unwrap() {
        let entry = entry.unwrap();
        shader_files.push(entry.to_string_lossy().to_string());
    }

    for shader in shader_files.iter() {
        println!("cargo:rerun-if-changed={}", shader);

        build_stage(out_dir, Path::new(shader), ShaderStage::Vertex);
        build_stage(out_dir, Path::new(shader), ShaderStage::Pixel);
    }
}

pub struct ShaderIncluder;

impl ShaderIncluder {
    const BUFFER_SIZE: usize = 4 * 1024 * 1024;
    const MEM_LAYOUT: Layout = Layout::new::<[u8; Self::BUFFER_SIZE]>();
}

#[allow(non_snake_case, clippy::not_unsafe_ptr_arg_deref)]
impl ID3DInclude_Impl for ShaderIncluder {
    fn Open(
        &self,
        _includetype: D3D_INCLUDE_TYPE,
        pfilename: &::windows::core::PCSTR,
        _pparentdata: *const c_void,
        ppdata: *mut *mut c_void,
        pbytes: *mut u32,
    ) -> ::windows::core::Result<()> {
        // TODO(cohae): Local includes
        // if includetype == D3D_INCLUDE_LOCAL {
        let filename = unsafe { pfilename.to_string() }.unwrap_or_default();
        let mut path = std::path::PathBuf::from("assets/shaders/include/");
        path.push(&filename);
        println!("cargo:rerun-if-changed={}", path.to_string_lossy());

        let data_result = File::open(path.as_path());
        match data_result {
            Ok(mut file) => {
                let file_size = file.metadata().unwrap().len();
                if file_size > Self::BUFFER_SIZE as u64 {
                    return Err(Error::new(E_FAIL, "File size too large"));
                }

                let ptr = unsafe { alloc_zeroed(Self::MEM_LAYOUT) };
                let slice = unsafe { std::slice::from_raw_parts_mut(ptr, Self::BUFFER_SIZE) };
                let result = file.read(slice);

                match result {
                    Ok(_) => {
                        unsafe {
                            *ppdata = ptr as *mut c_void;
                            *pbytes = file_size as u32;
                        }
                        Ok(())
                    }
                    Err(_) => Err(Error::new(E_FAIL, "Failed to read from file")),
                }
            }
            Err(_error) => Err(Error::new(E_FAIL, "Failed to open file")),
        }
        // } else {
        //     Err(Error::new(E_NOTIMPL, "Unsupported include type"))
        // }
    }

    fn Close(&self, pdata: *const c_void) -> ::windows::core::Result<()> {
        unsafe { dealloc(pdata as *mut u8, Self::MEM_LAYOUT) };
        Ok(())
    }
}

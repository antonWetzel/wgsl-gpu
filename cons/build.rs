use cargo_gpu::install::Install;
use cargo_gpu::spirv_builder::{ShaderPanicStrategy, SpirvMetadata};
use naga::valid::{Capabilities, ValidationFlags};
use std::path::PathBuf;

pub fn main() -> anyhow::Result<()> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let crate_path = [manifest_dir, "..", "shaders"]
        .iter()
        .copied()
        .collect::<PathBuf>();

    let install = Install::from_shader_crate(crate_path.clone());
    // install.rebuild_codegen = true;
    let backend = install.run()?;
    let mut builder = backend.to_spirv_builder(crate_path, "spirv-unknown-vulkan1.3");
    builder.build_script.defaults = true;
    builder.shader_panic_strategy = ShaderPanicStrategy::SilentExit;
    builder.spirv_metadata = SpirvMetadata::Full;

    let compile_result = builder.build()?;
    let spv_path = compile_result.module.unwrap_single();
    println!("cargo::rustc-env=SHADER_SPV_PATH={}", spv_path.display());

    // let data = include_str!("/home/antonwetzel/Documents/rust-v/example.wgsl");
    // let module = naga::front::wgsl::parse_str(data).unwrap();

    // let info = naga::valid::Validator::new(ValidationFlags::default(), Capabilities::default())
    //     .subgroup_stages(naga::valid::ShaderStages::empty())
    //     .subgroup_operations(naga::valid::SubgroupOperationSet::all())
    //     .validate(&module)
    //     .unwrap();

    // let data =
    //     naga::back::spv::write_vec(&module, &info, &naga::back::spv::Options::default(), None)
    //         .unwrap();
    // std::fs::write("test.spirv", bytemuck::cast_slice(&data)).unwrap();

    let data = std::fs::read(spv_path).unwrap();
    let module = naga::front::spv::parse_u8_slice(
        bytemuck::cast_slice(&data),
        &naga::front::spv::Options {
            adjust_coordinate_space: false,
            strict_capabilities: false,
            block_ctx_dump_prefix: None,
        },
    )
    .unwrap();

    let info = naga::valid::Validator::new(ValidationFlags::default(), Capabilities::default())
        .subgroup_stages(naga::valid::ShaderStages::empty())
        .subgroup_operations(naga::valid::SubgroupOperationSet::all())
        .validate(&module)
        .unwrap();

    let source =
        naga::back::wgsl::write_string(&module, &info, naga::back::wgsl::WriterFlags::empty())
            .unwrap();

    Ok(())
}

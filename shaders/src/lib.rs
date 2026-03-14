#![no_std]

use bytemuck::{Pod, Zeroable};
use core::f32::consts::PI;
use glam::{Vec3, Vec4, vec2, vec3};
#[cfg(target_arch = "spirv")]
use spirv_std::num_traits::Float;
use spirv_std::spirv;

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct ShaderConstants {
    pub width: u32,
    pub height: u32,
    pub time: f32,
}

pub fn main_vs(
    vert_id: u32,
    constants: &ShaderConstants,
    instance: InstanceValues,
) -> VertexOutput {
    let speed = 0.4;
    let time = constants.time * speed + vert_id as f32 * (2. * PI * 120. / 360.);
    let position = vec2(f32::sin(time), f32::cos(time));

    let scale = instance.a * instance.b * instance.c;

    VertexOutput {
        vtx_pos: Vec4::from((position, 0.0, 1.0)),
        vtx_color: [vec3(1., 0., 0.), vec3(0., 1., 0.), vec3(0., 0., 1.)][vert_id as usize % 3]
            * scale,
    }
}

#[derive(Debug, wgsl_gpu::WgslGpuArguments)]
pub struct InstanceValues {
    #[wgsl_gpu(location = 3)]
    pub a: f32,
    #[wgsl_gpu(location = 4)]
    pub b: f32,
    #[wgsl_gpu(location = 5)]
    pub c: f32,
}

#[derive(Debug, wgsl_gpu::WgslGpuArguments)]
pub struct VertexOutput {
    #[wgsl_gpu(position)] // as output
    #[wgsl_gpu(frag_coord)] // as input
    vtx_pos: Vec4,
    #[wgsl_gpu(location = 0)]
    vtx_color: Vec3,
}

wgsl_gpu::create_wrapper_function!(
    (#[spirv(vertex)] pub fn main_vs_gpu), main_vs,
    (
        wgsl_gpu_InstanceValues_transform,
        wgsl_gpu::arg_identity_transform,
    ),
    wgsl_gpu_VertexOutput_transform,
    (
        (#[spirv(vertex_index)] vert_id: u32),
        (#[spirv(descriptor_set = 0, binding = 0, storage_buffer)] constants: &ShaderConstants),
        (instance: InstanceValues),
    ),
);

pub fn main_fs(input: VertexOutput) -> FragmentOutput {
    FragmentOutput {
        color: Vec4::from((input.vtx_color, 1.0)) * input.vtx_pos,
    }
}

#[derive(Debug, wgsl_gpu::WgslGpuArguments)]
pub struct FragmentOutput {
    #[wgsl_gpu(location = 0)]
    pub color: Vec4,
}

wgsl_gpu::create_wrapper_function!(
    (#[spirv(fragment)] pub fn main_fs_gpu), main_fs,
    (
        wgsl_gpu_VertexOutput_transform,
    ),
    wgsl_gpu_FragmentOutput_transform,
    (
        (input: VertexOutput),
    ),
);

// todo: macro to create wrapper macro invocation from function

#![no_std]

use bytemuck::{Pod, Zeroable};
use core::f32::consts::PI;
use glam::{Vec2, Vec3, Vec4};
#[cfg(target_arch = "spirv")]
use spirv_std::num_traits::Float;

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct ShaderConstants {
    pub width: u32,
    pub height: u32,
    pub time: f32,
}

#[derive(Debug, wgsl_gpu::Arguments)]
pub struct InstanceValues {
    #[wgsl_gpu(location = 3)]
    pub a: f32,
    #[wgsl_gpu(location = 4)]
    pub b: f32,
    #[wgsl_gpu(location = 5)]
    pub c: f32,
}

#[derive(Debug, wgsl_gpu::Arguments)]
pub struct VertexOutput {
    #[wgsl_gpu(position)] // as output
    #[wgsl_gpu(frag_coord)] // as input
    vtx_pos: Vec4,
    #[wgsl_gpu(location = 0)]
    vtx_color: Vec3,
}

#[wgsl_gpu::entry]
#[spirv(vertex)]
pub fn main_vs(
    #[spirv(vertex_index)] vert_id: u32,
    #[spirv(descriptor_set = 0, binding = 0, storage_buffer)] constants: &ShaderConstants,
    #[wgsl_gpu(arguments)] instance: InstanceValues,
) -> VertexOutput {
    let speed = 0.4;
    let time = constants.time * speed + vert_id as f32 * (2. * PI * 120. / 360.);
    let position = Vec2::new(f32::sin(time), f32::cos(time));

    let scale = instance.a * instance.b * instance.c;

    VertexOutput {
        vtx_pos: Vec4::from((position, 0.0, 1.0)),
        vtx_color: [Vec3::X, Vec3::Y, Vec3::Z][vert_id as usize % 3] * scale,
    }
}

#[wgsl_gpu::entry]
#[spirv(fragment)]
pub fn main_fs(#[wgsl_gpu(arguments)] input: VertexOutput) -> shaders_dep::FragmentOutput {
    shaders_dep::FragmentOutput {
        color: Vec4::from((input.vtx_color, 1.0)) * input.vtx_pos,
    }
}

#![cfg_attr(not(feature = "native"), no_std)]

use bytemuck::{Pod, Zeroable};
use core::f32::consts::PI;
use glam::{Vec2, Vec3, Vec4};
#[cfg(target_arch = "spirv")]
use spirv_std::num_traits::Float;
use spirv_std::{Image, Sampler, image::ImageWithMethods};

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct ShaderUniform {
    pub speed: f32,
    pub time: f32,
    pub color_scale: f32,
}

#[derive(Debug, Copy, Clone, Pod, Zeroable, wgsl_gpu::Arguments)]
#[repr(C)]
#[wgsl_gpu(attributes)]
pub struct Vertex {
    #[wgsl_gpu(location = 0)]
    pub position: Vec2,
}

#[derive(Debug, Copy, Clone, Pod, Zeroable, wgsl_gpu::Arguments)]
#[repr(C)]
#[wgsl_gpu(attributes)]
pub struct Instance {
    #[wgsl_gpu(location = 1)]
    pub color: Vec3,
    #[wgsl_gpu(location = 2)]
    pub offset: f32,
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
    // #[spirv(vertex_index)] vert_id: u32,
    #[spirv(descriptor_set = 0, binding = 0, uniform)] uniform: &ShaderUniform,

    #[wgsl_gpu(arguments, step_mode = Vertex)] vertex: Vertex,
    #[wgsl_gpu(arguments, step_mode = Instance)] instance: Instance,
) -> VertexOutput {
    let angle = uniform.time * uniform.speed + instance.offset;
    let position = glam::Mat2::from_angle(angle).mul_vec2(vertex.position);

    VertexOutput {
        vtx_pos: Vec4::from((position, 0.0, 1.0)),
        vtx_color: instance.color,
    }
}

#[wgsl_gpu::entry]
#[spirv(fragment)]
pub fn main_fs(
    #[spirv(descriptor_set = 0, binding = 0, uniform)] uniform: &ShaderUniform,
    #[spirv(descriptor_set = 1, binding = 0)] image: &Image!(2D, type=f32, sampled),
    #[spirv(descriptor_set = 1, binding = 1)] sampler: &Sampler,
    #[wgsl_gpu(arguments)] input: VertexOutput,
) -> shaders_dep::FragmentOutput {
    let color = image.sample(*sampler, Vec2::new(0.0, 0.0));
    shaders_dep::FragmentOutput {
        color: Vec4::from((input.vtx_color * uniform.color_scale, 1.0)) * color,
    }
}

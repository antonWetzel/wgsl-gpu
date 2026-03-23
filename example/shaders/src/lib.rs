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

// todo: generate this macro from the function signature
// macro data arguments
// - total set count
// - total bindings per set
// - edits to the bindings per set
#[macro_export]
macro_rules! wgsl_gpu_main_vs_bind_groups_macro {
    ($target:path, $context:tt, $entry:ident) => {
        $target!(
            $context,
            (
                1,
                [1, 0],
                [
                    {
                        $entry[0].binding = 0;
                        $entry[0].visibility =
                            $entry[0].visibility.union(wgpu::ShaderStages::VERTEX);
                        $entry[0].ty = wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        };
                    },
                    {}
                ]
            )
        );
    };
}

#[macro_export]
macro_rules! wgsl_gpu_main_fs_bind_groups_macro {
    ($target:path, $context:tt, $entry:ident) => {
        $target!(
            $context,
            (
                2,
                [1, 2],
                [
                    {
                        $entry[0].binding = 0;
                        $entry[0].visibility =
                            $entry[0].visibility.union(wgpu::ShaderStages::FRAGMENT);
                        $entry[0].ty = wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        };
                    },
                    {
                        $entry[0].binding = 0;
                        $entry[0].visibility =
                            $entry[0].visibility.union(wgpu::ShaderStages::FRAGMENT);
                        $entry[0].ty = wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        };

                        $entry[1].binding = 1;
                        $entry[1].visibility =
                            $entry[1].visibility.union(wgpu::ShaderStages::FRAGMENT);
                        $entry[1].ty =
                            wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering);
                    }
                ]
            )
        );
    };
}

// todo: write a proc macro to generate this macro call
// proc macro can map function names to macro names
wgsl_gpu::__pipeline_bind_groups!(
    MAIN_BIND_GROUPS,
    wgsl_gpu_main_vs_bind_groups_macro,
    wgsl_gpu_main_fs_bind_groups_macro
);

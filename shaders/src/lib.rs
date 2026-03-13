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

#[spirv(fragment)]
pub fn main_fs(vtx_color: Vec3, output: &mut Vec4) {
    *output = Vec4::from((vtx_color, 1.));
}

#[spirv(vertex)]
pub fn main_vs(
    #[spirv(vertex_index)] vert_id: u32,
    #[spirv(descriptor_set = 0, binding = 0, storage_buffer)] constants: &ShaderConstants,
    #[spirv(position)] vtx_pos: &mut Vec4,
    // #[spirv(uniform_constant)] instance: &InstanceValues,
    vtx_color: &mut Vec3,
) {
    let speed = 0.4;
    let time = constants.time * speed + vert_id as f32 * (2. * PI * 120. / 360.);
    let position = vec2(f32::sin(time), f32::cos(time));
    *vtx_pos = Vec4::from((position, 0.0, 1.0));

    let scale = 1.0;
    *vtx_color =
        [vec3(1., 0., 0.), vec3(0., 1., 0.), vec3(0., 0., 1.)][vert_id as usize % 3] * scale;
}

pub struct InstanceValues {
    pub temp_a: f32,
    pub b: f32,
    pub c: f32,
}

impl InstanceValues {
    pub fn from_wgsl_gpu(temp_a: f32, b: f32, c: f32) -> Self {
        Self { temp_a, b, c }
    }
}

macro_rules! make_call {
    ($wrap_name:ident, $name:ident, ($($args:tt)*), ($($params:tt)*)) => {
        pub fn $wrap_name($($args)*) {
            $name($($params)*)
        }
    };
    ($f: expr, ($($args:tt)*) I $($params:tt)*) => {
        make_call!($f, ($($args)* 1,) $($params)*)
    };
}

macro_rules! transform_arg_a {
    ($target:ident, ($($args:tt)*), ()) => {
        $target!($($args)*)
    };
    // ($target:ident, ($($args:tt)*), ($($args_s:tt)*)) => {
    //     $target!($($args)*)
    // };
    ($target:ident, ($($args:tt)*), ($arg_s:tt $($args_s:tt)*)) => {
        transform_arg_a!($target, ($($args)* $arg_s), ($($args_s)*))
    };
}

fn test() {
    transform_arg_a!(panic, (), ("{}", 0));
}

// todo:
// - declerate rust struct similar to wgsl with locations
// - some way to use the struct as mut argument
//     - tt muncher
//     - manuel args + macro to construct at first line + some drop write
// - some way to use the struct as value arugment
//     - tt muncher
//     - manuel args + macro to construct at first line
//

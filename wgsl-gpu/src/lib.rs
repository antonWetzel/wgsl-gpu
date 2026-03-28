#![cfg_attr(target_arch = "spirv", no_std)]

pub use wgsl_gpu_macros::*;

#[macro_export]
macro_rules! create_wrapper_function {
    // keep the argument as is, continue with the next argument
    (
        __param,
        $wrapper_name:tt, $name:ident, $ret_macro:path,
        ($($args:tt)*), ($($params:tt)*),
        (__keep => ($(#[$arg_attr:meta])* $arg_name:ident: $arg_ty:ty), $($arg_tail:tt)*),
    ) => {
        $crate::create_wrapper_function!(
            __param,
            $wrapper_name, $name, $ret_macro,
            ($($args)* $(#[$arg_attr])* $arg_name: $arg_ty,), ($($params)* $arg_name,),
            ($($arg_tail)*),
        );
    };

    // transform the argument
    (
        __param,
        $wrapper_name:tt, $name:ident, $ret_macro:path,
        $args:tt, $params:tt,
        (__expand $macro:ident => $arg:tt, $($arg_tail:tt)*),
    ) => {
        $macro!(
            __arg,
            $crate::create_wrapper_function,
            (__param_ret, $wrapper_name, $name, $ret_macro, $args, $params, ($($arg_tail)*))
        );
    };

    // return from the arg tranformation
    // extend the changed args and params
    // continue with next argument
    (
        (
            __param_ret,
            $wrapper_name:tt, $name:ident, $ret_macro:path,
            ($($args:tt)*), ($($params:tt)*), $arg_tail:tt
        ),
        ($($arg:tt)*), ($($param:tt)*),
    ) => {
        $crate::create_wrapper_function!(
            __param,
            $wrapper_name, $name, $ret_macro,
            ($($args)* $($arg)*), ($($params)* $($param)*),
            $arg_tail,
        );
    };

    // all arguments transformed, add return type
    (
        __param,
        $wrapper_name:tt, $name:ident, $ret_macro:path,
        $args:tt, $params:tt,
        (),
    ) => {
        $ret_macro!(
            __ret, $crate::create_wrapper_function,
            (__ret, $wrapper_name, $name, $args, $params)
        );
    };

    // all arguments transformed, create function
    (
        (__ret, ($($wrapper_name:tt)*), $name:ident, ($($args:tt)*), $params:tt),
        ($($ret_args:tt)*), $output:ident, ($($output_set:stmt;)*),
    ) => {
    	#[allow(clippy::too_many_arguments)]
    	$($wrapper_name)* ($($args)* $($ret_args)*) {
            let $output = $name $params;
            $($output_set;)*
        }
    };

    // entry point with somewhat sane syntax
    (
        $wrapper_name:tt, $name:ident, $ret_macro:path, $args:tt,
    ) => {
        #[spirv_std::macros::spirv_recursive_for_testing]
        $crate::create_wrapper_function!(
            __param,
            $wrapper_name, $name, $ret_macro,
            (), (),
            $args,
        );
    };
}

pub trait Arguments {
	type Arguments;

	fn from_arguments(arguments: Self::Arguments) -> Self;
}

#[cfg(not(target_arch = "spirv"))]
#[doc(hidden)]
pub const fn __const_slice<const N: usize, T>(array: &[T; N], len: usize) -> &[T] {
	assert!(len <= N);
	unsafe { std::slice::from_raw_parts(array.as_ptr(), len) }
}

#[cfg(not(target_arch = "spirv"))]
#[doc(hidden)]
pub const fn __const_max(a: usize, b: usize) -> usize {
	if a > b { a } else { b }
}

#[macro_export]
#[doc(hidden)]
macro_rules! __pipeline_bind_groups {
    ($name:ident, $vertex:path, $fragment:path) => {
        $vertex!(
            $crate::__pipeline_bind_groups,
            (__expand_vertex, $name, entry, $fragment),
            entry
        );
    };

    ((__expand_vertex, $name:ident, $entry:ident, $fragment:path), $vertex:tt) => {
        $fragment!(
            $crate::__pipeline_bind_groups,
            (__expand_fragment, $name, $entry, $vertex),
            $entry
        );
    };

    ((__expand_fragment, $name:ident, $entry:ident, $vertex:tt), $fragment:tt) => {
        $crate::__pipeline_bind_groups!(__internal, $name, $entry, [0, 1, 2, 3, 4, 5, 6, 7], $vertex, $fragment);
    };

    (
        __internal, $name:ident, $entry:ident,
        [$($i_edits:expr),*],
        ($v_size:expr, [$($v_sizes:expr,)*], [$($v_edits:tt,)*]),
        ($f_size:expr, [$($f_sizes:expr,)*], [$($f_edits:tt,)*])
    ) => {
        #[cfg(not(target_arch = "spirv"))]
        pub const $name: &[&[wgpu::BindGroupLayoutEntry]] = {
        	pub const MAX_SETS: usize = 8;
        	pub const MAX_BINDINGS: usize = 8;
            pub const SIZE: usize = wgsl_gpu::__const_max($v_size, $f_size);

            pub const BIND_GROUPS_ENTRIES: [[wgpu::BindGroupLayoutEntry; MAX_BINDINGS]; MAX_SETS] = const {
                let mut entries = [[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::empty(),
                    ty: wgpu::BindingType::ExternalTexture,
                    count: None,
                }; MAX_BINDINGS]; MAX_SETS];

                $(
                	#[allow(unused_mut, unused)]
                    let mut $entry = &mut entries[$i_edits];
                    $v_edits;
                    $f_edits;
                )*

                entries
            };

            pub const BIND_GROUPS_REFS: [&[wgpu::BindGroupLayoutEntry]; MAX_SETS] = [
                $(
                    wgsl_gpu::__const_slice(&BIND_GROUPS_ENTRIES[$i_edits], wgsl_gpu::__const_max($v_sizes, $f_sizes)),
                )*
            ];
            wgsl_gpu::__const_slice(&BIND_GROUPS_REFS, SIZE)
        };


    };
}

#![no_std]

pub use wgsl_gpu_macros::{Arguments, entry};

#[macro_export]
macro_rules! create_wrapper_function {
    // send an argument to the macro chain for transformation
    (
        __param,
        $wrapper_name:tt, $name:ident, ($macro:path, $($macro_tail:tt)*), $ret_macro:path,
        $args:tt, $params:tt,
        ($arg:tt, $($arg_tail:tt)*),
    ) => {
        $macro!(
            $crate::create_wrapper_function,
            (__param_ret, $wrapper_name, $name, ($macro, $($macro_tail)*), $ret_macro, $args, $params, ($($arg_tail)*)),
            ($($macro_tail)*),
            $arg
        );
    };

    // return from the arg tranformation
    // extend the changed args and params
    // continue with next argument
    (
        __param_ret,
        $wrapper_name:tt, $name:ident, $macros:tt, $ret_macro:path,
        ($($args:tt)*), ($($params:tt)*), $arg_tail:tt,
        ($($arg:tt)*), ($($param:tt)*),
    ) => {
        $crate::create_wrapper_function!(
            __param,
            $wrapper_name, $name, $macros, $ret_macro,
            ($($args)* $($arg)*,), ($($params)* $($param)*,),
            $arg_tail,
        );
    };

    // all arguments transformed, add return type
    (
        __param,
        $wrapper_name:tt, $name:ident, $macros:tt, $ret_macro:path,
        $args:tt, $params:tt,
        (),
    ) => {
        $ret_macro!(
            $crate::create_wrapper_function,
            (__ret, $wrapper_name, $name, $args, $params)
        );
    };

    // all arguments transformed, create function
    (
        __ret,
        ($($wrapper_name:tt)*), $name:ident,
        ($($args:tt)*), $params:tt,
        ($($ret_args:tt)*), $output:ident, ($($output_set:stmt;)*)
    ) => {
        $($wrapper_name)* ($($args)* $($ret_args)*) {
            let $output = $name $params;
            $($output_set)*
        }
    };

    // entry point with somewhat sane syntax
    (
        $wrapper_name:tt, $name:ident, ($($macros:path,)*), $ret_macro:path, $args:tt,
    ) => {
        #[spirv_std::macros::spirv_recursive_for_testing]
        $crate::create_wrapper_function!(
            __param,
            $wrapper_name, $name, ($($macros,)* $crate::arg_identity_transform,), $ret_macro,
            (), (),
            $args,
        );
    };
}

// base case for transformation macros
#[macro_export]
macro_rules! arg_identity_transform {
    ($target:path, ($($context:tt)*), (), ($(#[$attr:meta])* $name:ident: $ty:ty)) => {
        $target!($($context)*, ($(#[$attr])* $name: $ty), ($name),);
    };
}

pub trait Arguments {
    type Arguments;

    fn from_arguments(arguments: Self::Arguments) -> Self;
}

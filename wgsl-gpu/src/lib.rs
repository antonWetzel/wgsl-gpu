#![no_std]

pub use wgsl_gpu_macros::{Arguments, entry};

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
            ($($args)* $($arg)*,), ($($params)* $($param)*,),
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
        ($($ret_args:tt)*), $output:ident, ($($output_set:stmt;)*)
    ) => {
        $($wrapper_name)* ($($args)* $($ret_args)*) {
            let $output = $name $params;
            $($output_set)*
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

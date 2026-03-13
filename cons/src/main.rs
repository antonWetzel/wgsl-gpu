fn main() {
    test_f_w(1.0, 2.0, 3.0, -1, 4.0, 5.0, false);
}

pub trait FromArguments {
    type Arguments;

    fn from_arguments(arguments: Self::Arguments) -> Self;
}

#[derive(Debug)]
pub struct InstanceValues {
    pub a: f32,
    pub b: f32,
    pub c: f32,
}

impl FromArguments for InstanceValues {
    type Arguments = (f32, f32, f32);

    fn from_arguments(arguments: Self::Arguments) -> Self {
        Self {
            a: arguments.0,
            b: arguments.1,
            c: arguments.2,
        }
    }
}

macro_rules! instances_transform_arg {
    // arg matches, transform and return to main macro
    ($target:ident, ($($context:tt)*), $macros:tt, ($name:ident: InstanceValues)) => {
        $target!($($context)*, (a: f32, b: f32, c: f32), (FromArguments::from_arguments((a, b, c))),);
    };
    // no match, contiue with other tranform macros
    ($target:ident, $context:tt, ($macro:ident, $($macro_tail:tt)*), $arg:tt) => {
        $macro!($target, $context, ($($macro_tail)*), $arg);
    };
}

// base case for transformation macros
macro_rules! identity_transform_arg {
    ($target:ident, ($($context:tt)*), (), ($name:ident: $ty:ty)) => {
        $target!($($context)*, ($name: $ty), ($name),);
    };
}

#[derive(Debug)]
pub struct VertexValues {
    pub x: f32,
    pub y: f32,
}

impl FromArguments for VertexValues {
    type Arguments = (f32, f32);

    fn from_arguments(arguments: Self::Arguments) -> Self {
        Self {
            x: arguments.0,
            y: arguments.1,
        }
    }
}

macro_rules! vertex_transform_arg {
    ($target:ident, ($($context:tt)*), $macros:tt, ($name:ident: VertexValues)) => {
        $target!($($context)*, (x: f32, y: f32), (FromArguments::from_arguments((x, y))),);
    };
    ($target:ident, $context:tt, ($macro:ident, $($macro_tail:tt)*), $arg:tt) => {
        $macro!($target, $context, ($($macro_tail)*), $arg);
    };
}

fn test_f(instance: InstanceValues, cool: i32, vertex: VertexValues, wow: bool) {
    println!("{:?} {:?} {:?} {:?}", instance, cool, vertex, wow);
}

macro_rules! create_wrapper_function {
    // send an argument to the macro chain for transformation
    (
        __param,
        $wrapper_name:ident, $name:ident, ($macro:ident, $($macro_tail:tt)*),
        $args:tt, $params:tt,
        ($arg:tt, $($arg_tail:tt)*),
    ) => {
        $macro!(
            create_wrapper_function,
            (__param_ret, $wrapper_name, $name, ($macro, $($macro_tail)*), $args, $params, ($($arg_tail)*)),
            ($($macro_tail)*),
            $arg
        );
    };

    // return from the arg tranformation
    // extend the changed args and params
    // continue with next argument
    (
        __param_ret,
        $wrapper_name:ident, $name:ident, $macros:tt,
        ($($args:tt)*), ($($params:tt)*), $arg_tail:tt,
        ($($arg:tt)*), ($($param:tt)*),
    ) => {
        create_wrapper_function!(
            __param,
            $wrapper_name, $name, $macros,
            ($($args)* $($arg)*,), ($($params)* $($param)*,),
            $arg_tail,
        );
    };

    // all arguments transformed, create function
    (
        __param,
        $wrapper_name:ident, $name:ident, $macros:tt,
        $args:tt, $params:tt,
        (),
    ) => {
        fn $wrapper_name $args {
            $name $params;
        }
    };

    // entry point with somewhat sane syntax
    (
        $wrapper_name:ident, $name:ident, $macros:tt, $args:tt,
    ) => {
        create_wrapper_function!(
            __param,
            $wrapper_name, $name, $macros,
            (), (),
            $args,
        );
    };
}

create_wrapper_function!(
    test_f_w, test_f,
    (
        instances_transform_arg,
        vertex_transform_arg,
        identity_transform_arg,
    ),
    (
        (instance: InstanceValues),
        (cool: i32),
        (vertex: VertexValues),
        (wow: bool),
    ),
);

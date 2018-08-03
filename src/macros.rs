#[macro_export]
macro_rules! routes {
    (@method $p:ident $m:ident $h:expr) => {
        $crate::$m($h)
    };
    (@path ["/"] => |$p:ident| { $fm:ident $fh:expr; $($m:ident $h:expr;)*}) => {
        {
            let $p = $crate::path::index();
            routes!(@method $p $fm $fh)
            $(
            .or(routes!(@method $p $m $h))
            )*
        }
    };
    (@path [$($path:tt)*] => |$p:ident| { $fm:ident $fh:expr; $($m:ident $h:expr;)*}) => {
        {
            let $p = path!($($path)*).and($crate::path::index());
            routes!(@method $p $fm $fh)
            $(
            .or(routes!(@method $p $m $h))
            )*
        }
    };
    ([$($path:tt)*] => |$p:ident| { $fm:ident $fh:expr; $($m:ident $h:expr;)*}
     $([$($tpath:tt)*] => |$tp:ident| { $tfm:ident $tfh:expr; $($tm:ident $th:expr;)*})*) => {
        routes!(@path [$($path)*] => |$p| { $fm $fh; $($m $h;)* })
        $(
        .or(routes!(@path [$($tpath)*] => |$tp| { $tfm $tfh; $($tm $th;)* }))
        )*
    }
}

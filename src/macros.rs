// Convenience for writing to stderr thanks to https://github.com/BurntSushi
macro_rules! wlnerr(
    ($($arg:tt)*) => ({
        use std::io::{Write, stderr};
        writeln!(&mut stderr(), $($arg)*).ok();
    })
);

macro_rules! werr(
    ($($arg:tt)*) => ({
        use std::io::{Write, stderr};
        write!(&mut stderr(), $($arg)*).ok();
    })
);

macro_rules! regex(
    ($s:expr) => (::regex::Regex::new($s).unwrap());
);

#[cfg(feature = "debug")]
macro_rules! debugln {
    ($fmt:expr) => (println!(concat!("**DEBUG** ", $fmt)));
    ($fmt:expr, $($arg:tt)*) => (println!(concat!("**DEBUG** ",$fmt), $($arg)*));
}

#[cfg(feature = "debug")]
macro_rules! debug {
    ($fmt:expr) => (print!(concat!("**DEBUG** ", $fmt)));
    ($fmt:expr, $($arg:tt)*) => (println!(concat!("**DEBUG** ",$fmt), $($arg)*));
}

#[cfg(not(feature = "debug"))]
macro_rules! debugln {
    ($fmt:expr) => {};
    ($fmt:expr, $($arg:tt)*) => {};
}

#[cfg(not(feature = "debug"))]
macro_rules! debug {
    ($fmt:expr) => {};
    ($fmt:expr, $($arg:tt)*) => {};
}

/// Convenience macro taken from https://github.com/kbknapp/clap-rs to generate more complete enums
/// with variants to be used as a type when parsing arguments. This enum also provides a
/// `variants()` function which can be used to retrieve a `Vec<&'static str>` of the variant names.
///
/// **NOTE:** Case insensitivity is supported for ASCII characters
///
/// **NOTE:** This macro automaically implements std::str::FromStr and std::fmt::Display
///
/// These enums support pub (or not) and use of the #[derive()] traits
///
///
/// # Example
///
/// ```no_run
/// clog_enum!{
///     #[derive(Debug)]
///     pub enum Foo {
///         Bar,
///         Baz,
///         Qux
///     }
/// }
/// ```
macro_rules! clog_enum {
    ($(#[$meta:meta])* enum $e:ident { $($v:ident),+ } ) => {
        $(#[$meta])*
        enum $e {
            $($v),+
        }

        impl ::std::str::FromStr for $e {
            type Err = String;

            fn from_str(s: &str) -> Result<Self,Self::Err> {
                match s {
                    $(stringify!($v) |
                    _ if s.eq_ignore_ascii_case(stringify!($v)) => Ok($e::$v),)+
                    _                => Err({
                                            let v = vec![
                                                $(stringify!($v),)+
                                            ];
                                            format!("valid values:{}",
                                                v.iter().fold(String::new(), |a, i| {
                                                    a + &format!(" {}", i)[..]
                                                }))
                                        })
                }
            }
        }

        impl ::std::fmt::Display for $e {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                match *self {
                    $($e::$v => write!(f, stringify!($v)),)+
                }
            }
        }

        impl $e {
            #[allow(dead_code)]
            fn variants() -> Vec<&'static str> {
                vec![
                    $(stringify!($v),)+
                ]
            }
        }
    };
    ($(#[$meta:meta])* pub enum $e:ident { $($v:ident),+ } ) => {
        $(#[$meta])*
        pub enum $e {
            $($v),+
        }

        impl ::std::str::FromStr for $e {
            type Err = String;

            fn from_str(s: &str) -> Result<Self,Self::Err> {
                match s {
                    $(stringify!($v) |
                    _ if s.eq_ignore_ascii_case(stringify!($v)) => Ok($e::$v),)+
                    _                => Err({
                                            let v = vec![
                                                $(stringify!($v),)+
                                            ];
                                            format!("valid values:{}",
                                                v.iter().fold(String::new(), |a, i| {
                                                    a + &format!(" {}", i)[..]
                                                }))
                                        })
                }
            }
        }

        impl ::std::fmt::Display for $e {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                match *self {
                    $($e::$v => write!(f, stringify!($v)),)+
                }
            }
        }

        impl $e {
            #[allow(dead_code)]
            pub fn variants() -> Vec<&'static str> {
                vec![
                    $(stringify!($v),)+
                ]
            }
        }
    };
}

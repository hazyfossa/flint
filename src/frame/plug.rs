pub trait Tagged {
    const TAG: &'static str;
}

pub trait FromConfig: Sized {
    type Config;
    type ConfigCell;
    fn from_config(config: &Self::Config) -> Option<&Self::ConfigCell>;
}

pub struct SimpleConfigCell<T> {
    config: T,
}

// TODO: $trait:path (traits not alongside plug!)
// plug trait rename?

#[macro_export]
macro_rules! plug {(
    (trait: $trait:ident, config_cell: $config_cell:path)
    {$(
        $impl:path = $tag:expr
    ),*}
) => { paste::paste! {

        use $crate::frame::plug::*;

        $crate::trait_alias!(pub trait [<$trait Plug>] = $trait + Tagged + FromConfig);

        $(
            impl Tagged for $impl {
                const TAG: &'static str = $tag;
            }

            impl FromConfig for $impl {
                type Config = Config;
                type ConfigCell = $config_cell<$impl>;

                fn from_config(config: &Self::Config) -> Option<&Self::ConfigCell> {
                    config.[<$tag>].as_ref()
                }
            }
        )+

        #[allow(non_snake_case)]
        #[derive(facet::Facet, Default)]
        pub struct Config {
            $(
                #[facet(rename = $tag)]
                [<$tag>]: Option<$config_cell<$impl>>,
            )+
        }

        $crate::scope!{($all:tt) => {
            #[macro_export]
            macro_rules! [<dispatch_ $trait:lower>] {
                // string => function(*arguments)
                ($provided_tag:expr => $function:ident($all($args:tt)*) $all($post:tt)?) => {
                    match $provided_tag {
                        // T::TAG => function::<T>(*arguments)
                        $( <$impl>::TAG => // TODO: replace with $tag
                        $function::<$impl>($all($args)*) $all($post)?, )+
                        //
                        other => anyhow::bail!("{other} is not a valid session type."),
                    }
                }
            }
        }}
    }}
}

#[macro_export]
macro_rules! plug_mod {
    (
        (trait: $trait:ident, config_cell: $config_cell:path, name: $name:ident)
        {$(
            $vis:vis $mod:ident = $tag:expr,
        )*}
    ) => { paste::paste! {
        $( $vis mod $mod; )+

        $crate::plug! {
            (trait: $trait, config_cell: $config_cell) {
                $($mod::$name = $tag),*
            }
        }
    }};
}

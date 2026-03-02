use facet::Facet;

// trait_alias!(pub trait ConfigContainer = for<'f> Facet<'f> + Default);

pub trait Tagged {
    const TAG: &'static str;
}

pub trait FromConfig<T, Common>: Sized {
    type Config;
    type ConfigCell;
    fn from_config(config: &Self::Config) -> Option<&Self::ConfigCell>;
}

#[derive(Facet, Default)]
pub struct ConfigCell<Common, Specific> {
    #[facet(flatten)]
    common: Common,
    #[facet(flatten)]
    specific: Specific,
}

// TODO: $trait:path (traits not alongside plug!)
// plug trait rename?

#[macro_export]
macro_rules! plug {(
    (trait: $trait:ident, common: $common:ident)
    {$(
        $impl:path = $tag:expr
    ),*}
) => { paste::paste! {

        use $crate::plug::*;

        $crate::trait_alias!(pub trait [<$trait Plug>] = $trait + Tagged + FromConfig<Config, $common>);

        $(
            impl Tagged for $impl {
                const TAG: &'static str = $tag;
            }

            impl FromConfig<Config, $common> for $impl {
                type Config = Config;
                type ConfigCell = ConfigCell<$common, $impl>;

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
                [<$tag>]: Option<ConfigCell<$common, $impl>>,
            )+
        }

        $crate::scope!{($all:tt) => {
            #[macro_export]
            macro_rules! [<dispatch_ $trait:lower>] {
                // string => function(*arguments)
                ($provided_tag:expr => $function:ident($all($args:tt)*) $all($post:tt)?) => {
                    match $provided_tag {
                        // T::TAG => function::<T>(*arguments)
                        $( <$impl>::TAG =>
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
        (trait: $trait:path, common: $common:ident, name: $name:ident)
        {$(
            $vis:vis $mod:ident = $tag:expr,
        )*}
    ) => { paste::paste! {
        $(
            $vis mod $mod;
        )+

        $crate::plug! {
            (trait: $trait, common: $common) {
                $($mod::$name = $tag),*
            }
        }
    }};
}

mod test {
    #[derive(Facet, Default)]
    struct TestCommon {
        a: u16,
    }

    pub trait TestType {
        const TEST: u32 = 0;

        async fn test(&self) -> u16;
    }

    mod b {
        use super::*;
        #[derive(Facet, Default)]
        pub struct TestImpl;
    }

    plug!((trait: TestType, common: TestCommon) {
        b::TestImpl = "TAG_1"
    });
}

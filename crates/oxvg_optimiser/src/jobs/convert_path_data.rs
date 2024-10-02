use std::rc::Rc;

use markup5ever::local_name;
use oxvg_path::{convert, geometry::MakeArcs, Path};
use oxvg_selectors::Element;
use serde::Deserialize;

use crate::{Context, Job};

#[derive(Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ConvertPathData {
    remove_useless: Option<bool>,
    smart_arc_rounding: Option<bool>,
    straight_curves: Option<bool>,
    convert_to_q: Option<bool>,
    line_shorthands: Option<bool>,
    collapse_repeated: Option<bool>,
    curve_smooth_shorthands: Option<bool>,
    convert_to_z: Option<bool>,
    force_absolute_path: Option<bool>,
    negative_extra_space: Option<bool>,
    make_arcs: Option<MakeArcs>,
    float_precision: Option<i32>,
    utilize_absolute: Option<bool>,
    // TODO: Do we want to have apply_transforms as an option, or is it better to have this plugin
    // just *before* this one
    // apply_transforms: Option<bool>,
    // apply_transforms_stroked: Option<bool>,
    // transform_precision: Option<usize>,
}

impl Job for ConvertPathData {
    fn use_style(&self, node: &Rc<rcdom::Node>) -> bool {
        let element = Element::new(node.clone());
        element.get_attr(&local_name!("d")).is_some()
    }

    fn run(&self, node: &Rc<rcdom::Node>, context: &Context) {
        let element = Element::new(node.clone());
        let Some(d) = element.get_attr(&local_name!("d")) else {
            return;
        };

        let style_info = convert::StyleInfo::gather(
            &context.style.computed(),
            element.get_attr(&local_name!("marker-start")).is_some()
                || element.get_attr(&local_name!("marker-end")).is_some(),
        );
        dbg!("ConvertPathData::run: gained style info", &style_info);

        let path = match Path::parse(&d.value) {
            Ok(path) => path,
            Err(e) => {
                dbg!("ConvertPathData::run: failed to parse path", e);
                return;
            }
        };
        if path.0.is_empty() {
            return;
        }

        let path = convert::run(
            &path,
            &convert::Options {
                flags: self.into(),
                make_arcs: self.make_arcs.clone().unwrap_or_default(),
                precision: self.float_precision.unwrap_or(DEFAULT_FLOAT_PRECISION),
            },
            &style_info,
        );

        element.set_attr(&local_name!("d"), String::from(path).into());
    }
}

impl From<&ConvertPathData> for convert::Flags {
    fn from(val: &ConvertPathData) -> Self {
        use convert::Flags;

        let mut output = convert::Flags::default();
        if let Some(f) = val.remove_useless {
            output.set(Flags::remove_useless_flag, f);
        }
        if let Some(f) = val.smart_arc_rounding {
            output.set(Flags::smart_arc_rounding_flag, f);
        }
        if let Some(f) = val.straight_curves {
            output.set(Flags::straight_curves_flag, f);
        }
        if let Some(f) = val.convert_to_q {
            output.set(Flags::convert_to_q_flag, f);
        }
        if let Some(f) = val.line_shorthands {
            output.set(Flags::line_shorthands_flag, f);
        }
        if let Some(f) = val.collapse_repeated {
            output.set(Flags::collapse_repeated_flag, f);
        }
        if let Some(f) = val.curve_smooth_shorthands {
            output.set(Flags::curve_smooth_shorthands_flag, f);
        }
        if let Some(f) = val.convert_to_z {
            output.set(Flags::convert_to_z_flag, f);
        }
        if let Some(f) = val.force_absolute_path {
            output.set(Flags::force_absolute_path_flag, f);
        }
        if let Some(f) = val.negative_extra_space {
            output.set(Flags::negative_extra_space_flag, f);
        }
        if let Some(f) = val.utilize_absolute {
            output.set(Flags::utilize_absolute_flag, f);
        }
        output
    }
}

static DEFAULT_FLOAT_PRECISION: i32 = 3;

#[test]
#[allow(clippy::too_many_lines)]
fn convert_path_data() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "convertPathData": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- Optimise move commands -->
    <path d="M 10,50"/>
    <path d="M 10 50"/>
    <path d="M10 50"/>
    <path d="M10,50"/>
    <path d="M10-3.05176e-005"/>
    <path d="M10-50.2.30-2"/>
    <path d="M10-50l.2.30"/>
    <path d="M 10 , 50"/>
    <path d="M -10,-50"/>
    <path d="M -10 -50"/>
    <path d="M-10 -50"/>
    <path d="M-10-50"/>
    <path d="M-10,-50"/>
    <path d="M -10 , -50"/>
    <path d="..."/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertPathData": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <path d="M 10,50 L 20,30"/>
    <path d="M 10,50 C 20,30 40,50 60,70"/>
    <path d="M 10,50 C 20,30 40,50 60,70 S 20,30 30,60"/>
    <path d="M 10,50 Q 30,60 30,70"/>
    <path d="M 10,50 Q 30,60 30,70 T 40,70"/>
    <path d="M 10,50 A 20,60 45 0,1 40,70"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertPathData": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <path d="M 10,50 M 20,60"/>
    <path d="M 10,50 20,60"/>
    <path d="M 10,50 L 20,30 L 40,60"/>
    <path d="M 10,50 L 20,30 40,60"/>
    <path d="M 10,50 C 20,30 40,50 60,70 C 40,40 50,60 70,80"/>
    <path d="M 10,50 C 20,30 40,50 60,70 40,40 50,60 70,80"/>
    <path d="M 10,50 C 20,30 40,50 60,70 S 30,30 40,50 S 60,70 80,100"/>
    <path d="M 10,50 C 20,30 40,50 60,70 S 30,30 40,50 60,70 80,100"/>
    <path d="M 10,50 Q 30,60 30,70 Q 40,70 50,90"/>
    <path d="M 10,50 Q 30,60 30,70 40,70 50,90"/>
    <path d="M 10,50 Q 30,60 30,70 T 40,70 T 50,90"/>
    <path d="M 10,50 Q 30,60 30,70 T 40,70 50,90"/>
    <path d="M 10,50 A 20,60 45 0,1 40,70 A 30,50 -30 1,1 50,70"/>
    <path d="M 10,50 A 20,60 45 0,1 40,70 30,50 -30 1,1 50,70"/>
    <style>
      .marker-mid { marker-mid: url(#); }
    </style>
    <path d="M0,0 0,5 0,10" class="marker-mid"/>
    <path d="M0,0 0,5 0,10" marker-mid="url(#)"/>
    <style>
      .linecap-round { stroke: black; stroke-linecap: round; }
      .linecap-butt { stroke: black; stroke-linecap: butt; }
    </style>
    <path d="M0,0 0,0" stroke="black" stroke-linecap="round"/>
    <path d="M0,0 0,0" class="linecap-round"/>
    <path d="M0,0 0,0" stroke="black" stroke-linecap="butt"/>
    <path d="M0,0 0,0" class="linecap-butt"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertPathData": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <path d="M 10,50 l 20,30 L 20,30"/>
    <path d="M 10,50 c 20,30 40,50 60,70 C 20,30 40,50 60,70"/>
    <path d="M 10,50 c 20,30 40,50 60,70 s 20,40 40,50 L 10,20"/>
    <path d="M 10,50 q 20,60 30,70 Q 20,60 30,70"/>
    <path d="M 10,50 q 20,60 30,70 t 40,70 L 10,20"/>
    <path d="M 10,50 a 20,60 45 0,1 40,70 A 20,60 45 0,1 40,70"/>
    <path d="M1 1 v8 c0-2 0-4 0-6"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertPathData": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <path d="M 10.3467,50.09 L 10.0000,50.20"/>
    <path d="m 10 10 l 1 1 M 20 20"/>
    <path d="m 0 0 l .1133 1 l .1133 2 l .1133 3"/>
    <path d="m 0 0 l .0025 3 .0025 2 .0025 3 .0025 2"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertPathData": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <path d="M 10,50 L 10,50"/>
    <path d="M 10,50 L 20,50"/>
    <path d="M 10,50 L 10,60"/>
    <path d="M 10,50 L 20,30 10,30"/>
    <path d="M 10,50 L 20,30 20,20"/>
    <path d="M 10,50 L 20,30 10,30 40,50"/>
    <path d="M 10,50 L 20,30 20,20 40,50"/>
    <path d="M 10,50 L 20,50 L 30,50"/>
    <path d="M 10,50 L 20,50 30,50"/>
    <path d="M 10,50 L 20,50 L 30,50 L 40,50"/>
    <path d="M 10,50 L 10,60 L 10,70"/>
    <path d="M 10,50 L 10,60 10,70"/>
    <path d="M 10,50 L 10,60 L 10,70 L 10,80"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertPathData": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <path d="m 0,0"/>
    <path d="m 0,0l 0,0"/>
    <path d="m 0,0h 0"/>
    <path d="m 0,0v 0"/>
    <path d="m 0,0c 0,0 0,0 0,0 s 0,0 0,0"/>
    <path d="m 0,0q 0,0 0,0 t 0,0"/>
    <path d="m 0,0a 25,25 -30 0,1 0,0"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertPathData": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <path d="M100,200 C200,200 300,200 400,200"/>
    <path d="M100,200 C100,200 250,200 250,200 S300,200 400,200"/>
    <path d="M100,200 C100,200 250,200 250,200 S300,300 400,210"/>
    <path d="M100,200 S250,250 250,250 S400,250 500,250"/>
    <path d="M100,200 Q200,200 300,200"/>
    <path d="M100,200 Q400,200 600,200 T800,200"/>
    <path d="M100,200 Q400,200 600,200 T800,300"/>
    <path d="M100,200 Q200,200 200,300 T200,500 T300,500"/>
    <path d="M100,200 Q400,200 600,200 T800,200 T900,300"/>
    <path d="M100,200 T800,300"/>
    <path d="M100,200 A0,150 0 0,0 150,150"/>
    <path d="M100,200 A150,0 0 0,0 150,150"/>
    <path d="M100,200 c-2.5 10.5-4 21-4 32 0 64 63.5 128 127.5 128s32.5 0 96.5 0 128-64 128-128-64-128-128-128"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertPathData": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <path d="M100,200 C100,100 450,100 250,200 C50,300 400,300 400,200"/>
    <path d="M100,200 S250,100 250,200 C250,300 300,250 400,200"/>
    <path d="M100,200 C100,200 250,100 250,200"/>
    <path d="M200,300 Q400,50 600,300 Q 800,550 1000,300"/>
    <path d="M200,300 Q400,50 600,300 T1000,300 Q1200,50 1400,300"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertPathData": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <path d="m100,200 300,400 z m100,200 L 300,400"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertPathData": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <path d="M10 50h30h-30"/>
    <path d="M10 50h-30h30"/>
    <path d="M10 50h-30h-50"/>
    <path d="M10 50h30h50"/>
    <path d="M10 50v30v-30"/>
    <path d="M10 50v-30v30"/>
    <path d="M10 50v-30v-50"/>
    <path d="M10 50v30v50"/>
    <path d="M10 50L10 80L10 0"/>
    <path d="M10 50L10 10L10 80"/>
    <path d="M10 50l10 10l20 20l10 10"/>
    <path d="M10 50L80 50L0 50"/>
    <path d="M10 50L0 50L80 50"/>
    <path d="M10 50L0 50M80 50M30 10L10 80"/>
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertPathData": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <path d="M213 543q0 -41 20 -66.5q20 -25.5 50 -45.5l49 228q-54 -4 -86.5 -34q-32.5 -30 -32.5 -82zt0 0zM371 48z" />
</svg>"#
        )
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "convertPathData": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <path d="M0 0L0 0c2.761 0 5 2.239 5 5"/>
    <path d="M0 0L0 0c2.761 0 5 2.239 5 5l5-5"/>
    <path d="M15 10c-2.761 0-5-2.239-5-5s2.239-5 5-5s5 2.239 5 5l-5 5"/>
    <path d="M41.008 0.102c1.891 0.387 3.393 1.841 3.849 3.705"/>
    <path d="M7.234 19.474C6.562 19.811 5.803 20 5 20c-2.761 0-5-2.239-5-5 0-1.767 0.917-3.32 2.301-4.209"/>
    <path d="M60 0c-2.761 0-5 2.239-5 5s2.239 5 5 5s5-2.239 5-5S62.761 0 60 0z"/>
    <path d="M15 23.54 c-2.017,0 -3.87,-.7 -5.33,-1.87 -.032,-.023 -.068,-.052 -.11,-.087 .042,.035 .078,.064 .11,.087 .048,.04 .08,.063 .08,.063 "/>
    <path d="M-9.5,82.311c-2.657,0-4.81-2.152-4.81-4.811c0-2.656,2.153-4.811,4.81-4.811S-4.69,74.844-4.69,77.5 C-4.69,80.158-6.843,82.311-9.5,82.311z"/>
    <path d="M1.5,13.4561 C1.5,15.3411 3.033,16.8751 4.918,16.8751 C6.478,16.8751 7.84,15.8201 8.229,14.3101 Z"/>
</svg>"#
        )
    )?);

    // TODO: Rest of tests to be added in next commit
    // NOTE: The following SVGO tests should be used for apply_transforms
    // convertPathData.11.svg.txt

    Ok(())
}

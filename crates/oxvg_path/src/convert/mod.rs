mod cleanup;
pub mod filter;
mod mixed;
mod relative;

pub use crate::convert::cleanup::cleanup;
pub use crate::convert::filter::filter;
pub use crate::convert::mixed::mixed;
pub use crate::convert::relative::relative;
use crate::geometry::MakeArcs;
use crate::math::to_fixed;
use crate::{command, Path};

#[cfg(feature = "oxvg")]
use oxvg_style;
#[cfg(feature = "oxvg")]
use std::collections::BTreeMap;

bitflags! {
    /// External style information that may be relevant when optimising a path
    #[derive(Debug)]
    pub struct StyleInfo: usize {
        const has_marker_mid = 0b0_0001;
        const maybe_has_stroke = 0b0010;
        const maybe_has_linecap = 0b100;
        const is_safe_to_use_z = 0b1000;
        const has_marker = 0b_0001_0000;
    }
}

bitflags! {
    /// Control flags for certain behaviours while optimising a path
    #[derive(Debug)]
    pub struct Flags: usize {
        const remove_useless_flag= 0b0000_0000_0000_0001;
        const smart_arc_rounding_flag= 0b_0000_0000_0010;
        const straight_curves_flag = 0b00_0000_0000_0100;
        const convert_to_q_flag = 0b_0000_0000_0000_1000;
        const line_shorthands_flag = 0b00_0000_0001_0000;
        const collapse_repeated_flag = 0b_0000_0010_0000;
        const curve_smooth_shorthands_flag = 0b0100_0000;
        const convert_to_z_flag = 0b_0000_0000_1000_0000;
        const force_absolute_path_flag = 0b001_0000_0000;
        const negative_extra_space_flag = 0b10_0000_0000;
        const utilize_absolute_flag = 0b0_0100_0000_0000;
    }
}

#[derive(Debug)]
pub struct Options {
    pub flags: Flags,
    pub make_arcs: MakeArcs,
    pub precision: i32,
}

pub fn run(path: &Path, options: &Options, style_info: &StyleInfo) -> Path {
    let includes_vertices = path
        .0
        .iter()
        .any(|c| !matches!(c, command::Data::MoveBy(_) | command::Data::MoveTo(_)));
    // The general optimisation process: original -> naively relative -> filter redundant ->
    // optimal mixed
    dbg!("convert::run: converting path", path.to_string());
    let mut positioned_path = relative(path);
    let mut state = filter::State::new(&positioned_path, options, style_info);
    positioned_path = filter(&positioned_path, options, &mut state, style_info);
    if options.flags.utilize_absolute() {
        positioned_path = mixed(&positioned_path, options);
    }
    positioned_path = cleanup(&positioned_path);

    let mut path = positioned_path.take();
    let has_marker = style_info.contains(StyleInfo::has_marker);
    let is_markers_only_path = has_marker
        && includes_vertices
        && path
            .0
            .iter()
            .all(|c| matches!(c, command::Data::MoveBy(_) | command::Data::MoveTo(_)));
    if is_markers_only_path {
        path.0.push(command::Data::ClosePath);
    }
    for command in &mut path.0 {
        options.round_data(command.args_mut(), options.error());
    }
    dbg!("convert::run: done", path.to_string());
    path
}

impl StyleInfo {
    #[cfg(feature = "oxvg")]
    pub fn gather(
        computed_styles: &BTreeMap<oxvg_style::SVGStyleID, &oxvg_style::SVGStyle>,
        has_marker: bool,
    ) -> Self {
        use lightningcss::properties::svg::{StrokeLinecap, StrokeLinejoin};

        let has_marker_mid = computed_styles.contains_key(&oxvg_style::SVGStyleID::MarkerMid);

        let stroke = computed_styles.get(&oxvg_style::SVGStyleID::Stroke);
        let maybe_has_stroke = stroke.is_some_and(|property| {
            !matches!(
                property,
                oxvg_style::SVGStyle::Stroke(oxvg_style::SVGPaint::None)
            )
        });

        let linecap = computed_styles.get(&oxvg_style::SVGStyleID::SrokeLinecap);
        let maybe_has_linecap = linecap.as_ref().is_some_and(|property| {
            !matches!(
                property,
                oxvg_style::SVGStyle::StrokeLinecap(StrokeLinecap::Butt)
            )
        });

        let linejoin = computed_styles.get(&oxvg_style::SVGStyleID::StrokeLinejoin);
        let is_safe_to_use_z = if maybe_has_stroke {
            linecap.is_some_and(|property| {
                matches!(
                    property,
                    oxvg_style::SVGStyle::StrokeLinecap(StrokeLinecap::Round)
                )
            }) && linejoin.is_some_and(|property| {
                matches!(
                    property,
                    oxvg_style::SVGStyle::StrokeLinejoin(StrokeLinejoin::Round)
                )
            })
        } else {
            true
        };

        let mut result = Self::empty();
        result.set(Self::has_marker_mid, has_marker_mid);
        result.set(Self::maybe_has_stroke, maybe_has_stroke);
        result.set(Self::maybe_has_linecap, maybe_has_linecap);
        result.set(Self::is_safe_to_use_z, is_safe_to_use_z);
        result.set(Self::has_marker, has_marker);
        result
    }
}

impl Default for StyleInfo {
    fn default() -> Self {
        Self::empty()
    }
}

impl Flags {
    fn remove_useless(&self) -> bool {
        self.contains(Self::remove_useless_flag)
    }

    fn smart_arc_rounding(&self) -> bool {
        self.contains(Self::smart_arc_rounding_flag)
    }

    fn straight_curves(&self) -> bool {
        self.contains(Self::straight_curves_flag)
    }

    fn convert_to_q(&self) -> bool {
        self.contains(Self::convert_to_q_flag)
    }

    fn line_shorthands(&self) -> bool {
        self.contains(Self::line_shorthands_flag)
    }

    fn collapse_repeated(&self) -> bool {
        self.contains(Self::collapse_repeated_flag)
    }

    fn curve_smooth_shorthands(&self) -> bool {
        self.contains(Self::curve_smooth_shorthands_flag)
    }

    fn convert_to_z(&self) -> bool {
        self.contains(Self::convert_to_z_flag)
    }

    fn force_absolute_path(&self) -> bool {
        self.contains(Self::force_absolute_path_flag)
    }

    fn negative_extra_space(&self) -> bool {
        self.contains(Self::negative_extra_space_flag)
    }

    fn utilize_absolute(&self) -> bool {
        self.contains(Self::utilize_absolute_flag)
    }
}

impl Default for Flags {
    fn default() -> Self {
        let mut flags = Self::all();
        flags.set(Self::force_absolute_path_flag, false);
        flags
    }
}

impl Options {
    pub fn error(&self) -> f64 {
        let trunc_by = f64::powi(10.0, self.precision);
        f64::trunc(f64::powi(0.1, self.precision) * trunc_by) / trunc_by
    }

    pub fn round(&self, data: f64, error: f64) -> f64 {
        match self.precision {
            p if p > 0 && p < 20 => {
                let fixed = to_fixed(data, p);
                if (fixed - data).abs() == 0.0 {
                    return data;
                }
                let rounded = to_fixed(data, p - 1);
                if to_fixed((rounded - data).abs(), p + 1) >= error {
                    fixed
                } else {
                    rounded
                }
            }
            _ => data.round(),
        }
    }

    pub fn round_data(&self, data: &mut [f64], error: f64) {
        data.iter_mut().for_each(|d| *d = self.round(*d, error));
    }
}

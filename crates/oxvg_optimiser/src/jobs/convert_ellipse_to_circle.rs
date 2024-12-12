use oxvg_ast::element::Element;
use oxvg_derive::OptionalDefault;
use serde::Deserialize;

use crate::{Context, Job, JobDefault, PrepareOutcome};

#[derive(Deserialize, Clone, OptionalDefault)]
#[serde(rename_all = "camelCase")]
pub struct ConvertEllipseToCircle(bool);

impl<E: Element> Job<E> for ConvertEllipseToCircle {
    fn prepare(&mut self, _document: &E::ParentChild) -> PrepareOutcome {
        if self.0 {
            PrepareOutcome::None
        } else {
            PrepareOutcome::Skip
        }
    }

    #[allow(clippy::similar_names)]
    fn run(&self, element: &E, _context: &Context<E>) {
        let name = element.local_name();
        if name.as_ref() != "ellipse" {
            return;
        }

        let rx_name = &"rx".into();
        let ry_name = &"ry".into();
        let rx = element
            .get_attribute_local(rx_name)
            .map_or(String::from("0"), |attr| attr.to_string());
        let ry = element
            .get_attribute_local(ry_name)
            .map_or(String::from("0"), |attr| attr.to_string());

        if rx != ry && rx != "auto" && ry != "auto" {
            return;
        }
        let radius = if rx == "auto" { ry } else { rx };
        element.remove_attribute_local(rx_name);
        element.remove_attribute_local(ry_name);
        element.set_attribute_local("r".into(), radius.into());
    }
}

impl Default for ConvertEllipseToCircle {
    fn default() -> Self {
        Self(true)
    }
}

#[test]
fn convert_ellipse_to_circle() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "convertEllipseToCircle": true }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- Convert circular ellipses to circles -->
    <ellipse rx="5" ry="5"/>
    <ellipse rx="auto" ry="5"/>
    <ellipse rx="5" ry="auto"/>
    <ellipse />
</svg>"#
        )
    )?);

    Ok(())
}

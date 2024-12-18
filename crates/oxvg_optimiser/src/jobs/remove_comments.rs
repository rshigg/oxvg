use oxvg_ast::{
    element::Element,
    node::{self, Node},
    visitor::Visitor,
};
use oxvg_derive::OptionalDefault;
use serde::Deserialize;
use serde_json::Value;

use crate::{Job, PrepareOutcome};

use super::ContextFlags;

#[derive(Deserialize, Clone, Default, OptionalDefault)]
#[serde(rename_all = "camelCase")]
pub struct RemoveComments {
    preserve_patterns: Option<Vec<PreservePattern>>,
}

#[derive(Debug, Clone)]
pub struct PreservePattern(regex::Regex);

impl<E: Element> Job<E> for RemoveComments {
    fn prepare(
        &mut self,
        document: &E::ParentChild,
        _context_flags: &ContextFlags,
    ) -> PrepareOutcome {
        for child in document.child_nodes_iter() {
            if child.node_type() != node::Type::Comment {
                continue;
            }
            self.remove_comment(&child);
        }
        PrepareOutcome::None
    }
}

impl<E: Element> Visitor<E> for RemoveComments {
    type Error = String;

    fn comment(&mut self, comment: &mut <E as Node>::Child) -> Result<(), Self::Error> {
        self.remove_comment(comment);
        Ok(())
    }
}

impl RemoveComments {
    fn remove_comment(&self, comment: &impl Node) {
        let value = comment
            .node_value()
            .expect("Comment nodes should always have a value");

        if self
            .preserve_patterns
            .as_ref()
            .unwrap_or(&DEFAULT_PRESERVE_PATTERNS)
            .iter()
            .any(|pattern| pattern.0.is_match(value.as_ref()))
        {
            return;
        }

        comment.remove();
    }
}

#[derive(Debug)]
enum DeserializePreservePatternError {
    InvalidType,
    InvalidRegex,
}

impl std::fmt::Display for DeserializePreservePatternError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidType => f.write_str("expected a string"),
            Self::InvalidRegex => f.write_str("expected valid regex string"),
        }
    }
}

impl serde::de::StdError for DeserializePreservePatternError {}

impl<'de> Deserialize<'de> for PreservePattern {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        let Value::String(string) = value else {
            return Err(serde::de::Error::custom(
                DeserializePreservePatternError::InvalidType,
            ));
        };

        let regex = regex::Regex::new(&string)
            .map_err(|_| serde::de::Error::custom(DeserializePreservePatternError::InvalidRegex))?;
        Ok(Self(regex))
    }
}

lazy_static! {
    static ref DEFAULT_PRESERVE_PATTERNS: Vec<PreservePattern> =
        vec![PreservePattern(regex::Regex::new("^!").unwrap())];
}

#[test]
fn remove_comments() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "removeComments": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!--- test -->
    <g>
        <!--- test -->
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeComments": {} }"#,
        Some(
            r#"<!--!Icon Font v1 by @iconfont - Copyright 2023 Icon Font CIC.-->
<svg xmlns="http://www.w3.org/2000/svg">
    test
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "removeComments": { "preservePatterns": [] } }"#,
        Some(
            r#"<!--!Icon Font v1 by @iconfont - Copyright 2023 Icon Font CIC.-->
<svg xmlns="http://www.w3.org/2000/svg">
    test
</svg>"#
        ),
    )?);

    Ok(())
}

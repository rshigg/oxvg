use std::{borrow::BorrowMut, cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    file_reader::{Child, Element, Parent, SAXState},
    syntactic_constructs::{name, whitespace},
};

use super::{
    attributes::Attribute,
    text::{Script, Text},
    State, ID,
};

/// <foo
pub struct OpenTag;
/// <foo /
pub struct OpenTagSlash;
/// </foo
pub struct CloseTag;
/// <foo \s
pub struct CloseTagSawWhite;

impl State for OpenTag {
    fn next(self: Box<Self>, sax: &mut SAXState, char: char) -> Box<dyn State> {
        match char {
            c if name::is(c) => {
                sax.tag_name.push(c);
                self
            }
            '>' => Self::handle_end(sax, false),
            '/' => Box::new(OpenTagSlash),
            c => {
                if !whitespace::is(c) {
                    sax.error_char("Expected a valid tag name character");
                }
                Box::new(Attribute)
            }
        }
    }

    fn id(&self) -> ID {
        ID::OpenTag
    }
}

impl OpenTag {
    pub fn handle_end(sax: &mut SAXState, is_self_closing: bool) -> Box<dyn State> {
        sax.saw_root = true;
        let state: Box<dyn State> = if !is_self_closing && sax.tag_name.to_lowercase() == "script" {
            Box::new(Script)
        } else {
            Box::new(Text)
        };
        if let Parent::Element(e) = &mut sax.tag {
            let element: &RefCell<Element> = e.borrow_mut();
            element.borrow_mut().name = std::mem::take(&mut sax.tag_name);
            element.borrow_mut().attributes = std::mem::take(&mut sax.attribute_map);
            sax.tags.push(e.clone());
            if sax.root_tag.is_none() {
                sax.root_tag = Some(e.clone());
            }
        }

        if sax.get_options().xmlns {
            todo!();
        }
        state
    }
}

impl State for OpenTagSlash {
    fn next(self: Box<Self>, sax: &mut SAXState, char: char) -> Box<dyn State> {
        if char == '>' {
            OpenTag::handle_end(sax, true);
            CloseTag::handle_end(sax)
        } else {
            sax.error_char("Expected a `>` to end self-closing tag");
            Box::new(Attribute)
        }
    }

    fn id(&self) -> ID {
        ID::OpenTagSlash
    }
}

impl State for CloseTag {
    fn next(self: Box<Self>, sax: &mut SAXState, char: char) -> Box<dyn State> {
        match char {
            c if sax.tag_name.is_empty() && whitespace::is(c) => self,
            c if sax.tag_name.is_empty() && !name::is_start(c) => {
                if !sax.script.is_empty() {
                    sax.script.push_str(&format!("</{c}"));
                    return Box::new(Script);
                }
                sax.error_char("Expected a valid starting tag name character");
                self
            }
            c if name::is(c) => {
                sax.tag_name.push(c);
                self
            }
            '>' => Self::handle_end(sax),
            c if !sax.script.is_empty() => {
                sax.script.push_str(&format!("</{c}"));
                sax.tag_name = String::new();
                Box::new(Script)
            }
            c if whitespace::is(c) => Box::new(CloseTagSawWhite),
            _ => {
                sax.error_char("Expected a valid tag name character");
                self
            }
        }
    }

    fn id(&self) -> ID {
        ID::CloseTag
    }
}

impl CloseTag {
    pub fn handle_end(sax: &mut SAXState) -> Box<dyn State> {
        if sax.tag_name.is_empty() {
            if sax.get_options().strict {
                sax.error_tag("start of tag name");
            }
            sax.text_node = "</>".into();
            return Box::new(Text);
        }

        if !sax.script.is_empty() {
            if sax.tag_name.to_lowercase() != "script" {
                sax.script.push_str(&format!("</{}>", sax.tag_name));
                sax.tag_name = String::default();
                return Box::new(Script);
            }
            sax.script = String::default();
        }

        let new_state = Box::new(Text);
        let normalised_tag_name = sax.tag_name.to_lowercase();
        // Find the matching opening tag, it should be at the end of `sax.tags`, unless...
        // <a><b></c></b></a>
        let mut opening_tag_index = None;
        for (i, matching_open) in sax.tags.iter_mut().enumerate().rev() {
            let e: &RefCell<Element> = matching_open.borrow_mut();
            let e = e.borrow_mut();
            if e.is_self_closing {
                continue;
            }
            if e.name.to_lowercase() == normalised_tag_name {
                opening_tag_index = Some(i);
                break;
            }
        }

        // No matching tag, abort!
        if opening_tag_index.is_none() {
            sax.error_tag("Matching opening tag not found");
            sax.text_node.push_str(&format!("</{}>", sax.tag_name));
            return new_state;
        }

        // Say goodbye to our opening tag, and any baddies between us
        if let Some(i) = opening_tag_index {
            for _ in 0..sax.tags.len() - i - 1 {
                sax.tags.pop();
            }
            let opening_tag = sax.tags.pop();
            if i == 0 {
                sax.closed_root = true;
            }
            if let Some(o) = opening_tag {
                match sax.tags.last() {
                    Some(t) => Parent::Element(t.clone()).push_child(Child::Element(o.take())),
                    None => sax
                        .root
                        .children
                        .push(Rc::new(RefCell::new(Child::Element(o.take())))),
                };
            } else {
                unreachable!("The opening tag was accidentally lost");
            }
        }

        sax.tag_name = String::default();
        sax.attribute_map = HashMap::new();
        sax.attribute_name = String::default();
        new_state
    }
}

impl State for CloseTagSawWhite {
    fn next(self: Box<Self>, sax: &mut SAXState, char: char) -> Box<dyn State> {
        match char {
            c if whitespace::is(c) => self,
            '>' => CloseTag::handle_end(sax),
            _ => {
                sax.error_char("Expected `>` to end closing tag");
                self
            }
        }
    }

    fn id(&self) -> ID {
        ID::CloseTagSawWhite
    }
}

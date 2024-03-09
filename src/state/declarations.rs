use crate::{
    diagnostics::SVGError,
    file_reader::{Child, SAXState},
};

use super::{text::Text, FileReaderState, State};

/// <!BLARG
pub struct SGMLDeclaration;
/// <!BLARG foo "bar"
pub struct SGMLDeclarationQuoted;
/// <![CDATA[ foo
pub struct CData;
/// <![CDATA[ foo ]
pub struct CDataEnding;
/// <![CDATA[ foo ]]
pub struct CDataEnded;
/// <!--
pub struct Comment;
/// <!-- foo -
pub struct CommentEnding;
/// <!-- foo --
pub struct CommentEnded;
/// <!DOCTYPE
pub struct Doctype;
/// <!DOCTYPE "foo
pub struct DoctypeQuoted;
/// <!DOCTYPE "foo" [ ...
pub struct DoctypeDTD;
/// <!DOCTYPE "foo" [ "bar
pub struct DoctypeDTDQuoted;

impl FileReaderState for SGMLDeclaration {
    fn next(self: Box<Self>, sax: &mut SAXState, char: &char) -> Box<dyn FileReaderState> {
        match &sax.sgml_declaration {
            d if d.to_uppercase() == "[CDATA[" => {
                sax.sgml_declaration = String::default();
                sax.cdata = String::default();
                return Box::new(CData);
            }
            d if d == "-" && char == &'-' => {
                sax.comment = String::default();
                sax.sgml_declaration = String::default();
                return Box::new(Comment);
            }
            d if d.to_uppercase() == "DOCTYPE" => {
                if !sax.doctype.is_empty() || sax.saw_root {
                    sax.error_state("Doctype should only be declared before root");
                }
                sax.doctype = String::default();
                sax.doctype = String::default();
                sax.sgml_declaration = String::default();
                return Box::new(Doctype);
            }
            _ => {}
        }
        match char {
            '>' => {
                let value = std::mem::take(&mut sax.sgml_declaration);
                sax.add_child(Child::SGMLDeclaration { value });
                sax.sgml_declaration = String::default();
                Box::new(Text)
            }
            '"' | '\'' => {
                sax.sgml_declaration.push(*char);
                sax.quote = Some(*char);
                Box::new(SGMLDeclarationQuoted)
            }
            c => {
                sax.sgml_declaration.push(*c);
                self
            }
        }
    }

    fn id(&self) -> State {
        State::SGMLDeclaration
    }
}

impl FileReaderState for SGMLDeclarationQuoted {
    fn next(self: Box<Self>, sax: &mut SAXState, char: &char) -> Box<dyn FileReaderState> {
        if Some(*char) == sax.quote {
            sax.quote = None;
            return Box::new(SGMLDeclaration);
        }
        sax.sgml_declaration.push(*char);
        self
    }

    fn id(&self) -> State {
        State::SGMLDeclarationQuoted
    }
}

impl FileReaderState for CData {
    fn next(self: Box<Self>, sax: &mut SAXState, char: &char) -> Box<dyn FileReaderState> {
        match char {
            ']' => Box::new(CDataEnding),
            c => {
                sax.cdata.push(*c);
                self
            }
        }
    }

    fn id(&self) -> State {
        State::CData
    }
}

impl FileReaderState for CDataEnding {
    fn next(self: Box<Self>, sax: &mut SAXState, char: &char) -> Box<dyn FileReaderState> {
        match char {
            ']' => Box::new(CDataEnded),
            c => {
                sax.cdata.push(']');
                sax.cdata.push(*c);
                Box::new(CData)
            }
        }
    }

    fn id(&self) -> State {
        State::CDataEnding
    }
}

impl FileReaderState for CDataEnded {
    fn next(self: Box<Self>, sax: &mut SAXState, char: &char) -> Box<dyn FileReaderState>
    where
        Self: std::marker::Sized,
    {
        match char {
            '>' => {
                let value = std::mem::take(&mut sax.cdata);
                sax.add_child(Child::CData { value });
                sax.cdata = String::default();
                Box::new(Text)
            }
            ']' => {
                sax.cdata.push(*char);
                self
            }
            c => {
                sax.cdata.push_str("]]");
                sax.cdata.push(*c);
                Box::new(CData)
            }
        }
    }

    fn id(&self) -> State {
        State::CDataEnded
    }
}

impl FileReaderState for Comment {
    fn next(self: Box<Self>, sax: &mut SAXState, char: &char) -> Box<dyn FileReaderState> {
        match char {
            '-' => Box::new(CommentEnding),
            c => {
                sax.comment.push(*c);
                self
            }
        }
    }

    fn id(&self) -> State {
        State::Comment
    }
}

impl FileReaderState for CommentEnding {
    fn next(self: Box<Self>, sax: &mut SAXState, char: &char) -> Box<dyn FileReaderState> {
        match char {
            '-' => Box::new(CommentEnded),
            c => {
                sax.comment.push('-');
                sax.comment.push(*c);
                Box::new(Comment)
            }
        }
    }

    fn id(&self) -> State {
        State::CommentEnding
    }
}

impl FileReaderState for CommentEnded {
    fn next(self: Box<Self>, sax: &mut SAXState, char: &char) -> Box<dyn FileReaderState> {
        match char {
            '>' => {
                let value = std::mem::take(&mut sax.comment);
                sax.add_child(Child::Comment { value });
                Box::new(Text)
            }
            c => {
                if sax.get_options().strict {
                    sax.add_error(SVGError::new(
                        "`--` in comments should be avoided".into(),
                        (sax.get_position().end - 2, sax.get_position().end).into(),
                    ))
                }
                sax.comment.push_str("--");
                sax.comment.push(*c);
                Box::new(Comment)
            }
        }
    }

    fn id(&self) -> State {
        State::CommentEnded
    }
}

impl FileReaderState for Doctype {
    fn next(self: Box<Self>, sax: &mut SAXState, char: &char) -> Box<dyn FileReaderState> {
        match char {
            '>' => {
                if !sax.tag.is_root() {
                    sax.error_token("Doctype is only allowed in the root")
                }
                let data = std::mem::take(&mut sax.doctype);
                sax.add_child(Child::Doctype { data });
                Box::new(Text)
            }
            '[' => {
                sax.doctype.push(*char);
                Box::new(DoctypeDTD)
            }
            '"' | '\'' => {
                sax.doctype.push(*char);
                sax.quote = Some(*char);
                Box::new(DoctypeQuoted)
            }
            _ => {
                sax.doctype.push(*char);
                self
            }
        }
    }

    fn id(&self) -> State {
        State::Doctype
    }
}

impl FileReaderState for DoctypeDTD {
    fn next(self: Box<Self>, sax: &mut SAXState, char: &char) -> Box<dyn FileReaderState> {
        sax.doctype.push(*char);
        match char {
            ']' => Box::new(Doctype),
            '"' | '\'' => {
                sax.quote = Some(*char);
                Box::new(DoctypeDTDQuoted)
            }
            _ => self,
        }
    }

    fn id(&self) -> State {
        State::DoctypeDTD
    }
}

impl FileReaderState for DoctypeQuoted {
    fn next(self: Box<Self>, sax: &mut SAXState, char: &char) -> Box<dyn FileReaderState> {
        sax.doctype.push(*char);
        match char {
            c if Some(*c) == sax.quote => {
                sax.quote = None;
                Box::new(Doctype)
            }
            _ => self,
        }
    }

    fn id(&self) -> State {
        State::DoctypeQuoted
    }
}

impl FileReaderState for DoctypeDTDQuoted {
    fn next(self: Box<Self>, sax: &mut SAXState, char: &char) -> Box<dyn FileReaderState> {
        sax.doctype.push(*char);
        match char {
            c if Some(*c) == sax.quote => {
                sax.quote = None;
                Box::new(DoctypeDTD)
            }
            _ => self,
        }
    }

    fn id(&self) -> State {
        State::DoctypeDTDQuoted
    }
}

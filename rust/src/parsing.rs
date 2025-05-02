// This file was originally copied from the pyimportparse crate.
// License copied below, as per terms of the license.
//
// MIT License
//
// Copyright (c) 2025 Peter Byfield
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.


use nom::branch::alt;
use nom::bytes::complete::{tag, take_until};
use nom::character::complete::{
    line_ending, multispace1, not_line_ending, space0, space1,
};
use nom_unicode::complete::alphanumeric1;
use nom::combinator::{all_consuming, opt, recognize, value, verify};
use nom::multi::{many0, many1, separated_list1};
use nom::sequence::{delimited, preceded, terminated};
use nom::{IResult, Input, Parser};
use nom_locate::{LocatedSpan, position};

type Span<'a> = LocatedSpan<&'a str>;

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Import {
    pub imported_object: String,
    pub line_number: u32,
    pub line_contents: String,
    pub typechecking_only: bool,
}

impl Import {
    pub fn new(
        imported_object: String,
        line_number: u32,
        line_contents: String,
        typechecking_only: bool,
    ) -> Self {
        Self {
            imported_object,
            line_number,
            line_contents,
            typechecking_only,
        }
    }
}

pub fn parse_imports(python_file_contents: &str) -> Result<Vec<Import>, String> {
    let span = Span::new(python_file_contents);
    let (_, imports) = all_consuming(parse_block(false))
        .parse(span)
        .map_err(|e| e.to_string())?;
    Ok(with_corrected_line_contents(python_file_contents, imports))
}

// Return the imports, but with the full line contents.
// Currently our nom parsing only pulls out to the latest token, so we correct it here
// by finding the corresponding whole line, based on the line number.
// TODO: adjust the parsing code so it figures it out correctly in the first place.
fn with_corrected_line_contents(python_file_contents: &str, imports: Vec<Import>) -> Vec<Import> {
    let lines: Vec<&str> = python_file_contents.lines().collect();
    imports.into_iter().map(
        |import| Import {
            imported_object: import.imported_object,
            line_number: import.line_number,
            line_contents: lines[(import.line_number as usize) - 1].trim_start().to_string(),
            typechecking_only: import.typechecking_only,
        }
    ).collect()
}

fn parse_block(typechecking_only: bool) -> impl Fn(Span) -> IResult<Span, Vec<Import>> {
    move |s| {
        let (s, result) = many0(alt((
            parse_if_typechecking,
            value(vec![], parse_space1),
            value(vec![], line_ending),
            value(vec![], parse_multiline_comment),
            value(vec![], parse_comment),
            parse_import_statement_list(typechecking_only),
            value(vec![], verify(not_line_ending, |s: &Span| !s.is_empty())),
        )))
            .parse(s)?;
        Ok((s, result.into_iter().flatten().collect()))
    }
}

fn parse_import_statement_list(
    typechecking_only: bool,
) -> impl Fn(Span) -> IResult<Span, Vec<Import>> {
    move |s| {
        let (s, imports) = separated_list1(
            delimited(parse_space0, tag(";"), parse_space0),
            alt((
                parse_import_statement(typechecking_only),
                parse_from_import_statement(typechecking_only),
                parse_multiline_from_import_statement(typechecking_only),
                parse_wildcard_from_import_statement(typechecking_only),
            )),
        )
            .parse(s)?;
        let (s, _) = (opt(parse_space0), opt(tag(";"))).parse(s)?;
        Ok((s, imports.into_iter().flatten().collect()))
    }
}

fn parse_import_statement(typechecking_only: bool) -> impl Fn(Span) -> IResult<Span, Vec<Import>> {
    move |s| {
        let input = s;
        let (s, position) = position.parse(s)?;
        let (s, _) = (tag("import"), parse_space1).parse(s)?;
        let (s, imported_modules) = separated_list1(
            delimited(parse_space0, tag(","), parse_space0),
            terminated(
                parse_module,
                opt((parse_space1, tag("as"), parse_space1, parse_identifier)),
            ),
        )
            .parse(s)?;

        let (_, span) = input.take_split(s.location_offset() - input.location_offset());
        Ok((
            s,
            imported_modules
                .into_iter()
                .map(|imported_module| {
                    Import::new(
                        imported_module.to_owned(),
                        position.location_line(),
                        (*span.fragment()).to_owned(),
                        typechecking_only,
                    )
                })
                .collect(),
        ))
    }
}

fn parse_from_import_statement(
    typechecking_only: bool,
) -> impl Fn(Span) -> IResult<Span, Vec<Import>> {
    move |s| {
        let input = s;
        let (s, position) = position.parse(s)?;
        let (s, _) = (tag("from"), parse_space1).parse(s)?;
        let (s, imported_module_base) = parse_relative_module.parse(s)?;
        let (s, _) = (parse_space1, tag("import"), parse_space1).parse(s)?;

        let (s, imported_identifiers) = separated_list1(
            delimited(parse_space0, tag(","), parse_space0),
            terminated(
                parse_identifier,
                opt((parse_space1, tag("as"), parse_space1, parse_identifier)),
            ),
        )
            .parse(s)?;

        let (_, span) = input.take_split(s.location_offset() - input.location_offset());
        Ok((
            s,
            imported_identifiers
                .into_iter()
                .map(|imported_identifier| {
                    let imported_object = if imported_module_base.ends_with(".") {
                        format!("{}{}", imported_module_base, imported_identifier)
                    } else {
                        format!("{}.{}", imported_module_base, imported_identifier)
                    };
                    Import::new(
                        imported_object,
                        position.location_line(),
                        (*span.fragment()).to_owned(),
                        typechecking_only,
                    )
                })
                .collect(),
        ))
    }
}

fn parse_multiline_from_import_statement(
    typechecking_only: bool,
) -> impl Fn(Span) -> IResult<Span, Vec<Import>> {
    move |s| {
        let input = s;
        let (s, position) = position.parse(s)?;
        let (s, _) = (tag("from"), parse_space1).parse(s)?;
        let (s, imported_module_base) = parse_relative_module.parse(s)?;
        let (s, _) = (parse_space1, tag("import"), parse_space1).parse(s)?;

        let (s, imported_identifiers) = delimited(
            (tag("("), parse_multispace0_or_comment),
            separated_list1(
                delimited(
                    parse_multispace0_or_comment,
                    tag(","),
                    parse_multispace0_or_comment,
                ),
                terminated(
                    parse_identifier,
                    opt((multispace1, tag("as"), multispace1, parse_identifier)),
                ),
            ),
            (
                parse_multispace0_or_comment,
                opt(tag(",")),
                parse_multispace0_or_comment,
                tag(")"),
            ),
        )
            .parse(s)?;

        let (_, span) = input.take_split(s.location_offset() - input.location_offset());
        Ok((
            s,
            imported_identifiers
                .into_iter()
                .map(|imported_identifier| {
                    let imported_object = if imported_module_base.ends_with(".") {
                        format!("{}{}", imported_module_base, imported_identifier)
                    } else {
                        format!("{}.{}", imported_module_base, imported_identifier)
                    };
                    Import::new(
                        imported_object,
                        position.location_line(),
                        (*span.fragment()).to_owned(),
                        typechecking_only,
                    )
                })
                .collect(),
        ))
    }
}

fn parse_wildcard_from_import_statement(
    typechecking_only: bool,
) -> impl Fn(Span) -> IResult<Span, Vec<Import>> {
    move |s| {
        let input = s;
        let (s, position) = position.parse(s)?;
        let (s, _) = (tag("from"), parse_space1).parse(s)?;
        let (s, imported_module) = parse_relative_module.parse(s)?;
        let (s, _) = (parse_space1, tag("import"), parse_space1, tag("*")).parse(s)?;

        let imported_object = if imported_module.ends_with(".") {
            format!("{}*", imported_module)
        } else {
            format!("{}.*", imported_module)
        };

        let (_, span) = input.take_split(s.location_offset() - input.location_offset());
        Ok((
            s,
            vec![Import::new(
                imported_object,
                position.location_line(),
                (*span.fragment()).to_owned(),
                typechecking_only,
            )],
        ))
    }
}

fn parse_module(s: Span) -> IResult<Span, &str> {
    let (s, result) = recognize(separated_list1(tag("."), parse_identifier)).parse(s)?;
    Ok((s, result.fragment()))
}

fn parse_relative_module(s: Span) -> IResult<Span, &str> {
    let (s, result) = alt((
        recognize((many0(tag(".")), parse_module)),
        recognize(many1(tag("."))),
    ))
        .parse(s)?;
    Ok((s, result.fragment()))
}


// Parse a valid Python identifier.
//
// Note this is not implemented as thoroughly as in the Python spec.
// Some identifiers will be valid here (e.g. ones that begin with digits)
// that aren't actually valid in Python. Unicode identifiers are supported.
//
// See https://docs.python.org/3/reference/lexical_analysis.html#identifiers
fn parse_identifier(s: Span) -> IResult<Span, &str> {
    let (s, result) = recognize(
        many1(alt((alphanumeric1, tag("_"))))
    ).parse(s)?;
    Ok((s, result.fragment()))
}

fn parse_comment(s: Span) -> IResult<Span, ()> {
    let (s, _) = (tag("#"), not_line_ending).parse(s)?;
    Ok((s, ()))
}

fn parse_multispace0_or_comment(s: Span) -> IResult<Span, ()> {
    let (s, _) = many0(alt((value((), multispace1), parse_comment))).parse(s)?;
    Ok((s, ()))
}

fn parse_multiline_comment(s: Span) -> IResult<Span, ()> {
    let (s, _) = alt((
        delimited(tag(r#"""""#), take_until(r#"""""#), tag(r#"""""#)),
        delimited(tag(r#"'''"#), take_until(r#"'''"#), tag(r#"'''"#)),
    ))
        .parse(s)?;
    Ok((s, ()))
}

fn parse_space0(s: Span) -> IResult<Span, ()> {
    let (s, _) = many0(alt((space1, tag("\\\n")))).parse(s)?;
    Ok((s, ()))
}

fn parse_space1(s: Span) -> IResult<Span, ()> {
    let (s, _) = many1(alt((space1, tag("\\\n")))).parse(s)?;
    Ok((s, ()))
}


fn parse_if_typechecking(s: Span) -> IResult<Span, Vec<Import>> {
    let (s, _) = (
        tag("if"),
        parse_space1,
        alt(
            (
                tag("TYPE_CHECKING"),
                preceded(parse_identifier, tag(".TYPE_CHECKING")),
            ),
        ),
        parse_space0,
        tag(":"),
    )
        .parse(s)?;

    if let Ok((s, imports)) = preceded(
        parse_space0,
        terminated(
            parse_import_statement_list(true),
            (parse_space0, opt(parse_comment)),
        ),
    )
        .parse(s)
    {
        return Ok((s, imports));
    };

    let (s, _) = (parse_space0, opt(parse_comment), line_ending).parse(s)?;
    let (s, indented_block) = parse_indented_block.parse(s)?;
    let (_, imports) = all_consuming(parse_block(true)).parse(indented_block)?;
    Ok((s, imports))
}

fn parse_indented_block(s: Span) -> IResult<Span, Span> {
    let input = s;

    let (s, _) = many0((space0, line_ending)).parse(s)?;
    let (s, (indentation, _, _)) = (space1, not_line_ending, opt(line_ending)).parse(s)?;

    let (s, _) = many0(alt((
        value((), (space0, line_ending)),
        value(
            (),
            (
                tag(*indentation.fragment()),
                not_line_ending,
                opt(line_ending),
            ),
        ),
    )))
        .parse(s)?;

    Ok(input.take_split(s.location_offset() - input.location_offset()))
}

#[cfg(test)]
mod tests {
    use super::parse_imports;
    use parameterized::parameterized;

    #[test]
    fn test_parse_empty_string() {
        let imports = parse_imports("").unwrap();
        assert!(imports.is_empty());
    }

    fn parse_and_check(case: (&str, &[&str])) {
        let (code, expected_imports) = case;
        let imports = parse_imports(code).unwrap();
        assert_eq!(
            expected_imports,
            imports
                .into_iter()
                .map(|i| i.imported_object)
                .collect::<Vec<_>>()
        );
    }

    fn parse_and_check_with_typechecking_only(case: (&str, &[(&str, bool)])) {
        let (code, expected_imports) = case;
        let imports = parse_imports(code).unwrap();
        assert_eq!(
            expected_imports
                .iter()
                .map(|i| (i.0.to_owned(), i.1))
                .collect::<Vec<_>>(),
            imports
                .into_iter()
                .map(|i| (i.imported_object, i.typechecking_only))
                .collect::<Vec<_>>()
        );
    }

    #[parameterized(case = {
        ("import foo", &["foo"]),
        ("import foo_FOO_123", &["foo_FOO_123"]),
        ("import foo.bar", &["foo.bar"]),
        ("import foo.bar.baz", &["foo.bar.baz"]),
        ("import foo, bar, bax", &["foo", "bar", "bax"]),
        ("import foo as FOO", &["foo"]),
        ("import foo as FOO, bar as BAR", &["foo", "bar"]),
        ("import  foo  as  FOO ,  bar  as  BAR", &["foo", "bar"]),
        ("import foo # Comment", &["foo"]),
    })]
    fn test_parse_import_statement(case: (&str, &[&str])) {
        parse_and_check(case);
    }

    #[parameterized(case = {
        ("from foo import bar", &["foo.bar"]),
        ("from foo import bar_BAR_123", &["foo.bar_BAR_123"]),
        ("from .foo import bar", &[".foo.bar"]),
        ("from ..foo import bar", &["..foo.bar"]),
        ("from . import foo", &[".foo"]),
        ("from .. import foo", &["..foo"]),
        ("from foo.bar import baz", &["foo.bar.baz"]),
        ("from .foo.bar import baz", &[".foo.bar.baz"]),
        ("from ..foo.bar import baz", &["..foo.bar.baz"]),
        ("from foo import bar, baz, bax", &["foo.bar", "foo.baz", "foo.bax"]),
        ("from foo import bar as BAR", &["foo.bar"]),
        ("from foo import bar as BAR, baz as BAZ", &["foo.bar", "foo.baz"]),
        ("from  foo  import  bar  as  BAR ,  baz  as  BAZ", &["foo.bar", "foo.baz"]),
        ("from foo import bar # Comment", &["foo.bar"]),
    })]
    fn test_parse_from_import_statement(case: (&str, &[&str])) {
        parse_and_check(case);
    }

    #[parameterized(case = {
        ("from foo import (bar)", &["foo.bar"]),
        ("from foo import (bar,)", &["foo.bar"]),
        ("from foo import (bar, baz)", &["foo.bar", "foo.baz"]),
        ("from foo import (bar, baz,)", &["foo.bar", "foo.baz"]),
        ("from foo import (bar as BAR, baz as BAZ,)", &["foo.bar", "foo.baz"]),
        ("from  foo  import  ( bar  as  BAR , baz  as  BAZ , )", &["foo.bar", "foo.baz"]),
        ("from foo import (bar, baz,) # Comment", &["foo.bar", "foo.baz"]),

        (r#"
from foo import (
    bar,
    baz
)
        "#, &["foo.bar", "foo.baz"]),

        (r#"
from foo import (
    bar,
    baz,
)
        "#, &["foo.bar", "foo.baz"]),

        (r#"
from foo import (
    a, b,
    c, d,
)
        "#, &["foo.a", "foo.b", "foo.c", "foo.d"]),

        // As name
        (r#"
from foo import (
    bar as BAR,
    baz as BAZ,
)
        "#, &["foo.bar", "foo.baz"]),

        // Whitespace
        (r#"
from  foo  import  (

    bar  as  BAR ,

       baz  as  BAZ ,

)
        "#, &["foo.bar", "foo.baz"]),

        // Comments
        (r#"
from foo import ( # C
    # C
    bar as BAR, # C
    # C
    baz as BAZ, # C
    # C
) # C
        "#, &["foo.bar", "foo.baz"]),
    })]
    fn test_parse_multiline_from_import_statement(case: (&str, &[&str])) {
        parse_and_check(case);
    }

    #[parameterized(case = {
        ("from foo import *", &["foo.*"]),
        ("from .foo import *", &[".foo.*"]),
        ("from ..foo import *", &["..foo.*"]),
        ("from . import *", &[".*"]),
        ("from .. import *", &["..*"]),
        ("from  foo  import  *", &["foo.*"]),
        ("from foo import * # Comment", &["foo.*"]),
    })]
    fn test_parse_wildcard_from_import_statement(case: (&str, &[&str])) {
        parse_and_check(case);
    }

    #[parameterized(case = {
        ("import a; import b", &["a", "b"]),
        ("import a; import b;", &["a", "b"]),
        ("import  a ;  import  b ;", &["a", "b"]),
        ("import a; import b # Comment", &["a", "b"]),
        ("import a; from b import c; from d import (e); from f import *", &["a", "b.c", "d.e", "f.*"]),
    })]
    fn test_parse_import_statement_list(case: (&str, &[&str])) {
        parse_and_check(case);
    }

    #[parameterized(case = {
        (r#"
import a, b, \
       c, d
        "#, &["a", "b", "c", "d"]),

        (r#"
from foo import a, b, \
                c, d
        "#, &["foo.a", "foo.b", "foo.c", "foo.d"]),

        (r#"
from foo \
    import *
        "#, &["foo.*"]),
    })]
    fn test_backslash_continuation(case: (&str, &[&str])) {
        parse_and_check(case);
    }

    #[parameterized(case = {
        (r#"
import a
def foo():
    import b
import c
        "#, &["a", "b", "c"]),

        (r#"
import a
class Foo:
    import b
import c
        "#, &["a", "b", "c"]),
    })]
    fn test_parse_nested_imports(case: (&str, &[&str])) {
        parse_and_check(case);
    }

    #[parameterized(case = {
        (r#"
import foo
if typing.TYPE_CHECKING: import bar
import baz
"#, &[("foo", false), ("bar", true), ("baz", false)]),

        (r#"
import foo
if TYPE_CHECKING: import bar
import baz
"#, &[("foo", false), ("bar", true), ("baz", false)]),

        (r#"
import foo
if t.TYPE_CHECKING: import bar
import baz
"#, &[("foo", false), ("bar", true), ("baz", false)]),

        (r#"
import foo
if some_WE1RD_alias.TYPE_CHECKING: import bar
import baz
"#, &[("foo", false), ("bar", true), ("baz", false)]),

        (r#"
import foo
if  TYPE_CHECKING :  import bar
import baz
"#, &[("foo", false), ("bar", true), ("baz", false)]),

        (r#"
import foo
if TYPE_CHECKING: import bar as BAR
import baz
"#, &[("foo", false), ("bar", true), ("baz", false)]),

        (r#"
import foo # C
if TYPE_CHECKING: import bar # C
import baz # C
"#, &[("foo", false), ("bar", true), ("baz", false)]),
    })]
    fn test_singleline_if_typechecking(case: (&str, &[(&str, bool)])) {
        parse_and_check_with_typechecking_only(case);
    }

    #[parameterized(case = {
        (r#"
import foo
if typing.TYPE_CHECKING:
    import bar
import baz
"#, &[("foo", false), ("bar", true), ("baz", false)]),

        (r#"
import foo
if TYPE_CHECKING:
    import bar
import baz
"#, &[("foo", false), ("bar", true), ("baz", false)]),

        (r#"
import  foo

if  TYPE_CHECKING :

    import  bar

import  baz
"#, &[("foo", false), ("bar", true), ("baz", false)]),

        (r#"
import foo
if TYPE_CHECKING:
    import bar as BAR
import baz
"#, &[("foo", false), ("bar", true), ("baz", false)]),

        (r#"
import foo # C
if TYPE_CHECKING: # C
    # C
    import bar # C
    # C
import baz # C
"#, &[("foo", false), ("bar", true), ("baz", false)]),

        (r#"
import foo
if TYPE_CHECKING:
    """
    Comment
    """
    import bar
import baz
"#, &[("foo", false), ("bar", true), ("baz", false)]),
    })]
    fn test_multiline_if_typechecking(case: (&str, &[(&str, bool)])) {
        parse_and_check_with_typechecking_only(case);
    }

    #[parameterized(case = {
        (r#"
import foo
"""
import bar
"""
import baz
"#, &["foo", "baz"]),

        (r#"
import foo
"""
import bar
""" # foo
import baz
"#, &["foo", "baz"]),

        (r#"
import foo
'''
import bar
'''
import baz
"#, &["foo", "baz"]),
    })]
    fn test_multiline_strings(case: (&str, &[&str])) {
        parse_and_check(case);
    }

    #[test]
    fn test_parse_line_numbers() {
        let imports = parse_imports(
            "
import a
from b import c
from d import (e)
from f import *",
        )
            .unwrap();
        assert_eq!(
            vec![
                ("a".to_owned(), 2_u32),
                ("b.c".to_owned(), 3_u32),
                ("d.e".to_owned(), 4_u32),
                ("f.*".to_owned(), 5_u32),
            ],
            imports
                .into_iter()
                .map(|i| (i.imported_object, i.line_number))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_parse_line_numbers_if_typechecking() {
        let imports = parse_imports(
            "
import a
if TYPE_CHECKING:
    from b import c
from d import (e)
if TYPE_CHECKING:
    from f import *",
        )
            .unwrap();
        assert_eq!(
            vec![
                ("a".to_owned(), 2_u32, false),
                ("b.c".to_owned(), 4_u32, true),
                ("d.e".to_owned(), 5_u32, false),
                ("f.*".to_owned(), 7_u32, true),
            ],
            imports
                .into_iter()
                .map(|i| (i.imported_object, i.line_number, i.typechecking_only))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_parse_line_contents() {
        let imports = parse_imports(
            "
import a
import a.b  # Comment afterwards.
from b import c
from d import (e)
from f import *
from something.foo import *  # Comment afterwards.
if True:
    from indented import foo
from ñon_ascii_变 import ラーメン
",
        )
            .unwrap();
        assert_eq!(
            vec![
                ("a".to_owned(), "import a".to_owned()),
                ("a.b".to_owned(), "import a.b  # Comment afterwards.".to_owned()),
                ("b.c".to_owned(), "from b import c".to_owned()),
                ("d.e".to_owned(), "from d import (e)".to_owned()),
                ("f.*".to_owned(), "from f import *".to_owned()),
                ("something.foo.*".to_owned(), "from something.foo import *  # Comment afterwards.".to_owned()),
                ("indented.foo".to_owned(), "from indented import foo".to_owned()),
                ("ñon_ascii_变.ラーメン".to_owned(), "from ñon_ascii_变 import ラーメン".to_owned()),
            ],
            imports
                .into_iter()
                .map(|i| (i.imported_object, i.line_contents))
                .collect::<Vec<_>>()
        );
    }
}
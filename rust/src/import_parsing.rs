use crate::errors::{GrimpError, GrimpResult};
use ruff_python_ast::statement_visitor::{StatementVisitor, walk_body, walk_stmt};
use ruff_python_ast::{Expr, Stmt};
use ruff_python_parser::parse_module;
use ruff_source_file::{LineIndex, SourceCode};
use std::fs;
use std::path::Path;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ImportedObject {
    pub name: String,
    pub line_number: usize,
    pub line_contents: String,
    pub typechecking_only: bool,
}

impl ImportedObject {
    fn new(
        name: String,
        line_number: usize,
        line_contents: String,
        typechecking_only: bool,
    ) -> Self {
        Self {
            name,
            line_number,
            line_contents,
            typechecking_only,
        }
    }
}

pub fn parse_imports(path: &Path) -> GrimpResult<Vec<ImportedObject>> {
    let code = fs::read_to_string(path).expect("failed to read file");
    parse_imports_from_code(&code)
}

pub fn parse_imports_from_code(code: &str) -> GrimpResult<Vec<ImportedObject>> {
    let line_index = LineIndex::from_source_text(code);
    let source_code = SourceCode::new(code, &line_index);

    let ast = match parse_module(code) {
        Ok(ast) => ast,
        Err(e) => {
            let location_index = source_code.line_index(e.location.start());
            let line_number = location_index.get();
            let text = source_code.line_text(location_index).trim();
            Err(GrimpError::ParseError {
                line_number,
                text: text.to_owned(),
                parse_error: e,
            })?
        }
    };

    let mut visitor = Visitor::new(source_code);
    walk_body(&mut visitor, &ast.syntax().body);

    Ok(visitor.imported_objects)
}

#[derive(Debug)]
struct Visitor<'a> {
    source_code: SourceCode<'a, 'a>,
    pub imported_objects: Vec<ImportedObject>,
    pub typechecking_only: bool,
}

impl<'a> Visitor<'a> {
    fn new(source_code: SourceCode<'a, 'a>) -> Self {
        Self {
            source_code,
            imported_objects: vec![],
            typechecking_only: false,
        }
    }
}

impl<'a> StatementVisitor<'a> for Visitor<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::Import(import_stmt) => {
                let line_number = self.source_code.line_index(import_stmt.range.start());
                for name in import_stmt.names.iter() {
                    self.imported_objects.push(ImportedObject::new(
                        name.name.id.clone(),
                        line_number.get(),
                        self.source_code.line_text(line_number).trim().to_string(),
                        self.typechecking_only,
                    ))
                }
                walk_stmt(self, stmt);
            }
            Stmt::ImportFrom(import_from_stmt) => {
                let line_number = self.source_code.line_index(import_from_stmt.range.start());
                for name in import_from_stmt.names.iter() {
                    let imported_object_name = match import_from_stmt.module {
                        Some(ref module) => {
                            format!(
                                "{}{}.{}",
                                ".".repeat(import_from_stmt.level as usize),
                                &module.id,
                                &name.name.id
                            )
                        }
                        None => {
                            format!(
                                "{}{}",
                                ".".repeat(import_from_stmt.level as usize),
                                &name.name.id
                            )
                        }
                    };
                    self.imported_objects.push(ImportedObject::new(
                        imported_object_name,
                        line_number.get(),
                        self.source_code.line_text(line_number).trim().to_string(),
                        self.typechecking_only,
                    ))
                }
                walk_stmt(self, stmt);
            }
            Stmt::If(if_stmt) => match if_stmt.test.as_ref() {
                Expr::Name(expr) => {
                    if expr.id == "TYPE_CHECKING" {
                        self.typechecking_only = true;
                        walk_stmt(self, stmt);
                        self.typechecking_only = false;
                    } else {
                        walk_stmt(self, stmt);
                    }
                }
                Expr::Attribute(expr) => {
                    if expr.attr.id == "TYPE_CHECKING" {
                        self.typechecking_only = true;
                        walk_stmt(self, stmt);
                        self.typechecking_only = false;
                    } else {
                        walk_stmt(self, stmt);
                    }
                }
                _ => {
                    walk_stmt(self, stmt);
                }
            },
            _ => {
                walk_stmt(self, stmt);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse_imports_from_code;
    use parameterized::parameterized;

    #[test]
    fn test_parse_empty_string() {
        let imports = parse_imports_from_code("").unwrap();
        assert!(imports.is_empty());
    }

    fn parse_and_check(case: (&str, &[&str])) {
        let (code, expected_imports) = case;
        let imports = parse_imports_from_code(code).unwrap();
        assert_eq!(
            expected_imports,
            imports.into_iter().map(|i| i.name).collect::<Vec<_>>()
        );
    }

    fn parse_and_check_with_typechecking_only(case: (&str, &[(&str, bool)])) {
        let (code, expected_imports) = case;
        let imports = parse_imports_from_code(code).unwrap();
        assert_eq!(
            expected_imports
                .iter()
                .map(|i| (i.0.to_owned(), i.1))
                .collect::<Vec<_>>(),
            imports
                .into_iter()
                .map(|i| (i.name, i.typechecking_only))
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
# import bar
import baz
"#, &["foo", "baz"]),
    })]
    fn test_comments(case: (&str, &[&str])) {
        parse_and_check(case);
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

        (r#"
import foo
s = """
import bar
"""
import baz
"#, &["foo", "baz"]),

        (r#"
import foo
s = '''
import bar
'''
import baz
"#, &["foo", "baz"]),
    })]
    fn test_multiline_strings(case: (&str, &[&str])) {
        parse_and_check(case);
    }

    #[parameterized(case = {
        (r#"
import foo
x = 42 # """
import bar
"#, &["foo", "bar"]),

(r#"
import foo
print('"""')
import bar
"#, &["foo", "bar"]),

        (r#"
import foo
x = 42 # """
import bar
x = 42 # """
import baz
"#, &["foo", "bar", "baz"]),
    })]
    fn test_weird_inputs(case: (&str, &[&str])) {
        parse_and_check(case);
    }

    #[test]
    fn test_parse_line_numbers() {
        let imports = parse_imports_from_code(
            "
import a
from b import c
from d import (e)
from f import *",
        )
        .unwrap();
        assert_eq!(
            vec![
                ("a".to_owned(), 2),
                ("b.c".to_owned(), 3),
                ("d.e".to_owned(), 4),
                ("f.*".to_owned(), 5),
            ],
            imports
                .into_iter()
                .map(|i| (i.name, i.line_number))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_parse_line_numbers_if_typechecking() {
        let imports = parse_imports_from_code(
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
                ("a".to_owned(), 2, false),
                ("b.c".to_owned(), 4, true),
                ("d.e".to_owned(), 5, false),
                ("f.*".to_owned(), 7, true),
            ],
            imports
                .into_iter()
                .map(|i| (i.name, i.line_number, i.typechecking_only))
                .collect::<Vec<_>>()
        );
    }
}

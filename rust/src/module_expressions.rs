use crate::errors::{GrimpError, GrimpResult};
use const_format::formatcp;
use itertools::Itertools;
use lazy_static::lazy_static;
use regex::Regex;
use std::fmt::Display;
use std::str::FromStr;

lazy_static! {
    static ref MODULE_EXPRESSION_PATTERN: Regex =
        Regex::new(r"^(\w+|\*{1,2})(\.(\w+|\*{1,2}))*$").unwrap();
}

/// A module expression is used to refer to sets of modules.
///
/// - `*` stands in for a module name, without including subpackages.
/// - `**` includes subpackages too.
#[derive(Debug, Clone)]
pub struct ModuleExpression {
    expression: String,
    pattern: Regex,
}

impl Display for ModuleExpression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.expression)
    }
}

impl AsRef<str> for ModuleExpression {
    fn as_ref(&self) -> &str {
        &self.expression
    }
}

impl FromStr for ModuleExpression {
    type Err = GrimpError;

    fn from_str(expression: &str) -> Result<Self, Self::Err> {
        if !MODULE_EXPRESSION_PATTERN.is_match(expression) {
            return Err(GrimpError::InvalidModuleExpression(expression.to_owned()));
        }

        for (part, next_part) in expression.split(".").tuple_windows() {
            match (part, next_part) {
                ("*", "**") | ("**", "*") | ("**", "**") => {
                    return Err(GrimpError::InvalidModuleExpression(expression.to_owned()))
                }
                _ => {}
            }
        }

        Ok(Self {
            expression: expression.to_owned(),
            pattern: Self::create_pattern(expression)?,
        })
    }
}

const MODULE_NAME_PATTERN: &str = r"[^\.]+";
const ONE_OR_MANY_MODULE_NAMES_PATTERN: &str =
    formatcp!(r"{module}(\.{module})*?", module = MODULE_NAME_PATTERN);

impl ModuleExpression {
    pub fn is_match(&self, module_name: &str) -> bool {
        self.pattern.is_match(module_name)
    }

    fn create_pattern(expression: &str) -> GrimpResult<Regex> {
        let mut pattern_parts = vec![];

        for part in expression.split(".") {
            if part == "*" {
                pattern_parts.push(part.replace("*", MODULE_NAME_PATTERN))
            } else if part == "**" {
                pattern_parts.push(part.replace("**", ONE_OR_MANY_MODULE_NAMES_PATTERN));
            } else {
                pattern_parts.push(part.to_owned());
            }
        }

        Ok(Regex::new(&(r"^".to_owned() + &pattern_parts.join(r"\.") + r"$")).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use derive_new::new;
    use parameterized::parameterized;

    #[derive(Debug, new)]
    struct NewModuleExpressionTestCase {
        #[new(into)]
        expression: String,
        expect_valid: bool,
    }

    #[parameterized(case = {
        // Valid
        NewModuleExpressionTestCase::new(r"foo", true),
        NewModuleExpressionTestCase::new(r"foo_bar_123", true),
        NewModuleExpressionTestCase::new(r"foo.bar", true),
        NewModuleExpressionTestCase::new(r"foo.*", true),
        NewModuleExpressionTestCase::new(r"foo.**", true),
        NewModuleExpressionTestCase::new(r"foo.*.bar", true),
        NewModuleExpressionTestCase::new(r"foo.**.bar", true),
        NewModuleExpressionTestCase::new(r"*.foo", true),
        NewModuleExpressionTestCase::new(r"**.foo", true),
        NewModuleExpressionTestCase::new(r"foo.*.bar.**", true),
        NewModuleExpressionTestCase::new(r"foo.**.bar.*", true),
        NewModuleExpressionTestCase::new(r"foo.*.*.bar", true),
        // Invalid
        NewModuleExpressionTestCase::new(r"foo.bar*", false),
        NewModuleExpressionTestCase::new(r".foo", false),
        NewModuleExpressionTestCase::new(r"foo.", false),
        NewModuleExpressionTestCase::new(r"foo..bar", false),
        NewModuleExpressionTestCase::new(r"foo.***", false),
        NewModuleExpressionTestCase::new(r"foo ", false),
        NewModuleExpressionTestCase::new(r"foo .bar", false),
        NewModuleExpressionTestCase::new(r"foo. *.bar", false),
        NewModuleExpressionTestCase::new(r"foo.*.**.bar", false),
        NewModuleExpressionTestCase::new(r"foo.**.*.bar", false),
        NewModuleExpressionTestCase::new(r"foo.**.**.bar", false),
    })]
    fn test_parse(case: NewModuleExpressionTestCase) -> GrimpResult<()> {
        let module_expression = case.expression.parse::<ModuleExpression>();
        if case.expect_valid {
            assert!(module_expression.is_ok());
        } else {
            assert!(module_expression.is_err());
            assert!(matches!(
                module_expression.unwrap_err(),
                GrimpError::InvalidModuleExpression(_)
            ))
        }
        Ok(())
    }

    #[derive(Debug, new)]
    struct MatchesModuleNameTestCase {
        #[new(into)]
        expression: String,
        #[new(into)]
        module_name: String,
        expect_match: bool,
    }

    #[parameterized(case = {
        // Exact match.
        MatchesModuleNameTestCase::new(r"foo", "foo", true),
        MatchesModuleNameTestCase::new(r"foo", "bar", false),
        // Exact match with dot.
        MatchesModuleNameTestCase::new(r"foo.bar", "foo.bar", true),
        MatchesModuleNameTestCase::new(r"foo.bar", "foo.baz", false),
        // Single wildcard at end
        MatchesModuleNameTestCase::new(r"foo.*", "foo.bar", true),
        MatchesModuleNameTestCase::new(r"foo.*", "foo", false),
        MatchesModuleNameTestCase::new(r"foo.*", "foo.bar.baz", false),
        // Double wildcard at end
        MatchesModuleNameTestCase::new(r"foo.**", "foo.bar", true),
        MatchesModuleNameTestCase::new(r"foo.**", "foo", false),
        MatchesModuleNameTestCase::new(r"foo.**", "foo.bar.baz", true),
        // Single wildcard in the middle
        MatchesModuleNameTestCase::new(r"foo.*.baz", "foo.bar.baz", true),
        MatchesModuleNameTestCase::new(r"foo.*.baz", "foo.bar.bax.baz", false),
        // Double wildcard in the middle
        MatchesModuleNameTestCase::new(r"foo.**.baz", "foo.bar.baz", true),
        MatchesModuleNameTestCase::new(r"foo.**.baz", "foo.bar.bax.baz", true),
        // Single wildcard at start
        MatchesModuleNameTestCase::new(r"*.foo", "bar.foo", true),
        MatchesModuleNameTestCase::new(r"*.foo", "foo", false),
        MatchesModuleNameTestCase::new(r"*.foo", "bar.baz.foo", false),
        // Double wildcard at start
        MatchesModuleNameTestCase::new(r"**.foo", "bar.foo", true),
        MatchesModuleNameTestCase::new(r"**.foo", "foo", false),
        MatchesModuleNameTestCase::new(r"**.foo", "bar.baz.foo", true),
        // Multiple single wildcards
        MatchesModuleNameTestCase::new(r"foo.*.*.bar", "foo.a.b.bar", true),
        MatchesModuleNameTestCase::new(r"foo.*.*.bar", "foo.a.bar", false),
        MatchesModuleNameTestCase::new(r"foo.*.*.bar", "foo.a.b.c.bar", false),
        // Mixing single and double wildcards
        MatchesModuleNameTestCase::new(r"foo.**.bar.*", "foo.a.bar.b", true),
        MatchesModuleNameTestCase::new(r"foo.**.bar.*", "foo.a.b.bar.c", true),
        MatchesModuleNameTestCase::new(r"foo.**.bar.*", "foo.bar", false),
        MatchesModuleNameTestCase::new(r"foo.**.bar.*", "foo.a.bar.b.c", false),
    })]
    fn test_is_match(case: MatchesModuleNameTestCase) -> GrimpResult<()> {
        let module_expression: ModuleExpression = case.expression.parse()?;
        assert_eq!(
            module_expression.is_match(&case.module_name),
            case.expect_match
        );
        Ok(())
    }
}

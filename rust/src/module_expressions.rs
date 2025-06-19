use crate::errors::{GrimpError, GrimpResult};
use const_format::formatcp;
use itertools::Itertools;
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashSet;
use std::fmt::Display;
use std::str::FromStr;
use tap::Tap;

lazy_static! {
    static ref MODULE_EXPRESSION_PATTERN: Regex =
        Regex::new(r"^(\w+|\*{1,2})(\.(\w+|\*{1,2}))*$").unwrap();
    static ref OPTIONAL_FRAGMENT_PATTERN: Regex = Regex::new(r"\[([^\[\]]*)\]").unwrap();
}

/// A module expression is used to refer to sets of modules.
///
/// - `*` stands in for a module name, without including subpackages.
/// - `**` includes subpackages too.
/// - `[...]` denotes an optional fragment that may or may not be present.
///   For example, `a.b[.**]` matches both `a.b` and `a.b.**`.
#[derive(Debug, Clone)]
pub struct ModuleExpression {
    expression: String,
    patterns: Vec<Regex>,
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
        // Handle optional fragments by expanding into all possible combinations
        let expanded_expressions = Self::expand_optional_fragments(expression);

        let mut patterns = Vec::new();
        for expanded in &expanded_expressions {
            // Validate each expanded expression
            if !MODULE_EXPRESSION_PATTERN.is_match(expanded) {
                return Err(GrimpError::InvalidModuleExpression(expanded.to_owned()));
            }
            for (part, next_part) in expanded.split(".").tuple_windows() {
                match (part, next_part) {
                    ("*", "**") | ("**", "*") | ("**", "**") => {
                        return Err(GrimpError::InvalidModuleExpression(expanded.to_owned()));
                    }
                    _ => {}
                }
            }
            patterns.push(Self::create_pattern(expanded)?);
        }

        Ok(Self {
            expression: expression.to_owned(),
            patterns,
        })
    }
}

const MODULE_NAME_PATTERN: &str = r"[^\.]+";
const ONE_OR_MANY_MODULE_NAMES_PATTERN: &str =
    formatcp!(r"{module}(\.{module})*?", module = MODULE_NAME_PATTERN);

impl ModuleExpression {
    pub fn is_match(&self, module_name: &str) -> bool {
        self.patterns
            .iter()
            .any(|pattern| pattern.is_match(module_name))
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

    /// Expands an expression with optional fragments into all possible combinations.
    ///
    /// For example:
    /// - a.b[.c] expands to ["a.b", "a.b.c"]
    /// - a[.b[.c]] expands to ["a", "a.b", "a.b.c"]
    fn expand_optional_fragments(expression: &str) -> HashSet<String> {
        if !OPTIONAL_FRAGMENT_PATTERN.is_match(expression) {
            // No optional fragments, return the original expression
            return [expression.to_owned()].iter().cloned().collect();
        }

        let mut result = HashSet::new();

        // Find the first optional fragment
        if let Some(captures) = OPTIONAL_FRAGMENT_PATTERN.captures(expression) {
            let full_match = captures.get(0).unwrap();
            let content = captures.get(1).unwrap();

            // Create the expression without this optional fragment
            let without_optional = expression
                .to_owned()
                .tap_mut(|s| s.replace_range(full_match.range(), ""));
            let without_optional_expanded = Self::expand_optional_fragments(&without_optional);
            result.extend(without_optional_expanded);

            // Create the expression with this optional fragment
            let with_optional = expression
                .to_owned()
                .tap_mut(|s| s.replace_range(full_match.range(), content.as_str()));
            let with_optional_expanded = Self::expand_optional_fragments(&with_optional);
            result.extend(with_optional_expanded);
        }

        result
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
        NewModuleExpressionTestCase::new(r"foo[.bar]", true),
        NewModuleExpressionTestCase::new(r"[foo.]bar", true),
        NewModuleExpressionTestCase::new(r"foo[.bar[.baz]]", true),
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
        NewModuleExpressionTestCase::new(r"foo[.bar.baz", false),
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

    #[parameterized(case = {
        // Optional fragments
        MatchesModuleNameTestCase::new(r"a.b[.c]", r"a.b", true),
        MatchesModuleNameTestCase::new(r"a.b[.c]", r"a.b.c", true),
        MatchesModuleNameTestCase::new(r"a.b[.c]", r"a.b.d", false),
        // Optional fragments with wildcards
        MatchesModuleNameTestCase::new(r"a.b[.**]", r"a.b", true),
        MatchesModuleNameTestCase::new(r"a.b[.**]", r"a.b.c", true),
        MatchesModuleNameTestCase::new(r"a.b[.**]", r"a.b.c.d", true),
        // Multiple optional fragments
        MatchesModuleNameTestCase::new(r"a[.b].c[.d]", r"a.c", true),
        MatchesModuleNameTestCase::new(r"a[.b].c[.d]", r"a.b.c", true),
        MatchesModuleNameTestCase::new(r"a[.b].c[.d]", r"a.c.d", true),
        MatchesModuleNameTestCase::new(r"a[.b].c[.d]", r"a.b.c.d", true),
        MatchesModuleNameTestCase::new(r"a[.b].c[.d]", r"a.b.c.d.e", false),
        // Optional at start
        MatchesModuleNameTestCase::new(r"[a.]b.c", r"b.c", true),
        MatchesModuleNameTestCase::new(r"[a.]b.c", r"a.b.c", true),
        MatchesModuleNameTestCase::new(r"[a.]b.c", r"a.a.b.c", false),
        // Nested optional fragments
        MatchesModuleNameTestCase::new(r"a[.b[.c]]", r"a", true),
        MatchesModuleNameTestCase::new(r"a[.b[.c]]", r"a.b", true),
        MatchesModuleNameTestCase::new(r"a[.b[.c]]", r"a.b.c", true),
        MatchesModuleNameTestCase::new(r"a[.b[.c]]", r"a.b.c.d", false),
    })]
    fn test_optional_fragments(case: MatchesModuleNameTestCase) -> GrimpResult<()> {
        let module_expression: ModuleExpression = case.expression.parse()?;
        assert_eq!(
            module_expression.is_match(&case.module_name),
            case.expect_match
        );
        Ok(())
    }
}

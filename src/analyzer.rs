use std::path::Path;

use anyhow::{Result, anyhow};
use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType, Span};
use oxc_syntax::scope::ScopeFlags;

#[derive(Debug, Clone)]
pub(crate) struct FunctionMetric {
    pub name: String,
    pub line: u32,
    pub complexity: u32,
    pub span: Span,
    pub body_span: Span,
}

struct ActiveFunction {
    name: String,
    line: u32,
    complexity: u32,
    span: Span,
    body_span: Span,
}

struct ComplexityVisitor<'source> {
    source: &'source str,
    active: Vec<ActiveFunction>,
    completed: Vec<FunctionMetric>,
    name_hint: Option<String>,
}

pub(crate) fn analyze_source(path: &Path, source: &str) -> Result<Vec<FunctionMetric>> {
    let source_type = SourceType::from_path(path)
        .map_err(|error| anyhow!("unsupported source file {}: {error}", path.display()))?;
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, source_type).parse();
    if !parsed.diagnostics.is_empty() {
        return Err(anyhow!(
            "failed to parse {}: {:?}",
            path.display(),
            parsed.diagnostics
        ));
    }

    let mut visitor = ComplexityVisitor {
        source,
        active: Vec::new(),
        completed: Vec::new(),
        name_hint: None,
    };
    visitor.visit_program(&parsed.program);
    visitor
        .completed
        .sort_by_key(|function| function.span.start);
    Ok(visitor.completed)
}

impl ComplexityVisitor<'_> {
    fn increment(&mut self) {
        if let Some(function) = self.active.last_mut() {
            function.complexity += 1;
        }
    }

    fn begin_function(&mut self, explicit_name: Option<&str>, span: Span, body_span: Span) {
        let (line, _) = line_column(self.source, span.start);
        let name = explicit_name
            .map(str::to_owned)
            .or_else(|| self.name_hint.take())
            .or_else(|| infer_name(self.source, span.start as usize))
            .unwrap_or_else(|| format!("<anonymous@{line}>"));
        self.active.push(ActiveFunction {
            name,
            line,
            complexity: 1,
            span,
            body_span,
        });
    }

    fn end_function(&mut self) {
        let Some(function) = self.active.pop() else {
            return;
        };
        self.completed.push(FunctionMetric {
            name: function.name,
            line: function.line,
            complexity: function.complexity,
            span: function.span,
            body_span: function.body_span,
        });
    }
}

impl<'ast> Visit<'ast> for ComplexityVisitor<'_> {
    fn visit_method_definition(&mut self, method: &MethodDefinition<'ast>) {
        let previous_hint = self.name_hint.take();
        self.name_hint = method.key.static_name().map(|name| name.into_owned());
        walk::walk_method_definition(self, method);
        self.name_hint = previous_hint;
    }

    fn visit_function(&mut self, function: &Function<'ast>, flags: ScopeFlags) {
        let Some(body) = function.body.as_ref() else {
            return;
        };
        let explicit_name = function.id.as_ref().map(|id| id.name.as_str());
        self.begin_function(explicit_name, function.span, body.span);
        walk::walk_function(self, function, flags);
        self.end_function();
    }

    fn visit_arrow_function_expression(&mut self, arrow: &ArrowFunctionExpression<'ast>) {
        self.begin_function(None, arrow.span, arrow.body.span());
        walk::walk_arrow_function_expression(self, arrow);
        self.end_function();
    }

    fn visit_if_statement(&mut self, statement: &IfStatement<'ast>) {
        self.increment();
        walk::walk_if_statement(self, statement);
    }

    fn visit_do_while_statement(&mut self, statement: &DoWhileStatement<'ast>) {
        self.increment();
        walk::walk_do_while_statement(self, statement);
    }

    fn visit_while_statement(&mut self, statement: &WhileStatement<'ast>) {
        self.increment();
        walk::walk_while_statement(self, statement);
    }

    fn visit_for_statement(&mut self, statement: &ForStatement<'ast>) {
        self.increment();
        walk::walk_for_statement(self, statement);
    }

    fn visit_for_in_statement(&mut self, statement: &ForInStatement<'ast>) {
        self.increment();
        walk::walk_for_in_statement(self, statement);
    }

    fn visit_for_of_statement(&mut self, statement: &ForOfStatement<'ast>) {
        self.increment();
        walk::walk_for_of_statement(self, statement);
    }

    fn visit_switch_case(&mut self, case: &SwitchCase<'ast>) {
        if case.test.is_some() {
            self.increment();
        }
        walk::walk_switch_case(self, case);
    }

    fn visit_catch_clause(&mut self, clause: &CatchClause<'ast>) {
        self.increment();
        walk::walk_catch_clause(self, clause);
    }

    fn visit_conditional_expression(&mut self, expression: &ConditionalExpression<'ast>) {
        self.increment();
        walk::walk_conditional_expression(self, expression);
    }

    fn visit_logical_expression(&mut self, expression: &LogicalExpression<'ast>) {
        self.increment();
        walk::walk_logical_expression(self, expression);
    }

    fn visit_assignment_expression(&mut self, expression: &AssignmentExpression<'ast>) {
        if expression.operator.is_logical() {
            self.increment();
        }
        walk::walk_assignment_expression(self, expression);
    }

    fn visit_assignment_pattern(&mut self, pattern: &AssignmentPattern<'ast>) {
        self.increment();
        walk::walk_assignment_pattern(self, pattern);
    }

    fn visit_formal_parameter(&mut self, parameter: &FormalParameter<'ast>) {
        if parameter.initializer.is_some() {
            self.increment();
        }
        walk::walk_formal_parameter(self, parameter);
    }

    fn visit_chain_expression(&mut self, expression: &ChainExpression<'ast>) {
        self.increment();
        walk::walk_chain_expression(self, expression);
    }
}

fn line_column(source: &str, offset: u32) -> (u32, u32) {
    let prefix = &source[..offset as usize];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() as u32 + 1;
    let column = prefix
        .rsplit_once('\n')
        .map_or(prefix.len(), |(_, tail)| tail.len()) as u32;
    (line, column)
}

fn infer_name(source: &str, start: usize) -> Option<String> {
    let line_start = source[..start].rfind('\n').map_or(0, |index| index + 1);
    let prefix = source[line_start..start].trim_end();

    if let Some(separator) = prefix.rfind(['=', ':']) {
        let before = prefix[..separator].trim_end();
        if let Some(name) = trailing_identifier(before) {
            return Some(name.to_owned());
        }
    }

    let suffix = source[start..].trim_start();
    let candidate = suffix
        .strip_prefix("async ")
        .unwrap_or(suffix)
        .strip_prefix("function ")
        .unwrap_or_else(|| suffix.strip_prefix("function").unwrap_or(suffix));
    let name: String = candidate
        .chars()
        .take_while(|character| character.is_alphanumeric() || matches!(character, '_' | '$' | '#'))
        .collect();
    (!name.is_empty()).then_some(name)
}

fn trailing_identifier(value: &str) -> Option<&str> {
    let end = value
        .char_indices()
        .rev()
        .find(|(_, character)| character.is_alphanumeric() || matches!(character, '_' | '$'))?
        .0;
    let tail = &value[..=end];
    let start = tail
        .char_indices()
        .rev()
        .take_while(|(_, character)| character.is_alphanumeric() || matches!(character, '_' | '$'))
        .last()
        .map_or(0, |(index, _)| index);
    Some(&tail[start..])
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::analyze_source;

    #[test]
    fn analyzes_typescript_and_tsx_functions() {
        let source = r#"
export function classify(value: number): string {
  if (value > 10 && value < 20) return "middle";
  return value > 20 ? "high" : "low";
}

export const render = (ready: boolean) => ready ? <div /> : null;

class Picker {
  choose(value: number) {
    switch (value) {
      case 1: return "one";
      case 2: return "two";
      default: return "other";
    }
  }
}
"#;

        let functions = analyze_source(Path::new("sample.tsx"), source).unwrap();
        let summary: Vec<_> = functions
            .iter()
            .map(|function| (function.name.as_str(), function.complexity))
            .collect();

        assert_eq!(summary, vec![("classify", 4), ("render", 2), ("choose", 3)]);
    }
}

use super::ast::{Expression, InfixOperatorKind, PrefixOperatorKind};
use super::error::Result;

pub fn eval(expression: Expression) -> Result<i64> {
    match expression {
        Expression::IntegerLiteral(value) => Ok(value),
        Expression::PrefixOperator(node) => {
            let right = eval(*node.right)?;
            Ok(eval_prefix_expression(node.operator, right)?)
        }
        Expression::InfixOperator(node) => {
            let left = eval(*node.left)?;
            let right = eval(*node.right)?;
            Ok(eval_infix_expression(node.operator, left, right)?)
        }
    }
}

fn eval_prefix_expression(operator: PrefixOperatorKind, right: i64) -> Result<i64> {
    match operator {
        PrefixOperatorKind::Negative => Ok(-right),
    }
}

fn eval_infix_expression(operator: InfixOperatorKind, left: i64, right: i64) -> Result<i64> {
    match operator {
        InfixOperatorKind::Add => Ok(left + right),
        InfixOperatorKind::Subtract => Ok(left - right),
        InfixOperatorKind::Multiply => Ok(left * right),
        InfixOperatorKind::Divide => Ok(left / right),
    }
}

use crate::error::{SqlError, SqlResult};
use crate::parser::ast::*;
use crate::types::{DataType, Value};
use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case, take_while1},
    character::complete::{alpha1, alphanumeric1, char, digit1, multispace0, multispace1},
    combinator::{map, opt, recognize},
    multi::{many0, separated_list0, separated_list1},
    sequence::{delimited, pair, preceded, terminated, tuple},
    IResult,
};

pub struct SqlParser;

impl SqlParser {
    pub fn new() -> Self {
        SqlParser
    }

    pub fn parse(&self, input: &str) -> SqlResult<Statement> {
        match statement(input.trim()) {
            Ok((remaining, stmt)) => {
                if remaining.trim().is_empty() {
                    Ok(stmt)
                } else {
                    Err(SqlError::parse_error(format!(
                        "Unexpected input after statement: {}",
                        remaining
                    )))
                }
            }
            Err(e) => Err(SqlError::parse_error(format!("Parse error: {}", e))),
        }
    }
}

// Main statement parser
fn statement(input: &str) -> IResult<&str, Statement> {
    alt((
        map(select_statement, Statement::Select),
        map(insert_statement, Statement::Insert),
        map(update_statement, Statement::Update),
        map(delete_statement, Statement::Delete),
        map(create_table_statement, Statement::CreateTable),
        map(drop_table_statement, Statement::DropTable),
        map(create_index_statement, Statement::CreateIndex),
        map(drop_index_statement, Statement::DropIndex),
    ))(input)
}

// SELECT statement parser
fn select_statement(input: &str) -> IResult<&str, SelectStatement> {
    let (input, _) = ws(tag_no_case("SELECT"))(input)?;
    let (input, columns) = select_columns(input)?;
    let (input, from) = opt(from_clause)(input)?;
    let (input, where_clause) = opt(where_clause)(input)?;
    let (input, group_by) = opt(group_by_clause)(input)?;
    let (input, having) = opt(having_clause)(input)?;
    let (input, order_by) = opt(order_by_clause)(input)?;
    let (input, limit) = opt(limit_clause)(input)?;

    Ok((
        input,
        SelectStatement {
            columns,
            from,
            where_clause,
            group_by,
            having,
            order_by,
            limit,
        },
    ))
}

// SELECT columns parser
fn select_columns(input: &str) -> IResult<&str, Vec<SelectColumn>> {
    separated_list1(ws(char(',')), select_column)(input)
}

fn select_column(input: &str) -> IResult<&str, SelectColumn> {
    alt((
        map(ws(char('*')), |_| SelectColumn::Wildcard),
        map(
            pair(expression, opt(preceded(ws(tag_no_case("AS")), identifier))),
            |(expr, alias)| SelectColumn::Expression { expr, alias },
        ),
    ))(input)
}

// FROM clause parser
fn from_clause(input: &str) -> IResult<&str, FromClause> {
    let (input, _) = ws(tag_no_case("FROM"))(input)?;
    let (input, table) = identifier(input)?;
    let (input, alias) = opt(preceded(ws(tag_no_case("AS")), identifier))(input)?;
    let (input, joins) = many0(join_clause)(input)?;

    Ok((
        input,
        FromClause {
            source: FromSource::Table(table),
            alias,
            joins,
        },
    ))
}

// JOIN clause parser
fn join_clause(input: &str) -> IResult<&str, JoinClause> {
    let (input, join_type) = join_type(input)?;
    let (input, _) = ws(tag_no_case("JOIN"))(input)?;
    let (input, table) = identifier(input)?;
    let (input, alias) = opt(preceded(ws(tag_no_case("AS")), identifier))(input)?;
    let (input, _) = ws(tag_no_case("ON"))(input)?;
    let (input, condition) = expression(input)?;

    Ok((
        input,
        JoinClause {
            join_type,
            table,
            alias,
            condition,
        },
    ))
}

fn join_type(input: &str) -> IResult<&str, JoinType> {
    alt((
        map(ws(tag_no_case("INNER")), |_| JoinType::Inner),
        map(ws(tag_no_case("LEFT")), |_| JoinType::Left),
        map(ws(tag_no_case("RIGHT")), |_| JoinType::Right),
        map(ws(tag_no_case("FULL")), |_| JoinType::Full),
    ))(input)
}

// WHERE clause parser
fn where_clause(input: &str) -> IResult<&str, Expression> {
    preceded(ws(tag_no_case("WHERE")), expression)(input)
}

// GROUP BY clause parser
fn group_by_clause(input: &str) -> IResult<&str, Vec<Expression>> {
    preceded(
        ws(tuple((tag_no_case("GROUP"), multispace1, tag_no_case("BY")))),
        separated_list1(ws(char(',')), expression),
    )(input)
}

// HAVING clause parser
fn having_clause(input: &str) -> IResult<&str, Expression> {
    preceded(ws(tag_no_case("HAVING")), expression)(input)
}

// ORDER BY clause parser
fn order_by_clause(input: &str) -> IResult<&str, Vec<OrderByClause>> {
    preceded(
        ws(tuple((tag_no_case("ORDER"), multispace1, tag_no_case("BY")))),
        separated_list1(ws(char(',')), order_by_item),
    )(input)
}

fn order_by_item(input: &str) -> IResult<&str, OrderByClause> {
    let (input, expression) = expression(input)?;
    let (input, direction) = opt(alt((
        map(ws(tag_no_case("ASC")), |_| OrderDirection::Asc),
        map(ws(tag_no_case("DESC")), |_| OrderDirection::Desc),
    )))(input)?;

    Ok((
        input,
        OrderByClause {
            expression,
            direction: direction.unwrap_or(OrderDirection::Asc),
        },
    ))
}

// LIMIT clause parser
fn limit_clause(input: &str) -> IResult<&str, LimitClause> {
    let (input, _) = ws(tag_no_case("LIMIT"))(input)?;
    let (input, count) = number(input)?;
    let (input, offset) = opt(preceded(ws(tag_no_case("OFFSET")), number))(input)?;

    Ok((
        input,
        LimitClause {
            count: count as u64,
            offset: offset.map(|o| o as u64),
        },
    ))
}

// INSERT statement parser
fn insert_statement(input: &str) -> IResult<&str, InsertStatement> {
    let (input, _) = ws(tag_no_case("INSERT"))(input)?;
    let (input, _) = ws(tag_no_case("INTO"))(input)?;
    let (input, table) = identifier(input)?;
    let (input, columns) = opt(delimited(
        ws(char('(')),
        separated_list1(ws(char(',')), identifier),
        ws(char(')')),
    ))(input)?;
    let (input, _) = ws(tag_no_case("VALUES"))(input)?;
    let (input, values) = separated_list1(
        ws(char(',')),
        delimited(
            ws(char('(')),
            separated_list1(ws(char(',')), expression),
            ws(char(')')),
        ),
    )(input)?;

    Ok((
        input,
        InsertStatement {
            table,
            columns,
            values,
        },
    ))
}

// UPDATE statement parser
fn update_statement(input: &str) -> IResult<&str, UpdateStatement> {
    let (input, _) = ws(tag_no_case("UPDATE"))(input)?;
    let (input, table) = identifier(input)?;
    let (input, _) = ws(tag_no_case("SET"))(input)?;
    let (input, assignments) = separated_list1(ws(char(',')), assignment)(input)?;
    let (input, where_clause) = opt(where_clause)(input)?;

    Ok((
        input,
        UpdateStatement {
            table,
            assignments,
            where_clause,
        },
    ))
}

fn assignment(input: &str) -> IResult<&str, Assignment> {
    let (input, column) = identifier(input)?;
    let (input, _) = ws(char('='))(input)?;
    let (input, value) = expression(input)?;

    Ok((input, Assignment { column, value }))
}

// DELETE statement parser
fn delete_statement(input: &str) -> IResult<&str, DeleteStatement> {
    let (input, _) = ws(tag_no_case("DELETE"))(input)?;
    let (input, _) = ws(tag_no_case("FROM"))(input)?;
    let (input, table) = identifier(input)?;
    let (input, where_clause) = opt(where_clause)(input)?;

    Ok((input, DeleteStatement { table, where_clause }))
}

// CREATE TABLE statement parser
fn create_table_statement(input: &str) -> IResult<&str, CreateTableStatement> {
    let (input, _) = ws(tag_no_case("CREATE"))(input)?;
    let (input, _) = ws(tag_no_case("TABLE"))(input)?;
    let (input, table_name) = identifier(input)?;
    let (input, _) = ws(char('('))(input)?;
    let (input, columns) = separated_list1(ws(char(',')), column_definition)(input)?;
    let (input, _) = ws(char(')'))(input)?;

    Ok((
        input,
        CreateTableStatement {
            table_name,
            columns,
            constraints: Vec::new(), // TODO: Parse table constraints
        },
    ))
}

fn column_definition(input: &str) -> IResult<&str, ColumnDefinition> {
    let (input, name) = identifier(input)?;
    let (input, data_type) = data_type(input)?;
    let (input, constraints) = many0(column_constraint)(input)?;

    Ok((
        input,
        ColumnDefinition {
            name,
            data_type,
            constraints,
        },
    ))
}

fn data_type(input: &str) -> IResult<&str, DataType> {
    alt((
        map(ws(tag_no_case("INTEGER")), |_| DataType::Integer),
        map(ws(tag_no_case("REAL")), |_| DataType::Real),
        map(ws(tag_no_case("TEXT")), |_| DataType::Text),
        map(ws(tag_no_case("BLOB")), |_| DataType::Blob),
        map(ws(tag_no_case("BOOLEAN")), |_| DataType::Boolean),
    ))(input)
}

fn column_constraint(input: &str) -> IResult<&str, ColumnConstraint> {
    alt((
        map(ws(tuple((tag_no_case("NOT"), multispace1, tag_no_case("NULL")))), |_| ColumnConstraint::NotNull),
        map(ws(tuple((tag_no_case("PRIMARY"), multispace1, tag_no_case("KEY")))), |_| ColumnConstraint::PrimaryKey),
        map(ws(tag_no_case("UNIQUE")), |_| ColumnConstraint::Unique),
        map(preceded(ws(tag_no_case("DEFAULT")), value), |v| ColumnConstraint::Default(v)),
    ))(input)
}

// DROP TABLE statement parser
fn drop_table_statement(input: &str) -> IResult<&str, DropTableStatement> {
    let (input, _) = ws(tag_no_case("DROP"))(input)?;
    let (input, _) = ws(tag_no_case("TABLE"))(input)?;
    let (input, if_exists) = opt(ws(tuple((tag_no_case("IF"), multispace1, tag_no_case("EXISTS")))))(input)?;
    let (input, table_name) = identifier(input)?;

    Ok((
        input,
        DropTableStatement {
            table_name,
            if_exists: if_exists.is_some(),
        },
    ))
}

// CREATE INDEX statement parser
fn create_index_statement(input: &str) -> IResult<&str, CreateIndexStatement> {
    let (input, _) = ws(tag_no_case("CREATE"))(input)?;
    let (input, unique) = opt(ws(tag_no_case("UNIQUE")))(input)?;
    let (input, _) = ws(tag_no_case("INDEX"))(input)?;
    let (input, index_name) = identifier(input)?;
    let (input, _) = ws(tag_no_case("ON"))(input)?;
    let (input, table_name) = identifier(input)?;
    let (input, _) = ws(char('('))(input)?;
    let (input, columns) = separated_list1(ws(char(',')), identifier)(input)?;
    let (input, _) = ws(char(')'))(input)?;

    Ok((
        input,
        CreateIndexStatement {
            index_name,
            table_name,
            columns,
            unique: unique.is_some(),
        },
    ))
}

// DROP INDEX statement parser
fn drop_index_statement(input: &str) -> IResult<&str, DropIndexStatement> {
    let (input, _) = ws(tag_no_case("DROP"))(input)?;
    let (input, _) = ws(tag_no_case("INDEX"))(input)?;
    let (input, if_exists) = opt(ws(tuple((tag_no_case("IF"), multispace1, tag_no_case("EXISTS")))))(input)?;
    let (input, index_name) = identifier(input)?;

    Ok((
        input,
        DropIndexStatement {
            index_name,
            if_exists: if_exists.is_some(),
        },
    ))
}

// Expression parser
fn expression(input: &str) -> IResult<&str, Expression> {
    or_expression(input)
}

fn or_expression(input: &str) -> IResult<&str, Expression> {
    let (input, left) = and_expression(input)?;
    let (input, rights) = many0(pair(ws(tag_no_case("OR")), and_expression))(input)?;

    Ok((
        input,
        rights.into_iter().fold(left, |acc, (_, right)| {
            Expression::BinaryOp {
                left: Box::new(acc),
                op: BinaryOperator::Or,
                right: Box::new(right),
            }
        }),
    ))
}

fn and_expression(input: &str) -> IResult<&str, Expression> {
    let (input, left) = equality_expression(input)?;
    let (input, rights) = many0(pair(ws(tag_no_case("AND")), equality_expression))(input)?;

    Ok((
        input,
        rights.into_iter().fold(left, |acc, (_, right)| {
            Expression::BinaryOp {
                left: Box::new(acc),
                op: BinaryOperator::And,
                right: Box::new(right),
            }
        }),
    ))
}

fn equality_expression(input: &str) -> IResult<&str, Expression> {
    let (input, left) = comparison_expression(input)?;
    let (input, op_right) = opt(pair(
        alt((
            map(ws(char('=')), |_| BinaryOperator::Equal),
            map(ws(tag("!=")), |_| BinaryOperator::NotEqual),
            map(ws(tag("<>")), |_| BinaryOperator::NotEqual),
        )),
        comparison_expression,
    ))(input)?;

    match op_right {
        Some((op, right)) => Ok((
            input,
            Expression::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            },
        )),
        None => Ok((input, left)),
    }
}

fn comparison_expression(input: &str) -> IResult<&str, Expression> {
    let (input, left) = additive_expression(input)?;
    let (input, op_right) = opt(pair(
        alt((
            map(ws(tag("<=")), |_| BinaryOperator::LessThanOrEqual),
            map(ws(tag(">=")), |_| BinaryOperator::GreaterThanOrEqual),
            map(ws(char('<')), |_| BinaryOperator::LessThan),
            map(ws(char('>')), |_| BinaryOperator::GreaterThan),
            map(ws(tag_no_case("LIKE")), |_| BinaryOperator::Like),
            map(ws(tuple((tag_no_case("NOT"), multispace1, tag_no_case("LIKE")))), |_| BinaryOperator::NotLike),
        )),
        additive_expression,
    ))(input)?;

    match op_right {
        Some((op, right)) => Ok((
            input,
            Expression::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            },
        )),
        None => Ok((input, left)),
    }
}

fn additive_expression(input: &str) -> IResult<&str, Expression> {
    let (input, left) = multiplicative_expression(input)?;
    let (input, rights) = many0(pair(
        alt((
            map(ws(char('+')), |_| BinaryOperator::Add),
            map(ws(char('-')), |_| BinaryOperator::Subtract),
            map(ws(tag("||")), |_| BinaryOperator::Concat),
        )),
        multiplicative_expression,
    ))(input)?;

    Ok((
        input,
        rights.into_iter().fold(left, |acc, (op, right)| {
            Expression::BinaryOp {
                left: Box::new(acc),
                op,
                right: Box::new(right),
            }
        }),
    ))
}

fn multiplicative_expression(input: &str) -> IResult<&str, Expression> {
    let (input, left) = unary_expression(input)?;
    let (input, rights) = many0(pair(
        alt((
            map(ws(char('*')), |_| BinaryOperator::Multiply),
            map(ws(char('/')), |_| BinaryOperator::Divide),
            map(ws(char('%')), |_| BinaryOperator::Modulo),
        )),
        unary_expression,
    ))(input)?;

    Ok((
        input,
        rights.into_iter().fold(left, |acc, (op, right)| {
            Expression::BinaryOp {
                left: Box::new(acc),
                op,
                right: Box::new(right),
            }
        }),
    ))
}

fn unary_expression(input: &str) -> IResult<&str, Expression> {
    alt((
        map(
            pair(
                alt((
                    map(ws(char('+')), |_| UnaryOperator::Plus),
                    map(ws(char('-')), |_| UnaryOperator::Minus),
                    map(ws(tag_no_case("NOT")), |_| UnaryOperator::Not),
                )),
                primary_expression,
            ),
            |(op, operand)| Expression::UnaryOp {
                op,
                operand: Box::new(operand),
            },
        ),
        primary_expression,
    ))(input)
}

fn primary_expression(input: &str) -> IResult<&str, Expression> {
    alt((
        map(value, Expression::Literal),
        map(qualified_column, |(table, column)| {
            Expression::QualifiedColumn { table, column }
        }),
        map(identifier, Expression::Column),
        function_call,
        delimited(ws(char('(')), expression, ws(char(')'))),
        is_null_expression,
        is_not_null_expression,
        in_expression,
        between_expression,
    ))(input)
}

fn qualified_column(input: &str) -> IResult<&str, (String, String)> {
    let (input, table) = identifier(input)?;
    let (input, _) = ws(char('.'))(input)?;
    let (input, column) = identifier(input)?;
    Ok((input, (table, column)))
}

fn function_call(input: &str) -> IResult<&str, Expression> {
    let (input, name) = identifier(input)?;
    let (input, _) = ws(char('('))(input)?;
    let (input, args) = separated_list0(ws(char(',')), expression)(input)?;
    let (input, _) = ws(char(')'))(input)?;

    Ok((input, Expression::Function { name, args }))
}

fn is_null_expression(input: &str) -> IResult<&str, Expression> {
    let (input, expr) = primary_expression(input)?;
    let (input, _) = ws(tuple((tag_no_case("IS"), multispace1, tag_no_case("NULL"))))(input)?;
    Ok((input, Expression::IsNull(Box::new(expr))))
}

fn is_not_null_expression(input: &str) -> IResult<&str, Expression> {
    let (input, expr) = primary_expression(input)?;
    let (input, _) = ws(tuple((
        tag_no_case("IS"),
        multispace1,
        tag_no_case("NOT"),
        multispace1,
        tag_no_case("NULL"),
    )))(input)?;
    Ok((input, Expression::IsNotNull(Box::new(expr))))
}

fn in_expression(input: &str) -> IResult<&str, Expression> {
    let (input, expr) = primary_expression(input)?;
    let (input, _) = ws(tag_no_case("IN"))(input)?;
    let (input, _) = ws(char('('))(input)?;
    let (input, list) = separated_list1(ws(char(',')), expression)(input)?;
    let (input, _) = ws(char(')'))(input)?;

    Ok((
        input,
        Expression::In {
            expr: Box::new(expr),
            list,
        },
    ))
}

fn between_expression(input: &str) -> IResult<&str, Expression> {
    let (input, expr) = primary_expression(input)?;
    let (input, _) = ws(tag_no_case("BETWEEN"))(input)?;
    let (input, low) = expression(input)?;
    let (input, _) = ws(tag_no_case("AND"))(input)?;
    let (input, high) = expression(input)?;

    Ok((
        input,
        Expression::Between {
            expr: Box::new(expr),
            low: Box::new(low),
            high: Box::new(high),
        },
    ))
}

// Value parser
fn value(input: &str) -> IResult<&str, Value> {
    alt((
        map(ws(tag_no_case("NULL")), |_| Value::Null),
        map(ws(tag_no_case("TRUE")), |_| Value::Boolean(true)),
        map(ws(tag_no_case("FALSE")), |_| Value::Boolean(false)),
        map(float_number, Value::Real),
        map(integer_number, Value::Integer),
        map(string_literal, Value::Text),
    ))(input)
}

fn integer_number(input: &str) -> IResult<&str, i64> {
    map(ws(digit1), |s: &str| s.parse().unwrap())(input)
}

fn float_number(input: &str) -> IResult<&str, f64> {
    map(
        ws(recognize(tuple((
            digit1,
            char('.'),
            digit1,
            opt(tuple((alt((char('e'), char('E'))), opt(alt((char('+'), char('-')))), digit1))),
        )))),
        |s: &str| s.parse().unwrap(),
    )(input)
}

fn number(input: &str) -> IResult<&str, i64> {
    map(ws(digit1), |s: &str| s.parse().unwrap())(input)
}

fn string_literal(input: &str) -> IResult<&str, String> {
    alt((
        delimited(char('\''), take_while1(|c| c != '\''), char('\'')),
        delimited(char('"'), take_while1(|c| c != '"'), char('"')),
    ))(input)
    .map(|(remaining, s)| (remaining, s.to_string()))
}

// Identifier parser
fn identifier(input: &str) -> IResult<&str, String> {
    map(
        ws(recognize(pair(
            alt((alpha1, tag("_"))),
            many0(alt((alphanumeric1, tag("_")))),
        ))),
        |s: &str| s.to_string(),
    )(input)
}

// Whitespace wrapper
fn ws<'a, F: 'a, O>(inner: F) -> impl FnMut(&'a str) -> IResult<&'a str, O>
where
    F: FnMut(&'a str) -> IResult<&'a str, O>,
{
    delimited(multispace0, inner, multispace0)
}

// Copyright 2020-2021 The Datafuse Authors.
//
// SPDX-License-Identifier: Apache-2.0.

use std::fmt;
use std::sync::Arc;

use common_aggregate_functions::AggregateFunctionFactory;
use common_aggregate_functions::IAggregateFunction;
use common_datavalues::DataField;
use common_datavalues::DataSchemaRef;
use common_datavalues::DataType;
use common_datavalues::DataValue;
use common_exception::ErrorCodes;
use common_exception::Result;
use common_functions::FunctionFactory;

use crate::PlanNode;

#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq)]
pub struct ExpressionPlan {
    pub exprs: Vec<Expression>,
    pub schema: DataSchemaRef,
    pub input: Arc<PlanNode>,
    pub desc: String,
}

impl ExpressionPlan {
    pub fn schema(&self) -> DataSchemaRef {
        self.schema.clone()
    }

    pub fn set_input(&mut self, node: &PlanNode) {
        self.input = Arc::new(node.clone());
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq)]
pub enum Expression {
    /// An expression with a alias name.
    Alias(String, Box<Expression>),
    /// Column name.
    Column(String),
    /// Constant value.
    Literal(DataValue),

    /// A unary expression such as "NOT foo"
    UnaryExpression { op: String, expr: Box<Expression> },

    /// A binary expression such as "age > 40"
    BinaryExpression {
        left: Box<Expression>,
        op: String,
        right: Box<Expression>,
    },

    /// ScalarFunction with a set of arguments.
    /// Note: BinaryFunction is a also kind of functions function
    ScalarFunction { op: String, args: Vec<Expression> },

    /// AggregateFunction with a set of arguments.
    AggregateFunction { op: String, args: Vec<Expression> },

    /// A sort expression, that can be used to sort values.
    Sort {
        /// The expression to sort on
        expr: Box<Expression>,
        /// The direction of the sort
        asc: bool,
        /// Whether to put Nulls before all other data values
        nulls_first: bool,
    },
    /// All fields(*) in a schema.
    Wildcard,
    /// Casts the expression to a given type and will return a runtime error if the expression cannot be cast.
    /// This expression is guaranteed to have a fixed type.
    Cast {
        /// The expression being cast
        expr: Box<Expression>,
        /// The `DataType` the expression will yield
        data_type: DataType,
    },
}

impl Expression {
    pub fn column_name(&self) -> String {
        match self {
            Expression::Alias(name, _expr) => name.clone(),
            _ => format!("{:?}", self),
        }
    }

    pub fn to_data_field(&self, input_schema: &DataSchemaRef) -> Result<DataField> {
        let name = self.column_name();
        self.to_data_type(&input_schema).and_then(|return_type| {
            self.nullable(&input_schema)
                .map(|nullable| DataField::new(&name, return_type, nullable))
        })
    }

    // TODO
    pub fn nullable(&self, _input_schema: &DataSchemaRef) -> Result<bool> {
        Ok(false)
    }

    pub fn to_data_type(&self, input_schema: &DataSchemaRef) -> Result<DataType> {
        match self {
            Expression::Alias(_, expr) => expr.to_data_type(input_schema),
            Expression::Column(s) => Ok(input_schema.field_with_name(s)?.data_type().clone()),
            Expression::Literal(v) => Ok(v.data_type()),
            Expression::BinaryExpression { op, left, right } => {
                let arg_types = vec![
                    left.to_data_type(input_schema)?,
                    right.to_data_type(input_schema)?,
                ];
                let func = FunctionFactory::get(op)?;
                func.return_type(&arg_types)
            }

            Expression::UnaryExpression { op, expr } => {
                let arg_types = vec![expr.to_data_type(input_schema)?];
                let func = FunctionFactory::get(op)?;
                func.return_type(&arg_types)
            }

            Expression::ScalarFunction { op, args } => {
                let mut arg_types = Vec::with_capacity(args.len());
                for arg in args {
                    arg_types.push(arg.to_data_type(input_schema)?);
                }
                let func = FunctionFactory::get(op)?;
                func.return_type(&arg_types)
            }
            Expression::AggregateFunction { op, args } => {
                let mut arg_types = Vec::with_capacity(args.len());
                for arg in args {
                    arg_types.push(arg.to_data_type(input_schema)?);
                }
                let func = AggregateFunctionFactory::get(op)?;
                func.return_type(&arg_types)
            }
            Expression::Wildcard => Result::Err(ErrorCodes::IllegalDataType(
                "Wildcard expressions are not valid to get return type",
            )),
            Expression::Cast { data_type, .. } => Ok(data_type.clone()),
            Expression::Sort { expr, .. } => expr.to_data_type(input_schema),
        }
    }

    pub fn to_aggregate_function(&self) -> Result<Box<dyn IAggregateFunction>> {
        match self {
            Expression::AggregateFunction { op, .. } => AggregateFunctionFactory::get(op),
            _ => Err(ErrorCodes::LogicalError(
                "Expression must be aggregated function",
            )),
        }
    }

    pub fn to_aggregate_function_args(&self) -> Result<Vec<String>> {
        match self {
            Expression::AggregateFunction { args, .. } => {
                let arg_names = args.iter().map(|arg| arg.column_name()).collect::<Vec<_>>();
                Ok(arg_names)
            }
            _ => Err(ErrorCodes::LogicalError(
                "Expression must be aggregated function",
            )),
        }
    }
}

// Also used as expression column name
impl fmt::Debug for Expression {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Expression::Alias(alias, v) => write!(f, "{:?} as {:#}", v, alias),
            Expression::Column(ref v) => write!(f, "{:#}", v),
            Expression::Literal(ref v) => write!(f, "{:#}", v),
            Expression::BinaryExpression { op, left, right } => {
                write!(f, "({:?} {} {:?})", left, op, right,)
            }

            Expression::UnaryExpression { op, expr } => {
                write!(f, "({} {:?})", op, expr)
            }

            Expression::ScalarFunction { op, args } => {
                write!(f, "{}(", op)?;

                for (i, _) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{:?}", args[i],)?;
                }
                write!(f, ")")
            }

            Expression::AggregateFunction { op, args } => {
                write!(f, "{}(", op)?;
                for (i, _) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{:?}", args[i],)?;
                }
                write!(f, ")")
            }

            Expression::Sort { expr, .. } => write!(f, "{:?}", expr),
            Expression::Wildcard => write!(f, "*"),
            Expression::Cast { expr, data_type } => {
                write!(f, "cast({:?} as {:?})", expr, data_type)
            }
        }
    }
}

use std::{
    collections::{HashMap, HashSet},
    fs, io,
    path::Path,
};

use chrono::{FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
use thiserror::Error;
use toml_edit::DocumentMut;

#[derive(Debug)]
pub struct Spec {
    pub types: Vec<TypeSpec>,
    pub rpcs: Vec<RpcSpec>,
    pub produces: Vec<ProduceSpec>,
    pub consumes: Vec<ConsumeSpec>,
    pub tables: Vec<TableSpec>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct EnumVariant {
    pub value: String,
    pub desc: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct StructField {
    pub name: String,
    pub type_: Type,
    pub desc: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Type {
    Bool,
    I16,
    I32,
    U16,
    U32,
    F32,
    F64,
    String,
    Timestamp,
    Alias(String),
    Field(Box<Type>, String),
    Array(Box<Type>),
    Optional(Box<Type>),
    Result(Box<Type>, Box<Type>),
    Enum(Vec<EnumVariant>),
    Struct(Vec<StructField>),
}

#[derive(Debug)]
pub struct TypeVariant {
    pub value: String,
    pub desc: Option<String>,
}

#[derive(Debug)]
pub struct TypeSpec {
    pub name: String,
    pub kind: Type,
    pub desc: Option<String>,
}

#[derive(Debug)]
pub struct RpcSpec {
    pub name: String,
    pub request: Type,
    pub response: Type,
    pub desc: Option<String>,
}

#[derive(Debug)]
pub struct ProduceSpec {
    pub name: String,
    pub type_: Type,
    pub desc: Option<String>,
}

#[derive(Debug)]
pub struct ConsumeSpec {
    pub event: String,
    pub from: String,
    pub desc: Option<String>,
}

#[derive(Debug)]
pub enum TableColumnDefault {
    SQL(String),
    String(String),
    Integer(i64),
    Float(f64),
    Bool(bool),
    Timestamp(i64),
}

#[derive(Debug)]
pub struct TableColumn {
    pub name: String,
    pub type_: Type,
    pub unique: bool,
    pub desc: Option<String>,
    pub default: Option<TableColumnDefault>,
}

#[derive(Debug)]
pub enum OnDeleteAction {
    Cascade,
    SetNull,
    Restrict,
    NoAction,
}

#[derive(Debug)]
pub enum OnUpdateAction {
    Cascade,
    SetNull,
    Restrict,
    NoAction,
}

#[derive(Debug)]
pub struct TableForeignKey {
    pub columns: Vec<String>,
    pub reference_table: String,
    pub reference_columns: Vec<String>,
    pub on_delete: OnDeleteAction,
    pub on_update: OnUpdateAction,
    pub desc: Option<String>,
}

#[derive(Debug)]
pub struct TableIndex {
    pub name: String,
    pub columns: Vec<String>,
    pub unique: bool,
    pub desc: Option<String>,
}

#[derive(Debug)]
pub struct TableCheck {
    pub name: String,
    pub expression: String,
    pub desc: Option<String>,
}

#[derive(Debug)]
pub struct TableSpec {
    pub name: String,
    pub desc: Option<String>,
    pub columns: Vec<TableColumn>,
    pub primary_key: Vec<String>,
    pub foreign_keys: Vec<TableForeignKey>,
    pub indexes: Vec<TableIndex>,
    pub checks: Vec<TableCheck>,
}

#[derive(Debug, Error)]
pub enum SpecError {
    #[error("IO error: {0}")]
    IoError(io::Error),
    #[error("TOML parseing error: {0}")]
    TomlError(toml_edit::TomlError),
    #[error("Unknown TOML section: {0}")]
    UnknownSection(String),
    #[error("Type error: {0}")]
    TypeError(String),
    #[error("Spec format error: {0}")]
    FormatError(String),
    #[error("Datetime error: {0}")]
    DatetimeError(String),
}

pub type SpecResult<T> = Result<T, SpecError>;

impl Spec {
    pub fn new() -> Self {
        Spec {
            types: Vec::new(),
            rpcs: Vec::new(),
            produces: Vec::new(),
            consumes: Vec::new(),
            tables: Vec::new(),
        }
    }

    pub fn load_dir(&mut self, path: &Path) -> SpecResult<()> {
        let files = fs::read_dir(path).map_err(SpecError::IoError)?;

        for file in files {
            let file = file.map_err(SpecError::IoError)?;
            let path = file.path();

            if path.is_file() && path.extension().is_some_and(|ext| ext == "toml") {
                self.load_file(&path)?;
            } else if path.is_dir() {
                self.load_dir(&path)?;
            }
        }

        Ok(())
    }

    pub fn load_file(&mut self, path: &Path) -> SpecResult<()> {
        let file = fs::read_to_string(path).map_err(SpecError::IoError)?;
        self.load_string(&file)
    }

    pub fn load_string(&mut self, string: &str) -> SpecResult<()> {
        let document = string
            .parse::<DocumentMut>()
            .map_err(SpecError::TomlError)?;

        for (key, value) in document.into_table() {
            match key.as_str() {
                "types" => self.load_types(value)?,
                "rpc" => self.load_rpcs(value)?,
                "produces" => self.load_produces(value)?,
                "consumes" => self.load_consumes(value)?,
                "tables" => self.load_tables(value)?,
                _ => return Err(SpecError::UnknownSection(key)),
            }
        }

        Ok(())
    }

    fn parse_type_expr(expr: &str) -> SpecResult<Type> {
        let expr = expr.trim();

        if let Some((ok, err)) = expr.split_once("?:") {
            return Ok(Type::Result(
                Box::new(Spec::parse_type_expr(ok)?),
                Box::new(Spec::parse_type_expr(err)?),
            ));
        }

        if let Some(stripped) = expr.strip_suffix('?') {
            return Ok(Type::Optional(Box::new(Spec::parse_type_expr(stripped)?)));
        }

        if let Some(stripped) = expr.strip_suffix("[]") {
            return Ok(Type::Array(Box::new(Spec::parse_type_expr(stripped)?)));
        }

        if let Some((parent, field)) = expr.split_once('.') {
            return Ok(Type::Field(
                Box::new(Spec::parse_type_expr(parent)?),
                field.trim().to_string(),
            ));
        }

        match expr {
            "bool" => Ok(Type::Bool),
            "i16" => Ok(Type::I16),
            "i32" => Ok(Type::I32),
            "u16" => Ok(Type::U16),
            "u32" => Ok(Type::U32),
            "f32" => Ok(Type::F32),
            "f64" => Ok(Type::F64),
            "string" => Ok(Type::String),
            "timestamp" => Ok(Type::Timestamp),
            _ => {
                if expr.is_empty() {
                    return Err(SpecError::TypeError("Expected a type, got an empty string. Make sure your types are defined correctly.".to_string()));
                }

                if !expr.chars().all(|c| c.is_ascii_alphanumeric()) {
                    return Err(SpecError::TypeError(format!(
                        "Invalid type name: '{}'. Only alphanumeric characters are allowed (/[A-Z][a-zA-Z0-9]*/).",
                        expr
                    )));
                }

                if !expr[0..1]
                    .chars()
                    .next()
                    .expect("expr isn't empty, expr[0..1] should be Some")
                    .is_ascii_uppercase()
                {
                    return Err(SpecError::TypeError(format!(
                        "Invalid type name: '{}'. Type names must start with an uppercase letter.",
                        expr
                    )));
                }

                Ok(Type::Alias(expr.to_string()))
            }
        }
    }

    fn load_types(&mut self, value: toml_edit::Item) -> SpecResult<()> {
        let array = value.as_array_of_tables().ok_or_else(|| {
            SpecError::FormatError("Expected an array of tables for `types` section".to_string())
        })?;

        for table in array {
            let name = table
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    SpecError::FormatError("Missing field `name` in type definition".to_string())
                })?
                .to_string();

            let kind = table
                .get("kind")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    SpecError::FormatError(format!(
                        "Missing or invalid field `kind` for type `{}`",
                        name
                    ))
                })?
                .to_string();

            let desc = table
                .get("desc")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let type_spec = match kind.as_str() {
                "enum" => {
                    let mut variants = Vec::new();

                    if let Some(variants_item) = table.get("variants")
                        && let Some(variants_array) = variants_item.as_array_of_tables()
                    {
                        for v in variants_array {
                            let value = v
                                .get("value")
                                .and_then(|x| x.as_str())
                                .ok_or_else(|| {
                                    SpecError::FormatError(format!(
                                        "Missing or invalid field `value` for type `{}`",
                                        name
                                    ))
                                })?
                                .to_string();

                            let desc = v
                                .get("desc")
                                .and_then(|x| x.as_str())
                                .map(|s| s.to_string());
                            variants.push(EnumVariant { value, desc });
                        }
                    }

                    TypeSpec {
                        name,
                        kind: Type::Enum(variants),
                        desc,
                    }
                }
                "struct" => {
                    let mut fields = Vec::new();

                    if let Some(fields_item) = table.get("fields")
                        && let Some(fields_array) = fields_item.as_array_of_tables()
                    {
                        for f in fields_array {
                            let field_name = f
                                .get("name")
                                .and_then(|x| x.as_str())
                                .ok_or_else(|| {
                                    SpecError::FormatError(format!(
                                        "Missing or invalid field `name` for type `{}`",
                                        name
                                    ))
                                })?
                                .to_string();

                            let field_type = f
                                .get("type")
                                .and_then(|x| x.as_str())
                                .map(Spec::parse_type_expr)
                                .ok_or_else(|| {
                                    SpecError::FormatError(format!(
                                        "Missing or invalid field `type` for type `{}`",
                                        name
                                    ))
                                })??;

                            let desc = f
                                .get("desc")
                                .and_then(|x| x.as_str())
                                .map(|s| s.to_string());

                            fields.push(StructField {
                                name: field_name,
                                type_: field_type,
                                desc,
                            });
                        }
                    }

                    TypeSpec {
                        name,
                        kind: Type::Struct(fields),
                        desc,
                    }
                }
                other => TypeSpec {
                    name,
                    kind: Spec::parse_type_expr(other)?,
                    desc,
                },
            };

            self.types.push(type_spec);
        }

        Ok(())
    }

    fn load_rpcs(&mut self, value: toml_edit::Item) -> SpecResult<()> {
        let array = value.as_array_of_tables().ok_or_else(|| {
            SpecError::FormatError("`rpc` must be an array of tables".to_string())
        })?;

        for table in array {
            let name = table
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    SpecError::FormatError("RPC entry missing field `name`".to_string())
                })?
                .to_string();

            let request = table
                .get("request")
                .and_then(|v| v.as_str())
                .map(Spec::parse_type_expr)
                .ok_or_else(|| {
                    SpecError::FormatError(format!(
                        "Missing or invalid field `request` for RPC entry `{}`",
                        name
                    ))
                })??;

            let response = table
                .get("response")
                .and_then(|v| v.as_str())
                .map(Spec::parse_type_expr)
                .ok_or_else(|| {
                    SpecError::FormatError(format!(
                        "Missing or invalid field `response` for RPC entry `{}`",
                        name
                    ))
                })??;

            let desc = table
                .get("desc")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            self.rpcs.push(RpcSpec {
                name,
                request,
                response,
                desc,
            })
        }

        Ok(())
    }

    fn load_produces(&mut self, value: toml_edit::Item) -> SpecResult<()> {
        let array = value.as_array_of_tables().ok_or_else(|| {
            SpecError::FormatError("`produces` must be an array of tables".to_string())
        })?;

        for table in array {
            let name = table
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    SpecError::FormatError("Produce entry missing field `name`".to_string())
                })?
                .to_string();

            let type_ = table
                .get("type")
                .and_then(|v| v.as_str())
                .map(Spec::parse_type_expr)
                .ok_or_else(|| {
                    SpecError::FormatError(format!(
                        "Missing or invalid field `type` for produce entry `{}`",
                        name
                    ))
                })??;

            let desc = table
                .get("desc")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            self.produces.push(ProduceSpec { name, type_, desc });
        }

        Ok(())
    }

    fn load_consumes(&mut self, value: toml_edit::Item) -> SpecResult<()> {
        let array = value.as_array_of_tables().ok_or_else(|| {
            SpecError::FormatError("`consumes` must be an array of tables".to_string())
        })?;

        for table in array {
            let event = table
                .get("event")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    SpecError::FormatError("Consume entry missing field `event`".to_string())
                })?
                .to_string();

            let from = table
                .get("from")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    SpecError::FormatError("Consume entry missing field `from`".to_string())
                })?
                .to_string();

            let desc = table
                .get("desc")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            self.consumes.push(ConsumeSpec { event, from, desc });
        }

        Ok(())
    }

    fn load_tables(&mut self, value: toml_edit::Item) -> SpecResult<()> {
        let array = value.as_array_of_tables().ok_or_else(|| {
            SpecError::FormatError("`tables` must be an array of tables".to_string())
        })?;

        for table in array {
            let name = table
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    SpecError::FormatError("Table entry missing field `name`".to_string())
                })?
                .to_string();

            let desc = table
                .get("desc")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let mut columns = Vec::new();

            if let Some(cols) = table.get("columns").and_then(|v| v.as_array_of_tables()) {
                for col in cols {
                    let col_name = col
                        .get("name")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            SpecError::FormatError(format!(
                                "Column has missing or invalid field `name` in table `{}`",
                                name
                            ))
                        })?
                        .to_string();

                    let type_ = col
                        .get("type")
                        .and_then(|v| v.as_str())
                        .map(Spec::parse_type_expr)
                        .ok_or_else(|| {
                            SpecError::FormatError(format!(
                                "Column `{}` has missing or invalid field `type` in table `{}`",
                                col_name, name
                            ))
                        })??;

                    let unique = col.get("unique").and_then(|x| x.as_bool()).unwrap_or(false);
                    let desc = col
                        .get("desc")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let default = col
                        .get("default")
                        .map(|item| match item {
                            toml_edit::Item::Value(v) => match v {
                                toml_edit::Value::String(s) => {
                                    let is_sql = s.as_repr().and_then(|r| r.as_raw().as_str()).map(|s| s.starts_with("'''") && s.ends_with("'''")).unwrap_or(false);

                                    if is_sql {
                                        Ok(Some(TableColumnDefault::SQL(s.value().clone())))
                                    } else {
                                        Ok(Some(TableColumnDefault::String(s.value().clone())))
                                    }
                                },
                                toml_edit::Value::Integer(i) => Ok(Some(TableColumnDefault::Integer(*i.value()))),
                                toml_edit::Value::Float(f) => Ok(Some(TableColumnDefault::Float(*f.value()))),
                                toml_edit::Value::Boolean(b) => Ok(Some(TableColumnDefault::Bool(*b.value()))),
                                toml_edit::Value::Datetime(d) => {
                                    let datetime = *d.value();
                                    if let Some(date) = datetime.date && let Some(time) = datetime.time && let Some(offset) = datetime.offset {
                                        let date = NaiveDate::from_ymd_opt(date.year.into(), date.month.into(), date.day.into()).ok_or_else(|| SpecError::DatetimeError(format!("Column `{}` in table `{}` has an invalid datetime as its default value", col_name, name)))?;
                                        let time = NaiveTime::from_hms_nano_opt(time.hour.into(), time.minute.into(), time.second.into(), time.nanosecond).ok_or_else(|| SpecError::DatetimeError(format!("Column `{}` in table `{}` has an invalid datetime as its default value", col_name, name)))?;
                                        let naive_dt = NaiveDateTime::new(date, time);

                                        let off = FixedOffset::east_opt(match offset {
                                            toml_edit::Offset::Custom { minutes } => minutes,
                                            toml_edit::Offset::Z => 0,
                                        }.into()).ok_or_else(|| SpecError::DatetimeError(format!("Column `{}` in table `{}` has an invalid datetime as its default value", col_name, name)))?;

                                        let dt = off.from_local_datetime(&naive_dt).unwrap();
                                        let timestamp = dt.timestamp_micros();

                                        Ok(Some(TableColumnDefault::Timestamp(timestamp)))
                                    } else {
                                        Err(SpecError::DatetimeError(format!(
                                            "Column `{}` in table `{}` has an invalid datetime as its default value",
                                            col_name, name
                                        )))
                                    }
                                },
                                _ => Err(SpecError::FormatError(format!(
                                    "Column `{}` in table `{}` has an unsupported default value",
                                    col_name, name
                                ))),
                            },
                            toml_edit::Item::None => Ok(None),
                            _ => Err(SpecError::FormatError(format!(
                                "Column `{}` in table `{}` has an unsupported default value",
                                col_name, name
                            ))),
                        }).unwrap_or(Ok(None))?;

                    columns.push(TableColumn {
                        name: col_name,
                        type_,
                        unique,
                        desc,
                        default,
                    });
                }
            }

            let primary_key: Vec<String> = table
                .get("primary-key")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    Some(
                        arr.iter()
                            .filter_map(|x| x.as_str().map(|s| s.to_string()))
                            .collect(),
                    )
                })
                .unwrap_or_else(|| {
                    table
                        .get("primary-key")
                        .and_then(|v| v.as_str())
                        .map(|s| vec![s.to_string()])
                })
                .ok_or_else(|| {
                    SpecError::FormatError(format!(
                        "Table `{}` missing or invalid field `primary-key`",
                        name
                    ))
                })?;

            let mut foreign_keys = Vec::new();

            if let Some(fks) = table
                .get("foreign_keys")
                .and_then(|v| v.as_array_of_tables())
            {
                for fk in fks {
                    let columns: Vec<String> = fk
                        .get("columns")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            Some(
                                arr.iter()
                                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                                    .collect(),
                            )
                        })
                        .unwrap_or_else(|| {
                            fk.get("column")
                                .and_then(|v| v.as_str())
                                .map(|s| vec![s.to_string()])
                        })
                        .ok_or_else(|| {
                            SpecError::FormatError(format!(
                                "Foreign key in table `{}` missing or invalid field `column(s)`",
                                name
                            ))
                        })?;

                    let reference_table = fk.get("reference-table")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| SpecError::FormatError(format!("Foreign key in table `{}` missing or invalid field `reference-table`", name)))?
                        .to_string();

                    let reference_columns: Vec<String> = fk
                        .get("reference-columns")
                        .and_then(|v| v.as_array())
                        .map(|arr| Some(arr.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect()))
                        .unwrap_or_else(|| fk
                            .get("reference-column")
                            .and_then(|v| v.as_str())
                            .map(|s| vec![s.to_string()])
                        )
                        .ok_or_else(|| SpecError::FormatError(format!("Foreign key in table `{}` missing or invalid field `reference-column(s)`", name)))?;

                    let on_delete = match fk
                        .get("on-delete")
                        .and_then(|x| {
                            x.as_str()
                                .map(|s| s.to_lowercase().replace(['-', '_'], " "))
                        })
                        .ok_or_else(|| {
                            SpecError::FormatError(format!(
                                "Foreign key in table `{}` missing or invalid field `on-delete`",
                                name
                            ))
                        })?
                        .as_str()
                    {
                        "cascade" => OnDeleteAction::Cascade,
                        "set null" => OnDeleteAction::SetNull,
                        "restrict" => OnDeleteAction::Restrict,
                        "no action" => OnDeleteAction::NoAction,
                        _ => {
                            return Err(SpecError::FormatError(format!(
                                "Invalid `on-delete` action in foreign key for table `{}`",
                                name
                            )));
                        }
                    };

                    let on_update = match fk
                        .get("on-update")
                        .and_then(|x| {
                            x.as_str()
                                .map(|s| s.to_lowercase().replace(['-', '_'], " "))
                        })
                        .ok_or_else(|| {
                            SpecError::FormatError(format!(
                                "Foreign key in table `{}` missing or invalid field `on-update`",
                                name
                            ))
                        })?
                        .as_str()
                    {
                        "cascade" => OnUpdateAction::Cascade,
                        "set null" => OnUpdateAction::SetNull,
                        "restrict" => OnUpdateAction::Restrict,
                        "no action" => OnUpdateAction::NoAction,
                        _ => {
                            return Err(SpecError::FormatError(format!(
                                "Invalid `on-update` action in foreign key for table `{}`",
                                name
                            )));
                        }
                    };

                    let desc = fk
                        .get("desc")
                        .and_then(|x| x.as_str())
                        .map(|s| s.to_string());

                    foreign_keys.push(TableForeignKey {
                        columns,
                        reference_table,
                        reference_columns,
                        on_delete,
                        on_update,
                        desc,
                    });
                }
            }

            let mut indexes = Vec::new();

            if let Some(idxs) = table.get("indexes").and_then(|v| v.as_array_of_tables()) {
                for idx in idxs {
                    let idx_name = idx
                        .get("name")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            SpecError::FormatError(format!(
                                "Index in table `{}` missing or invalid field `name`",
                                name
                            ))
                        })?
                        .to_string();

                    let columns: Vec<String> = idx
                        .get("columns")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            Some(
                                arr.iter()
                                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                                    .collect(),
                            )
                        })
                        .unwrap_or_else(|| {
                            idx.get("column")
                                .and_then(|v| v.as_str())
                                .map(|s| vec![s.to_string()])
                        })
                        .ok_or_else(|| {
                            SpecError::FormatError(format!(
                                "Index `{}` in table `{}` missing or invalid field `column(s)`",
                                idx_name, name
                            ))
                        })?;

                    let unique = idx.get("unique").and_then(|v| v.as_bool()).ok_or_else(|| {
                        SpecError::FormatError(format!(
                            "Index `{}` in table `{}` missing or invalid field `unique`",
                            idx_name, name
                        ))
                    })?;

                    let desc = idx
                        .get("desc")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    indexes.push(TableIndex {
                        name: idx_name,
                        columns,
                        unique,
                        desc,
                    });
                }
            }

            let mut checks = Vec::new();

            if let Some(chks) = table.get("checks").and_then(|v| v.as_array_of_tables()) {
                for chk in chks {
                    let chk_name = chk
                        .get("name")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            SpecError::FormatError(format!(
                                "Check in table `{}` missing or invalid field `name`",
                                name
                            ))
                        })?
                        .to_string();

                    let expression = chk
                        .get("expression")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            SpecError::FormatError(format!(
                                "Check `{}` in table `{}` missing or invalid field `expression`",
                                chk_name, name,
                            ))
                        })?
                        .to_string();

                    let desc = chk
                        .get("desc")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    checks.push(TableCheck {
                        name: chk_name,
                        expression,
                        desc,
                    });
                }
            }

            self.tables.push(TableSpec {
                name,
                primary_key,
                desc,
                columns,
                foreign_keys,
                indexes,
                checks,
            });
        }

        Ok(())
    }

    pub fn validate(&self) -> SpecResult<()> {
        self.validate_types()?;

        Ok(())
    }

    fn validate_types(&self) -> SpecResult<()> {
        let mut types = HashMap::new();

        for type_spec in &self.types {
            match types.insert(type_spec.name.clone(), &type_spec.kind) {
                Some(_) => Err(SpecError::TypeError(format!(
                    "Duplicate type definition: {}",
                    type_spec.name
                ))),
                None => Ok(()),
            }?;
        }

        for type_spec in &self.types {
            Spec::validate_type(&type_spec.kind, &mut types)?;
        }

        Ok(())
    }

    fn validate_type(t: &Type, types: &mut HashMap<String, &Type>) -> SpecResult<()> {
        match t {
            Type::String
            | Type::I16
            | Type::I32
            | Type::U16
            | Type::U32
            | Type::F32
            | Type::F64
            | Type::Bool
            | Type::Timestamp => Ok(()),
            Type::Enum(variants) => {
                let mut variant_names = HashSet::new();

                for variant in variants {
                    if !variant_names.insert(&variant.value) {
                        return Err(SpecError::TypeError(format!(
                            "Duplicate enum variant: {}",
                            variant.value
                        )));
                    }
                }

                Ok(())
            }
            Type::Alias(name) => {
                if !types.contains_key(name) {
                    return Err(SpecError::TypeError(format!("Unknown type: {}", name)));
                }

                Ok(())
            }
            Type::Field(struct_, field) => match struct_.as_ref() {
                Type::Alias(name) => {
                    let struct_type = types
                        .get(name)
                        .ok_or_else(|| SpecError::TypeError(format!("Unknown type: {}", name)))?;

                    match struct_type {
                        Type::Struct(fields) => {
                            if !fields.iter().any(|f| &f.name == field) {
                                return Err(SpecError::TypeError(format!(
                                    "Unknown field '{}' in struct '{}'",
                                    field, name
                                )));
                            }

                            Ok(())
                        }
                        _ => Err(SpecError::TypeError(format!(
                            "Type '{}' is not a struct, cannot access field '{}'",
                            name, field
                        ))),
                    }
                }
                _ => Err(SpecError::TypeError(
                    "Field access must be on an alias type".to_string(),
                )),
            },
            Type::Array(inner) | Type::Optional(inner) => Spec::validate_type(inner, types),
            Type::Result(ok, err) => {
                Spec::validate_type(ok, types)?;
                Spec::validate_type(err, types)
            }
            Type::Struct(fields) => {
                let mut field_names = HashSet::new();

                for field in fields {
                    if !field_names.insert(&field.name) {
                        return Err(SpecError::TypeError(format!(
                            "Duplicate struct field: {}",
                            field.name
                        )));
                    }

                    Spec::validate_type(&field.type_, types)?;
                }

                Ok(())
            }
        }
    }
}

// TODO: when validating indexes, make sure that a unique index with only one column is not unique,
// as postgres automatically creates a unique index for unique columns.

// TODO: make sure git tree is clean before syncing specs. when files are written, store a hash of
// the specs in services/[service]/Cargo.toml with the commit hash of the newest spec version.

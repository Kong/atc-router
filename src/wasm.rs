use std::str::FromStr;
use uuid::Uuid;
use wasm_bindgen::prelude::*;

use crate::ast::{Expression, Value};
use crate::context::Context;
use crate::parser;
use crate::router::Router;
use crate::schema::Schema;
use crate::semantics::Validate;
use pest::error::{InputLocation, LineColLocation};
use serde::{Deserialize, Serialize};

// Wraps the context::Context since wasm_bindgen does not support lifetime
#[wasm_bindgen]
pub struct StaticContext(Context<'static>);

// Wraps the router::Router since wasm_bindgen does not support lifetime
#[wasm_bindgen]
pub struct StaticRouter(Router<'static>);

#[derive(Serialize, Deserialize)]
#[serde(remote = "InputLocation")]
enum SerializableInputLocation {
    Pos(usize),
    Span((usize, usize)),
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "LineColLocation")]
enum SerializableLineColLocation {
    Pos((usize, usize)),
    Span((usize, usize), (usize, usize)),
}

#[derive(Serialize, Deserialize)]
pub struct SerializableParseError {
    pub message: String,
    #[serde(with = "SerializableInputLocation")]
    pub location: InputLocation,
    #[serde(with = "SerializableLineColLocation", rename(serialize = "lineCol"))]
    pub line_col: LineColLocation,
}

#[derive(Serialize, Deserialize)]
pub enum ParseValidationError {
    ParseError(SerializableParseError),
    ValidationError(String),
}

#[derive(Serialize, Deserialize)]
pub struct ParseValidationResult {
    result: Option<Expression>,
    error: Option<ParseValidationError>,
}

#[wasm_bindgen(typescript_custom_section)]
const TYPE_AST_TYPE: &'static str =
    r#"export type AstType = "String" | "IpCidr" | "IpAddr" | "Int" | "Regex" | undefined;"#;

#[wasm_bindgen(typescript_custom_section)]
const TYPE_AST_VALUE: &'static str = r#"export type AstValue = { String: string } | { IpCidr: string } | { IpAddr: string } | { Regex: string } | { Int: number };"#;

#[wasm_bindgen(typescript_custom_section)]
const TYPE_AST_VALUES: &'static str = r#"export type AstValues = AstValue[];"#;

#[wasm_bindgen(typescript_custom_section)]
const TYPE_PARSE_VALIDATION_RESULT: &'static str = r#"
export type ParseValidationResult = {
    result?: any
    error?: {
      ParseError: {
        message: string
        location: { Pos: number } | { Span: [number, number] }
        lineCol: { Pos: [number, number] } | { Span: [[number, number], [number, number]] }
      }
    } | {
      ValidationError: string
    }
  };
"#;

#[wasm_bindgen(typescript_custom_section)]
const TYPE_ERROR_MESSAGE: &'static str = r#"export type ErrorMessage = string | undefined;"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "AstType")]
    pub type WasmAstType;
    #[wasm_bindgen(typescript_type = "AstValue")]
    pub type WasmAstValue;
    #[wasm_bindgen(typescript_type = "AstValues")]
    pub type WasmAstValues;
    #[wasm_bindgen(typescript_type = "ParseValidationResult")]
    pub type WasmParseValidationResult;
    #[wasm_bindgen(typescript_type = "ErrorMessage")]
    pub type WasmErrorMessage;
}

#[wasm_bindgen]
pub struct WasmParser();

#[wasm_bindgen]
pub struct WasmSchema(*mut Schema);

#[wasm_bindgen]
pub struct WasmContext(*mut StaticContext);

#[wasm_bindgen]
pub struct WasmRouter(*mut StaticRouter);

#[wasm_bindgen]
impl WasmParser {
    #[wasm_bindgen]
    pub unsafe fn parse(expressions: &str, schema: &WasmSchema) -> WasmParseValidationResult {
        match parser::parse(expressions) {
            Err(e) => WasmParseValidationResult::from(
                serde_wasm_bindgen::to_value(&ParseValidationResult {
                    result: None,
                    error: Some(ParseValidationError::ParseError(SerializableParseError {
                        message: e.variant.message().to_string(),
                        location: e.location,
                        line_col: e.line_col,
                    })),
                })
                .unwrap_throw(),
            ),
            Ok(exp) => {
                if schema.0 as usize != 0 {
                    let s = schema.0.as_ref().unwrap_throw();
                    // TODO: Don't know if this is correct but this will keep the schema alive.
                    Box::into_raw(Box::new(s));
                    match exp.validate(s) {
                        Err(e) => WasmParseValidationResult::from(
                            serde_wasm_bindgen::to_value(&ParseValidationResult {
                                result: None,
                                error: Some(ParseValidationError::ValidationError(e)),
                            })
                            .unwrap_throw(),
                        ),
                        Ok(_) => WasmParseValidationResult::from(
                            serde_wasm_bindgen::to_value(&ParseValidationResult {
                                result: Some(exp),
                                error: None,
                            })
                            .unwrap_throw(),
                        ),
                    }
                } else {
                    WasmParseValidationResult::from(
                        serde_wasm_bindgen::to_value(&ParseValidationResult {
                            result: Some(exp),
                            error: None,
                        })
                        .unwrap_throw(),
                    )
                }
            }
        }
    }
}

#[wasm_bindgen]
impl WasmSchema {
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmSchema {
        WasmSchema(Box::into_raw(Box::new(Schema::default())))
    }

    pub fn null() -> WasmSchema {
        WasmSchema(0 as *mut Schema)
    }

    #[wasm_bindgen(js_name = addField)]
    pub unsafe fn add_field(&mut self, field: &str, typ: WasmAstType) {
        let t: JsValue = typ.into();
        self.0
            .as_mut()
            .unwrap_throw()
            .add_field(field, serde_wasm_bindgen::from_value(t).unwrap_throw());
    }

    #[wasm_bindgen(js_name = typeOf)]
    pub unsafe fn type_of(&self, field: &str) -> WasmAstType {
        WasmAstType::from(match self.0.as_mut().unwrap_throw().type_of(field) {
            Some(typ) => serde_wasm_bindgen::to_value(typ).unwrap_throw(),
            None => JsValue::UNDEFINED,
        })
    }
}

impl Drop for WasmSchema {
    fn drop(&mut self) {
        unsafe { Box::from_raw(self.0) };
    }
}

#[wasm_bindgen]
impl WasmContext {
    #[wasm_bindgen(constructor)]
    pub unsafe fn new(schema: &WasmSchema) -> WasmContext {
        WasmContext(Box::into_raw(Box::new(StaticContext(Context::new(
            schema.0.as_ref().unwrap_throw(),
        )))))
    }

    #[wasm_bindgen(js_name = addValue)]
    pub unsafe fn add_value(&mut self, field: &str, value: WasmAstValue) {
        let value: JsValue = value.into();
        let value: Value = serde_wasm_bindgen::from_value(value).unwrap_throw();
        self.0.as_mut().unwrap_throw().0.add_value(field, value)
    }

    #[wasm_bindgen(js_name = valueOf)]
    pub unsafe fn value_of(&self, field: &str) -> WasmAstValues {
        WasmAstValues::from(match self.0.as_mut().unwrap_throw().0.value_of(field) {
            Some(v) => serde_wasm_bindgen::to_value(&v).unwrap_throw(),
            None => JsValue::UNDEFINED,
        })
    }
}

impl Drop for WasmContext {
    fn drop(&mut self) {
        unsafe { Box::from_raw(self.0) };
    }
}

#[wasm_bindgen]
impl WasmRouter {
    #[wasm_bindgen(constructor)]
    pub unsafe fn new(schema: &WasmSchema) -> WasmRouter {
        WasmRouter(Box::into_raw(Box::new(StaticRouter(Router::new(
            schema.0.as_mut().unwrap_throw(),
        )))))
    }

    #[wasm_bindgen(js_name = addMatcher)]
    pub unsafe fn add_matcher(
        &mut self,
        priority: usize,
        uuid: &str,
        atc: &str,
    ) -> WasmErrorMessage {
        let u = Uuid::from_str(uuid).unwrap_throw();
        match self
            .0
            .as_mut()
            .unwrap_throw()
            .0
            .add_matcher(priority, u, atc)
        {
            Ok(_) => WasmErrorMessage::from(JsValue::UNDEFINED),
            Err(e) => {
                WasmErrorMessage::from(serde_wasm_bindgen::to_value(e.as_str()).unwrap_throw())
            }
        }
    }

    #[wasm_bindgen(js_name = removeMatcher)]
    pub unsafe fn remove_matcher(&mut self, priority: usize, uuid: &str) -> bool {
        let u = Uuid::from_str(uuid).unwrap_throw();
        self.0.as_mut().unwrap_throw().0.remove_matcher(priority, u)
    }

    #[wasm_bindgen(js_name = execute)]
    pub unsafe fn execute(&self, context: &mut WasmContext) -> bool {
        let c = &mut context.0.as_mut().unwrap_throw().0;
        self.0.as_mut().unwrap_throw().0.execute(c)
    }
}

impl Drop for WasmRouter {
    fn drop(&mut self) {
        unsafe { Box::from_raw(self.0) };
    }
}

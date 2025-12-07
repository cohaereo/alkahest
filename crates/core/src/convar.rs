use std::collections::HashMap;

use lazy_static::lazy_static;
use parking_lot::RwLock;
use tracing::{error, warn};

lazy_static! {
    static ref CONVARS: RwLock<HashMap<String, ConVarData>> = RwLock::new(HashMap::new());
}

pub struct ConVars {
    _priv: (), // Prevents instantiation
}

impl ConVars {
    pub fn register(name: &str, default: impl Into<ConVarValue>) {
        if name.len() > 64 {
            error!(
                "Convar name {} is too long, can only be up to 64 bytes",
                name
            );
            return;
        }

        let mut conv = CONVARS.write();
        let default = default.into();
        if let Some(other) = conv.get(name) {
            if default.kind() != other.ty {
                warn!(
                    "Convar {} is already registered with a different type",
                    name
                );
                return;
            }
        }
        conv.insert(
            name.to_string(),
            ConVarData {
                ty: default.kind(),
                value: default.clone(),
                default_value: default,
            },
        );
    }

    pub fn get_raw(name: &str) -> Option<ConVarValue> {
        let conv = CONVARS.read();
        conv.get(name).map(|v| v.value.clone())
    }

    pub fn get_default_raw(name: &str) -> Option<ConVarValue> {
        let conv = CONVARS.read();
        conv.get(name).map(|v| v.default_value.clone())
    }

    pub fn get<T: TryFrom<ConVarValue> + Clone>(name: &str) -> Option<T> {
        let conv = CONVARS.read();
        conv.get(name)
            .map(|v| v.value.clone())
            .and_then(|v| v.try_into().ok())
    }

    pub fn get_flag(name: &str) -> bool {
        Self::get(name) == Some(true)
    }

    pub fn set(name: &str, value: ConVarValue) -> Result<(), String> {
        let mut conv = CONVARS.write();
        if let Some(v) = conv.get_mut(name) {
            if v.ty != value.kind() {
                return Err("Invalid type".to_string());
            }
            v.value = value;
            Ok(())
        } else {
            Err("ConVar not found".to_string())
        }
    }

    pub fn reset(name: &str) -> Result<(), String> {
        let mut conv = CONVARS.write();
        if let Some(v) = conv.get_mut(name) {
            v.value = v.default_value.clone();
            Ok(())
        } else {
            Err("ConVar not found".to_string())
        }
    }

    pub fn get_all() -> HashMap<String, ConVarData> {
        let conv = CONVARS.read();
        conv.clone()
    }
}

#[derive(Clone)]
pub struct ConVarData {
    pub ty: ConVarType,
    value: ConVarValue,
    default_value: ConVarValue,
}

#[repr(u8)]
#[derive(PartialEq, Clone, Copy)]
pub enum ConVarType {
    String = 1,
    Uint = 2,
    Int = 3,
    Float = 4,
    Bool = 5,
}

#[derive(PartialEq, Clone)]
pub enum ConVarValue {
    String(String),
    Uint(u32),
    Int(i32),
    Float(f32),
    Bool(bool),
}

impl ConVarValue {
    pub fn kind(&self) -> ConVarType {
        match self {
            ConVarValue::String(_) => ConVarType::String,
            ConVarValue::Uint(_) => ConVarType::Uint,
            ConVarValue::Int(_) => ConVarType::Int,
            ConVarValue::Float(_) => ConVarType::Float,
            ConVarValue::Bool(_) => ConVarType::Bool,
        }
    }

    pub fn parse(s: &str, ty: ConVarType) -> Result<ConVarValue, String> {
        match ty {
            ConVarType::String => Ok(ConVarValue::String(s.to_string())),
            ConVarType::Uint => {
                if s.to_lowercase().starts_with("0x") {
                    match u32::from_str_radix(&s[2..], 16) {
                        Ok(v) => Ok(ConVarValue::Uint(v)),
                        Err(_) => Err("Invalid hex uint".to_string()),
                    }
                } else {
                    match s.parse::<u32>() {
                        Ok(v) => Ok(ConVarValue::Uint(v)),
                        Err(_) => Err("Invalid uint".to_string()),
                    }
                }
            }
            ConVarType::Int => match s.parse::<i32>() {
                Ok(v) => Ok(ConVarValue::Int(v)),
                Err(_) => Err("Invalid int".to_string()),
            },
            ConVarType::Float => match s.parse::<f32>() {
                Ok(v) => Ok(ConVarValue::Float(v)),
                Err(_) => Err("Invalid float".to_string()),
            },
            ConVarType::Bool => match s.parse::<bool>() {
                Ok(v) => Ok(ConVarValue::Bool(v)),
                Err(_) => match s.parse::<u8>() {
                    Ok(v) => Ok(ConVarValue::Bool(v != 0)),
                    Err(_) => Err("Invalid bool".to_string()),
                },
            },
        }
    }
}

// TODO(cohae): These can be simplified using a macro
// From trait implementations
impl From<&str> for ConVarValue {
    fn from(s: &str) -> Self {
        ConVarValue::String(s.to_string())
    }
}

impl From<String> for ConVarValue {
    fn from(s: String) -> Self {
        ConVarValue::String(s)
    }
}

impl From<u32> for ConVarValue {
    fn from(v: u32) -> Self {
        ConVarValue::Uint(v)
    }
}

impl From<i32> for ConVarValue {
    fn from(v: i32) -> Self {
        ConVarValue::Int(v)
    }
}

impl From<f32> for ConVarValue {
    fn from(v: f32) -> Self {
        ConVarValue::Float(v)
    }
}

impl From<bool> for ConVarValue {
    fn from(v: bool) -> Self {
        ConVarValue::Bool(v)
    }
}

// Into trait implementations
impl TryFrom<ConVarValue> for String {
    type Error = String;

    fn try_from(value: ConVarValue) -> Result<Self, Self::Error> {
        match value {
            ConVarValue::String(s) => Ok(s),
            _ => Err("Invalid type".to_string()),
        }
    }
}

impl TryFrom<ConVarValue> for u32 {
    type Error = String;

    fn try_from(value: ConVarValue) -> Result<Self, Self::Error> {
        match value {
            ConVarValue::Uint(v) => Ok(v),
            _ => Err("Invalid type".to_string()),
        }
    }
}

impl TryFrom<ConVarValue> for i32 {
    type Error = String;

    fn try_from(value: ConVarValue) -> Result<Self, Self::Error> {
        match value {
            ConVarValue::Int(v) => Ok(v),
            _ => Err("Invalid type".to_string()),
        }
    }
}

impl TryFrom<ConVarValue> for f32 {
    type Error = String;

    fn try_from(value: ConVarValue) -> Result<Self, Self::Error> {
        match value {
            ConVarValue::Float(v) => Ok(v),
            _ => Err("Invalid type".to_string()),
        }
    }
}

impl TryFrom<ConVarValue> for bool {
    type Error = String;

    fn try_from(value: ConVarValue) -> Result<Self, Self::Error> {
        match value {
            ConVarValue::Bool(v) => Ok(v),
            _ => Err("Invalid type".to_string()),
        }
    }
}

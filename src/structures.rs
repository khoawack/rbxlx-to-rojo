use rbx_dom_weak::{types::Variant, Instance};
use serde_json::{json, Value};
use serde::{Deserialize, Serialize, Serializer};
use std::{
    borrow::Cow,
    collections::BTreeMap,
    path::{Path, PathBuf},
};

// Windows issues!
fn replace_backslashes<S: Serializer>(
    path: &Option<PathBuf>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match path {
        Some(value) => value
            .to_string_lossy()
            .replace("\\", "/")
            .serialize(serializer),

        None => serializer.serialize_none(),
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct TreePartition {
    #[serde(rename = "$className")]
    pub class_name: String,

    #[serde(flatten)]
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub children: BTreeMap<String, TreePartition>,

    #[serde(rename = "$ignoreUnknownInstances")]
    pub ignore_unknown_instances: bool,

    #[serde(rename = "$path")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(serialize_with = "replace_backslashes")]
    pub path: Option<PathBuf>,

    #[serde(rename = "$properties")]
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub properties: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub(crate) struct MetaFile {
    #[serde(rename = "className")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_name: Option<String>,

    #[serde(rename = "properties")]
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub properties: BTreeMap<String, Value>,

    #[serde(rename = "ignoreUnknownInstances")]
    pub ignore_unknown_instances: bool,
}

#[derive(Clone, Debug)]
pub enum Instruction<'a> {
    AddToTree {
        name: String,
        partition: TreePartition,
    },

    CreateFile {
        filename: Cow<'a, Path>,
        contents: Cow<'a, [u8]>,
    },

    CreateFolder {
        folder: Cow<'a, Path>,
    },
}

impl<'a> Instruction<'a> {
    pub fn add_to_tree(
        instance: &Instance,
        path: PathBuf,
        properties: BTreeMap<String, Value>,
    ) -> Self {
        Instruction::AddToTree {
            name: instance.name.clone(),
            partition: Instruction::partition(&instance, path, properties),
        }
    }

    pub fn partition(
        instance: &Instance,
        path: PathBuf,
        properties: BTreeMap<String, Value>,
    ) -> TreePartition {
        TreePartition {
            class_name: instance.class.to_string(),
            children: BTreeMap::new(),
            ignore_unknown_instances: true,
            path: Some(path),
            properties,
        }
    }
}

pub fn rojo_property_value(value: &Variant) -> Option<Value> {
    fn explicit(ty: &str, value: Value) -> Value {
        json!({ ty: value })
    }

    fn explicit_sanitized(ty: &str, mut value: Value) -> Value {
        sanitize_numeric_value(&mut value);
        explicit(ty, value)
    }

    fn sanitize_numeric_value(value: &mut Value) {
        match value {
            // serde_json cannot represent NaN/Inf and encodes them as null.
            // Rojo rejects null for numeric fields, so coerce to a large finite fallback.
            Value::Null => *value = json!(999_999_999.0),
            Value::Array(values) => {
                for value in values {
                    sanitize_numeric_value(value);
                }
            }
            Value::Object(values) => {
                for value in values.values_mut() {
                    sanitize_numeric_value(value);
                }
            }
            _ => {}
        }
    }

    fn finite_json_number(value: f64) -> Value {
        if value.is_finite() {
            json!(value)
        } else if value.is_sign_negative() {
            json!(-999_999_999.0)
        } else {
            json!(999_999_999.0)
        }
    }

    match value {
        Variant::Attributes(_) => None,
        Variant::Axes(value) => serde_json::to_value(value).ok().map(|value| explicit("Axes", value)),
        Variant::Bool(value) => Some(explicit("Bool", Value::Bool(*value))),
        Variant::BrickColor(value) => serde_json::to_value(value).ok().map(|value| explicit("BrickColor", value)),
        Variant::CFrame(value) => serde_json::to_value(value).ok().map(|value| explicit_sanitized("CFrame", value)),
        Variant::Color3(value) => serde_json::to_value(value).ok().map(|value| explicit("Color3", value)),
        Variant::Color3uint8(value) => serde_json::to_value(value).ok().map(|value| explicit("Color3uint8", value)),
        Variant::ColorSequence(value) => serde_json::to_value(value).ok().map(|value| explicit("ColorSequence", value)),
        Variant::Content(content) => {
            if let Some(uri) = content.as_uri() {
                Some(explicit("Content", json!({ "Uri": uri })))
            } else if content.as_object().is_none() {
                Some(explicit("Content", json!("None")))
            } else {
                None
            }
        }
        Variant::Enum(value) => Some(explicit("Enum", json!(value.to_u32()))),
        Variant::EnumItem(item) => Some(explicit("Enum", json!(item.value))),
        Variant::Faces(value) => serde_json::to_value(value).ok().map(|value| explicit("Faces", value)),
        Variant::Float32(value) => Some(explicit("Float32", finite_json_number(f64::from(*value)))),
        Variant::Float64(value) => Some(explicit("Float64", finite_json_number(*value))),
        Variant::Font(font) => Some(explicit("Font", json!({
            "family": font.family,
            "weight": serde_json::to_value(font.weight).ok()?,
            "style": serde_json::to_value(font.style).ok()?
        }))),
        Variant::Int32(value) => Some(explicit("Int32", json!(value))),
        Variant::Int64(value) => Some(explicit("Int64", json!(value))),
        Variant::MaterialColors(value) => serde_json::to_value(value).ok().map(|value| explicit("MaterialColors", value)),
        Variant::NetAssetRef(_)
        | Variant::OptionalCFrame(_)
        | Variant::BinaryString(_)
        | Variant::Ref(_)
        | Variant::Region3(_)
        | Variant::Region3int16(_)
        | Variant::SecurityCapabilities(_)
        | Variant::SharedString(_)
        | Variant::UniqueId(_) => None,
        Variant::NumberRange(value) => serde_json::to_value(value).ok().map(|value| explicit_sanitized("NumberRange", value)),
        Variant::NumberSequence(value) => serde_json::to_value(value).ok().map(|value| explicit_sanitized("NumberSequence", value)),
        Variant::PhysicalProperties(value) => serde_json::to_value(value).ok().map(|value| explicit_sanitized("PhysicalProperties", value)),
        Variant::Ray(value) => serde_json::to_value(value).ok().map(|value| explicit_sanitized("Ray", value)),
        Variant::Rect(value) => serde_json::to_value(value).ok().map(|value| explicit_sanitized("Rect", value)),
        Variant::String(value) => Some(explicit("String", Value::String(value.clone()))),
        Variant::Tags(value) => serde_json::to_value(value).ok().map(|value| explicit("Tags", value)),
        Variant::UDim(value) => serde_json::to_value(value).ok().map(|value| explicit_sanitized("UDim", value)),
        Variant::UDim2(value) => serde_json::to_value(value).ok().map(|value| explicit_sanitized("UDim2", value)),
        Variant::Vector2(value) => serde_json::to_value(value).ok().map(|value| explicit_sanitized("Vector2", value)),
        Variant::Vector2int16(value) => serde_json::to_value(value).ok().map(|value| explicit("Vector2int16", value)),
        Variant::Vector3(value) => serde_json::to_value(value).ok().map(|value| explicit_sanitized("Vector3", value)),
        Variant::Vector3int16(value) => serde_json::to_value(value).ok().map(|value| explicit("Vector3int16", value)),
        _ => None,
    }
}

pub trait InstructionReader {
    fn finish_instructions(&mut self) {}
    fn read_instruction<'a>(&mut self, instruction: Instruction<'a>);

    fn read_instructions<'a>(&mut self, instructions: Vec<Instruction<'a>>) {
        for instruction in instructions {
            self.read_instruction(instruction);
        }
    }
}

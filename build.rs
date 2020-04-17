use anyhow::Context;
use fehler::throws;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::env;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

const DESTINATION_DIR: &str = "src";
const MODULE_ROOT: &str = "lib.rs";
const MODULE_ROOT_TEMPLATE: &str = "lib.template.rs";
const DATA_FILES_ROOT: &str = "src/data";
const MODELS_DATA_FILES_PATH: &str = "models";

#[throws(anyhow::Error)]
fn main() {
    let from_dir = env::current_dir()?
        .join(DATA_FILES_ROOT)
        .join(MODELS_DATA_FILES_PATH);
    let to_dir = env::current_dir()?.join(DESTINATION_DIR);
    let mut modules = vec![];
    for from_path in from_dir.read_dir()? {
        let from_path = from_path?.path();
        let module_name = from_path.file_stem().unwrap().to_str().unwrap().to_string();
        modules.push(module_name);
        let mut from_file = File::open(&from_path)?;
        let to_path = change_ext(&from_path);
        let mut to_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&to_path)
            .with_context(|| "trying to open to_file")?;

        let models: ModelFile = serde_json::from_reader(&mut from_file)?;
        writeln!(&mut to_file, "{}", models)?;
        for model in models.models.iter() {
            writeln!(&mut to_file, "{}", model)?;
        }
        for r#enum in models.enums.iter() {
            writeln!(&mut to_file, "{}", r#enum)?;
        }
    }
    let mod_path = to_dir.join(MODULE_ROOT);
    let mod_template_path = Path::new(DATA_FILES_ROOT).join(MODULE_ROOT_TEMPLATE);
    let mut mod_template = File::open(mod_template_path)?;
    let mut mod_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&mod_path)
        .with_context(|| "trying to open mod_file")?;
    io::copy(&mut mod_template, &mut mod_file)?;
    for module in modules {
        writeln!(&mut mod_file, "pub mod {};", module)?;
    }
}

fn change_ext(s: &Path) -> PathBuf {
    let extless = s.file_name().unwrap();
    let dir = PathBuf::from(DESTINATION_DIR);
    let mut p = PathBuf::from(extless);
    p.set_extension("rs");
    dir.join(p)
}

#[derive(Serialize, Deserialize)]
struct ModelFile {
    includes: Vec<String>,
    models: Vec<Model>,
    #[serde(default = "default_enum_vec")]
    enums: Vec<Enum>,
}

fn default_enum_vec() -> Vec<Enum> {
    vec![]
}

#[derive(Serialize, Deserialize)]
struct Enum {
    name: String,
    description: String,
    attributes: Vec<String>,
    #[serde(default = "default_str_vec")]
    derives: Vec<String>,
    variants: Vec<Variant>,
}

#[derive(Serialize, Deserialize)]
struct Variant {
    name: String,
    #[serde(default = "default_str_vec")]
    args: Vec<String>,
    #[serde(default = "default_str_vec")]
    attributes: Vec<String>,
    description: String,
}

impl fmt::Display for Enum {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let attrs = self
            .attributes
            .iter()
            .map(|attr| format!("#[{}]", attr))
            .collect::<Vec<_>>()
            .join("\n");
        let attrs = format!("{}\n", attrs);
        let mut derives = HashSet::new();
        derives.extend(
            ["Debug", "Clone", "PartialEq", "Deserialize", "Serialize"]
                .iter()
                .map(|s| s.to_string()),
        );
        derives.extend(self.derives.iter().cloned());
        let derives = derives.into_iter().collect::<Vec<_>>();
        let derives = derives.join(", ");
        let variants = self
            .variants
            .iter()
            .map(|var| var.to_string())
            .collect::<Vec<_>>();
        let variants = variants.join("\n");
        writeln!(f, r#"#[doc = "{}"]"#, self.description)?;
        writeln!(f, r#"#[derive({})]"#, derives)?;
        writeln!(f, "{}pub enum {} {{", attrs, self.name)?;
        writeln!(f, "{}", variants)?;
        writeln!(f, "}}")
    }
}

impl fmt::Display for Variant {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let args = if self.args.is_empty() {
            String::from("")
        } else {
            format!("({})", self.args.join(", "))
        };
        let attrs = self
            .attributes
            .iter()
            .map(|attr| format!("#[{}]", attr))
            .collect::<Vec<_>>()
            .join("\n");
        let attrs = format!("{}\n", attrs);
        writeln!(f, r#"    #[doc = "{}"]"#, self.description)?;
        writeln!(f, "    {}    {}{},", attrs, self.name, args)
    }
}

#[derive(Serialize, Deserialize)]
struct Model {
    name: String,
    description: String,
    fields: Vec<Field>,
}

#[derive(Serialize, Deserialize)]
struct Field {
    name: String,
    description: String,
    r#type: String,
    #[serde(default = "default_str_vec")]
    attributes: Vec<String>,
    #[serde(default)]
    optional: bool,
}

fn default_str_vec() -> Vec<String> {
    vec![]
}

impl fmt::Display for ModelFile {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let includes = self
            .includes
            .iter()
            .map(|include| format!("use {};", include))
            .collect::<Vec<_>>()
            .join("\n");
        writeln!(
            f,
            r#"#![allow(unused_imports)]
use serde::{{Deserialize, Serialize}};
use derive_builder::Builder;
use derive_getters::Getters;
{includes}

// auto-generated from build.rs"#,
            includes = includes
        )
    }
}

impl fmt::Display for Model {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let fields = self
            .fields
            .iter()
            .map(|field| field.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        writeln!(
            f,
            r#"#[doc = "{description}"]
#[derive(Debug, Clone, PartialEq, Builder, Deserialize, Serialize, Getters)]
pub struct {name} {{
    {fields}
}}
impl {name} {{
    #[doc ="Builder for the `{name}` model"]
    pub fn builder() -> {name}Builder {{
        {name}Builder::default()
    }}
}}"#,
            description = self.description,
            name = self.name,
            fields = fields
        )
    }
}

impl Field {
    fn r#type(&self) -> String {
        if self.optional {
            format!("Option<{}>", self.r#type)
        } else {
            format!("{}", self.r#type)
        }
    }
}

impl fmt::Display for Field {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let attributes = self
            .attributes
            .iter()
            .map(|attr| format!("#[{}]", attr))
            .collect::<Vec<_>>()
            .join("\n");
        let attributes = format!("{}\n", attributes);
        writeln!(
            f,
            r#"#[doc = "{description}"]
{attributes}{name}: {type},"#,
            description = self.description,
            name = self.name,
            r#type = self.r#type(),
            attributes = attributes,
        )
    }
}

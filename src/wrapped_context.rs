use log::{debug, trace, warn};
use serde_json::{self};
use std::{fmt::Debug, fs, path::Path, path::PathBuf};
use tera::Context;

const _1KB: usize = 1024;

#[derive(Debug)]
pub struct WrappedContext {
    context: Context,
    context_path: PathBuf,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SupportedType {
    _Json,
}

impl WrappedContext {
    pub fn new(path: &Path) -> Self {
        let mut me = Self {
            context: Context::new(),
            context_path: PathBuf::from(path),
        };

        me.init();

        me
    }

    pub fn context(&self) -> &Context {
        &self.context
    }

    pub fn append_json(&mut self, str: &str) {
        debug!("Appending json");
        let json = str.parse::<serde_json::Value>().expect("JSON parsing");
        let object = json.as_object().expect("JSON as object");

        for (k, v) in object.iter() {
            self.handle_collision("json", k, v);
        }
    }

    fn handle_collision<K, V>(&mut self, _from: &str, k: K, v: V)
    where
        K: Debug + AsRef<str>,
        V: Debug + serde::Serialize,
    {
        trace!("key: {:?}", k);
        let exist = self.context.get(k.as_ref());
        if let Some(current) = exist {
            warn!("Key '{}' is being overwritten by the ENV", k.as_ref());
            warn!("  - Current value: {:?}", current);
            warn!("  - New value    : {:?}", v);
        }
        self.context.insert(k.as_ref(), &v);
    }

    pub fn init(&mut self) {
        // here we know that we have a Path since --stdin is not passed
        let input = fs::read_to_string(&self.context_path).unwrap();

        match self.context_path.extension() {
            Some(ext) if ext == "json" => self.append_json(&input),
            ext => {
                panic!("Extension not supported: {:?}", ext)
            }
        };
    }
}

#[cfg(test)]
mod test_context {
    use super::*;

    #[test]
    fn test_get_type_json() {
        let data = json!({
            "name": "John Doe",
            "age": 43u8,
            "phones": [
                "+44 1234567",
                "+44 2345678"
            ]
        })
        .to_string();

        assert!(WrappedContext::get_type(&data) == Some(SupportedType::Json));
    }

    #[test]
    fn test_get_type_na() {
        let data = r##"
        foobar
    	"##
        .to_string();

        assert!(WrappedContext::get_type(&data) == None);
    }
}

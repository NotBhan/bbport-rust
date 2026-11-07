//! bbnet loader glue, ported from `bbnet/bbnet_glue.cpp`.

use std::collections::HashMap;

/// A registered library entry.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GlueFunction {
    pub name: String,
    pub library: String,
    pub target: String,
}

/// The glue registration state.
#[derive(Debug, Default)]
pub struct GlueState {
    pub functions: HashMap<String, Vec<GlueFunction>>,
    pub initialized: bool,
}

impl GlueState {
    pub fn initialize(&mut self) {
        self.initialized = true;
    }

    pub fn register(&mut self, library: &str, function: GlueFunction) {
        if self.initialized {
            self.functions
                .entry(library.to_string())
                .or_default()
                .push(function);
        }
    }

    pub fn functions_for(&self, library: &str) -> &[GlueFunction] {
        self.functions.get(library).map_or(&[], Vec::as_slice)
    }

    pub fn count(&self) -> usize {
        self.functions.values().map(Vec::len).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glue_registers_functions_per_library() {
        let mut glue = GlueState::default();
        let function = GlueFunction {
            name: "sceNetSocket".to_string(),
            library: "libSceNet".to_string(),
            target: "sys_socket".to_string(),
        };
        glue.register("libSceNet", function.clone());
        assert_eq!(glue.count(), 0);
        glue.initialize();
        glue.register("libSceNet", function);
        assert_eq!(glue.functions_for("libSceNet").len(), 1);
        assert_eq!(glue.count(), 1);
    }
}

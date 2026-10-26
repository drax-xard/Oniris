use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MixSpec {
    #[serde(default)]
    pub packs: Vec<MixEntry>,
    #[serde(default = "default_master_gain")]
    pub master_gain: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MixEntry {
    pub id: String,
    #[serde(default)]
    pub gain_db: f32,
    #[serde(default)]
    pub muted: bool,
}

fn default_master_gain() -> f32 {
    1.0
}

impl Default for MixSpec {
    fn default() -> Self {
        Self {
            packs: Vec::new(),
            master_gain: 1.0,
        }
    }
}

impl MixSpec {
    pub fn new(packs: Vec<MixEntry>) -> Self {
        Self {
            packs,
            master_gain: 1.0,
        }
    }
}

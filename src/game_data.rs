use serde::Deserialize;

// Starting with the 7.0.0 data dump, fields holding their default value
// (0, false, "", or the first enum variant) are omitted from the JSON
// entirely.  All fields are marked `#[serde(default)]` so entries missing
// them still deserialize, with optional id fields typed as `Option`.

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct AvatarExcelConfigDataEntry {
    #[serde(default)]
    pub id: u32,
    #[serde(default, rename = "nameTextMapHash")]
    pub name_text_map_hash: u32,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct AvatarSkillDepotExcelConfigDataEntry {
    #[serde(default, rename = "energySkill")]
    pub energy_skill: Option<u32>,
    #[serde(default)]
    skills: Vec<u32>,
}

impl AvatarSkillDepotExcelConfigDataEntry {
    // Skill slots hold 0 as a placeholder for empty slots.
    pub fn skill(&self, index: usize) -> Option<u32> {
        self.skills.get(index).copied().filter(|&id| id != 0)
    }
}

// `RELIQUARY_ITEM` is `displayType`'s first enum variant, so it is what an
// omitted field means.
fn default_display_type() -> String {
    "RELIQUARY_ITEM".to_string()
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct DisplayItemExcelConfigDataEntry {
    #[serde(default = "default_display_type", rename = "displayType")]
    pub display_type: String,
    #[serde(default, rename = "nameTextMapHash")]
    pub name_text_map_hash: u32,
    #[serde(default)]
    pub param: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct MaterialExcelConfigDataEntry {
    #[serde(default)]
    pub id: u32,
    #[serde(default, rename = "nameTextMapHash")]
    pub name_text_map_hash: u32,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ReliquaryAffixExcelConfigDataEntry {
    #[serde(default)]
    pub id: u32,
    #[serde(default, rename = "propType")]
    pub prop_type: String,
    #[serde(default, rename = "propValue")]
    pub prop_value: f64,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ReliquaryExcelConfigDataEntry {
    #[serde(default, rename = "equipType")]
    pub equip_type: String,
    #[serde(default)]
    pub id: u32,
    #[serde(default, rename = "rankLevel")]
    pub rank_level: u32,
    #[serde(default, rename = "setId")]
    pub set_id: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ReliquaryMainPropExcelConfigDataEntry {
    #[serde(default)]
    pub id: u32,
    #[serde(default, rename = "propType")]
    pub prop_type: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct WeaponExcelConfigDataEntry {
    #[serde(default)]
    pub id: u32,
    #[serde(default, rename = "nameTextMapHash")]
    pub name_text_map_hash: u32,
    #[serde(default, rename = "rankLevel")]
    pub rank_level: u32,
}

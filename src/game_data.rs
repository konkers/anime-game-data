use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct AvatarExcelConfigDataEntry {
    pub id: u32,
    #[serde(rename = "nameTextMapHash")]
    pub name_text_map_hash: u32,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct AvatarSkillDepotExcelConfigDataEntry {
    #[serde(rename = "energySkill")]
    pub energy_skill: u32,
    pub skills: Vec<u32>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct DisplayItemExcelConfigDataEntry {
    #[serde(rename = "displayType")]
    pub display_type: String,
    #[serde(rename = "nameTextMapHash")]
    pub name_text_map_hash: u32,
    pub param: u32,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct MaterialExcelConfigDataEntry {
    pub id: u32,
    #[serde(rename = "nameTextMapHash")]
    pub name_text_map_hash: u32,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ReliquaryAffixExcelConfigDataEntry {
    pub id: u32,
    #[serde(rename = "propType")]
    pub prop_type: String,
    #[serde(rename = "propValue")]
    pub prop_value: f64,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ReliquaryExcelConfigDataEntry {
    #[serde(rename = "equipType")]
    pub equip_type: String,
    pub id: u32,
    #[serde(rename = "rankLevel")]
    pub rank_level: u32,
    #[serde(rename = "setId")]
    pub set_id: u32,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ReliquaryMainPropExcelConfigDataEntry {
    pub id: u32,
    #[serde(rename = "propType")]
    pub prop_type: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct WeaponExcelConfigDataEntry {
    pub id: u32,
    #[serde(rename = "nameTextMapHash")]
    pub name_text_map_hash: u32,
    #[serde(rename = "rankLevel")]
    pub rank_level: u32,
}

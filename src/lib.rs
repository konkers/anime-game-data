use std::collections::HashMap;

use anyhow::Result;

mod dimbreath;
mod game_data;
mod types;

use dimbreath::Dimbreath;
pub use types::*;

use crate::game_data::{
    AvatarExcelConfigDataEntry, MaterialExcelConfigDataEntry,
    ReliquaryMainPropExcelConfigDataEntry, WeaponExcelConfigDataEntry,
};

#[derive(Debug)]
pub struct AnimeGameData {
    latest_git_hash: Option<String>,
    affix_map: HashMap<u32, Affix>,
    artifact_map: HashMap<u32, Artifact>,
    character_map: HashMap<u32, String>,
    material_map: HashMap<u32, String>,
    property_map: HashMap<u32, Property>,
    set_map: HashMap<u32, String>,
    skill_type_map: HashMap<u32, SkillType>,
    weapon_map: HashMap<u32, Weapon>,
}

impl AnimeGameData {
    pub fn new() -> Result<Self> {
        Ok(Self {
            latest_git_hash: None,
            affix_map: HashMap::new(),
            artifact_map: HashMap::new(),
            character_map: HashMap::new(),
            material_map: HashMap::new(),
            property_map: HashMap::new(),
            set_map: HashMap::new(),
            skill_type_map: HashMap::new(),
            weapon_map: HashMap::new(),
        })
    }

    pub async fn update(&mut self) -> Result<()> {
        let dimbreath = Dimbreath::new()?;
        let latest_git_hash = dimbreath.get_latest_hash().await?;
        if Some(latest_git_hash.clone()) == self.latest_git_hash {
            return Ok(());
        }

        // Fetch all data first before updating `self` to ensure consistency.
        let text_map = Self::fetch_text_map(&dimbreath, &latest_git_hash).await?;
        let skill_type_map = Self::fetch_skill_type_map(&dimbreath, &latest_git_hash).await?;
        let set_map = Self::fetch_set_map(&dimbreath, &latest_git_hash, &text_map).await?;
        let artifact_map = Self::fetch_artifact_map(&dimbreath, &latest_git_hash, &set_map).await?;
        let property_map = Self::fetch_property_map(&dimbreath, &latest_git_hash).await?;
        let affix_map = Self::fetch_affix_map(&dimbreath, &latest_git_hash).await?;
        let weapon_map = Self::fetch_weapon_map(&dimbreath, &latest_git_hash, &text_map).await?;
        let material_map =
            Self::fetch_material_map(&dimbreath, &latest_git_hash, &text_map).await?;
        let character_map =
            Self::fetch_character_map(&dimbreath, &latest_git_hash, &text_map).await?;
        println!("{character_map:#?}");

        self.latest_git_hash = Some(latest_git_hash);
        self.affix_map = affix_map;
        self.artifact_map = artifact_map;
        self.character_map = character_map;
        self.material_map = material_map;
        self.property_map = property_map;
        self.set_map = set_map;
        self.skill_type_map = skill_type_map;
        self.weapon_map = weapon_map;
        Ok(())
    }

    pub async fn fetch_affix_map(
        dimbreath: &Dimbreath,
        git_ref: &str,
    ) -> Result<HashMap<u32, Affix>> {
        let data: Vec<game_data::ReliquaryAffixExcelConfigDataEntry> = dimbreath
            .get_json_file(git_ref, "ExcelBinOutput/ReliquaryAffixExcelConfigData.json")
            .await?;

        Ok(data
            .iter()
            .filter_map(|entry| {
                let property = entry.prop_type.parse::<Property>().ok()?;
                let value = if property.is_percentage() {
                    entry.prop_value * 100.
                } else {
                    entry.prop_value
                };
                Some((entry.id, Affix { property, value }))
            })
            .collect())
    }

    pub async fn fetch_artifact_map(
        dimbreath: &Dimbreath,
        git_ref: &str,
        set_map: &HashMap<u32, String>,
    ) -> Result<HashMap<u32, Artifact>> {
        let data: Vec<game_data::ReliquaryExcelConfigDataEntry> = dimbreath
            .get_json_file(git_ref, "ExcelBinOutput/ReliquaryExcelConfigData.json")
            .await?;

        let map = data
            .iter()
            .filter_map(|entry| {
                let set = set_map.get(&entry.set_id)?.to_string();
                let slot = ArtifactSlot::from_game_data_name(&entry.equip_type)?;
                Some((
                    entry.id,
                    Artifact {
                        set,
                        slot,
                        rarity: entry.rank_level,
                    },
                ))
            })
            .collect();

        Ok(map)
    }

    pub async fn fetch_character_map(
        dimbreath: &Dimbreath,
        git_ref: &str,
        text_map: &HashMap<u32, String>,
    ) -> Result<HashMap<u32, String>> {
        let data: Vec<AvatarExcelConfigDataEntry> = dimbreath
            .get_json_file(git_ref, "ExcelBinOutput/AvatarExcelConfigData.json")
            .await?;

        Ok(data
            .iter()
            .filter_map(|entry| Some((entry.id, text_map.get(&entry.name_text_map_hash)?.clone())))
            .collect())
    }

    pub async fn fetch_material_map(
        dimbreath: &Dimbreath,
        git_ref: &str,
        text_map: &HashMap<u32, String>,
    ) -> Result<HashMap<u32, String>> {
        let data: Vec<MaterialExcelConfigDataEntry> = dimbreath
            .get_json_file(git_ref, "ExcelBinOutput/MaterialExcelConfigData.json")
            .await?;

        Ok(data
            .iter()
            .filter_map(|entry| Some((entry.id, text_map.get(&entry.name_text_map_hash)?.clone())))
            .collect())
    }

    pub async fn fetch_property_map(
        dimbreath: &Dimbreath,
        git_ref: &str,
    ) -> Result<HashMap<u32, Property>> {
        let data: Vec<ReliquaryMainPropExcelConfigDataEntry> = dimbreath
            .get_json_file(
                git_ref,
                "ExcelBinOutput/ReliquaryMainPropExcelConfigData.json",
            )
            .await?;

        Ok(data
            .iter()
            .filter_map(|entry| Some((entry.id, entry.prop_type.parse::<Property>().ok()?)))
            .collect())
    }

    pub async fn fetch_set_map(
        dimbreath: &Dimbreath,
        git_ref: &str,
        text_map: &HashMap<u32, String>,
    ) -> Result<HashMap<u32, String>> {
        let data: Vec<game_data::DisplayItemExcelConfigDataEntry> = dimbreath
            .get_json_file(git_ref, "ExcelBinOutput/DisplayItemExcelConfigData.json")
            .await?;

        Ok(data
            .iter()
            .filter_map(|entry| {
                if entry.display_type != "RELIQUARY_ITEM" {
                    return None;
                }
                let name = text_map.get(&entry.name_text_map_hash)?;
                Some((entry.param, name.clone()))
            })
            .collect())
    }

    pub async fn fetch_skill_type_map(
        dimbreath: &Dimbreath,
        git_ref: &str,
    ) -> Result<HashMap<u32, SkillType>> {
        let data: Vec<game_data::AvatarSkillDepotExcelConfigDataEntry> = dimbreath
            .get_json_file(
                git_ref,
                "ExcelBinOutput/AvatarSkillDepotExcelConfigData.json",
            )
            .await?;

        let mut type_map = HashMap::new();
        for config in data {
            type_map.insert(config.energy_skill, SkillType::Burst);
            type_map.insert(config.skills[0], SkillType::Auto);
            type_map.insert(config.skills[1], SkillType::Skill);
        }

        Ok(type_map)
    }

    pub async fn fetch_text_map(
        dimbreath: &Dimbreath,
        git_ref: &str,
    ) -> Result<HashMap<u32, String>> {
        dimbreath
            .get_json_file(git_ref, "TextMap/TextMapEN.json")
            .await
    }

    pub async fn fetch_weapon_map(
        dimbreath: &Dimbreath,
        git_ref: &str,
        text_map: &HashMap<u32, String>,
    ) -> Result<HashMap<u32, Weapon>> {
        let data: Vec<WeaponExcelConfigDataEntry> = dimbreath
            .get_json_file(git_ref, "ExcelBinOutput/WeaponExcelConfigData.json")
            .await?;

        Ok(data
            .iter()
            .filter_map(|entry| {
                let name = text_map.get(&entry.name_text_map_hash)?;
                Some((
                    entry.id,
                    Weapon {
                        name: name.clone(),
                        rarity: entry.rank_level,
                    },
                ))
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {}

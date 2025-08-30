use std::collections::HashMap;

use anyhow::Result;

mod dimbreath;
mod game_data;
mod types;

use dimbreath::Dimbreath;
use serde::de::DeserializeOwned;
pub use types::*;

use crate::game_data::{
    AvatarExcelConfigDataEntry, MaterialExcelConfigDataEntry,
    ReliquaryMainPropExcelConfigDataEntry, WeaponExcelConfigDataEntry,
};

trait GameDataSource {
    async fn get_latest_hash(&self) -> Result<String>;
    async fn get_json_file<T: DeserializeOwned>(&self, git_ref: &str, path: &str) -> Result<T>;
}

fn lookup_text(text_map: &HashMap<u32, String>, id: u32) -> Option<&String> {
    let res = text_map.get(&id);
    if res.is_none() {
        // TODO: replace with logging or tracing.
        println!("Unable to lookup text for hash {id}");
    }
    res
}

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
        self.update_impl(&dimbreath).await
    }

    async fn update_impl<Source: GameDataSource>(&mut self, source: &Source) -> Result<()> {
        let latest_git_hash = source.get_latest_hash().await?;
        if Some(latest_git_hash.clone()) == self.latest_git_hash {
            return Ok(());
        }

        // Fetch all data first before updating `self` to ensure consistency.
        let text_map = Self::fetch_text_map(source, &latest_git_hash).await?;
        let skill_type_map = Self::fetch_skill_type_map(source, &latest_git_hash).await?;
        let set_map = Self::fetch_set_map(source, &latest_git_hash, &text_map).await?;
        let artifact_map = Self::fetch_artifact_map(source, &latest_git_hash, &set_map).await?;
        let property_map = Self::fetch_property_map(source, &latest_git_hash).await?;
        let affix_map = Self::fetch_affix_map(source, &latest_git_hash).await?;
        let weapon_map = Self::fetch_weapon_map(source, &latest_git_hash, &text_map).await?;
        let material_map = Self::fetch_material_map(source, &latest_git_hash, &text_map).await?;
        let character_map = Self::fetch_character_map(source, &latest_git_hash, &text_map).await?;

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

    async fn fetch_affix_map<Source: GameDataSource>(
        source: &Source,
        git_ref: &str,
    ) -> Result<HashMap<u32, Affix>> {
        let data: Vec<game_data::ReliquaryAffixExcelConfigDataEntry> = source
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

    async fn fetch_artifact_map<Source: GameDataSource>(
        source: &Source,
        git_ref: &str,
        set_map: &HashMap<u32, String>,
    ) -> Result<HashMap<u32, Artifact>> {
        let data: Vec<game_data::ReliquaryExcelConfigDataEntry> = source
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

    async fn fetch_character_map<Source: GameDataSource>(
        source: &Source,
        git_ref: &str,
        text_map: &HashMap<u32, String>,
    ) -> Result<HashMap<u32, String>> {
        let data: Vec<AvatarExcelConfigDataEntry> = source
            .get_json_file(git_ref, "ExcelBinOutput/AvatarExcelConfigData.json")
            .await?;

        Ok(data
            .iter()
            .filter_map(|entry| {
                Some((
                    entry.id,
                    lookup_text(text_map, entry.name_text_map_hash)?.clone(),
                ))
            })
            .collect())
    }

    async fn fetch_material_map<Source: GameDataSource>(
        source: &Source,
        git_ref: &str,
        text_map: &HashMap<u32, String>,
    ) -> Result<HashMap<u32, String>> {
        let data: Vec<MaterialExcelConfigDataEntry> = source
            .get_json_file(git_ref, "ExcelBinOutput/MaterialExcelConfigData.json")
            .await?;

        Ok(data
            .iter()
            .filter_map(|entry| {
                Some((
                    entry.id,
                    lookup_text(text_map, entry.name_text_map_hash)?.clone(),
                ))
            })
            .collect())
    }

    async fn fetch_property_map<Source: GameDataSource>(
        source: &Source,
        git_ref: &str,
    ) -> Result<HashMap<u32, Property>> {
        let data: Vec<ReliquaryMainPropExcelConfigDataEntry> = source
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

    async fn fetch_set_map<Source: GameDataSource>(
        source: &Source,
        git_ref: &str,
        text_map: &HashMap<u32, String>,
    ) -> Result<HashMap<u32, String>> {
        let data: Vec<game_data::DisplayItemExcelConfigDataEntry> = source
            .get_json_file(git_ref, "ExcelBinOutput/DisplayItemExcelConfigData.json")
            .await?;

        Ok(data
            .iter()
            .filter_map(|entry| {
                if entry.display_type != "RELIQUARY_ITEM" {
                    return None;
                }
                let name = lookup_text(text_map, entry.name_text_map_hash)?;
                Some((entry.param, name.clone()))
            })
            .collect())
    }

    async fn fetch_skill_type_map<Source: GameDataSource>(
        source: &Source,
        git_ref: &str,
    ) -> Result<HashMap<u32, SkillType>> {
        let data: Vec<game_data::AvatarSkillDepotExcelConfigDataEntry> = source
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

    async fn fetch_text_map<Source: GameDataSource>(
        source: &Source,
        git_ref: &str,
    ) -> Result<HashMap<u32, String>> {
        source
            .get_json_file(git_ref, "TextMap/TextMapEN.json")
            .await
    }

    async fn fetch_weapon_map<Source: GameDataSource>(
        source: &Source,
        git_ref: &str,
        text_map: &HashMap<u32, String>,
    ) -> Result<HashMap<u32, Weapon>> {
        let data: Vec<WeaponExcelConfigDataEntry> = source
            .get_json_file(git_ref, "ExcelBinOutput/WeaponExcelConfigData.json")
            .await?;

        Ok(data
            .iter()
            .filter_map(|entry| {
                let name = lookup_text(text_map, entry.name_text_map_hash)?;
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
mod tests {
    use anyhow::anyhow;

    use super::*;

    struct TestDataSoruce;

    impl GameDataSource for TestDataSoruce {
        async fn get_latest_hash(&self) -> Result<String> {
            Ok("13be4fd7343fe4cee8fa0096fe854b1c5b01b124".into())
        }

        async fn get_json_file<T: DeserializeOwned>(
            &self,
            _git_ref: &str,
            path: &str,
        ) -> Result<T> {
            let json_string = match path {
                "ExcelBinOutput/ReliquaryAffixExcelConfigData.json" => {
                    include_str!("test_data/ExcelBinOutput/ReliquaryAffixExcelConfigData.json")
                }
                "ExcelBinOutput/ReliquaryExcelConfigData.json" => {
                    include_str!("test_data/ExcelBinOutput/ReliquaryExcelConfigData.json")
                }
                "ExcelBinOutput/AvatarExcelConfigData.json" => {
                    include_str!("test_data/ExcelBinOutput/AvatarExcelConfigData.json")
                }
                "ExcelBinOutput/MaterialExcelConfigData.json" => {
                    include_str!("test_data/ExcelBinOutput/MaterialExcelConfigData.json")
                }
                "ExcelBinOutput/ReliquaryMainPropExcelConfigData.json" => {
                    include_str!("test_data/ExcelBinOutput/ReliquaryMainPropExcelConfigData.json")
                }
                "ExcelBinOutput/DisplayItemExcelConfigData.json" => {
                    include_str!("test_data/ExcelBinOutput/DisplayItemExcelConfigData.json")
                }
                "ExcelBinOutput/AvatarSkillDepotExcelConfigData.json" => {
                    include_str!("test_data/ExcelBinOutput/AvatarSkillDepotExcelConfigData.json")
                }
                "ExcelBinOutput/WeaponExcelConfigData.json" => {
                    include_str!("test_data/ExcelBinOutput/WeaponExcelConfigData.json")
                }
                "TextMap/TextMapEN.json" => {
                    include_str!("test_data/TextMap/TextMapEN.json")
                }
                _ => return Err(anyhow!("no test data for {path}")),
            };

            let data = serde_json::from_str(json_string)?;
            Ok(data)
        }
    }

    #[tokio::test]
    async fn character_map_returns_correct_character() {
        let source = TestDataSoruce;
        let mut data = AnimeGameData::new().unwrap();
        data.update_impl(&source).await.unwrap();
        assert_eq!(data.character_map.get(&10000061), Some(&"Kirara".into()));
    }

    #[tokio::test]
    async fn skill_type_map_returns_correct_type() {
        let source = TestDataSoruce;
        let mut data = AnimeGameData::new().unwrap();
        data.update_impl(&source).await.unwrap();
        assert_eq!(data.skill_type_map.get(&10024), Some(&SkillType::Auto));
        assert_eq!(data.skill_type_map.get(&10018), Some(&SkillType::Skill));
        assert_eq!(data.skill_type_map.get(&10019), Some(&SkillType::Burst));
    }

    #[tokio::test]
    async fn set_map_returns_correct_set() {
        let source = TestDataSoruce;
        let mut data = AnimeGameData::new().unwrap();
        data.update_impl(&source).await.unwrap();
        assert_eq!(
            data.set_map.get(&15031),
            Some(&"Marechaussee Hunter".into())
        );
    }

    #[tokio::test]
    async fn material_map_returns_correct_material() {
        let source = TestDataSoruce;
        let mut data = AnimeGameData::new().unwrap();
        data.update_impl(&source).await.unwrap();
        assert_eq!(data.material_map.get(&100002), Some(&"Sunsettia".into()));
    }

    #[tokio::test]
    async fn affix_map_returns_correct_affix() {
        let source = TestDataSoruce;
        let mut data = AnimeGameData::new().unwrap();
        data.update_impl(&source).await.unwrap();

        // Flat affixes contain their vaule unmodified.
        assert_eq!(
            data.affix_map.get(&501022),
            Some(&Affix {
                property: Property::Hp,
                value: 239.0
            })
        );

        // Prercentage affixes get multipled by 100 from thier data value.
        assert_eq!(
            data.affix_map.get(&982001),
            Some(&Affix {
                property: Property::GeoDamage,
                value: 80.0
            })
        );
    }

    #[tokio::test]
    async fn artifact_map_returns_correct_artifact() {
        let source = TestDataSoruce;
        let mut data = AnimeGameData::new().unwrap();
        data.update_impl(&source).await.unwrap();

        assert_eq!(
            data.artifact_map.get(&31534),
            Some(&Artifact {
                set: "Marechaussee Hunter".into(),
                slot: ArtifactSlot::Circlet,
                rarity: 5
            })
        );
    }

    #[tokio::test]
    async fn proptery_map_returns_correct_property() {
        let source = TestDataSoruce;
        let mut data = AnimeGameData::new().unwrap();
        data.update_impl(&source).await.unwrap();

        assert_eq!(data.property_map.get(&50960), Some(&Property::PyroDamage));
    }

    #[tokio::test]
    async fn weapon_map_returns_correct_weapon() {
        let source = TestDataSoruce;
        let mut data = AnimeGameData::new().unwrap();
        data.update_impl(&source).await.unwrap();

        assert_eq!(
            data.weapon_map.get(&11505),
            Some(&Weapon {
                name: "Primordial Jade Cutter".into(),
                rarity: 5
            })
        );
    }
}

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};

mod dimbreath;
mod game_data;
mod types;

use dimbreath::Dimbreath;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
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
        tracing::debug!("Unable to lookup text for hash {id}");
    }
    res
}

const DATABASE_VERSION: u32 = 0;

#[derive(Debug, Deserialize, Serialize)]
struct Database {
    version: u32,
    git_hash: String,
    affix_map: HashMap<u32, Affix>,
    artifact_map: HashMap<u32, Artifact>,
    character_map: HashMap<u32, String>,
    material_map: HashMap<u32, String>,
    property_map: HashMap<u32, Property>,
    set_map: HashMap<u32, String>,
    skill_type_map: HashMap<u32, SkillType>,
    weapon_map: HashMap<u32, Weapon>,
}

impl Database {
    pub fn new(git_hash: &str) -> Self {
        Self {
            version: DATABASE_VERSION,
            git_hash: git_hash.into(),
            affix_map: HashMap::new(),
            artifact_map: HashMap::new(),
            character_map: HashMap::new(),
            material_map: HashMap::new(),
            property_map: HashMap::new(),
            set_map: HashMap::new(),
            skill_type_map: HashMap::new(),
            weapon_map: HashMap::new(),
        }
    }

    pub fn load_from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let db = serde_json::from_reader(reader)?;
        Ok(db)
    }
}

#[derive(Debug)]
pub struct AnimeGameData {
    cache_path: Option<PathBuf>,
    db: Option<Database>,
}

impl AnimeGameData {
    pub fn new() -> Result<Self> {
        Ok(Self {
            cache_path: None,
            db: None,
        })
    }

    pub fn new_with_cache<P: AsRef<Path>>(cache_path: P) -> Result<Self> {
        let cache_path = cache_path.as_ref();

        // Try to load cached data ignoring errors and instead leave and empty
        // database.
        let db = Database::load_from_path(cache_path).ok();

        Ok(Self {
            cache_path: Some(cache_path.to_owned()),
            db,
        })
    }

    fn db(&self) -> Result<&Database> {
        self.db.as_ref().ok_or_else(|| anyhow!("No data loaded"))
    }

    pub fn get_affix(&self, id: u32) -> Result<&Affix> {
        self.db()?
            .affix_map
            .get(&id)
            .ok_or_else(|| anyhow!("Unable to fetch affix {id}"))
    }

    pub fn get_artifact(&self, id: u32) -> Result<&Artifact> {
        self.db()?
            .artifact_map
            .get(&id)
            .ok_or_else(|| anyhow!("Unable to fetch artifact {id}"))
    }

    pub fn get_character(&self, id: u32) -> Result<&String> {
        self.db()?
            .character_map
            .get(&id)
            .ok_or_else(|| anyhow!("Unable to fetch character {id}"))
    }

    pub fn get_material(&self, id: u32) -> Result<&String> {
        self.db()?
            .material_map
            .get(&id)
            .ok_or_else(|| anyhow!("Unable to fetch material {id}"))
    }

    pub fn get_property(&self, id: u32) -> Result<&Property> {
        self.db()?
            .property_map
            .get(&id)
            .ok_or_else(|| anyhow!("Unable to fetch property {id}"))
    }

    pub fn get_set(&self, id: u32) -> Result<&String> {
        self.db()?
            .set_map
            .get(&id)
            .ok_or_else(|| anyhow!("Unable to fetch set {id}"))
    }

    pub fn get_skill_type(&self, id: u32) -> Result<&SkillType> {
        self.db()?
            .skill_type_map
            .get(&id)
            .ok_or_else(|| anyhow!("Unable to fetch skill type {id}"))
    }

    pub fn get_weapon(&self, id: u32) -> Result<&Weapon> {
        self.db()?
            .weapon_map
            .get(&id)
            .ok_or_else(|| anyhow!("Unable to fetch weapon {id}"))
    }

    pub fn has_data(&self) -> bool {
        self.db.is_some()
    }

    pub async fn needs_update(&self) -> Result<bool> {
        self.needs_update_impl(&Dimbreath::new()?).await
    }

    async fn needs_update_impl<Source: GameDataSource>(&self, source: &Source) -> Result<bool> {
        let Some(db) = &self.db else {
            return Ok(true);
        };
        Ok(db.git_hash != source.get_latest_hash().await?)
    }

    pub async fn update(&mut self) -> Result<()> {
        self.update_impl(&Dimbreath::new()?).await
    }

    async fn update_impl<Source: GameDataSource>(&mut self, source: &Source) -> Result<()> {
        // Check if data is already up to date
        let latest_git_hash = source.get_latest_hash().await?;
        if let Some(db) = &self.db
            && db.git_hash == latest_git_hash
        {
            return Ok(());
        }

        // Index all data into a separate DB to ensure consistency.
        let mut db = Database::new(&latest_git_hash);
        let text_map = Self::fetch_text_map(source, &latest_git_hash).await?;
        db.skill_type_map = Self::fetch_skill_type_map(source, &latest_git_hash).await?;
        db.set_map = Self::fetch_set_map(source, &latest_git_hash, &text_map).await?;
        db.artifact_map = Self::fetch_artifact_map(source, &latest_git_hash, &db.set_map).await?;
        db.property_map = Self::fetch_property_map(source, &latest_git_hash).await?;
        db.affix_map = Self::fetch_affix_map(source, &latest_git_hash).await?;
        db.weapon_map = Self::fetch_weapon_map(source, &latest_git_hash, &text_map).await?;
        db.material_map = Self::fetch_material_map(source, &latest_git_hash, &text_map).await?;
        db.character_map = Self::fetch_character_map(source, &latest_git_hash, &text_map).await?;

        self.db = Some(db);

        let _ = self.try_save_db();
        Ok(())
    }

    fn try_save_db(&self) -> Result<()> {
        let Some(cache_path) = &self.cache_path else {
            return Err(anyhow!("no cache path provided"));
        };

        let file = File::create(cache_path)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, &self.db)?;

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
    use tempfile::NamedTempFile;

    use super::*;

    struct TestDataSource;

    impl GameDataSource for TestDataSource {
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

    struct TestDataSource2;

    impl GameDataSource for TestDataSource2 {
        async fn get_latest_hash(&self) -> Result<String> {
            Ok("13be4fd7343fe4cee8fa0096fe854b1c5b01b124-2".into())
        }

        async fn get_json_file<T: DeserializeOwned>(&self, git_ref: &str, path: &str) -> Result<T> {
            let json_string = match path {
                "ExcelBinOutput/ReliquaryAffixExcelConfigData.json" => {
                    include_str!("test_data/ExcelBinOutput/ReliquaryAffixExcelConfigData2.json")
                }
                _ => return TestDataSource {}.get_json_file(git_ref, path).await,
            };
            let data = serde_json::from_str(json_string)?;
            Ok(data)
        }
    }

    #[tokio::test]
    async fn character_map_returns_correct_character() {
        let source = TestDataSource;
        let mut data = AnimeGameData::new().unwrap();
        data.update_impl(&source).await.unwrap();
        assert_eq!(data.get_character(10000061).unwrap(), &"Kirara".to_string());
    }

    #[tokio::test]
    async fn skill_type_map_returns_correct_type() {
        let source = TestDataSource;
        let mut data = AnimeGameData::new().unwrap();
        data.update_impl(&source).await.unwrap();
        assert_eq!(data.get_skill_type(10024).unwrap(), &SkillType::Auto);
        assert_eq!(data.get_skill_type(10018).unwrap(), &SkillType::Skill);
        assert_eq!(data.get_skill_type(10019).unwrap(), &SkillType::Burst);
    }

    #[tokio::test]
    async fn set_map_returns_correct_set() {
        let source = TestDataSource;
        let mut data = AnimeGameData::new().unwrap();
        data.update_impl(&source).await.unwrap();
        assert_eq!(
            data.get_set(15031).unwrap(),
            &"Marechaussee Hunter".to_string()
        );
    }

    #[tokio::test]
    async fn material_map_returns_correct_material() {
        let source = TestDataSource;
        let mut data = AnimeGameData::new().unwrap();
        data.update_impl(&source).await.unwrap();
        assert_eq!(data.get_material(100002).unwrap(), &"Sunsettia".to_string());
    }

    #[tokio::test]
    async fn affix_map_returns_correct_affix() {
        let source = TestDataSource;
        let mut data = AnimeGameData::new().unwrap();
        data.update_impl(&source).await.unwrap();

        // Flat affixes contain their vaule unmodified.
        assert_eq!(
            data.get_affix(501022).unwrap(),
            &Affix {
                property: Property::Hp,
                value: 239.0
            }
        );

        // Prercentage affixes get multipled by 100 from thier data value.
        assert_eq!(
            data.get_affix(982001).unwrap(),
            &Affix {
                property: Property::GeoDamage,
                value: 80.0
            }
        );
    }

    #[tokio::test]
    async fn artifact_map_returns_correct_artifact() {
        let source = TestDataSource;
        let mut data = AnimeGameData::new().unwrap();
        data.update_impl(&source).await.unwrap();

        assert_eq!(
            data.get_artifact(31534).unwrap(),
            &Artifact {
                set: "Marechaussee Hunter".into(),
                slot: ArtifactSlot::Circlet,
                rarity: 5
            }
        );
    }

    #[tokio::test]
    async fn proptery_map_returns_correct_property() {
        let source = TestDataSource;
        let mut data = AnimeGameData::new().unwrap();
        data.update_impl(&source).await.unwrap();

        assert_eq!(data.get_property(50960).unwrap(), &Property::PyroDamage);
    }

    #[tokio::test]
    async fn weapon_map_returns_correct_weapon() {
        let source = TestDataSource;
        let mut data = AnimeGameData::new().unwrap();
        data.update_impl(&source).await.unwrap();

        assert_eq!(
            data.get_weapon(11505).unwrap(),
            &Weapon {
                name: "Primordial Jade Cutter".into(),
                rarity: 5
            }
        );
    }

    #[tokio::test]
    async fn data_is_cached_by_update() {
        let tempfile = NamedTempFile::new().unwrap();

        let source = TestDataSource;
        let mut data = AnimeGameData::new_with_cache(tempfile.path()).unwrap();

        // Affix does not exist before update
        assert!(data.get_affix(501022).is_err());

        data.update_impl(&source).await.unwrap();

        // Affix exists after update
        assert_eq!(
            data.get_affix(501022).unwrap(),
            &Affix {
                property: Property::Hp,
                value: 239.0
            }
        );

        drop(data);

        // Re-open data with valid cache.
        let data = AnimeGameData::new_with_cache(tempfile.path()).unwrap();

        // Affix exists after loading from cache
        assert_eq!(
            data.get_affix(501022).unwrap(),
            &Affix {
                property: Property::Hp,
                value: 239.0
            }
        );
    }

    #[tokio::test]
    async fn cached_data_is_updated_from_source() {
        let tempfile = NamedTempFile::new().unwrap();

        let source = TestDataSource;
        let mut data = AnimeGameData::new_with_cache(tempfile.path()).unwrap();

        // Affix does not exist before update
        assert!(data.get_affix(501022).is_err());

        data.update_impl(&source).await.unwrap();

        // Affix exists after update
        assert_eq!(
            data.get_affix(501022).unwrap(),
            &Affix {
                property: Property::Hp,
                value: 239.0
            }
        );

        drop(data);

        // Re-open data with valid cache.
        let mut data = AnimeGameData::new_with_cache(tempfile.path()).unwrap();

        // Affix exists after loading from cache
        assert_eq!(
            data.get_affix(501022).unwrap(),
            &Affix {
                property: Property::Hp,
                value: 239.0
            }
        );

        let source = TestDataSource2;
        data.update_impl(&source).await.unwrap();

        // Affix is updated with second source data
        assert_eq!(
            data.get_affix(501022).unwrap(),
            &Affix {
                property: Property::Hp,
                value: 240.0
            }
        );
    }

    #[tokio::test]
    async fn needs_update_is_correct_across_caches_and_updates() {
        let tempfile = NamedTempFile::new().unwrap();

        let mut data = AnimeGameData::new_with_cache(tempfile.path()).unwrap();

        let source = TestDataSource;
        let source2 = TestDataSource2;
        // A new database always needs updating.
        assert!(data.needs_update_impl(&source).await.unwrap());

        // After updating an update is no longer needed.
        data.update_impl(&source).await.unwrap();
        assert!(!data.needs_update_impl(&source).await.unwrap());

        drop(data);

        let mut data = AnimeGameData::new_with_cache(tempfile.path()).unwrap();
        // After re-opening A new database doesn't need an update from the same
        // source.
        assert!(!data.needs_update_impl(&source).await.unwrap());

        // With a new source, it does need updating
        assert!(data.needs_update_impl(&source2).await.unwrap());

        // After updating an update is no longer needed.
        data.update_impl(&source2).await.unwrap();
        assert!(!data.needs_update_impl(&source2).await.unwrap());
    }
}

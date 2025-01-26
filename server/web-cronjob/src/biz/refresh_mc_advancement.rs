use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::fs::DirEntry;
use tokio::io::{AsyncReadExt, BufReader};
use web_db::mc_config::{delete_all_advancement, insert_advancement, Advancement};
use web_db::{begin_tx, create_connection, Transaction, RDS};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct McAdvancement {
    parent: Option<String>,
    display: Option<McAdvancementDisplay>,
    requirements: Option<Vec<Vec<String>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct McAdvancementDisplay {
    title: McTranslate,
    description: McTranslate,
    icon: Icon,
    frame: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Icon {
    item: Option<String>,
    block: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct McTranslate {
    translate: String,
}

pub async fn handle(path: &str, lang: &str) -> Result<()> {
    let lang = parse_lang(lang).await?;

    let mut conn = create_connection(RDS::McConfig).await?;
    let mut tx = begin_tx(&mut conn).await?;

    // delete_all_advancement(&mut tx).await?;

    let mut namespace_entries = tokio::fs::read_dir(path).await?;
    while let Some(entry) = namespace_entries.next_entry().await? {
        collect_advancement(&mut tx, entry, &lang).await?;
    }

    tx.commit().await?;
    Ok(())
}

async fn parse_lang(file: &str) -> Result<HashMap<String, String>> {
    let file = tokio::fs::File::open(file).await?;
    let mut reader = BufReader::new(file);
    let mut data = Vec::new();
    reader.read_to_end(&mut data).await?;

    Ok(serde_json::from_slice(data.as_slice())?)
}

async fn collect_advancement(
    tx: &mut Transaction<'_>,
    namespace_entry: DirEntry,
    lang: &HashMap<String, String>,
) -> Result<()> {
    let mut advancement_entries = tokio::fs::read_dir(namespace_entry.path()).await?;

    while let Some(entry) = advancement_entries.next_entry().await? {
        let file = tokio::fs::File::open(entry.path()).await?;
        let mut reader = BufReader::new(file);
        let mut data = Vec::new();
        reader.read_to_end(&mut data).await?;

        let mc_advancement: McAdvancement = serde_json::from_slice(data.as_slice())
            .map_err(|e| anyhow!("parse file {entry:?} fail: {e}"))?;

        let Some(display) = mc_advancement.display else {
            continue;
        };

        let mut advancement = Advancement {
            id: 0,
            mcid: format!(
                "blazeandcave:{}/{}",
                namespace_entry.file_name().to_string_lossy(),
                entry.file_name().to_string_lossy(),
            ),
            title: display.title.get(lang)?,
            description: display.description.get(lang)?,
            icon: get_icon(&display.icon),
            frame: display.frame.clone().unwrap_or("task".to_string()),
            parent: mc_advancement.parent.clone(),
            requirements: serde_json::ser::to_string(&mc_advancement.requirements)?,
        };

        insert_advancement(tx, &mut advancement).await?;
    }

    Ok(())
}

fn get_icon(icon: &Icon) -> Option<String> {
    if let Some(img) = &icon.item {
        Some(img.clone())
    } else {
        icon.block.clone()
    }
}

impl McTranslate {
    fn get(&self, lang: &HashMap<String, String>) -> Result<String> {
        Ok(lang
            .get(&self.translate)
            .ok_or(anyhow!("translate key not found"))?
            .clone())
    }
}

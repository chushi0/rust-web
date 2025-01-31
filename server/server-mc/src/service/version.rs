use anyhow::Result;
use chrono::{DateTime, Utc};
use common::tonic_idl_gen::*;
use reqwest::Client;
use serde::Deserialize;
use server_common::db::context::{Context, ContextRef};
use sqlx::Database;
use tracing::info;

use crate::dao::version::{ListVersionParameters, Version, VersionRepository, VersionType};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteVersion {
    id: String,
    r#type: RemoteVersionType,
    url: String,
    release_time: DateTime<Utc>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum RemoteVersionType {
    Release,
    Snapshot,
    OldBeta,
    OldAlpha,
}

#[derive(Debug, Deserialize)]
struct RemoteVersionManifest {
    latest: LatestVersion,
    versions: Vec<RemoteVersion>,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
struct LatestVersion {
    release: String,
    snapshot: String,
}

#[derive(Debug, Deserialize)]
struct VersionDetail {
    downloads: VersionDownloads,
}

#[derive(Debug, Deserialize)]
struct VersionDownloads {
    server: Option<VersionDownloadDetail>,
}

#[derive(Debug, Deserialize)]
struct VersionDownloadDetail {
    url: String,
}

async fn get_remote_version_manifest(client: &Client) -> Result<RemoteVersionManifest> {
    Ok(client
        .get("https://launchermeta.mojang.com/mc/game/version_manifest.json")
        .send()
        .await?
        .json()
        .await?)
}

async fn get_remote_version_detail(client: &Client, url: &str) -> Result<VersionDetail> {
    Ok(client.get(url).send().await?.json().await?)
}

pub async fn sync_version<DB: Database>(client: &Client, db: ContextRef<'_, '_, DB>) -> Result<()>
where
    for<'db> Context<'db, DB>: VersionRepository,
{
    let remote_versions = get_remote_version_manifest(client).await?;

    // 检查最新版本（含快照）是否在数据库中，如果存在，则无需更新
    let latest_release = remote_versions.latest.release;
    let latest_snapshot = remote_versions.latest.snapshot;
    if db.get_version_by_mcid(&latest_release).await?.is_some()
        && db.get_version_by_mcid(&latest_snapshot).await?.is_some()
    {
        info!("latest versions are already in db");
        return Ok(());
    }

    // 获取数据库中最新版本的发布时间
    let last_release_time = db
        .list_version(&ListVersionParameters {
            offset: 0,
            limit: 1,
            has_snapshot: true,
        })
        .await?
        .pop()
        .map(|version| version.release_time)
        .unwrap_or_default();

    // 将远程版本中发布时间晚于数据库中最新版本的版本插入数据库
    let mut remote_versions = remote_versions
        .versions
        .into_iter()
        .filter(|version| version.release_time > last_release_time)
        .collect::<Vec<_>>();
    remote_versions.sort_by_key(|remote| remote.release_time);

    for version in remote_versions {
        // 检查此版本是否在数据库
        if db.get_version_by_mcid(&version.id).await?.is_some() {
            continue;
        }

        // 获取版本详细信息
        info!("fetching version {} with url {}", version.id, version.url);
        let version_detail = get_remote_version_detail(client, &version.url).await?;

        let Some(server) = version_detail.downloads.server else {
            info!("version {} has no server download, skip", version.id);
            continue;
        };

        // 写入数据库
        db.create_version(&mut Version {
            mc_id: version.id,
            r#type: version.r#type.into(),
            server_url: server.url,
            release_time: version.release_time,
            ..Default::default()
        })
        .await?;
    }

    Ok(())
}

pub async fn list_mc_version<DB: Database>(
    db: ContextRef<'_, '_, DB>,
    req: ListMcVersionRequest,
) -> Result<ListMcVersionResponse>
where
    for<'db> Context<'db, DB>: VersionRepository,
{
    let params = ListVersionParameters {
        offset: req.offset,
        limit: req.count,
        has_snapshot: req.has_snapshot.unwrap_or(true),
    };

    let versions = db
        .list_version(&params)
        .await?
        .into_iter()
        .map(|version| McVersion {
            id: version.mc_id,
            r#type: McVersionType::from(version.r#type) as i32,
            release_time: version.release_time.timestamp(),
        })
        .collect();
    let count = db.count_version(&params).await?;

    Ok(ListMcVersionResponse {
        total: count,
        versions,
    })
}

impl From<RemoteVersionType> for VersionType {
    fn from(r#type: RemoteVersionType) -> Self {
        match r#type {
            RemoteVersionType::Release => VersionType::Release,
            RemoteVersionType::Snapshot => VersionType::Snapshot,
            RemoteVersionType::OldBeta => VersionType::OldBeta,
            RemoteVersionType::OldAlpha => VersionType::OldAlpha,
        }
    }
}

impl From<VersionType> for McVersionType {
    fn from(r#type: VersionType) -> Self {
        match r#type {
            VersionType::Release => McVersionType::Release,
            VersionType::Snapshot => McVersionType::Snapshot,
            VersionType::OldBeta => McVersionType::Snapshot,
            VersionType::OldAlpha => McVersionType::Snapshot,
        }
    }
}

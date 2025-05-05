use std::{collections::{BTreeSet, BTreeMap}, io::{stdout, Write}, result};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::resource_parse::{ResrcDependency, ResrcDescriptor, ResrcData, ResrcMethod};
use crate::config::DownloadServer;
use crate::USER_AGENT;

use reqwest::{Client, ClientBuilder, StatusCode};
use sha1::{Digest, Sha1};
use futures_util::future::BoxFuture;
use futures_util::FutureExt;
use tokio::sync::{AcquireError, Semaphore};
use tokio::task::JoinSet;
use anyhow::{anyhow, Result};
use thiserror::Error;

#[derive(Error, Debug)]
enum DownloadError {
    #[error("status code error: {0}")]
    StatusCode(StatusCode),
    #[error("io error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("semaphore acquire error: {0}")]
    SemaphoreAcquire(#[from] AcquireError),
    #[error("reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
}

#[derive(Clone)]
struct Downloader {
    client: Client,
    download_server: Arc<DownloadServer>,
    downloaded: Arc<Mutex<BTreeSet<[u8; 20]>>>,
    cache: Arc<Mutex<BTreeMap<[u8; 20], Vec<u8>>>>,
    semaphore: Arc<Semaphore>,

    successful: Arc<AtomicUsize>,
    failed: Arc<AtomicUsize>,
}

impl<'a> Downloader {
    fn new(download_server: DownloadServer, max_parallel: usize) -> Result<Self> {
        let client = ClientBuilder::new()
            .user_agent(USER_AGENT)
            .build()?;
        Ok(Self {
            client,
            download_server: Arc::new(download_server),
            downloaded: Arc::new(Mutex::new(BTreeSet::new())),
            cache: Arc::new(Mutex::new(BTreeMap::new())),
            semaphore: Arc::new(Semaphore::new(max_parallel)),

            successful: Arc::new(AtomicUsize::new(0)),
            failed: Arc::new(AtomicUsize::new(0)),
        })
    }

    fn is_downloaded(&self, hash: [u8; 20]) -> Result<bool> {
        let lock = self.downloaded.lock().map_err(|_| anyhow!("Couldn't acquire mutex in is_downloaded"))?;
        Ok((*lock).contains(&hash))
    }

    fn set_downloaded(&self, hash: [u8; 20]) -> Result<()> {
        let mut lock = self.downloaded.lock().map_err(|_| anyhow!("Couldn't acquire mutex in set_downloaded"))?;
        (*lock).insert(hash);
        Ok(())
    }

    fn add_to_cache(&self, hash: [u8; 20], data: Vec<u8>) -> Result<()> {
        let mut lock = self.cache.lock().map_err(|_| anyhow!("Couldn't acquire mutex in add_to_cache"))?;
        (*lock).insert(hash, data);
        Ok(())
    }

    async fn download_resource(&self, sha1: &[u8; 20]) -> result::Result<Vec<u8>, DownloadError> {
        let url = self.download_server.get_url(sha1);
        let mut resp = {
            let _permit = self.semaphore.acquire().await?;
            self.client.get(url).send().await?
        };

        if resp.status() != 200 {
            return Err(DownloadError::StatusCode(resp.status()));
        }

        let mut resource = match resp.content_length() {
            Some(len) => Vec::with_capacity(len as usize),
            None => Vec::new(),
        };

        while let Some(chunk) = resp.chunk().await? {
            resource.write_all(&chunk)?;
        }

        assert_eq!(Sha1::digest(&resource).as_slice(), sha1);

        Ok(resource)
    }

    fn download_boxed(&'a self, sha1: &'a [u8; 20]) -> BoxFuture<'a, Result<()>> {
        async move {
            self.download_with_dependencies(sha1).await
        }.boxed()
    }

    async fn download_with_dependencies(&self, sha1: &[u8; 20]) -> Result<()> {
        if self.is_downloaded(*sha1)? {
            return Ok(());
        }

        let resource = self.download_resource(sha1).await;
        let resource = match resource {
            Ok(resource) => {
                print!(".");
                stdout().flush()?;
                self.successful.fetch_add(1, Ordering::SeqCst);
                resource
            },
            Err(error) => {
                if let DownloadError::StatusCode(_) = error {
                    print!("!");
                    stdout().flush()?;
                    self.failed.fetch_add(1, Ordering::SeqCst);
                    return Ok(());
                }
                return Err(anyhow!(error));
            }
        };

        self.set_downloaded(*sha1)?;

        let metadata = ResrcData::new(&resource, false)?;

        self.add_to_cache(*sha1, resource)?;

        let mut tasks = JoinSet::new();

        if let ResrcMethod::Binary { dependencies, .. } = metadata.method {
            for ResrcDependency { desc, .. } in dependencies {
                if let ResrcDescriptor::Sha1(sha1) = desc {
                    let downloader = self.clone();
                    tasks.spawn(async move {
                        downloader.download_boxed(&sha1).await
                    });
                }
            }
        }

        // Wait for all dependencies to complete
        while let Some(res) = tasks.join_next().await {
            let _ = res?;
        }

        Ok(())
    }

    fn get_stats(&self) -> (usize, usize) {
        (self.successful.load(Ordering::SeqCst), self.failed.load(Ordering::SeqCst))
    }
}

pub struct DownloadResult {
    // please note:
    // fat entries NEED to be sorted by hash in the SaveArchive,
    // so we store all resources in a BTreeMap to have them automatically sorted
    pub resources: BTreeMap<[u8; 20], Vec<u8>>,
    pub success_count: usize,
    pub error_count: usize,
}

pub async fn download_level(
    root_sha1: [u8; 20],
    icon_sha1: Option<[u8; 20]>,
    download_server: DownloadServer,
    max_parallel: usize,
) -> Result<DownloadResult> {
    let downloader = Downloader::new(download_server, max_parallel)?;

    let mut tasks = JoinSet::new();

    {
        let downloader = downloader.clone();
        tasks.spawn(async move {
            downloader.download_with_dependencies(&root_sha1).await
        });
    }

    if let Some(icon_sha1) = icon_sha1 {
        let downloader = downloader.clone();
        tasks.spawn(async move {
            downloader.download_with_dependencies(&icon_sha1).await
        });
    }

    while let Some(res) = tasks.join_next().await {
        let _ = res?;
    }

    let (success_count, error_count) = downloader.get_stats();
    let resources = Arc::try_unwrap(downloader.cache)
        .map_err(|_| anyhow!("couldn't unwrap downloader cache"))?
        .into_inner()?;

    Ok(DownloadResult {
        resources,
        success_count,
        error_count,
    })
}
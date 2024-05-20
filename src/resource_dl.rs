use std::{collections::BTreeMap, io::{stdout, Write}};

use async_recursion::async_recursion;
use reqwest::Client;
use sha1::{Digest, Sha1};

use crate::resource_parse::*;
use crate::config::DownloadServer;

#[async_recursion]
pub async fn get_resource(
    sha1: &[u8; 20],
    client: &mut Client,
    hashes: &mut BTreeMap<[u8; 20], Option<Vec<u8>>>,
    dl_count: &mut usize,
    fail_count: &mut usize,
    dl_server: &DownloadServer,
) {
    if hashes.contains_key(sha1) {
        return
    }

    hashes.insert(*sha1, None);

    let resource = download_res(sha1, client, dl_count, fail_count, dl_server).await;
    let resource = match resource {
        Some(res) => res,
        None => return,
    };

    let resrc_id = ResrcId::new(&resource);

    hashes.insert(*sha1, Some(resource));

    if let ResrcMethod::Binary { dependencies, .. } = resrc_id.method {
        for ResrcDependency { desc, .. } in dependencies {
            if let ResrcDescriptor::Sha1(sha1) = desc {
                get_resource(&sha1, client, hashes, dl_count, fail_count, dl_server).await;
            }
        }
    }
}

async fn download_res(
    sha1: &[u8; 20],
    client: &mut Client,
    dl_count: &mut usize,
    fail_count: &mut usize,
    dl_server: &DownloadServer,
) -> Option<Vec<u8>> {
    let url = dl_server.get_url(sha1);
    let mut resp = client.get(url).send().await.unwrap();

    if resp.status() != 200 {
        print!("!");
        stdout().flush().unwrap();
        *fail_count += 1;
        return None;
    }

    let mut resource = match resp.content_length() {
        Some(len) => Vec::with_capacity(len as usize),
        None => Vec::new(),
    };

    while let Some(chunk) = resp.chunk().await.unwrap() {
        resource.write_all(&chunk).unwrap();
    }

    assert!(Sha1::digest(&resource).as_slice() == sha1);

    print!(".");
    stdout().flush().unwrap();
    *dl_count += 1;

    Some(resource)
}
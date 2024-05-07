use std::{env::args, fs::{self, File}, io::{self, Read, Seek, SeekFrom, Write}, path::Path, vec};
use byteorder::{ReadBytesExt, BigEndian};
use async_recursion::async_recursion;
use reqwest::Client;
use sha1::{Digest, Sha1};

#[derive(Debug, PartialEq, Eq, Hash)]
struct ResrcId {
    resrc_type: [u8; 3],
    method: ResrcMethod,
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum ResrcMethod {
    Null,
    Binary {
        is_encrypted: bool,
        dependencies: Vec<ResrcDependency>,
    },
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct ResrcDependency {
    dep_type: ResrcDependencyType,
    resrc_type: u32,
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum ResrcDependencyType {
    Sha1([u8; 20]),
    Guid(u32),
}

impl ResrcDependency {
    fn parse_table(file: &mut File) -> Vec<Self> {
        let table_offset = file.read_u32::<BigEndian>().unwrap();
        let orig_offset = file.stream_position().unwrap();

        file.seek(SeekFrom::Start(table_offset as u64)).unwrap();

        let mut dependencies = vec![];
        for _ in 0..file.read_u32::<BigEndian>().unwrap() {
            let dep_type = match file.read_u8().unwrap() {
                0 => { // lbp3 dynamic thermometer levels use this??? why??????
                    file.seek(SeekFrom::Current(4)).unwrap(); // resrc_type
                    continue;
                }, 
                1 => {
                    let mut sha1 = [0u8; 20];
                    file.read_exact(&mut sha1).unwrap();
                    ResrcDependencyType::Sha1(sha1)
                },
                2 => ResrcDependencyType::Guid(file.read_u32::<BigEndian>().unwrap()),
                _ => panic!("what the fuck???"),
            };

            let resrc_type = file.read_u32::<BigEndian>().unwrap();

            dependencies.push(Self {
                dep_type,
                resrc_type,
            })
        }

        file.seek(SeekFrom::Start(orig_offset)).unwrap();

        dependencies
    }
}

impl ResrcId {
    fn new(file: &mut File) -> Self {
        let mut resrc_type = [0u8; 3];
        file.read_exact(&mut resrc_type).unwrap();

        let method = file.read_u8().unwrap();

        let method = match method {
            b'b' | b'e' => {
                let revision = file.read_u32::<BigEndian>().unwrap();
                let dependencies = match revision >= 0x109 {
                    true => ResrcDependency::parse_table(file),
                    false => vec![],
                };

                ResrcMethod::Binary {
                    is_encrypted: method == b'e',
                    dependencies,
                }
            },
            _ => { /* shit which isn't implemented yet i guess lol */ ResrcMethod::Null },
        };

        Self {
            resrc_type,
            method,
        }
    }
}

async fn download_res(sha1: &str, path: &str, client: &mut Client) -> Result<(), ()> {
    println!("Downloading {sha1}...");

    let mut url = format!("https://lbp.littlebigrefresh.com/api/v3/assets/{}/download", sha1);
    let mut resp = client.get(url).send().await.unwrap();
    
    if resp.status() != 200 {
        println!("Resource download {sha1} failed with HTTP status code {}, trying from archive.org...", resp.status());

        url = format!("https://archive.org/download/dry23r{}/dry{}.zip/{}%2F{}%2F{}", sha1.chars().next().unwrap(), &sha1[..2], &sha1[..2], &sha1[2..4], sha1);
        resp = client.get(url).send().await.unwrap();

        if resp.status() != 200 {
            println!("Resource download {sha1} failed with HTTP status code {}, skipping...", resp.status());
            return Err(())
        }
    }

    let mut file = File::create(&path).unwrap();

    while let Some(chunk) = resp.chunk().await.unwrap() {
        file.write_all(&chunk).unwrap();
    }

    Ok(())
}

#[async_recursion]
async fn get_resource(sha1_bytes: &[u8; 20], out_dir: &Path, is_rootlvl: bool, client: &mut Client) {
    let sha1 = hex::encode(sha1_bytes);
    let sha1 = sha1.as_str();
    let path = match is_rootlvl {
        true => format!("{}/lvl.bin", out_dir.to_str().unwrap()),
        false => format!("{}/{}", out_dir.to_str().unwrap(), sha1),
    };

    if Path::exists(Path::new(&path)) {
        let mut hasher = Sha1::new();
        let mut file = File::open(&path).unwrap();

        io::copy(&mut file, &mut hasher).unwrap();
        let file_hash = hasher.finalize();

        if *sha1_bytes == *file_hash {
            println!("Resource {sha1} already downloaded!");
        } else {
            if let Err(()) = download_res(sha1, &path, client).await {
                return
            }
        }
    } else {
        if let Err(()) = download_res(sha1, &path, client).await {
            return
        }
    }

    let mut file = File::open(path).unwrap();

    let resrc_id = ResrcId::new(&mut file);
    if let ResrcMethod::Binary { is_encrypted: _, dependencies } = resrc_id.method {
        for ResrcDependency { dep_type, resrc_type: _ } in dependencies {
            if let ResrcDependencyType::Sha1(sha1) = dep_type {
                get_resource(&sha1, out_dir, false, client).await;
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let mut args = args();
    args.next();

    let root_level = args.next().expect("rootLevel not specified");

    let out_dir = args.next().expect("Output directory not specified");
    let out_dir = Path::new(&out_dir);
    fs::create_dir_all(out_dir).unwrap();

    let mut sha1 = [0u8; 20];
    hex::decode_to_slice(root_level, &mut sha1).unwrap();

    let mut client = Client::new();

    get_resource(&sha1, out_dir, true, &mut client).await;
}
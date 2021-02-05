use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::option::Option::Some;
use std::path::{Path, PathBuf};
use std::process::{Command, exit, Stdio};

use reqwest::{Client, Error, Response};
use serde::Deserialize;
use zip::ZipArchive;

#[cfg(target_os = "windows")]
fn get_os() -> &'static str {
    return "windows";
}

#[cfg(target_os = "linux")]
fn get_os() -> &'static str {
    return "linux";
}

#[cfg(target_os = "macos")]
fn get_os() -> &'static str {
    return "mac";
}

const MIRAI_REPO: &'static str = "https://gitee.com/peratx/mirai-repo/raw/master";

#[derive(Deserialize)]
struct Package {
    announcement: Option<String>,
    #[serde(rename = "type")]
    package_type: Option<String>,
    channels: HashMap<String, Vec<String>>,
    repo: Option<HashMap<String, RepoInfo>>,
}

#[derive(Deserialize)]
struct RepoInfo {
    archive: Option<String>,
    metadata: Option<String>,
}

fn str_to_int(str: &str) -> i32 {
    let i = str.trim().parse::<i32>();
    if i.is_ok() {
        return i.unwrap();
    }
    return 0;
}

fn read_line() -> String {
    let mut tmp = String::new();
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut tmp).ok().expect("error");
    return tmp;
}

async fn get(client: &Client, str: &str) -> Result<Response, Error> {
    return client.get(str)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/88.0.4324.104 Safari/537.36")
        .send()
        .await;
}

fn unzip(path: &str) -> String {
    let mut zip = ZipArchive::new(File::open(path).unwrap()).unwrap();

    let len = zip.len();
    for i in 0..zip.len() {
        let mut file = zip.by_index(i).unwrap();
        let outpath = match file.enclosed_name() {
            Some(path) => path.to_owned(),
            None => continue,
        };

        print!("\rExtracting [{}/{}] {}", i + 1, len, file.name());
        if (&*file.name()).ends_with('/') {
            fs::create_dir_all(&outpath).unwrap();
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(&p).unwrap();
                }
            }
            let mut outfile = File::create(&outpath).unwrap();
            io::copy(&mut file, &mut outfile).unwrap();
        }
    }
    println!();

    return format!("{}", zip.by_index(0).unwrap().name());
}

async fn download(client: &Client, url: &str, file: &str) {
    println!("Start Downloading: {}", url);

    let mut res = get(&client, &url).await.unwrap();
    let ttl = res.headers().get(reqwest::header::CONTENT_LENGTH).unwrap().to_str().unwrap();
    let total = str_to_int(ttl);
    let mut current = 0;
    fs::remove_file(file);

    {
        let mut file = File::create(file).unwrap();

        while let Some(chunk) = res.chunk().await.unwrap() {
            current += chunk.len();
            file.write(&*chunk);
            print!("\rDownloading: {}/{}", current, total);
        }

        println!();
    }
}

#[tokio::main]
async fn main() {
    println!("iTXTech MCL Installer v1.0.0 [OS: {}]", get_os());
    println!("Licensed under GNU AGPLv3.");
    println!("https://github.com/iTXTech/mcl-installer");
    println!();

    let client = reqwest::Client::new();

    let mut java = String::new();
    if Path::new("./java").exists() {
        println!("Existing Java Executable detected, skip download JRE.");
    } else {
        print!("Java version (8-15, default: 11): ");
        let mut ver = str_to_int(&read_line());
        ver = if ver >= 8 && ver <= 15 { ver } else { 11 };

        print!("JRE or JDK (1: JRE, 2: JDK, default: JRE): ");
        let jre = if str_to_int(&read_line()) == 2 { "jdk" } else { "jre" };

        print!("Binary Architecture (default: x64): ");
        let a = read_line();
        let arch = if a.trim().is_empty() { "x64" } else { a.trim() };

        println!("Fetching file list for {} version {} on {}", jre, ver, arch);

        let url = format!("https://mirrors.tuna.tsinghua.edu.cn/AdoptOpenJDK/{}/{}/{}/{}/", ver, jre, arch, get_os());
        let resp = get(&client, &url).await;
        if !resp.is_ok() {
            println!("Fail to fetch AdoptOpenJDK download list.");
            exit(1);
        }
        let text = resp.unwrap().text().await.unwrap();
        let lines = text.split("\n");
        let pack = format!("OpenJDK{}U-{}", ver, jre);

        for line in lines {
            if line.contains(&pack) && line.contains("hotspot") && (line.contains(".zip") || line.contains(".tar.gz")) {
                let start = line.find(&pack).unwrap();
                let end = line.find("\" title=\"").unwrap();
                let archive = format!("{}{}", url, &line[start..end]);

                download(&client, &archive, "java.arc").await;

                let mut java_dir = String::new();
                #[cfg(target_os = "windows")]
                    { //zip
                        java_dir = unzip("java.arc");
                    }

                #[cfg(target_os = "linux")]
                    { //tar.gz
                        let mut process = Command::new("tar").arg("-zxvf").arg("java.arc")
                            .stdout(Stdio::piped())
                            .spawn().unwrap();
                        {
                            let lines = BufReader::new(process.stdout.as_mut().unwrap()).lines();
                            let mut j = false;
                            for line in lines {
                                let l = format!("{}", line.unwrap().trim());
                                if !j {
                                    let end = l.find("/").unwrap();
                                    java_dir = format!("{}", &l[0..end]);
                                    j = true;
                                }
                                print!("\rExtracting {}", l);
                            }
                        }
                        process.wait().unwrap();
                        println!();
                    }

                #[cfg(target_os = "macos")]
                    {
                        println!("Extracting Archive...");
                        let mut process = Command::new("tar").arg("-zxf").arg("java.arc")
                            .spawn().unwrap().wait().unwrap();
                        let start = archive.find("hotspot_").unwrap();
                        let end = archive.find(".tar.gz").unwrap();
                        java_dir = format!("jdk-{}{}", &archive[start + 8..end].replace("_", "+"), if jre == "jre" { "-jre" } else { "" });
                    }

                fs::remove_file("java.arc");
                fs::rename(java_dir, "java");

                break;
            }
        }
    }

    #[cfg(target_os = "windows")]
        {
            java = format!("{}\\bin\\java.exe", Path::new("java").canonicalize().unwrap().to_str().unwrap());
            java = format!("{}", &java[4..java.len()]);
        }
    #[cfg(target_os = "linux")]
        {
            java = format!("{}/bin/java", fs::canonicalize(Path::new("java")).unwrap().to_str().unwrap());
        }
    #[cfg(target_os = "macos")]
        {
            java = format!("{}/Contents/Home/bin/java", fs::canonicalize(Path::new("java")).unwrap().to_str().unwrap());
        }

    println!("Testing Java Executable: {}", java);

    Command::new(&java).arg("-version").spawn().unwrap().wait();
    println!();

    if Path::new("mcl.jar").exists() {
        let mut zip = ZipArchive::new(File::open("mcl.jar").unwrap()).unwrap();
        let mut buf = String::new();
        zip.by_name("META-INF/MANIFEST.MF").unwrap().read_to_string(&mut buf).unwrap();
        let start = buf.find("\nVersion: ").unwrap();
        let ver = format!("{}", &buf[start + 10..start + 23]);
        let hyphen = ver.find("-").unwrap();
        let major = format!("{}", &ver[0..hyphen]);
        let rev = format!("{}", &ver[hyphen + 1..ver.len()]);

        println!("iTXTech Mirai Console Loader detected.");
        println!("Major Version: {} Revision: {}", major, rev);
        println!();
    }

    let manifest_url = format!("{}/org/itxtech/mcl/package.json", MIRAI_REPO);
    println!("Fetching iTXTech MCL Package Info from {}", manifest_url);
    let manifest = get(&client, &manifest_url).await.unwrap().json::<Package>().await.unwrap();
    println!("{}", manifest.announcement.unwrap());

    let latest = manifest.channels.get("stable").unwrap().last().unwrap().to_string();
    println!("The latest stable version of iTXTech MCL is {}", latest);

    print!("Would you like to download it? (Y/N, default: Y) ");
    let option = read_line().trim().to_lowercase();
    if option.is_empty() || option == "y" {
        let repo = manifest.repo.unwrap();
        let url = repo.get(&latest).unwrap().archive.as_ref().unwrap();
        download(&client, url, "mcl.zip").await;
        unzip("mcl.zip");
        fs::remove_file("mcl.zip");

        #[cfg(windows)]
        if Path::new("mcl.cmd").exists() {
            let j = format!("set JAVA_BINARY=\"{}\"", java);
            fs::write("mcl.cmd", fs::read_to_string("mcl.cmd").unwrap().replace("set JAVA_BINARY=java", &j));
        }

        #[cfg(unix)]
        if Path::new("mcl").exists() {
            let j = format!("export JAVA_BINARY=\"{}\"", java);
            fs::write("mcl", fs::read_to_string("mcl").unwrap().replace("export JAVA_BINARY=java", &j));
            Command::new("chmod").arg("777").arg("mcl").spawn().unwrap().wait();
        }

        println!("MCL startup script has been updated. Use \"./mcl.\" to start MCL.");
        println!();
    }

    println!("Press Enter to exit.");
    read_line();
}

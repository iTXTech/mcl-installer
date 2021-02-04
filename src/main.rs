use std::fs;
use std::fs::File;
use std::io::{self, Write};
use std::option::Option::Some;
use std::path::Path;
use std::process::{Command, exit};

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

#[tokio::main]
async fn main() {
    println!("iTXTech MCL Installer v1.0.0 [OS: {}]", get_os());
    println!("Licensed under GNU AGPLv3.");
    println!("https://github.com/iTXTech/mcl-installer");
    println!();

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
        let resp = reqwest::get(&url).await;
        if !resp.is_ok() {
            println!("Fail to fetch AdoptOpenJDK download list.");
            exit(1);
        }
        let text = resp.unwrap().text().await.unwrap();
        let lines = text.split("\n");
        let sign = "<tr><td class=\"link\"><a href=\"";
        for line in lines {
            if line.starts_with(sign) && line.contains("hotspot") && (line.contains(".zip") || line.contains(".tar.gz")) {
                let end = line.find("\" title=\"").unwrap();
                let archive = format!("{}{}", url, &line[sign.len()..end]);
                println!("Start Downloading: {}", archive);

                let mut res = reqwest::get(&archive).await.unwrap();
                let ttl = res.headers().get(reqwest::header::CONTENT_LENGTH).unwrap().to_str().unwrap();
                let total = str_to_int(ttl);
                let mut current = 0;
                fs::remove_file("java.arc");

                {
                    let mut file = File::create("java.arc").unwrap();

                    while let Some(chunk) = res.chunk().await.unwrap() {
                        current += chunk.len();
                        file.write(&*chunk);
                        print!("\rDownloading: {}/{}", current, total);
                    }

                    println!();
                }

                let mut java_dir = String::new();
                if get_os() == "windows" { //zip
                    let mut zip = ZipArchive::new(File::open("java.arc").unwrap()).unwrap();

                    java_dir = format!("{}", zip.by_index(0).unwrap().name());

                    let len = zip.len();
                    for i in 0..zip.len() {
                        let mut file = zip.by_index(i).unwrap();
                        let outpath = match file.enclosed_name() {
                            Some(path) => path.to_owned(),
                            None => continue,
                        };

                        print!("\rExtracting [{}/{}]", i + 1, len);
                        if (&*file.name()).ends_with('/') {
                            fs::create_dir_all(&outpath).unwrap();
                        } else {
                            if let Some(p) = outpath.parent() {
                                if !p.exists() {
                                    fs::create_dir_all(&p).unwrap();
                                }
                            }
                            let mut outfile = fs::File::create(&outpath).unwrap();
                            io::copy(&mut file, &mut outfile).unwrap();
                        }
                    }
                    println!();
                } else { //tar.gz
                }

                fs::remove_file("java.arc");
                fs::rename(java_dir, "java");

                let java = "java/bin/java";
                println!("Testing Java Executable: {}", java);

                Command::new(java).arg("-version").spawn().unwrap().wait();

                break;
            }
        }
    }

    println!("Press Enter to exit.");
    read_line();
}

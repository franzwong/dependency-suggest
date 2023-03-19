use std::fs::File;
use reqwest::StatusCode;
use serde::Deserialize;
use std::process::Command;
use std::env;
use std::path::PathBuf;

use std::error::Error;
use std::fmt;

#[derive(Deserialize)]
struct SearchResult {
    response: Response,
}

#[derive(Deserialize)]
struct Response {
    docs: Vec<Doc>,
}

#[derive(Deserialize)]
struct Doc {
    v: String,
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    response_body: String
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error returned from server. Status code: {}. Response body: {}", self.status, self.response_body)
    }
}

impl Error for ApiError {}

fn query_maven_central(group_id: &str, artifact_id: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let params = [
        ("core", "gav"),
        ("rows", "20"),
        ("wt", "json")
    ];
    let url = format!("https://search.maven.org/solrsearch/select?q=g:{group_id}+AND+a:{artifact_id}");
    let url = reqwest::Url::parse_with_params(url.as_str(), &params)?;

    let client = reqwest::blocking::Client::new();
    let http_response = client.get(url)
        .header("user-agent", "reqwest")
        .send()?;

    let status = http_response.status();
    let response_body = http_response.text()?;

    if !status.is_success() {
        let error = ApiError {
            status,
            response_body
        };
        return Err(error.into());
    }

    let response: SearchResult = serde_json::from_str(&response_body)?;
    let versions = response
        .response
        .docs
        .iter()
        .map(|doc| doc.v.clone())
        .collect::<Vec<String>>();
    Ok(versions)
}

fn get_major_version(version: &str) -> Result<&str, String> {
    if version.contains('.') {
        Ok(version.split('.').next().unwrap())
    } else {
        Err(format!("Version '{version}' is not in semver format."))
    }
}

fn extract_versions_with_same_major_version(major_version: &str, versions: &[String]) -> Vec<String> {
    versions
        .iter()
        .filter(|v| {
            get_major_version(v).unwrap() == major_version
        })
        .cloned()
        .collect()
}

fn download_jar(group_id: &str, artifact_id: &str, version: &str) -> Result<String, Box<dyn std::error::Error>> {
    let group_path = group_id.replace('.', "/");
    let file_name = format!("{artifact_id}-{version}.jar");
    let url = format!("https://repo1.maven.org/maven2/{group_path}/{artifact_id}/{version}/{file_name}");

    let full_path = std::env::current_dir()?.join(&file_name);

    if full_path.exists() {
        println!("File {file_name} already exists, skipping download.");
        return Ok(full_path.to_str().unwrap().to_owned());
    }

    println!("Downloading jar file '{}'", &file_name);

    let mut response = reqwest::blocking::get(url)?;
    let mut file = File::create(&file_name)?;
    response.copy_to(&mut file)?;

    println!("Jar file is downloaded");

    Ok(full_path.to_str().unwrap().to_owned())
}

fn check_vulnerabilities(jar_file_path: &str) -> Result<(), String> {
    let script_path = env::var("DEPENDENCY_CHECK_SCRIPT")
        .unwrap_or_else(|_| String::from("./dependency-check.sh"));

    let path_buf = PathBuf::from(&script_path);
    let script_dir = path_buf
        .parent()
        .ok_or_else(|| format!("Failed to get parent directory of Dependency-check script: {script_path}"))?;

    let output = Command::new(&script_path)
        .current_dir(script_dir)
        .arg("--out")
        .arg("./output")
        .arg("--scan")
        .arg(jar_file_path)
        .output()
        .map_err(|err| format!("Failed to execute Dependency-check! Error: {err}"))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "Dependency-check failed with status code {}! Error: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 4 {
        return Err(format!("Usage: {} <groupId> <artifactId> <version>", args[0]));
    }

    let group_id = &args[1];
    let artifact_id = &args[2];
    let version = &args[3];

    let versions = query_maven_central(group_id, artifact_id)
        .map_err(|err| format!("Failed to query maven central! Error: {err}"))?;

    let major_version = get_major_version(version)?;

    let matching_versions = extract_versions_with_same_major_version(major_version, &versions);

    println!("Versions having same major version: {matching_versions:?}");

    let latest_version = &matching_versions[0];

    if latest_version == version {
        return Err("Current version is already latest version".to_string());
    }

    let jar_file_name = download_jar(group_id, artifact_id, latest_version)
        .map_err(|err| format!("Failed to download jar file! Error: {err}"))?;

    println!("Check vulnerabilities");
    check_vulnerabilities(&jar_file_name)?;

    println!("There is no vulnerability in '{latest_version}'. It is safe to use.");

    Ok(())
}

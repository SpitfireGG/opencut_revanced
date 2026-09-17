// SPDX-License-Identifier: AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Jareer and Concat contributors

//! Compiles the Java the activity needs - the document picker fragment in
//! java/ - into one dex file the binary carries and loads at run time.
//! The same way Slint's Android backend builds its own helper: the SDK's
//! javac and d8, found through ANDROID_HOME and JAVA_HOME. Nothing to do
//! for any other target.

use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=java/ConcatFiles.java");
    println!("cargo:rerun-if-changed=java/app/concat/editor/ConcatActivity.java");
    let target = std::env::var("TARGET").unwrap_or_default();
    if !target.contains("android") {
        return;
    }

    let release = std::env::var("PROFILE").as_deref() == Ok("release");
    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR"));
    let classes = out_dir.join("java");
    if classes.exists() {
        let _ = std::fs::remove_dir_all(&classes);
    }
    std::fs::create_dir_all(&classes).expect("could not create the class directory");

    let android_jar = android_build::android_jar(None).expect("no Android platform found");
    
    // Download AndroidX Core AAR and extract classes.jar for WindowCompat
    let androidx_core_jar = download_androidx_core(&out_dir).expect("failed to download AndroidX Core");

    let compiled = android_build::JavaBuild::new()
        .file("java/ConcatFiles.java")
        .file("java/app/concat/editor/ConcatActivity.java")
        .class_path(&android_jar)
        .class_path(&androidx_core_jar)  // Add AndroidX Core to classpath
        .classes_out_dir(&classes)
        .java_source_version(8)
        .java_target_version(8)
        .debug_info(android_build::DebugInfo {
            line_numbers: !release,
            variables: !release,
            source_files: !release,
        })
        .command()
        .expect("could not build the javac command")
        .args(["-encoding", "UTF-8"])
        .output()
        .expect("could not run javac");
    if !compiled.status.success() {
        panic!(
            "javac failed: {}",
            String::from_utf8_lossy(&compiled.stderr)
        );
    }

    let dexed = android_build::Dexer::new()
        .android_jar(&android_jar)
        .class_path(&classes)
        .class_path(&androidx_core_jar)  // Add AndroidX Core to dex classpath
        .collect_classes(&classes)
        .expect("could not collect the classes")
        .release(release)
        .android_min_api(26)
        .out_dir(&out_dir)
        .command()
        .expect("could not build the d8 command")
        .output()
        .expect("could not run d8");
    if !dexed.status.success() {
        panic!("d8 failed: {}", String::from_utf8_lossy(&dexed.stderr));
    }
}

fn download_androidx_core(out_dir: &PathBuf) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let version = "1.12.0";
    let aar_name = format!("core-{}.aar", version);
    let aar_path = out_dir.join(&aar_name);
    let jar_path = out_dir.join(format!("core-{}.jar", version));

    // If already cached, return the jar path
    if jar_path.exists() {
        return Ok(jar_path);
    }

    // Download the AAR from Maven Central
    let url = format!(
        "https://repo1.maven.org/maven2/androidx/core/core/{}/core-{}.aar",
        version, version
    );
    
    println!("Downloading AndroidX Core from {}", url);
    
    let mut response = reqwest::blocking::get(&url)?;
    let mut file = std::fs::File::create(&aar_path)?;
    std::io::copy(&mut response, &mut file)?;

    // Extract classes.jar from the AAR (which is a zip file)
    let mut archive = zip::ZipArchive::new(std::fs::File::open(&aar_path)?)?;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        if file.name() == "classes.jar" {
            let mut out_file = std::fs::File::create(&jar_path)?;
            std::io::copy(&mut file, &mut out_file)?;
            break;
        }
    }

    if !jar_path.exists() {
        return Err("classes.jar not found in AAR".into());
    }

    Ok(jar_path)
}
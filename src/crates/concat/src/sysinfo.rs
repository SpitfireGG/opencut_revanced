// SPDX-License-Identifier: AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Jareer and Concat contributors

//! What Settings > About reports about this machine and this build.

use crate::i18n::t;

pub fn os_description() -> String {
    #[cfg(target_os = "linux")]
    {
        // The distribution's own name for itself, which is what a report
        // wants; the kernel version is the next question, not the first.
        let pretty = std::fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|release| {
                release
                    .lines()
                    .find_map(|line| line.strip_prefix("PRETTY_NAME="))
                    .map(|value| value.trim_matches('"').to_owned())
            })
            .filter(|value| !value.is_empty());
        pretty.unwrap_or_else(|| "Linux".into())
    }
    #[cfg(not(target_os = "linux"))]
    {
        std::env::consts::OS.to_owned()
    }
}

/// What Settings > About shows under "System information", and what its copy
/// button puts on the clipboard.
///
/// One list, read twice: the rows on screen and the block that is copied are
/// built from the same pairs, so a fact cannot be on the page and missing
/// from the report. Gathered once — every line of it is fixed for the life of
/// the process — and handed over as a model that is never replaced.
pub fn system_facts() -> Vec<(String, String)> {
    vec![
        (
            t("Application"),
            format!("Concat {}", env!("CARGO_PKG_VERSION")),
        ),
        (
            t("Build"),
            format!("{} · {}", env!("BUILD_PROFILE"), env!("BUILD_TARGET")),
        ),
        // Which of the two renderers in Cargo.toml this binary was built
        // with. The first question to ask about anything that looks wrong on
        // screen, and the one nobody can answer by looking at the window.
        (
            t("Renderer"),
            if cfg!(feature = "skia") {
                "Skia"
            } else {
                "FemtoVG (wgpu)"
            }
            .into(),
        ),
        (
            t("Engine"),
            format!("concat-engine · FFmpeg {}", concat_media::linked_version()),
        ),
        (t("Operating system"), os_description()),
        (
            t("Processor"),
            format!(
                "{} · {} threads",
                std::env::consts::ARCH,
                std::thread::available_parallelism().map_or(0, |count| count.get())
            ),
        ),
        (t("Toolchain"), env!("BUILD_RUSTC").into()),
    ]
}

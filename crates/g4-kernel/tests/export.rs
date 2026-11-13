use std::path::Path;

use g4_kernel::error::KernelError;

use g4_kernel::export::{
    embed_images, glb_cache_key, glb_cached, glb_fail_get, glb_fail_record, last_tool,
    nsbmd_to_glb, remember_tool, tool_fingerprint,
};

/// A stand-in tool hash for tests that do not run a real apicula.
const TOOL: u64 = 0x0123_4567_89ab_cdef;

#[test]
fn missing_apicula_is_tool_missing() {
    let out = tempfile::tempdir().unwrap();
    let err =
        nsbmd_to_glb(Path::new("/nonexistent/apicula"), b"BMD0", None, out.path()).unwrap_err();
    assert!(matches!(
        err,
        KernelError::ToolMissing {
            name: "apicula",
            ..
        }
    ));
}

#[test]
fn cache_key_depends_on_model_and_tileset() {
    assert_eq!(glb_cache_key(TOOL, b"BMD0", None).len(), 16);
    assert_ne!(
        glb_cache_key(TOOL, b"BMD0", None),
        glb_cache_key(TOOL, b"BMD0", Some(b"BTX0"))
    );
    assert_eq!(
        glb_cache_key(TOOL, b"BMD0", None),
        glb_cache_key(TOOL, b"BMD0", None)
    );
}

#[test]
fn cache_key_changes_with_the_tool() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("apicula-a");
    let b = dir.path().join("apicula-b");
    let same = dir.path().join("apicula-same-as-a");
    std::fs::write(&a, b"binary one").unwrap();
    std::fs::write(&b, b"binary two").unwrap();
    std::fs::write(&same, b"binary one").unwrap();
    let (ta, tb, ts) = (
        tool_fingerprint(&a).unwrap(),
        tool_fingerprint(&b).unwrap(),
        tool_fingerprint(&same).unwrap(),
    );
    assert_ne!(ta, tb);
    assert_eq!(ta, ts);
    assert_ne!(
        glb_cache_key(ta, b"BMD0", Some(b"BTX0")),
        glb_cache_key(tb, b"BMD0", Some(b"BTX0"))
    );
    assert_eq!(
        glb_cache_key(ta, b"BMD0", Some(b"BTX0")),
        glb_cache_key(ts, b"BMD0", Some(b"BTX0"))
    );
    // The fingerprint is the xxh3-64 of the file content.
    assert_eq!(ta, xxhash_rust::xxh3::xxh3_64(b"binary one"));
    // A missing or unreadable file is a ToolMissing error.
    let err = tool_fingerprint(&dir.path().join("gone")).unwrap_err();
    assert!(matches!(err, KernelError::ToolMissing { .. }), "{err}");
}

#[test]
fn a_new_tool_hash_invalidates_cached_models_and_fail_markers() {
    let dir = tempfile::tempdir().unwrap();
    let old = glb_cache_key(1, b"BMD0", Some(b"BTX0"));
    std::fs::write(dir.path().join(format!("{old}.glb")), b"glTFold").unwrap();
    glb_fail_record(dir.path(), 1, b"BMD0", Some(b"BTX0"), "old failure").unwrap();
    assert!(
        g4_kernel::export::glb_cache_get(dir.path(), 1, b"BMD0", Some(b"BTX0"))
            .unwrap()
            .is_some()
    );
    assert!(
        g4_kernel::export::glb_cache_get(dir.path(), 2, b"BMD0", Some(b"BTX0"))
            .unwrap()
            .is_none()
    );
    assert_eq!(
        glb_fail_get(dir.path(), 2, b"BMD0", Some(b"BTX0")).unwrap(),
        None
    );
}

#[test]
fn last_tool_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join("models");
    assert_eq!(last_tool(&cache), None);
    remember_tool(&cache, TOOL).unwrap();
    assert_eq!(last_tool(&cache), Some(TOOL));
    assert_eq!(
        std::fs::read_to_string(cache.join("last-tool")).unwrap(),
        "0123456789abcdef"
    );
    remember_tool(&cache, 7).unwrap();
    assert_eq!(last_tool(&cache), Some(7));
    // No temp file is left behind, and garbage reads as "nothing remembered".
    let names: Vec<_> = std::fs::read_dir(&cache)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(names, ["last-tool"]);
    std::fs::write(cache.join("last-tool"), "not a hash").unwrap();
    assert_eq!(last_tool(&cache), None);
}

/// A GLB made of a JSON chunk (padded with spaces) and, when given, a BIN chunk (padded with
/// zeros).
fn build_glb(json: &serde_json::Value, bin: Option<&[u8]>) -> Vec<u8> {
    let mut json = serde_json::to_vec(json).unwrap();
    while !json.len().is_multiple_of(4) {
        json.push(b' ');
    }
    let mut chunks = Vec::new();
    chunks.extend((json.len() as u32).to_le_bytes());
    chunks.extend(0x4E4F_534Au32.to_le_bytes());
    chunks.extend(&json);
    if let Some(bin) = bin {
        let mut bin = bin.to_vec();
        while !bin.len().is_multiple_of(4) {
            bin.push(0);
        }
        chunks.extend((bin.len() as u32).to_le_bytes());
        chunks.extend(0x004E_4942u32.to_le_bytes());
        chunks.extend(&bin);
    }
    let mut glb = Vec::new();
    glb.extend(b"glTF");
    glb.extend(2u32.to_le_bytes());
    glb.extend((12 + chunks.len() as u32).to_le_bytes());
    glb.extend(chunks);
    glb
}

/// Splits a GLB into its JSON and (optional) BIN chunk, checking the framing on the way: total
/// length, chunk lengths and 4-byte alignment.
fn split_glb(glb: &[u8]) -> (serde_json::Value, Option<Vec<u8>>) {
    assert_eq!(&glb[0..4], b"glTF");
    assert_eq!(u32::from_le_bytes(glb[4..8].try_into().unwrap()), 2);
    assert_eq!(
        u32::from_le_bytes(glb[8..12].try_into().unwrap()) as usize,
        glb.len()
    );
    let mut json = None;
    let mut bin = None;
    let mut at = 12;
    while at < glb.len() {
        let len = u32::from_le_bytes(glb[at..at + 4].try_into().unwrap()) as usize;
        let kind = u32::from_le_bytes(glb[at + 4..at + 8].try_into().unwrap());
        assert_eq!(len % 4, 0, "chunk length {len} is not 4-byte aligned");
        let data = &glb[at + 8..at + 8 + len];
        match kind {
            0x4E4F_534A => json = Some(serde_json::from_slice(data).unwrap()),
            0x004E_4942 => bin = Some(data.to_vec()),
            other => panic!("unexpected chunk {other:#x}"),
        }
        at += 8 + len;
    }
    assert_eq!(at, glb.len());
    (json.unwrap(), bin)
}

/// Bytes of the image's bufferView inside the BIN chunk.
fn image_bytes(json: &serde_json::Value, bin: &[u8], image: usize) -> Vec<u8> {
    let view = json["images"][image]["bufferView"].as_u64().unwrap() as usize;
    let v = &json["bufferViews"][view];
    assert_eq!(v["buffer"], 0);
    let start = v["byteOffset"].as_u64().unwrap() as usize;
    let len = v["byteLength"].as_u64().unwrap() as usize;
    bin[start..start + len].to_vec()
}

#[test]
fn embed_images_moves_a_referenced_png_into_the_bin_chunk() {
    let json = serde_json::json!({
        "asset": {"version": "2.0"},
        "buffers": [{"byteLength": 6}],
        "bufferViews": [{"buffer": 0, "byteOffset": 0, "byteLength": 6}],
        "images": [{"uri": "bed_h01.png"}],
    });
    let glb = build_glb(&json, Some(b"abcdef"));
    let png = b"\x89PNG-bed-seven".to_vec();
    let out = embed_images(&glb, |name| (name == "bed_h01.png").then(|| png.clone())).unwrap();
    let (json, bin) = split_glb(&out);
    let bin = bin.unwrap();
    let image = &json["images"][0];
    assert!(image.get("uri").is_none());
    assert_eq!(image["mimeType"], "image/png");
    assert_eq!(image["name"], "bed_h01");
    assert_eq!(image["bufferView"], 1);
    // The old view is untouched; the new one starts 4-byte aligned behind the old data.
    assert_eq!(json["bufferViews"][0]["byteLength"], 6);
    assert_eq!(
        json["bufferViews"][1]["byteOffset"].as_u64().unwrap() % 4,
        0
    );
    assert_eq!(image_bytes(&json, &bin, 0), png);
    assert_eq!(&bin[0..6], b"abcdef");
    assert_eq!(
        json["buffers"][0]["byteLength"].as_u64().unwrap() as usize,
        bin.len()
    );
    assert_eq!(bin.len() % 4, 0);
}

#[test]
fn embed_images_aligns_every_image_and_pads_with_zeros() {
    let json = serde_json::json!({
        "asset": {"version": "2.0"},
        "buffers": [{"byteLength": 5}],
        "images": [{"uri": "a.png"}, {"uri": "b.png"}, {"uri": "a.png"}],
    });
    let glb = build_glb(&json, Some(b"12345"));
    let out = embed_images(&glb, |name| match name {
        "a.png" => Some(b"AAAAA".to_vec()),
        "b.png" => Some(b"BB".to_vec()),
        _ => None,
    })
    .unwrap();
    let (json, bin) = split_glb(&out);
    let bin = bin.unwrap();
    assert_eq!(image_bytes(&json, &bin, 0), b"AAAAA");
    assert_eq!(image_bytes(&json, &bin, 1), b"BB");
    assert_eq!(image_bytes(&json, &bin, 2), b"AAAAA");
    for view in json["bufferViews"].as_array().unwrap() {
        assert_eq!(view["byteOffset"].as_u64().unwrap() % 4, 0);
    }
    // Gaps between the old data and the images are zero-filled.
    assert_eq!(&bin[5..8], &[0, 0, 0]);
    assert_eq!(bin.len() % 4, 0);
}

#[test]
fn embed_images_adds_a_bin_chunk_when_there_is_none() {
    let json = serde_json::json!({"asset": {"version": "2.0"}, "images": [{"uri": "t.png"}]});
    let glb = build_glb(&json, None);
    let out = embed_images(&glb, |_| Some(b"PNGDATA".to_vec())).unwrap();
    let (json, bin) = split_glb(&out);
    let bin = bin.expect("a BIN chunk was added");
    assert_eq!(
        json["buffers"][0]["byteLength"].as_u64().unwrap() as usize,
        bin.len()
    );
    assert!(json["buffers"][0].get("uri").is_none());
    assert_eq!(json["images"][0]["bufferView"], 0);
    assert_eq!(image_bytes(&json, &bin, 0), b"PNGDATA");
}

#[test]
fn embed_images_pads_the_json_chunk_with_spaces() {
    // Name lengths 0..4 cover every remainder of the JSON length modulo 4.
    let mut padded = false;
    for extra in 0..4 {
        let json = serde_json::json!({
            "asset": {"version": "2.0"},
            "images": [{"uri": format!("{}.png", "x".repeat(extra))}],
        });
        let out = embed_images(&build_glb(&json, None), |_| Some(b"P".to_vec())).unwrap();
        let json_len = u32::from_le_bytes(out[12..16].try_into().unwrap()) as usize;
        assert_eq!(json_len % 4, 0);
        let text = &out[20..20 + json_len];
        let content = text.trim_ascii_end();
        assert!(content.ends_with(b"}"), "{:?}", &text[text.len() - 4..]);
        assert!(text[content.len()..].iter().all(|&b| b == b' '));
        padded |= content.len() < text.len();
        split_glb(&out);
    }
    assert!(padded);
}

#[test]
fn embed_images_leaves_data_uris_and_embedded_images_alone() {
    let json = serde_json::json!({
        "asset": {"version": "2.0"},
        "bufferViews": [{"buffer": 0, "byteLength": 4}],
        "buffers": [{"byteLength": 4}],
        "images": [
            {"uri": "data:image/png;base64,AAAA"},
            {"bufferView": 0, "mimeType": "image/png"},
        ],
    });
    let glb = build_glb(&json, Some(b"abcd"));
    let out = embed_images(&glb, |name| panic!("must not read {name}")).unwrap();
    assert_eq!(out, glb);
}

#[test]
fn embed_images_without_images_is_unchanged() {
    let glb = build_glb(
        &serde_json::json!({"asset": {"version": "2.0"}}),
        Some(b"abcd"),
    );
    assert_eq!(embed_images(&glb, |_| None).unwrap(), glb);
}

#[test]
fn embed_images_names_a_missing_file() {
    let json = serde_json::json!({"asset": {"version": "2.0"}, "images": [{"uri": "gone.png"}]});
    let err = embed_images(&build_glb(&json, None), |_| None).unwrap_err();
    assert!(err.to_string().contains("gone.png"), "{err}");
}

#[test]
fn embed_images_refuses_uris_that_leave_the_output_directory() {
    for uri in [
        "/etc/passwd",
        "../secret.png",
        "sub/dir.png",
        "sub\\dir.png",
        "..",
        "",
        "C:\\x.png",
        "file:///etc/passwd",
    ] {
        let json = serde_json::json!({"asset": {"version": "2.0"}, "images": [{"uri": uri}]});
        let err = embed_images(&build_glb(&json, None), |name| {
            panic!("must not read {name:?}")
        })
        .unwrap_err();
        assert!(err.to_string().contains("uri"), "{uri}: {err}");
    }
}

#[test]
fn embed_images_rejects_input_that_is_not_a_glb() {
    assert!(embed_images(b"glTFxxxx", |_| None).is_err());
    assert!(embed_images(b"", |_| None).is_err());
    let mut glb = build_glb(&serde_json::json!({"asset": {"version": "2.0"}}), None);
    glb.truncate(glb.len() - 2);
    assert!(embed_images(&glb, |_| None).is_err());
}

#[test]
fn cache_key_is_salted_with_the_format_version() {
    // Pinned: bumping the salt invalidates previously cached GLBs (and .fail markers).
    assert_eq!(glb_cache_key(TOOL, b"BMD0", None).len(), 16);
    let mut h = xxhash_rust::xxh3::Xxh3::new();
    h.update(b"g4glb-v3\0");
    h.update(&TOOL.to_le_bytes());
    h.update(b"BMD0");
    assert_eq!(
        glb_cache_key(TOOL, b"BMD0", None),
        format!("{:016x}", h.digest())
    );
}

#[test]
fn fail_marker_round_trips_per_model_and_tileset_and_leaves_no_temp_file() {
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join("models");
    assert_eq!(
        glb_fail_get(&cache, TOOL, b"BMD0", Some(b"BTX0")).unwrap(),
        None
    );
    glb_fail_record(&cache, TOOL, b"BMD0", Some(b"BTX0"), "bad tileset").unwrap();
    assert_eq!(
        glb_fail_get(&cache, TOOL, b"BMD0", Some(b"BTX0"))
            .unwrap()
            .as_deref(),
        Some("bad tileset")
    );
    // Other tileset bytes, or none at all, are unaffected.
    assert_eq!(
        glb_fail_get(&cache, TOOL, b"BMD0", Some(b"BTX1")).unwrap(),
        None
    );
    assert_eq!(glb_fail_get(&cache, TOOL, b"BMD0", None).unwrap(), None);
    let names: Vec<_> = std::fs::read_dir(&cache)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(names.len(), 1, "{names:?}");
    assert!(names[0].ends_with(".fail"));
}

#[cfg(unix)]
mod fake_tool {
    use std::os::unix::fs::PermissionsExt;
    use std::time::{Duration, Instant};

    use g4_kernel::export::nsbmd_to_glb_with_timeout;

    use super::*;

    /// Writes an executable shell script standing in for apicula.
    fn fake(dir: &Path, body: &str) -> std::path::PathBuf {
        let path = dir.join("fake-apicula");
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    // apicula's argv: convert -f=glb <input> [<nsbtx>] -o <dir>; the script finds the directory
    // after `-o` and creates <dir>/<name>/<name>.glb.
    const FIND_OUT: &str = r#"while [ $# -gt 0 ]; do [ "$1" = "-o" ] && out="$2"; shift; done"#;
    const PRODUCE: &str = r#"while [ $# -gt 0 ]; do [ "$1" = "-o" ] && out="$2"; shift; done
mkdir -p "$out/model"; printf 'glTFxxxx' > "$out/model/model.glb""#;

    /// A fake apicula that counts its runs in a file; returns the script and the counter file.
    fn fake_apicula_counting(dir: &Path) -> (std::path::PathBuf, std::path::PathBuf) {
        let counter = dir.join("count");
        let valid = dir.join("valid.glb");
        std::fs::write(
            &valid,
            build_glb(&serde_json::json!({"asset": {"version": "2.0"}}), None),
        )
        .unwrap();
        let body = format!(
            "{FIND_OUT}\necho x >> '{c}'\nmkdir -p \"$out/model\"; cp '{v}' \"$out/model/model.glb\"",
            c = counter.display(),
            v = valid.display()
        );
        (fake(dir, &body), counter)
    }

    fn runs(counter: &Path) -> usize {
        std::fs::read_to_string(counter).map_or(0, |t| t.lines().count())
    }

    #[test]
    fn returns_the_single_glb_the_tool_produced() {
        let tools = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        let tool = fake(tools.path(), PRODUCE);
        let glb = nsbmd_to_glb(&tool, b"BMD0", None, out.path()).unwrap();
        assert_eq!(glb.extension().unwrap(), "glb");
        assert!(glb.starts_with(out.path()));
        assert_eq!(&std::fs::read(&glb).unwrap()[0..4], b"glTF");
    }

    /// Whether a process with this pid still exists (`kill -0`); a reaped or orphaned-and-
    /// collected process is gone.
    fn pid_alive(pid: &str) -> bool {
        std::process::Command::new("kill")
            .args(["-0", pid])
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap()
            .success()
    }

    #[test]
    fn slow_apicula_times_out_without_fail_marker() {
        let tools = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        // The script and a helper it started both record their pid, then wait, but only once
        // `slow` exists: a first, fast run warms the new executable up (macOS scans a fresh
        // file on first exec, which can take a second when many tests spawn at once and
        // would eat the whole time limit).
        let pids = tools.path().join("pids");
        let slow = tools.path().join("slow");
        let tool = fake(
            tools.path(),
            &format!(
                "[ -e '{slow}' ] || exit 0\necho $$ > '{p}.sh'; sleep 5 & echo $! > '{p}.sleep'; wait",
                slow = slow.display(),
                p = pids.display()
            ),
        );
        let _ =
            nsbmd_to_glb_with_timeout(&tool, b"BMD0", None, out.path(), Duration::from_secs(30));
        std::fs::write(&slow, "").unwrap();
        let started = Instant::now();
        let err =
            nsbmd_to_glb_with_timeout(&tool, b"BMD0", None, out.path(), Duration::from_secs(1))
                .unwrap_err();
        let took = started.elapsed();
        assert!(
            matches!(
                err,
                KernelError::ToolTimeout {
                    name: "apicula",
                    secs: 1
                }
            ),
            "{err:?}"
        );
        assert!(took < Duration::from_secs(3), "took {took:?}");
        assert_eq!(err.to_string(), "apicula did not finish within 1 s");
        // Neither the script nor the helper it started is left running.
        for suffix in ["sh", "sleep"] {
            let pid = std::fs::read_to_string(format!("{}.{suffix}", pids.display())).unwrap();
            let pid = pid.trim();
            let deadline = Instant::now() + Duration::from_secs(2);
            while pid_alive(pid) && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(20));
            }
            assert!(!pid_alive(pid), "{suffix} process {pid} is still running");
        }
        // glb_cached passes the timeout on, and a timeout leaves nothing in the cache dir
        // (no .glb, no .fail, no work directory).
        let cache = tempfile::tempdir().unwrap();
        let err = g4_kernel::export::glb_cached_with_timeout(
            cache.path(),
            &tool,
            TOOL,
            b"BMD0",
            None,
            Duration::from_secs(1),
        )
        .unwrap_err();
        assert!(matches!(err, KernelError::ToolTimeout { .. }), "{err:?}");
        assert_eq!(std::fs::read_dir(cache.path()).unwrap().count(), 0);
    }

    #[test]
    fn a_chatty_tool_does_not_block_on_full_pipes() {
        let tools = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        // 400 KB on each pipe, far beyond the pipe buffer, then a normal result.
        let tool = fake(
            tools.path(),
            &format!(
                "{FIND_OUT}\nhead -c 400000 /dev/zero >&2; head -c 400000 /dev/zero; \
                 mkdir -p \"$out/model\"; printf 'glTFxxxx' > \"$out/model/model.glb\""
            ),
        );
        let glb =
            nsbmd_to_glb_with_timeout(&tool, b"BMD0", None, out.path(), Duration::from_secs(10))
                .unwrap();
        assert_eq!(&std::fs::read(glb).unwrap()[0..4], b"glTF");
    }

    #[test]
    fn non_zero_exit_reports_stderr() {
        let tools = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        let tool = fake(tools.path(), "echo boom >&2; exit 3");
        let err = nsbmd_to_glb(&tool, b"BMD0", None, out.path()).unwrap_err();
        match err {
            KernelError::ToolMissing { name, reason } => {
                assert_eq!(name, "apicula");
                assert!(reason.contains("boom"), "{reason}");
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn no_glb_or_several_glbs_is_an_error() {
        let tools = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        let none = fake(tools.path(), "exit 0");
        assert!(nsbmd_to_glb(&none, b"BMD0", None, out.path()).is_err());
        let two = fake(
            tools.path(),
            &format!(
                r#"{FIND_OUT}; mkdir -p "$out/a" "$out/b"; : > "$out/a/a.glb"; : > "$out/b/b.glb""#
            ),
        );
        assert!(nsbmd_to_glb(&two, b"BMD0", None, out.path()).is_err());
    }

    #[test]
    fn tileset_is_passed_as_second_input_file() {
        let tools = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        // Fails unless argv is: convert -f=glb <x>.nsbmd <y>.nsbtx -o <dir> with both files present.
        let tool = fake(
            tools.path(),
            &format!(
                r#"case "$3" in *.nsbmd) ;; *) echo "bad model arg $3" >&2; exit 2;; esac
case "$4" in *.nsbtx) ;; *) echo "bad tileset arg $4" >&2; exit 2;; esac
[ "$(cat "$4")" = "BTX0tiles" ] || {{ echo "bad tileset content" >&2; exit 2; }}
[ "$5" = "-o" ] || {{ echo "bad -o" >&2; exit 2; }}
{FIND_OUT}
mkdir -p "$out/model"; printf 'glTFxxxx' > "$out/model/model.glb""#
            ),
        );
        nsbmd_to_glb(&tool, b"BMD0", Some(b"BTX0tiles"), out.path()).unwrap();
        // Without a tileset the same script must be rejected (no fourth argument).
        assert!(nsbmd_to_glb(&tool, b"BMD0", None, out.path()).is_err());
    }

    #[test]
    fn glb_cached_hits_the_cache_second_time() {
        let tools = tempfile::tempdir().unwrap();
        let (apicula, counter) = fake_apicula_counting(tools.path());
        let dir = tempfile::tempdir().unwrap();
        let a = glb_cached(dir.path(), &apicula, TOOL, b"BMD0one", None).unwrap();
        let b = glb_cached(dir.path(), &apicula, TOOL, b"BMD0one", None).unwrap();
        assert_eq!(a, b);
        assert_eq!(runs(&counter), 1);
        // Only the cache file remains: no work directories or temp files.
        let names: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(
            names,
            [format!("{}.glb", glb_cache_key(TOOL, b"BMD0one", None))]
        );
        // A different tileset is a different cache entry.
        glb_cached(dir.path(), &apicula, TOOL, b"BMD0one", Some(b"BTX0")).unwrap();
        assert_eq!(runs(&counter), 2);
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2);
    }

    #[test]
    fn glb_cached_embeds_the_pngs_apicula_wrote_next_to_the_glb() {
        let tools = tempfile::tempdir().unwrap();
        let src = tools.path().join("textured.glb");
        let json = serde_json::json!({"asset": {"version": "2.0"}, "images": [{"uri": "t.png"}]});
        std::fs::write(&src, build_glb(&json, None)).unwrap();
        let tool = fake(
            tools.path(),
            &format!(
                "{FIND_OUT}\nmkdir -p \"$out/model\"; cp '{}' \"$out/model/model.glb\"; printf 'PNGBYTES' > \"$out/model/t.png\"",
                src.display()
            ),
        );
        let dir = tempfile::tempdir().unwrap();
        let glb = glb_cached(dir.path(), &tool, TOOL, b"BMD0", Some(b"BTX0")).unwrap();
        let (json, bin) = split_glb(&glb);
        assert!(json["images"][0].get("uri").is_none());
        assert_eq!(image_bytes(&json, &bin.unwrap(), 0), b"PNGBYTES");
        // The cached file is the embedded one, and nothing else is left behind.
        let cached = std::fs::read(dir.path().join(format!(
            "{}.glb",
            glb_cache_key(TOOL, b"BMD0", Some(b"BTX0"))
        )))
        .unwrap();
        assert_eq!(cached, glb);
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);

        // A referenced image apicula did not write is an error naming it, nothing is cached.
        let broken = fake(
            tools.path(),
            &format!(
                "{FIND_OUT}\nmkdir -p \"$out/model\"; cp '{}' \"$out/model/model.glb\"",
                src.display()
            ),
        );
        let other = tempfile::tempdir().unwrap();
        let err = glb_cached(other.path(), &broken, TOOL, b"BMD0", Some(b"BTX0")).unwrap_err();
        assert!(err.to_string().contains("t.png"), "{err}");
        assert_eq!(std::fs::read_dir(other.path()).unwrap().count(), 0);
    }

    #[test]
    fn glb_cached_serves_hits_without_the_tool() {
        let dir = tempfile::tempdir().unwrap();
        let key = glb_cache_key(TOOL, b"BMD0", Some(b"BTX0"));
        std::fs::write(dir.path().join(format!("{key}.glb")), b"glTFcached").unwrap();
        let glb = glb_cached(
            dir.path(),
            Path::new("/nonexistent/apicula"),
            TOOL,
            b"BMD0",
            Some(b"BTX0"),
        )
        .unwrap();
        assert_eq!(glb, b"glTFcached");
        // A miss needs the tool.
        let err = glb_cached(
            dir.path(),
            Path::new("/nonexistent/apicula"),
            TOOL,
            b"BMD1",
            None,
        )
        .unwrap_err();
        assert!(matches!(err, KernelError::ToolMissing { .. }));
    }

    #[test]
    fn failed_conversion_leaves_no_files_in_the_cache_dir() {
        let tools = tempfile::tempdir().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let failing = fake(tools.path(), "echo boom >&2; exit 3");
        assert!(glb_cached(dir.path(), &failing, TOOL, b"BMD0", Some(b"BTX0")).is_err());
        let none = fake(tools.path(), "exit 0");
        assert!(glb_cached(dir.path(), &none, TOOL, b"BMD0", None).is_err());
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }
}

#[cfg(feature = "test-support")]
mod project_tileset {
    use g4_formats::map::MapFile;
    use g4_formats::matrix::{MATRIX_EMPTY, Matrix};
    use g4_kernel::project::{AssetKind, Project};

    use super::*;

    fn glb_json(glb: &[u8]) -> serde_json::Value {
        assert_eq!(&glb[0..4], b"glTF");
        let json_len = u32::from_le_bytes(glb[12..16].try_into().unwrap()) as usize;
        serde_json::from_slice(&glb[20..20 + json_len]).unwrap()
    }

    fn image_count(glb: &[u8]) -> usize {
        glb_json(glb)["images"].as_array().map_or(0, |a| a.len())
    }

    /// Checks that every image is embedded (a bufferView inside the BIN chunk, no uri) and
    /// returns the image count.
    fn assert_images_embedded(glb: &[u8]) -> usize {
        let (json, bin) = split_glb(glb);
        let bin = bin.expect("BIN chunk");
        let images = json["images"].as_array().cloned().unwrap_or_default();
        for (i, image) in images.iter().enumerate() {
            assert!(image.get("uri").is_none(), "image {i} still has a uri");
            assert_eq!(image["mimeType"], "image/png");
            let bytes = image_bytes(&json, &bin, i);
            assert_eq!(&bytes[1..4], b"PNG", "image {i} is not a PNG");
        }
        images.len()
    }

    #[test]
    fn map_tileset_bytes_follows_area_data_and_tolerates_out_of_range() {
        let fixture = g4_kernel::test_support::synthetic_project();
        let p = Project::open(fixture.dir.path()).unwrap();
        // Area data 0 selects map tileset 0 (the only texture set); area data 1 selects
        // tileset 2, which does not exist; area data 5 does not exist either.
        assert_eq!(
            p.map_tileset_bytes(0).unwrap().as_deref(),
            Some(&b"BTX0\0\0\0\0"[..])
        );
        assert_eq!(p.map_tileset_bytes(1).unwrap(), None);
        assert_eq!(p.map_tileset_bytes(5).unwrap(), None);
    }

    #[test]
    fn building_models_and_tilesets_tolerate_out_of_range() {
        let fixture = g4_kernel::test_support::synthetic_project();
        let p = Project::open(fixture.dir.path()).unwrap();
        // One exterior building model and one building texture set; Platinum has no interior
        // archive, so `interior` falls back to the exterior one.
        assert_eq!(
            p.building_model_bytes(0, false).unwrap().as_deref(),
            Some(&b"BMD0\0\0\0\0"[..])
        );
        assert_eq!(
            p.building_model_bytes(0, true).unwrap(),
            p.building_model_bytes(0, false).unwrap()
        );
        assert_eq!(p.building_model_bytes(1, false).unwrap(), None);
        assert_eq!(p.building_model_bytes(99_999, false).unwrap(), None);
        assert_eq!(p.building_model_bytes(u32::MAX, false).unwrap(), None);
        // Area data 0 selects building tileset 0, area data 1 (`buildings_tileset` 1) a
        // tileset the archive does not have, area data 5 does not exist.
        assert_eq!(
            p.building_tileset_bytes(0).unwrap().as_deref(),
            Some(&b"BTX0\0\0\0\0"[..])
        );
        assert_eq!(p.building_tileset_bytes(1).unwrap(), None);
        assert_eq!(p.building_tileset_bytes(5).unwrap(), None);
    }

    /// Gated: HGSS has an interior archive (222 models in HG DE) next to the 340 exterior ones;
    /// the other games have only the exterior one and ignore the flag. Works on a copy.
    #[test]
    fn vanilla_building_models_follow_the_interior_flag() {
        let Some((_copy, root)) = g4_kernel::test_support::vanilla_copy() else {
            return;
        };
        let p = Project::open(&root).unwrap();
        // Model 0 is the same file in both HGSS archives; model 1 differs.
        let exterior = p.building_model_bytes(1, false).unwrap().unwrap();
        let interior = p.building_model_bytes(1, true).unwrap().unwrap();
        let count = p.member_count(AssetKind::BuildingModel).unwrap();
        assert!(
            p.building_model_bytes(count as u32, false)
                .unwrap()
                .is_none()
        );
        if p.has_archive(AssetKind::InteriorBuildingModel) {
            assert_ne!(exterior, interior);
            let interior_count = p.member_count(AssetKind::InteriorBuildingModel).unwrap();
            assert!(interior_count < count);
            assert!(
                p.building_model_bytes(interior_count as u32, true)
                    .unwrap()
                    .is_none()
            );
            assert!(
                p.building_model_bytes(interior_count as u32, false)
                    .unwrap()
                    .is_some()
            );
        } else {
            assert_eq!(exterior, interior);
        }
    }

    /// Map model and texture set of a vanilla header, working on a copy of the project
    /// (unpacking writes `unpacked/`; the original is never touched).
    fn vanilla_model_and_tileset(
        header: usize,
    ) -> Option<(tempfile::TempDir, Vec<u8>, Option<Vec<u8>>)> {
        let (copy, root) = g4_kernel::test_support::vanilla_copy()?;
        let p = Project::open(&root).unwrap();
        let headers = p.headers().unwrap();
        let h = &headers[header.min(headers.len() - 1)];
        p.ensure_unpacked(AssetKind::Matrix).unwrap();
        p.ensure_unpacked(AssetKind::Map).unwrap();
        let matrix =
            Matrix::read(&std::fs::read(p.member_path(AssetKind::Matrix, h.matrix_id)).unwrap())
                .unwrap();
        let map_index = *matrix.maps.iter().find(|&&m| m != MATRIX_EMPTY).unwrap();
        let map = MapFile::read(
            &std::fs::read(p.member_path(AssetKind::Map, map_index)).unwrap(),
            p.family,
        )
        .unwrap();
        let tex = p.map_tileset_bytes(h.area_data_id).unwrap();
        Some((copy, map.model, tex))
    }

    #[test]
    fn vanilla_textured_map_glb_has_images() {
        // The project first: a test that needs a project is logged as one for the full run of the
        // gate (it runs per project there), whichever of the two is missing.
        let Some((_copy, model, tex)) = vanilla_model_and_tileset(415) else {
            return;
        };
        let Some(apicula) = g4_formats::test_support::gate_var("G4_APICULA") else {
            return;
        };
        assert!(tex.is_some());
        let cache = tempfile::tempdir().unwrap();
        let apicula = Path::new(&apicula);
        let tool = tool_fingerprint(apicula).unwrap();
        let glb = glb_cached(cache.path(), apicula, tool, &model, tex.as_deref()).unwrap();
        let images = assert_images_embedded(&glb);
        assert!(images > 0);
        eprintln!("embedded {images} images, {} bytes", glb.len());
    }

    #[test]
    fn glb_cached_without_tileset_still_converts() {
        // The project first, see `vanilla_textured_map_glb_has_images`.
        let Some((_copy, model, _tex)) = vanilla_model_and_tileset(415) else {
            return;
        };
        let Some(apicula) = g4_formats::test_support::gate_var("G4_APICULA") else {
            return;
        };
        let cache = tempfile::tempdir().unwrap();
        let apicula = Path::new(&apicula);
        let tool = tool_fingerprint(apicula).unwrap();
        let glb = glb_cached(cache.path(), apicula, tool, &model, None).unwrap();
        assert_eq!(image_count(&glb), 0);
    }
}

#[test]
fn vanilla_map_model_converts_to_glb() {
    // Both are read (the tuple), so a run without either logs each of them.
    let (Some(root), Some(apicula)) = (
        g4_formats::test_support::gate_var("G4_TEST_PROJECT"),
        g4_formats::test_support::gate_var("G4_APICULA"),
    ) else {
        return;
    };
    // Read-only: unpack into a copy so the original project is never written.
    let copy = tempfile::tempdir().unwrap();
    let status = std::process::Command::new("cp")
        .arg("-R")
        .arg(&root)
        .arg(copy.path().join("p"))
        .status()
        .unwrap();
    assert!(status.success());
    let p = g4_kernel::project::Project::open(copy.path().join("p")).unwrap();
    p.ensure_unpacked(g4_kernel::project::AssetKind::Map)
        .unwrap();
    let bytes = std::fs::read(p.member_path(g4_kernel::project::AssetKind::Map, 0)).unwrap();
    let map = g4_formats::map::MapFile::read(&bytes, p.family).unwrap();
    let out = tempfile::tempdir().unwrap();
    let glb = nsbmd_to_glb(Path::new(&apicula), &map.model, None, out.path()).unwrap();
    assert_eq!(&std::fs::read(&glb).unwrap()[0..4], b"glTF");
}

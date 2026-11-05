use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Temp {
    path: PathBuf,
}

impl Temp {
    fn new() -> Self {
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("jfs-cli-{}-{nanos}-{n}", std::process::id()));
        fs::create_dir_all(path.join("jdks")).unwrap();
        fs::create_dir_all(path.join("data")).unwrap();
        Self { path }
    }

    fn data(&self) -> PathBuf {
        self.path.join("data")
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let data = self.data();
        for name in ["current", "current.new", "current.bak"] {
            let path = data.join(name);
            if fs::symlink_metadata(&path).is_ok() {
                let _ = fs::remove_dir(&path);
            }
        }
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn fake_jdk(root: &Path, name: &str) -> PathBuf {
    let jdk = root.join(name);
    fs::create_dir_all(jdk.join("bin")).unwrap();
    fs::write(jdk.join("bin").join("java.exe"), b"fake").unwrap();
    fs::write(jdk.join("marker.txt"), name.as_bytes()).unwrap();
    jdk
}

fn run(data: &Path, args: &[&str]) -> (i32, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_jfs"))
        .args(args)
        .env("JFS_DIR", data)
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn jfs");
    (
        output.status.code().unwrap_or(1),
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
    )
}

#[test]
fn version_switch_keeps_jdk_files() {
    let temp = Temp::new();
    let data = temp.data();
    let jdk21 = fake_jdk(&temp.path.join("jdks"), "21");
    let jdk8 = fake_jdk(&temp.path.join("jdks"), "8");

    let (code, out, err) = run(&data, &["add", "21", jdk21.to_str().unwrap()]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("Added 21"));
    assert!(out.contains("Now using Java 21"));

    let (code, out, err) = run(&data, &["add", "8", jdk8.to_str().unwrap()]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!out.contains("Now using Java 8"));

    let (code, out, err) = run(&data, &["list"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(err.is_empty(), "{err}");
    assert!(out.contains("* 21"));
    assert!(out.contains("  8"));
    let star = out.find("* 21").unwrap();
    let eight = out.find("  8").unwrap();
    assert!(
        eight < star,
        "numeric order should list 8 before 21:\n{out}"
    );

    let (code, out, err) = run(&data, &["current"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(out.trim(), "21");
    assert_eq!(
        fs::read(data.join("current").join("marker.txt")).unwrap(),
        b"21"
    );

    let (code, out, err) = run(&data, &["use", "8"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("Now using Java 8"));
    assert_eq!(
        fs::read(data.join("current").join("marker.txt")).unwrap(),
        b"8"
    );
    assert_eq!(fs::read(jdk21.join("marker.txt")).unwrap(), b"21");
    assert_eq!(fs::read(jdk8.join("marker.txt")).unwrap(), b"8");

    let (code, out, err) = run(&data, &["rm", "8"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(err.contains("Cannot remove active version. Switch to another version first."));

    let (code, out, err) = run(&data, &["use", "21"]);
    assert_eq!(code, 0, "{out}{err}");
    let (code, out, err) = run(&data, &["rm", "8"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("Removed 8"));
    assert_eq!(fs::read(jdk8.join("marker.txt")).unwrap(), b"8");

    let (code, _, err) = run(&data, &["use", "8"]);
    assert_eq!(code, 1);
    assert!(err.contains("is not registered"));
}

#[test]
fn empty_registry_and_rejected_paths() {
    let temp = Temp::new();
    let data = temp.data();

    let (code, out, err) = run(&data, &["list"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(
        out.trim(),
        "No JDK versions registered. Use 'jfs add <name> <path>' to get started."
    );

    let (code, _, err) = run(&data, &["current"]);
    assert_eq!(code, 1);
    assert!(err.contains("No active Java version."));

    let empty = temp.path.join("not-a-jdk");
    fs::create_dir_all(&empty).unwrap();
    let (code, _, err) = run(&data, &["add", "nope", empty.to_str().unwrap()]);
    assert_eq!(code, 1);
    assert!(err.contains("missing bin\\java.exe"));

    let (code, _, err) = run(&data, &["add", "bad/name", empty.to_str().unwrap()]);
    assert_eq!(code, 1);
    assert!(err.contains("cannot contain"));
}

#[test]
fn refuses_to_delete_a_real_current_directory() {
    let temp = Temp::new();
    let data = temp.data();
    let jdk = fake_jdk(&temp.path.join("jdks"), "21");
    let current = data.join("current");
    fs::create_dir_all(&current).unwrap();
    fs::write(current.join("keep.txt"), b"mine").unwrap();

    let (code, _, err) = run(&data, &["add", "21", jdk.to_str().unwrap()]);
    assert_eq!(code, 1, "{err}");
    assert!(err.contains("real directory"));
    assert_eq!(fs::read(current.join("keep.txt")).unwrap(), b"mine");
    assert!(!data.join("config.json").exists());
}

#[test]
fn reports_its_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_jfs"))
        .arg("--version")
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.starts_with("jfs 0.1.0"));
}

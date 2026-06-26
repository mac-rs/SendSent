use anyhow::Result;
use std::path::{Path, PathBuf};

pub struct AtomicWriter { root: PathBuf, tmp_dir: PathBuf }

impl AtomicWriter {
    pub fn new(save_dir: &Path, session_id: &str) -> Result<Self> {
        let tmp_dir = save_dir.join(".sendsent-tmp").join(session_id);
        std::fs::create_dir_all(&tmp_dir)?;
        Ok(Self { root: save_dir.to_path_buf(), tmp_dir })
    }
    pub fn part_path(&self, rel: &str) -> PathBuf { self.tmp_dir.join(rel_to_tmp_name(rel)) }
    pub fn finalize(&self, rel: &str) -> Result<PathBuf> {
        let src = self.part_path(rel);
        let (dir, file) = split_rel(&self.root, rel);
        std::fs::create_dir_all(&dir)?;
        let dest = unique_path(&dir, &file);
        std::fs::rename(&src, &dest)?;
        Ok(dest)
    }
    pub fn cleanup(&self) {
        let _ = std::fs::remove_dir_all(&self.tmp_dir);
        // also remove the `.sendsent-tmp` parent when it is now empty
        // (remove_dir only succeeds on an empty dir, so other in-flight sessions are preserved)
        if let Some(parent) = self.tmp_dir.parent() {
            let _ = std::fs::remove_dir(parent);
        }
    }
}

fn split_rel(root: &Path, rel: &str) -> (PathBuf, String) {
    let p = root.join(rel);
    let dir = p.parent().unwrap_or(root).to_path_buf();
    let file = p.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "file".into());
    (dir, file)
}

fn rel_to_tmp_name(rel: &str) -> PathBuf {
    let flat: String = rel.chars().map(|c| match c { '/' | '\\' => '_' , _ => c}).collect();
    PathBuf::from(format!("{}.part", flat))
}

pub fn unique_path(dir: &Path, file: &str) -> PathBuf {
    let target = dir.join(file);
    if !target.exists() { return target; }
    if let Some(dot) = file.rfind('.') {
        let (stem, ext) = file.split_at(dot);
        for i in 1u32.. {
            let cand = dir.join(format!("{} ({}){}", stem, i, ext));
            if !cand.exists() { return cand; }
        }
    } else {
        for i in 1u32.. {
            let cand = dir.join(format!("{} ({})", file, i));
            if !cand.exists() { return cand; }
        }
    }
    target
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tmp() -> PathBuf { std::env::temp_dir().join(format!("ss-aw-{}", uuid::Uuid::new_v4())) }

    #[test]
    fn finalize_renames_to_final() {
        let root = tmp(); let w = AtomicWriter::new(&root, "s1").unwrap();
        let p = w.part_path("d/a.txt");
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, b"hi").unwrap();
        let dest = w.finalize("d/a.txt").unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), b"hi");
        assert!(!p.exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn collision_appends_suffix() {
        let root = tmp(); let w = AtomicWriter::new(&root, "s2").unwrap();
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("a.txt"), b"old").unwrap();
        let p = w.part_path("a.txt");
        std::fs::write(&p, b"new").unwrap();
        let dest = w.finalize("a.txt").unwrap();
        assert_eq!(dest, root.join("a (1).txt"));
        assert_eq!(std::fs::read(root.join("a.txt")).unwrap(), b"old");
        assert_eq!(std::fs::read(&dest).unwrap(), b"new");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn cleanup_removes_temp() {
        let root = tmp(); let w = AtomicWriter::new(&root, "s3").unwrap();
        let p = w.part_path("a.txt"); std::fs::write(&p, b"x").unwrap();
        w.cleanup();
        assert!(!p.exists());
        assert!(!root.join(".sendsent-tmp").exists(), "parent .sendsent-tmp should also be removed when empty");
        let _ = std::fs::remove_dir_all(&root);
    }
}

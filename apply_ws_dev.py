#!/usr/bin/env python3
"""Workbench arka uç yaması (idempotent). Proje kökünde çalıştırın:

    python3 apply_ws_dev.py

Yeni dosyaları (workspace_dev.rs, stage.rs) zip zaten yerleştirir; bu betik
mevcut dosyalardaki küçük eklemeleri yapar. İkinci çalıştırma hiçbir şey
değiştirmez.
"""
import pathlib
import sys

ROOT = pathlib.Path(".")
changed = []


def edit(rel, fn):
    p = ROOT / rel
    if not p.exists():
        sys.exit(f"HATA: {rel} bulunamadı (proje kökünde misiniz?)")
    old = p.read_text()
    new = fn(old)
    if new != old:
        p.write_text(new)
        changed.append(rel)


def must_contain(text, anchor, rel):
    if anchor not in text:
        sys.exit(f"HATA: {rel} içinde çapa bulunamadı: {anchor!r}")


def git_mod(t):
    rel = "workspace_core/src/git/mod.rs"
    if "mod stage;" not in t:
        must_contain(t, "mod status;\n", rel)
        t = t.replace("mod status;\n", "mod stage;\nmod status;\n", 1)
    if "pub use stage::" not in t:
        must_contain(t, "pub use status::status;\n", rel)
        t = t.replace(
            "pub use status::status;\n",
            "pub use stage::{commit, stage, unstage};\npub use status::status;\n",
            1,
        )
    return t


METHODS = '''    /// Dosyayı git sahnesine al. Yol doğrulanır; `.git` korumalıdır.
    pub fn git_stage(&self, relative: &str) -> Result<()> {
        let rel = self.git_pathspec(relative)?;
        git::stage(&self.root, &rel)
    }

    /// Dosyanın sahnelenmiş değişikliğini geri al.
    pub fn git_unstage(&self, relative: &str) -> Result<()> {
        let rel = self.git_pathspec(relative)?;
        git::unstage(&self.root, &rel)
    }

    /// Sahnelenmiş değişikliklerle commit oluştur.
    pub fn git_commit(&self, message: &str) -> Result<()> {
        git::commit(&self.root, message)
    }

    fn git_pathspec(&self, relative: &str) -> Result<String> {
        let full = self.resolve_mutable_nofollow(relative)?;
        Ok(self.rel(&full))
    }

'''


def lib_rs(t):
    rel = "workspace_core/src/lib.rs"
    if "pub fn git_stage" in t:
        return t
    anchor = "    pub fn git_worktree_list(&self)"
    must_contain(t, anchor, rel)
    return t.replace(anchor, METHODS + anchor, 1)


def types_rs(t):
    if "WorkspaceSearchHit" in t:
        return t
    extra = pathlib.Path("types_append.rs").read_text()
    return t.rstrip("\n") + "\n" + extra


def cargo_toml(t):
    old = 'aetheros-workspace = { path = "workspace_core" }'
    new = 'aetheros-workspace = { path = "workspace_core", features = ["watcher"] }'
    if new in t:
        return t
    must_contain(t, old, "Cargo.toml")
    return t.replace(old, new, 1)


def api_mod(t):
    rel = "src/bridge/api/mod.rs"
    if "pub mod workspace_dev;" not in t:
        must_contain(t, "pub mod workspace;\n", rel)
        t = t.replace(
            "pub mod workspace;\n", "pub mod workspace;\npub mod workspace_dev;\n", 1
        )
    if "pub use workspace_dev::*;" not in t:
        must_contain(t, "pub use workspace::*;\n", rel)
        t = t.replace(
            "pub use workspace::*;\n",
            "pub use workspace::*;\npub use workspace_dev::*;\n",
            1,
        )
    return t


edit("workspace_core/src/git/mod.rs", git_mod)
edit("workspace_core/src/lib.rs", lib_rs)
edit("src/bridge/types.rs", types_rs)
edit("Cargo.toml", cargo_toml)
edit("src/bridge/api/mod.rs", api_mod)

print("Değişen dosyalar:" if changed else "Zaten uygulanmış, değişiklik yok.")
for c in changed:
    print("  -", c)

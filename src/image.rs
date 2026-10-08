use crate::config::Config;
use crate::discovery::Discovery;
use anyhow::{Context, Result};
use colored::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

const BLOCK_SIZE: usize = 4096;
const SECTOR_SIZE: usize = 512;
const DIRENTRY_SIZE: usize = 128;
const NAME_MAX: usize = 48;
const INLINE_MAX: usize = 16;
const SUPERBLOCK_MAGIC_NE2: u32 = 0x0032454E;
const MODE_DIR: u16 = 0x0040;
const MODE_FILE: u16 = 0x0080;
const PERM_R: u16 = 0x0001;
const PERM_W: u16 = 0x0002;
const PERM_X: u16 = 0x0004;

/// Regiones libres que caben en un nodo tipo 3 de 4 KB:
/// `(4096 - 8 header - 8 next_lba) / 12`.
const REGIONS_PER_NODE: usize = 340;

/// Plan the free-list node chain.
///
/// The first node is placed at `first_lba`. If the region list needs more than
/// one node, the extra nodes are reserved from the front of the first free
/// region (which is shrunk accordingly). Returns `(node_lbas, adjusted_regions)`.
fn plan_freelist(
    mut free_regions: Vec<(u64, u32)>,
    first_lba: u64,
) -> Result<(Vec<u64>, Vec<(u64, u32)>)> {
    if free_regions.is_empty() {
        return Ok((Vec::new(), free_regions));
    }
    let num_nodes = (free_regions.len() + REGIONS_PER_NODE - 1) / REGIONS_PER_NODE;
    let mut node_lbas = vec![first_lba];
    let extra = num_nodes - 1;
    if extra > 0 {
        let (start, len) = free_regions[0];
        if (len as usize) < extra {
            anyhow::bail!(
                "not enough contiguous free space to store {} free-list chain nodes",
                num_nodes
            );
        }
        for i in 0..extra {
            node_lbas.push(start + i as u64);
        }
        if len as usize == extra {
            free_regions.remove(0);
        } else {
            free_regions[0] = (start + extra as u64, len - extra as u32);
        }
    }
    Ok((node_lbas, free_regions))
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFFFFFFu32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB88320;
            } else {
                crc >>= 1;
            }
        }
    }
    crc ^ 0xFFFFFFFF
}

fn default_perms(name: &str) -> u16 {
    let u = name.to_uppercase();
    if u.ends_with(".NXE") || u.ends_with(".COM") || u.ends_with(".EXE") { return PERM_R | PERM_X; }
    if u.ends_with(".NEM") { return PERM_R; }
    if u.ends_with(".NXL") { return PERM_R | PERM_X; }
    if u.ends_with(".BAT") || u.ends_with(".CMD") { return PERM_R | PERM_X; }
    if u.ends_with(".SYS") { return PERM_R; }
    if u.ends_with(".CFG") || u.ends_with(".INI") { return PERM_R | PERM_W; }
    if u.ends_with(".TXT") || u.ends_with(".MD") || u.ends_with(".LOG") { return PERM_R | PERM_W; }
    PERM_R | PERM_W
}

fn make_direntry(name: &str, mode: u16, size: u64, extent_lba: u64, extent_count: u32, inline_data: &[u8]) -> Vec<u8> {
    let mut buf = vec![0u8; DIRENTRY_SIZE];
    let nl = name.len().min(NAME_MAX);
    buf[0] = nl as u8;
    buf[1..=nl].copy_from_slice(&name.as_bytes()[..nl]);
    let off_inline = 1 + NAME_MAX;
    let il = inline_data.len().min(INLINE_MAX);
    buf[off_inline..off_inline + il].copy_from_slice(&inline_data[..il]);
    let mut off = off_inline + INLINE_MAX;
    put_u16_le(&mut buf, off, mode); off += 2;
    put_u64_le(&mut buf, off, size); off += 8;
    put_u64_le(&mut buf, off, 0); off += 8;
    put_u64_le(&mut buf, off, 0); off += 8;
    put_u32_le(&mut buf, off, 0); off += 4;
    put_u32_le(&mut buf, off, il as u32); off += 4;
    put_u64_le(&mut buf, off, extent_lba); off += 8;
    put_u32_le(&mut buf, off, extent_count);
    buf
}

/// Serialize a NE2 directory leaf.
///
/// Layout: `[u16 type=1][u16 count][u32 crc][ (u16 key_len)(key)(u16 val_len)(value) ... ]`
/// Payload available for entries is `BLOCK_SIZE - LEAF_HEADER`.
///
/// Fail-fast invariant: the declared `count` must always equal the number of
/// entries actually serialized.  If the caller passes more entries than fit in a
/// single leaf, this returns an error instead of writing a partial leaf (which
/// would corrupt directory lookups).  Multi-block directory leaves are
/// intentionally not implemented here — callers must rebalance directories.
fn make_btree_leaf(dirpath: &str, entries: &[(Vec<u8>, Vec<u8>)]) -> Result<Vec<u8>> {
    const LEAF_HEADER: usize = 8; // u16 type + u16 count + u32 crc
    let available = BLOCK_SIZE - LEAF_HEADER;
    let mut data = vec![0u8; BLOCK_SIZE];
    let mut off = LEAF_HEADER;
    let mut serialized = 0usize;
    let mut required = 0usize;
    let mut overflowed = false;
    for (key, value) in entries {
        let entry_size = 4 + key.len() + value.len();
        required += entry_size;
        // Once an entry no longer fits, stop serializing so the leaf remains a
        // sorted prefix; keep counting `required` for the diagnostic.
        if !overflowed && off + entry_size <= BLOCK_SIZE {
            put_u16_le(&mut data, off, key.len() as u16); off += 2;
            data[off..off + key.len()].copy_from_slice(key); off += key.len();
            put_u16_le(&mut data, off, value.len() as u16); off += 2;
            data[off..off + value.len()].copy_from_slice(value); off += value.len();
            serialized += 1;
        } else {
            overflowed = true;
        }
    }
    if overflowed || serialized != entries.len() {
        anyhow::bail!(
            "NE2 directory leaf overflow in '{}': {} entries requested, only {} fit \
             in a single {} byte leaf ({} bytes available for entries, {} bytes required). \
             Rebalance this directory (multi-block directory support is not implemented).",
            dirpath,
            entries.len(),
            serialized,
            BLOCK_SIZE,
            available,
            required
        );
    }
    put_u16_le(&mut data, 0, 1);
    put_u16_le(&mut data, 2, entries.len() as u16);
    let cksum = crc32(&data[8..]);
    put_u32_le(&mut data, 4, cksum);
    Ok(data)
}

/// Split directory entries into leaf-sized chunks (by serialized bytes).
fn leaf_chunks_of(entries: &[(Vec<u8>, Vec<u8>)]) -> Vec<Vec<usize>> {
    let mut chunks: Vec<Vec<usize>> = Vec::new();
    let mut cur: Vec<usize> = Vec::new();
    let mut used = 0usize;
    for (i, (k, v)) in entries.iter().enumerate() {
        let sz = 4 + k.len() + v.len();
        if !cur.is_empty() && used + sz > BLOCK_SIZE - 8 {
            chunks.push(core::mem::take(&mut cur));
            used = 0;
        }
        cur.push(i);
        used += sz;
    }
    if !cur.is_empty() { chunks.push(cur); }
    if chunks.is_empty() { chunks.push(Vec::new()); }
    chunks
}

/// Serialize a directory B-tree: leaves first, then (if more than one leaf) a
/// single internal node pointing at them. `lbas` holds one LBA per node, leaves
/// first; the internal node (last) references the leaves' LBAs.
fn build_dir_nodes(entries: &[(Vec<u8>, Vec<u8>)], lbas: &[u64]) -> Result<Vec<Vec<u8>>> {
    let chunks = leaf_chunks_of(entries);
    let mut out: Vec<Vec<u8>> = Vec::with_capacity(lbas.len());
    for (ci, idxs) in chunks.iter().enumerate() {
        let chunk: Vec<(Vec<u8>, Vec<u8>)> = idxs.iter().map(|&i| entries[i].clone()).collect();
        out.push(make_btree_leaf("<dir>", &chunk)?);
        let _ = ci;
    }
    if chunks.len() > 1 {
        let mut node = vec![0u8; BLOCK_SIZE];
        put_u16_le(&mut node, 0, 0); // node_type = internal
        put_u16_le(&mut node, 2, chunks.len() as u16);
        let mut off = 8usize;
        for (ci, idxs) in chunks.iter().enumerate() {
            let key: &[u8] = if ci == 0 { &[] } else { entries[idxs[0]].0.as_slice() };
            put_u16_le(&mut node, off, key.len() as u16); off += 2;
            node[off..off + key.len()].copy_from_slice(key); off += key.len();
            put_u16_le(&mut node, off, 8); off += 2;
            put_u64_le(&mut node, off, lbas[ci]); off += 8;
        }
        let cksum = crc32(&node[8..]);
        put_u32_le(&mut node, 4, cksum);
        out.push(node);
    }
    Ok(out)
}

/// Flush a bucket of locale NLTs into `/System/Locale/<dir>/`.
fn push_locale_bucket(
    files: &mut Vec<FileEntry>,
    loc_dirs: &[String],
    idx: usize,
    bucket: &mut Vec<(String, Vec<u8>)>,
) {
    let dir = &loc_dirs[idx];
    for (fname, content) in bucket.drain(..) {
        files.push(FileEntry {
            name: format!("/System/Locale/{}/{}", dir, fname),
            content,
            mode: MODE_FILE | PERM_R,
            is_dir: false,
        });
    }
}

fn put_u16_le(buf: &mut [u8], off: usize, val: u16) {
    buf[off] = (val & 0xFF) as u8;
    buf[off + 1] = ((val >> 8) & 0xFF) as u8;
}

fn put_u32_le(buf: &mut [u8], off: usize, val: u32) {
    buf[off] = (val & 0xFF) as u8;
    buf[off + 1] = ((val >> 8) & 0xFF) as u8;
    buf[off + 2] = ((val >> 16) & 0xFF) as u8;
    buf[off + 3] = ((val >> 24) & 0xFF) as u8;
}

fn put_u64_le(buf: &mut [u8], off: usize, val: u64) {
    put_u32_le(buf, off, (val & 0xFFFFFFFF) as u32);
    put_u32_le(buf, off + 4, ((val >> 32) & 0xFFFFFFFF) as u32);
}

fn read_u64_le(buf: &[u8], off: usize) -> u64 {
    let lo = u64::from(buf[off]) | (u64::from(buf[off + 1]) << 8) | (u64::from(buf[off + 2]) << 16) | (u64::from(buf[off + 3]) << 24);
    let hi = u64::from(buf[off + 4]) | (u64::from(buf[off + 5]) << 8) | (u64::from(buf[off + 6]) << 16) | (u64::from(buf[off + 7]) << 24);
    lo | (hi << 32)
}

struct FileEntry {
    name: String,
    content: Vec<u8>,
    mode: u16,
    is_dir: bool,
}

pub fn build_ne2_image(cfg: &Config, disc: &Discovery, output: &Path, label: &str, blocks: u64, build_drivers: bool) -> Result<()> {
    let start = Instant::now();
    println!("{} Building NE2 filesystem image...", "[*]".bold().cyan());
    // #21: never package stale NEM drivers. If drivers were not built in this
    // process, build them now so collect_files() uses the fresh
    // /tmp/nem_drivers_<pid> output instead of the gitignored data/nem_bin/**
    // fallback (NeoDOS #491).
    if build_drivers {
        let _ = crate::build::ensure_nem_drivers(disc);
    }
    let files = collect_files(cfg, disc)?;

    // Guard against silently packaging a driverless image: Boot-critical
    // driver tests (e.g. ob_set_datetime_rtc_write_acks) fail when no NEM
    // driver is present, which is confusing if the image looked "successful".
    let nem_count = files
        .iter()
        .filter(|f| f.name.starts_with("/System/Drivers/") && f.name.ends_with(".nem"))
        .count();
    if nem_count == 0 {
        eprintln!(
            "{} WARNING: packaging an image with 0 NEM drivers. No *.nem was found under \
             data/nem_bin or the build output. Boot-time driver tests will fail; use a full \
             'neodev build --image' (without --quick/--no-build).",
            "[!]".bold().yellow()
        );
    }

    let root_marker = "/";
    let mut dir_tree: HashMap<String, Vec<FileEntry>> = HashMap::new();
    dir_tree.insert(root_marker.to_string(), vec![]);
    for entry in &files {
        let path = entry.name.trim_start_matches('/');
        let parts: Vec<&str> = path.split('/').collect();
        let filename = parts.last().unwrap_or(&"");
        let parent = if parts.len() > 1 { format!("/{}", parts[..parts.len() - 1].join("/")) } else { root_marker.to_string() };
        dir_tree.entry(parent.clone()).or_default().push(FileEntry { name: filename.to_string(), content: entry.content.clone(), mode: entry.mode, is_dir: false });
        for i in 1..parts.len() {
            let dir_path = format!("/{}", parts[..i].join("/"));
            let dir_parent = if i > 1 { format!("/{}", parts[..i - 1].join("/")) } else { root_marker.to_string() };
            let dir_name = parts[i - 1].to_string();
            let already = dir_tree.get(&dir_parent).is_some_and(|entries| entries.iter().any(|e| e.name == dir_name && e.is_dir));
            if !already {
                dir_tree.entry(dir_parent).or_default().push(FileEntry { name: dir_name, content: vec![], mode: MODE_DIR | PERM_R | PERM_W | PERM_X | 0x0010, is_dir: true });
            }
            dir_tree.entry(dir_path).or_default();
        }
    }

    let mut dir_paths: Vec<&String> = dir_tree.keys().collect();
    dir_paths.sort_by_key(|k| k.matches('/').count());

    // ── Directory layout (multi-leaf, NeoDOS #396) ─────────────────
    // Each directory is a B-tree: one or more leaf nodes (type 1) plus, if the
    // entries do not fit in a single leaf, one internal node (type 0) pointing
    // at the leaves. Directory nodes start at block 2 (0/1 reserved).
    type DirEntries = Vec<(Vec<u8>, Vec<u8>)>; // (name, 128-byte direntry)
    let mut dir_entries: HashMap<String, DirEntries> = HashMap::new();
    for dirpath in dir_paths.iter() {
        let mut node_entries: DirEntries = vec![];
        if let Some(entries) = dir_tree.get(*dirpath) {
            for entry in entries {
                if entry.is_dir || entry.content.len() > INLINE_MAX {
                    // extent_lba/count se rellenan más tarde (raíz de subdir /
                    // extent de datos).
                    node_entries.push((entry.name.as_bytes().to_vec(), make_direntry(&entry.name, entry.mode, entry.content.len() as u64, 0, 0, &[])));
                } else {
                    node_entries.push((entry.name.as_bytes().to_vec(), make_direntry(&entry.name, entry.mode, entry.content.len() as u64, 0, 0, &entry.content)));
                }
            }
        }
        node_entries.sort_by(|a, b| a.0.cmp(&b.0));
        dir_entries.insert((*dirpath).clone(), node_entries);
    }

    // Asignar LBAs consecutivas a todos los nodos de cada directorio (hojas y,
    // si hace falta, un nodo interno). La raíz es el interno o la única hoja.
    let mut next_lba: u64 = 2;
    let mut dir_lbas: HashMap<String, (Vec<u64>, usize)> = HashMap::new();
    for dirpath in dir_paths.iter() {
        let chunks = leaf_chunks_of(dir_entries.get(*dirpath).unwrap());
        let num_leaves = chunks.len();
        let has_internal = num_leaves > 1;
        if has_internal && num_leaves > 60 {
            anyhow::bail!(
                "NE2 directory '{}' needs {} leaves; multi-level internal nodes are not implemented",
                dirpath, num_leaves
            );
        }
        let num_nodes = num_leaves + if has_internal { 1 } else { 0 };
        let lbas: Vec<u64> = (0..num_nodes as u64).map(|i| next_lba + i).collect();
        let root_idx = if has_internal { num_leaves } else { 0 };
        dir_lbas.insert((*dirpath).clone(), (lbas, root_idx));
        next_lba += num_nodes as u64;
    }
    let dir_lba_map: HashMap<String, u64> = dir_lbas
        .iter()
        .map(|(k, (lbas, ri))| (k.clone(), lbas[*ri]))
        .collect();

    // Rellenar la raíz de cada subdirectorio en su DirEntry (offset 99).
    for dirpath in dir_paths.iter() {
        let subdirs: Vec<String> = dir_tree
            .get(*dirpath)
            .map(|es| es.iter().filter(|e| e.is_dir).map(|e| {
                if *dirpath == root_marker { format!("/{}", e.name) }
                else { format!("/{}/{}", dirpath.trim_start_matches('/'), e.name) }
            }).collect())
            .unwrap_or_default();
        if let Some(entries) = dir_entries.get_mut(*dirpath) {
            for sub in &subdirs {
                let name = sub.rsplit('/').next().unwrap_or("").as_bytes();
                let lba = dir_lba_map.get(sub).copied().unwrap_or(0);
                if let Some((_, v)) = entries.iter_mut().find(|(k, _)| k.as_slice() == name) {
                    put_u64_le(v, 99, lba);
                }
            }
        }
    }

    // Asignar extents de datos (después de todos los nodos de directorio) y
    // rellenarlos en las entradas.
    for dirpath in dir_paths.iter() {
        let files: Vec<(String, usize)> = dir_tree
            .get(*dirpath)
            .map(|es| es.iter().filter(|e| !e.is_dir && e.content.len() > INLINE_MAX)
                .map(|e| (e.name.clone(), e.content.len())).collect())
            .unwrap_or_default();
        for (fname, len) in files {
            let block_count = len.div_ceil(BLOCK_SIZE);
            let extent_lba = next_lba;
            next_lba += block_count as u64;
            if let Some(entries) = dir_entries.get_mut(*dirpath) {
                if let Some((_, v)) = entries.iter_mut().find(|(k, _)| k.as_slice() == fname.as_bytes()) {
                    put_u64_le(v, 99, extent_lba);
                    put_u32_le(v, 107, block_count as u32);
                }
            }
        }
    }

    let total_blocks = next_lba.max(blocks);
    let image_size = (total_blocks as usize) * BLOCK_SIZE;
    let mut image = vec![0u8; image_size];

    let root_lba = dir_lba_map.get(root_marker).copied().unwrap_or(1);

    // NE2 v2 free list (NeoDOS #15): type-3 node(s) holding the free regions.
    // The first node lives at block 1 (unused by the directory/data allocator,
    // which starts at block 2); if the region list does not fit in one node,
    // the extra chain nodes are reserved from the front of the free space.
    let free_regions: Vec<(u64, u32)> = if next_lba < total_blocks {
        vec![(next_lba, (total_blocks - next_lba) as u32)]
    } else {
        Vec::new()
    };
    let (node_lbas, free_regions) = plan_freelist(free_regions, 1)?;
    let freelist_lba: u64 = node_lbas.first().copied().unwrap_or(0);
    for (idx, &lba) in node_lbas.iter().enumerate() {
        let start = (idx * REGIONS_PER_NODE).min(free_regions.len());
        let end = (start + REGIONS_PER_NODE).min(free_regions.len());
        let chunk = &free_regions[start..end];
        let mut node = vec![0u8; BLOCK_SIZE];
        put_u16_le(&mut node, 0, 3); // node_type = freelist
        put_u16_le(&mut node, 2, chunk.len() as u16);
        let mut off = 8usize;
        for (rs, rl) in chunk {
            put_u64_le(&mut node, off, *rs); off += 8;
            put_u32_le(&mut node, off, *rl); off += 4;
        }
        let next = node_lbas.get(idx + 1).copied().unwrap_or(0);
        put_u64_le(&mut node, off, next); // next_lba (0 = end of chain)
        let cksum = crc32(&node[8..]);
        put_u32_le(&mut node, 4, cksum);
        let block_off = (lba as usize) * BLOCK_SIZE;
        image[block_off..block_off + BLOCK_SIZE].copy_from_slice(&node);
    }

    let label_bytes = label.as_bytes();
    let extra_nodes = node_lbas.len().saturating_sub(1) as u64;
    let num_free: u64 = free_regions.iter().map(|(_, l)| *l as u64).sum();
    let mut sb = vec![0u8; SECTOR_SIZE];
    put_u32_le(&mut sb, 0, SUPERBLOCK_MAGIC_NE2);
    put_u32_le(&mut sb, 4, 2);
    put_u64_le(&mut sb, 8, root_lba);
    put_u64_le(&mut sb, 16, 1);
    put_u64_le(&mut sb, 24, 0);
    put_u64_le(&mut sb, 32, total_blocks);
    put_u64_le(&mut sb, 40, next_lba + extra_nodes);
    put_u64_le(&mut sb, 48, num_free);
    sb[56] = label_bytes.len().min(32) as u8;
    let lbl_len = label_bytes.len().min(32);
    sb[57..57 + lbl_len].copy_from_slice(&label_bytes[..lbl_len]);
    put_u64_le(&mut sb, 93, freelist_lba);
    put_u64_le(&mut sb, 101, 0); // snapshot_table_lba
    // CRC32 over the whole 512-byte superblock with the checksum field
    // (offset 109) zeroed. Must match the kernel's `superblock_crc`.
    let cksum = crc32(&sb[..SECTOR_SIZE]);
    put_u32_le(&mut sb, 109, cksum);
    image[..SECTOR_SIZE].copy_from_slice(&sb);

    // Escribir los nodos B-tree de cada directorio (hojas + interno).
    for dirpath in dir_paths.iter() {
        let entries = dir_entries.get(*dirpath).unwrap();
        let (lbas, _) = dir_lbas.get(*dirpath).unwrap();
        let nodes = build_dir_nodes(entries, lbas)?;
        for (ni, data) in nodes.iter().enumerate() {
            let off = (lbas[ni] as usize) * BLOCK_SIZE;
            if off + BLOCK_SIZE <= image.len() { image[off..off + BLOCK_SIZE].copy_from_slice(data); }
        }
    }

    // Escribir los bloques de datos de los ficheros.
    for (dirpath, entries) in &dir_tree {
        for entry in entries {
            if entry.is_dir || entry.content.len() <= INLINE_MAX { continue; }
            if let Some(des) = dir_entries.get(dirpath) {
                if let Some((_, v)) = des.iter().find(|(k, _)| k.as_slice() == entry.name.as_bytes()) {
                    let extent_lba = read_u64_le(v, 99);
                    let block_count = entry.content.len().div_ceil(BLOCK_SIZE);
                    let block_start = (extent_lba as usize) * BLOCK_SIZE;
                    for i in 0..block_count {
                        let chunk_start = i * BLOCK_SIZE;
                        let chunk_end = (chunk_start + BLOCK_SIZE).min(entry.content.len());
                        let data = &entry.content[chunk_start..chunk_end];
                        let dest = block_start + i * BLOCK_SIZE;
                        if dest + data.len() <= image.len() { image[dest..dest + data.len()].copy_from_slice(data); }
                    }
                }
            }
        }
    }

    std::fs::write(output, &image).context("Failed to write NE2 image")?;
    let actual = std::fs::metadata(output)?.len();
    let total_entries: usize = dir_entries.values().map(|v| v.len()).sum();
    println!("{} NE2 image: {} ({} blocks, {} entries)", "[✓]".bold().green(), output.display(), total_blocks, total_entries);
    println!("  Size: {} ({:.1} MB)", fmt_size(actual), actual as f64 / 1_048_576.0);
    println!("  Duration: {:.1}s", start.elapsed().as_secs_f64());
    Ok(())
}

fn collect_files(cfg: &Config, _disc: &Discovery) -> Result<Vec<FileEntry>> {
    let mut files: Vec<FileEntry> = vec![];
    let root = &cfg.neodos_root;

    files.push(FileEntry { name: "/README.TXT".into(), content: b"Welcome to NeoDOS v2!\r\n".to_vec(), mode: MODE_FILE | PERM_R | PERM_W, is_dir: false });
    files.push(FileEntry { name: "/Temp/.empty".into(), content: vec![], mode: MODE_FILE | PERM_R | PERM_W, is_dir: false });

    let hiv_path = root.join("data").join("system.hiv");
    if hiv_path.exists() {
        let content = std::fs::read(&hiv_path)?;
        files.push(FileEntry { name: "/System/Registry/SYSTEM.hiv".into(), content, mode: MODE_FILE | PERM_R, is_dir: false });
    }

    // A single NE2 directory leaf holds ~28 entries (4096 B block, 8 B header,
    // then 4 + len(name) + 128 B per entry).  Keep each group under that
    // capacity: `build_ne2_image` fails fast on a leaf overflow instead of
    // silently truncating entries.
    let programs_nxe = &[
        "neoshell", "neoinit", "cmdtest", "cd", "corehelp",
        "datetime", "neomem", "echo", "label",
        "coretype", "corecls", "corecopy", "coredel",
        "coreren", "coremd", "corerd", "drives", "ps", "keyb", "coredir",
        "poweroff", "colors", "neokey",
        "nxres", "nxlocale", "nxverify", "ping", "hostname",
    ];
    let tools_nxe = &[
        "kill", "pri", "fsck", "ndreg", "loadnem", "progress",
        "neotop", "dhcpd", "netcfg", "netapplier", "netd", "ipconfig", "cpuinfo", "neolocale", "dhcptest",
        "nslookup", "ntpd",
        // Moved out of /Programs to keep its single leaf within capacity.
        // The shell PATH includes System/Tools, so they stay discoverable.
        "reboot", "shtest", "stresscmd", "tree", "ver", "vol",
    ];

    for name in programs_nxe.iter().chain(tools_nxe) {
        let subdir = if programs_nxe.contains(name) { "Programs" } else { "System/Tools" };
        let p = root.join("userbin").join(format!("{}.nxe", name));
        if p.exists() {
            let content = std::fs::read(&p)?;
            files.push(FileEntry { name: format!("/{}/{}.nxe", subdir, name), content, mode: MODE_FILE | default_perms(&format!("{}.NXE", name)), is_dir: false });
        }
    }

    let nxl_map = [("libneodos.nxl", "fs.nxl"), ("libmath.nxl", "math.nxl"), ("console.nxl", "console.nxl"), ("net.nxl", "net.nxl")];
    for (src_name, dst_name) in &nxl_map {
        let p = root.join(src_name);
        if p.exists() {
            let content = std::fs::read(&p)?;
            files.push(FileEntry { name: format!("/System/Libraries/{}", dst_name), content, mode: MODE_FILE | PERM_R | PERM_X, is_dir: false });
        }
    }

    let kbd_layouts = &["US", "Spanish"];
    for layout_name in kbd_layouts {
        let p = root.join("data/keyboard").join(format!("{}.kbd", layout_name));
        if p.exists() {
            let content = std::fs::read(&p).unwrap_or_default();
            files.push(FileEntry { name: format!("/System/Keyboard/{}.kbd", layout_name), content, mode: MODE_FILE | PERM_R, is_dir: false });
        }
    }

    let locale_dir = root.join("data/locale");
    if locale_dir.exists() {
        if let Ok(lang_entries) = std::fs::read_dir(&locale_dir) {
            for lang_entry in lang_entries.flatten() {
                let lang_path = lang_entry.path();
                if !lang_path.is_dir() { continue; }
                let lang_name = match lang_path.file_name().and_then(|n| n.to_str()) { Some(n) => n, None => continue };

                // Collect this language's NLTs (sorted) so a leaf overflow is
                // deterministic and rebalanceable.
                let mut nlts: Vec<(String, Vec<u8>)> = Vec::new();
                if let Ok(nlt_entries) = std::fs::read_dir(&lang_path) {
                    for nlt_entry in nlt_entries.flatten() {
                        let p = nlt_entry.path();
                        if p.extension().and_then(|e| e.to_str()) != Some("nlt") { continue; }
                        let content = match std::fs::read(&p) { Ok(c) => c, Err(_) => continue };
                        let fname = match p.file_name().and_then(|n| n.to_str()) { Some(n) => n.to_string(), None => continue };
                        nlts.push((fname, content));
                    }
                }
                nlts.sort_by(|a, b| a.0.cmp(&b.0));

                // A single NE2 leaf holds ~28 NLTs, but a language has ~47.  The
                // i18n loader already consults `{lang}`, then `{lang-only}`, then
                // `en-US`, so distribute the sorted NLTs across the two dirs that
                // language resolves against.  No format or i18n change required,
                // and no NLT is dropped.
                let mut loc_dirs: Vec<String> = vec![lang_name.to_string()];
                if let Some(dash) = lang_name.find('-') {
                    loc_dirs.push(lang_name[..dash].to_string());
                }

                let mut bucket: Vec<(String, Vec<u8>)> = Vec::new();
                let mut bucket_bytes = 8usize; // leaf header
                let mut dir_idx = 0usize;
                for (fname, content) in nlts {
                    let entry_size = 4 + fname.len() + DIRENTRY_SIZE;
                    if bucket_bytes + entry_size > BLOCK_SIZE {
                        push_locale_bucket(&mut files, &loc_dirs, dir_idx, &mut bucket);
                        dir_idx += 1;
                        if dir_idx >= loc_dirs.len() {
                            anyhow::bail!(
                                "NE2 locale '{}' needs more than {} leaves ({} NLTs). \
                                 Multi-block directory support is not implemented.",
                                lang_name, loc_dirs.len(),
                                std::fs::read_dir(&lang_path).map(|d| d.count()).unwrap_or(0)
                            );
                        }
                        bucket_bytes = 8;
                    }
                    bucket_bytes += entry_size;
                    bucket.push((fname, content));
                }
                push_locale_bucket(&mut files, &loc_dirs, dir_idx, &mut bucket);
            }
        }
    }

    let nem_dir = format!("/tmp/nem_drivers_{}", std::process::id());
    let boot_drivers = &["ps2kbd", "ps2mouse", "rtc", "serial"];
    let sys_drivers = &["acpi", "pci", "ata", "ahci", "e1000", "virtio-blk"];
    for nem_name in boot_drivers.iter().chain(sys_drivers) {
        let cat = if boot_drivers.contains(nem_name) { "BOOT" } else { "SYSTEM" };
        let p = Path::new(&nem_dir).join(cat).join(format!("{}.nem", nem_name));
        let fallback = root.join("data").join("nem_bin").join(cat).join(format!("{}.nem", nem_name));
        let path = if p.exists() { &p } else { &fallback };
        if path.exists() {
            let content = std::fs::read(path)?;
            files.push(FileEntry { name: format!("/System/Drivers/{}.nem", nem_name), content, mode: MODE_FILE | PERM_R, is_dir: false });
        }
    }

    Ok(files)
}

pub fn create_esp_image(cfg: &Config) -> Result<std::path::PathBuf> {
    let start = Instant::now();
    println!("{} Creating ESP partition image (FAT32)...", "[*]".bold().cyan());
    let esp_image = cfg.neodos_root.join("tmp_esp.img");
    let esp_size = cfg.esp_size_mb;

    Command::new("dd")
        .args(["if=/dev/zero", &format!("of={}", esp_image.display()), "bs=1M", &format!("count={}", esp_size)])
        .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
        .status().context("Failed to create ESP image with dd")?;

    Command::new("mkfs.fat")
        .args(["-F", "32", &esp_image.to_string_lossy()])
        .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
        .status().context("mkfs.fat not found (install dosfstools)")?;

    if which("mmd").is_some() {
        for dir in &["/EFI", "/EFI/BOOT", "/EFI/NeoDOS"] {
            let _ = Command::new("mmd").args(["-i", &esp_image.to_string_lossy(), dir])
                .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status();
        }

        let bootloader_src = cfg.neodos_root.join("bootloader.efi");
        let kernel_src = cfg.neodos_root.join("kernel.elf");

        if bootloader_src.exists() {
            let _ = Command::new("mcopy").args(["-i", &esp_image.to_string_lossy(), &bootloader_src.to_string_lossy(), "::/EFI/BOOT/BOOTX64.EFI"])
                .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status();
            let _ = Command::new("mcopy").args(["-i", &esp_image.to_string_lossy(), &bootloader_src.to_string_lossy(), "::/EFI/NeoDOS/bootloader.efi"])
                .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status();
        }
        if kernel_src.exists() {
            let _ = Command::new("mcopy").args(["-i", &esp_image.to_string_lossy(), &kernel_src.to_string_lossy(), "::/EFI/NeoDOS/kernel.elf"])
                .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status();
        }

        let fs_image = cfg.neodos_root.join("data").join("neodos_image.img");
        if fs_image.exists() {
            let _ = Command::new("mcopy").args(["-i", &esp_image.to_string_lossy(), &fs_image.to_string_lossy(), "::/EFI/NeoDOS/neodos.fs"])
                .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status();
        }
        println!("{} Files copied to ESP", "[✓]".bold().green());
    } else {
        eprintln!("  mtools not found; files not copied to ESP image");
        eprintln!("  Install: sudo apt install mtools");
    }

    println!("  Duration: {:.1}s", start.elapsed().as_secs_f64());
    Ok(esp_image)
}

pub fn create_gpt_image(cfg: &Config, esp_image: &Path, neodos_image: &Path, output: &Path) -> Result<()> {
    let start = Instant::now();
    println!("{} Creating unified GPT disk image...", "[*]".bold().cyan());

    let esp_data = std::fs::read(esp_image)?;
    let neodos_data = std::fs::read(neodos_image)?;

    let esp_mb = cfg.esp_size_mb.max((esp_data.len() as f64 / 1_048_576.0).ceil() as u64);
    let neodos_mb = cfg.neodos_size_mb.max((neodos_data.len() as f64 / 1_048_576.0).ceil() as u64);
    let total_mb = esp_mb + neodos_mb + cfg.gpt_padding_mb;

    {
        let f = std::fs::File::create(output)?;
        f.set_len(total_mb * 1024 * 1024)?;
    }

    let esp_start = 2048u64;
    let esp_size_sectors = esp_mb * 1024 * 1024 / SECTOR_SIZE as u64;
    let neodos_start = esp_start + esp_size_sectors;
    let neodos_size_sectors = neodos_mb * 1024 * 1024 / SECTOR_SIZE as u64;

    let sfdisk_input = format!(
        "label: gpt\nstart={}, size={}, type=C12A7328-F81F-11D2-BA4B-00A0C93EC93B\nstart={}, size={}, type=EBD0A0A2-B9E5-4433-87C0-68B6B72699C7\n",
        esp_start, esp_size_sectors, neodos_start, neodos_size_sectors
    );

    let mut result = Command::new("sfdisk")
        .arg(output).stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped())
        .spawn().context("sfdisk not found (install util-linux)")?;

    {
        use std::io::Write;
        result.stdin.take().context("sfdisk stdin not available")?.write_all(sfdisk_input.as_bytes())?;
    }

    let sfdisk_output = result.wait_with_output()?;
    if !sfdisk_output.status.success() {
        anyhow::bail!("sfdisk failed: {}", String::from_utf8_lossy(&sfdisk_output.stderr));
    }

    let esp_offset = (esp_start as usize) * SECTOR_SIZE;
    let neodos_offset = (neodos_start as usize) * SECTOR_SIZE;

    use std::io::{Seek, Write};
    let mut disk = std::fs::OpenOptions::new().write(true).open(output)?;
    disk.seek(std::io::SeekFrom::Start(esp_offset as u64))?;
    disk.write_all(&esp_data)?;
    disk.seek(std::io::SeekFrom::Start(neodos_offset as u64))?;
    disk.write_all(&neodos_data)?;

    println!("{} GPT disk image: {} ({:.0} MB)", "[✓]".bold().green(), output.display(), total_mb);
    println!("  Partition 1 (ESP):    LBA {} - {}", esp_start, esp_start + esp_size_sectors - 1);
    println!("  Partition 2 (NeoDOS): LBA {} - {}", neodos_start, neodos_start + neodos_size_sectors - 1);
    println!("  Duration: {:.1}s", start.elapsed().as_secs_f64());
    Ok(())
}

fn ensure_gen_hiv(cfg: &Config) -> Result<()> {
    let gen_hiv_path = cfg.neodos_root.join("tools").join("gen-hiv").join("target").join("release").join("gen-hiv");
    if !gen_hiv_path.exists() {
        println!("  {} Building gen-hiv...", "[*]".bold().cyan());
        let status = Command::new("cargo")
            .args(["build", "--release"])
            .current_dir(cfg.neodos_root.join("tools").join("gen-hiv"))
            .status()
            .context("Failed to build gen-hiv")?;
        if !status.success() {
            anyhow::bail!("gen-hiv build failed");
        }
    }
    Ok(())
}

pub fn generate_registry_hive(cfg: &Config, enable_tests: bool, enable_network_test: bool) -> Result<()> {
    println!("{} Generating SYSTEM.HIV registry hive...", "[*]".bold().cyan());
    ensure_gen_hiv(cfg)?;
    let gen_hiv = cfg.neodos_root.join("tools").join("gen-hiv").join("target").join("release").join("gen-hiv");
    let output = cfg.neodos_root.join("data").join("system.hiv");

    let mut cmd = Command::new(&gen_hiv);
    cmd.arg(&output);
    if enable_tests { cmd.arg("--enable-tests"); }
    if enable_network_test { cmd.arg("--enable-network-test"); }
    let status = cmd.status().context("Failed to run gen-hiv")?;

    if !status.success() { anyhow::bail!("Registry hive generation failed"); }
    println!("{} SYSTEM.HIV: {}", "[✓]".bold().green(), output.display());
    Ok(())
}

pub fn generate_test_hive(cfg: &Config, enable_network_test: bool) -> Result<PathBuf> {
    ensure_gen_hiv(cfg)?;
    let gen_hiv = cfg.neodos_root.join("tools").join("gen-hiv").join("target").join("release").join("gen-hiv");
    let orig = cfg.neodos_root.join("data").join("system.hiv");
    let backup = cfg.neodos_root.join("data").join("system.hiv.bak");

    if orig.exists() { std::fs::copy(&orig, &backup)?; }

    let mut cmd = Command::new(&gen_hiv);
    cmd.arg(&orig).arg("--enable-tests");
    if enable_network_test { cmd.arg("--enable-network-test"); }

    let status = cmd.status().context("Failed to run gen-hiv for test hive")?;
    if !status.success() { anyhow::bail!("Test registry hive generation failed"); }
    println!("{} Test SYSTEM.HIV: {} {}", "[✓]".bold().green(), orig.display(),
        if enable_network_test { "(with network test enabled)" } else { "" });
    Ok(backup)
}

pub fn restore_hive(cfg: &Config) -> Result<()> {
    let orig = cfg.neodos_root.join("data").join("system.hiv");
    let backup = cfg.neodos_root.join("data").join("system.hiv.bak");
    if backup.exists() {
        std::fs::copy(&backup, &orig)?;
        let _ = std::fs::remove_file(&backup);
    }
    Ok(())
}

fn which(cmd: &str) -> Option<std::path::PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        for dir in std::env::split_paths(&paths) {
            let full = dir.join(cmd);
            if full.is_file() { return Some(full); }
        }
        None
    })
}

fn fmt_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut s = size as f64;
    for unit in UNITS {
        if s < 1024.0 { return format!("{:.1} {}", s, unit); }
        s /= 1024.0;
    }
    format!("{:.2} GB", s)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEAF_HEADER: usize = 8; // u16 type + u16 count + u32 crc

    fn entry(key: &str) -> (Vec<u8>, Vec<u8>) {
        (key.as_bytes().to_vec(), vec![0u8; DIRENTRY_SIZE])
    }

    /// Walk the serialized entries of a leaf and return their keys.
    /// Panics if any field runs past the block boundary.
    fn parse_leaf_keys(leaf: &[u8]) -> Vec<Vec<u8>> {
        let count = u16::from_le_bytes([leaf[2], leaf[3]]) as usize;
        let mut off = LEAF_HEADER;
        let mut keys = Vec::new();
        for _ in 0..count {
            assert!(off + 2 <= BLOCK_SIZE, "key_len out of bounds");
            let kl = u16::from_le_bytes([leaf[off], leaf[off + 1]]) as usize;
            off += 2;
            assert!(off + kl <= BLOCK_SIZE, "key out of bounds");
            let key = leaf[off..off + kl].to_vec();
            off += kl;
            assert!(off + 2 <= BLOCK_SIZE, "val_len out of bounds");
            let vl = u16::from_le_bytes([leaf[off], leaf[off + 1]]) as usize;
            off += 2;
            assert!(off + vl <= BLOCK_SIZE, "value out of bounds");
            off += vl;
            keys.push(key);
        }
        keys
    }

    #[test]
    fn leaf_exactly_at_capacity_declares_serialized_count() {
        // Fixed 8-byte keys + 128-byte values => 140 bytes per entry.
        let per_entry = 4 + 8 + DIRENTRY_SIZE;
        let capacity = (BLOCK_SIZE - LEAF_HEADER) / per_entry;
        let entries: Vec<_> = (0..capacity)
            .map(|i| entry(&format!("n{:07}", i)))
            .collect();
        let leaf = make_btree_leaf("/Programs", &entries).expect("leaf at capacity must fit");
        let declared = u16::from_le_bytes([leaf[2], leaf[3]]) as usize;
        assert_eq!(declared, capacity);
        assert_eq!(parse_leaf_keys(&leaf).len(), capacity);
    }

    #[test]
    fn leaf_over_capacity_fails_explicitly() {
        let per_entry = 4 + 8 + DIRENTRY_SIZE;
        let capacity = (BLOCK_SIZE - LEAF_HEADER) / per_entry;
        let entries: Vec<_> = (0..capacity + 1)
            .map(|i| entry(&format!("n{:07}", i)))
            .collect();
        let err = make_btree_leaf("/Programs", &entries).unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("/Programs"), "error should name the directory: {msg}");
        assert!(msg.contains("overflow"), "error should mention overflow: {msg}");
    }

    #[test]
    fn leaf_never_declares_more_than_serialized() {
        // Mixed key sizes: every accepted leaf must be internally consistent.
        let entries: Vec<_> = (0..25)
            .map(|i| entry(&format!("file{:02}.nxe", i)))
            .collect();
        let leaf = make_btree_leaf("/Mixed", &entries).expect("25 small entries fit");
        let declared = u16::from_le_bytes([leaf[2], leaf[3]]) as usize;
        let parsed = parse_leaf_keys(&leaf);
        assert_eq!(declared, parsed.len());
        assert_eq!(declared, entries.len());
    }

    #[test]
    fn plan_freelist_empty() {
        let (nodes, regions) = plan_freelist(Vec::new(), 1).unwrap();
        assert!(nodes.is_empty());
        assert!(regions.is_empty());
    }

    #[test]
    fn plan_freelist_single_node_is_unchanged() {
        let (nodes, regions) = plan_freelist(vec![(560, 25040)], 1).unwrap();
        assert_eq!(nodes, vec![1]);
        assert_eq!(regions, vec![(560, 25040)]);
    }

    #[test]
    fn plan_freelist_chains_when_more_than_one_node_needed() {
        let mut regions = Vec::new();
        let mut b = 2u64;
        for _ in 0..400 {
            regions.push((b, 1));
            b += 2;
        }
        let (nodes, adjusted) = plan_freelist(regions, 1).unwrap();
        assert_eq!(nodes, vec![1, 2]);
        assert_eq!(adjusted[0], (4, 1));
        assert_eq!(adjusted.len(), 399);
    }

    #[test]
    fn plan_freelist_fails_without_room_for_chain() {
        let mut regions = Vec::new();
        let mut b = 2u64;
        for _ in 0..400 {
            regions.push((b, 0));
            b += 2;
        }
        assert!(plan_freelist(regions, 1).is_err());
    }

    #[test]
    fn dir_nodes_single_leaf() {
        let entries: Vec<(Vec<u8>, Vec<u8>)> = (0..5)
            .map(|i| (format!("f{}", i).into_bytes(), vec![0u8; 128]))
            .collect();
        assert_eq!(leaf_chunks_of(&entries).len(), 1);
        let nodes = build_dir_nodes(&entries, &[10]).unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(u16::from_le_bytes([nodes[0][0], nodes[0][1]]), 1); // leaf
        assert_eq!(parse_leaf_keys(&nodes[0]).len(), 5);
    }

    #[test]
    fn dir_nodes_multi_leaf() {
        // 60 entries x ~140 B > 4 KB leaf -> multiple leaves + internal node.
        let entries: Vec<(Vec<u8>, Vec<u8>)> = (0..60)
            .map(|i| (format!("f{:03}", i).into_bytes(), vec![0u8; 128]))
            .collect();
        let chunks = leaf_chunks_of(&entries);
        assert!(chunks.len() >= 2, "expected multiple leaves, got {}", chunks.len());
        let lbas: Vec<u64> = (10..10 + chunks.len() as u64 + 1).collect();
        let nodes = build_dir_nodes(&entries, &lbas).unwrap();
        assert_eq!(nodes.len(), chunks.len() + 1);
        // Internal node is last and has one entry per leaf.
        let internal = &nodes[nodes.len() - 1];
        assert_eq!(u16::from_le_bytes([internal[0], internal[1]]), 0); // internal
        let ikeys = parse_leaf_keys(internal);
        assert_eq!(ikeys.len(), chunks.len());
        assert!(ikeys[0].is_empty());
        // Leaves hold all 60 keys in order.
        let mut all = Vec::new();
        for l in 0..chunks.len() { all.extend(parse_leaf_keys(&nodes[l])); }
        assert_eq!(all.len(), 60);
        let mut sorted = all.clone();
        sorted.sort();
        assert_eq!(all, sorted);
    }
}

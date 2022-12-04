use std::{
    collections::HashMap,
    ffi::CString,
    fs::{create_dir_all, File},
    io::{BufRead, Cursor},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
};

use crc::{Crc, CRC_32_CKSUM};
use md5::{Digest, Md5};

const HEADER_LENGTH: usize = 28;
const CRC32: Crc<u32> = Crc::<u32>::new(&CRC_32_CKSUM);

pub struct VPK {
    path: PathBuf,
    data: Cursor<Vec<u8>>,
    header: Option<VPKHeader>,
    index: HashMap<String, VPKMetadata>,
    files: HashMap<String, Vec<u8>>,
}

struct VPKHeader {
    signature: u32,
    version: u32,
    tree_length: u32,
    embed_chunk_length: u32,
    chunk_hashes_length: u32,
    self_hashes_length: u32,
    signature_length: u32,
}

struct VPKMetadata {
    preload: Vec<u8>,
    crc32: u32,
    preload_length: u16,
    archive_index: u16,
    archive_offset: u32,
    file_length: u32,
    suffix: u16,
}

impl VPKMetadata {
    fn validate(&mut self, header: &VPKHeader) {
        if self.suffix != 65535 {
            panic!("Error while parsing index.");
        }
        if self.archive_index == 32767 {
            self.archive_offset += HEADER_LENGTH as u32 + header.tree_length;
        }
    }
}

impl VPK {
    fn new(path: PathBuf) -> VPK {
        // Open file and load bytes
        let mut f = File::open(&path).unwrap();
        let mut vpk_data = Vec::new();
        f.read_to_end(&mut vpk_data).unwrap();
        let vpk_cursor = Cursor::new(vpk_data);

        VPK {
            path,
            header: None,
            index: HashMap::new(),
            data: vpk_cursor,
            files: HashMap::new(),
        }
    }

    fn read(&mut self) {
        self.read_header();
        self.populate_index();
        self.load_file_data();
    }

    fn read_header(&mut self) {
        let mut header = [b'0'; HEADER_LENGTH];
        self.data.read_exact(&mut header).unwrap();
        let header_data: Vec<u32> = header
            .chunks_exact(4)
            .map(|chunk| u32::from_le_bytes(chunk.try_into().unwrap()))
            .collect();

        let signature = header_data[0];
        if signature != 0x55aa1234 {
            panic!("Invalid VPK");
        }

        self.header = Some(VPKHeader {
            signature,
            version: header_data[1],
            tree_length: header_data[2],
            embed_chunk_length: header_data[3],
            chunk_hashes_length: header_data[4],
            self_hashes_length: header_data[5],
            signature_length: header_data[6],
        });
    }

    fn populate_index(&mut self) {
        self.data.set_position(HEADER_LENGTH.try_into().unwrap());
        let header = self.header.as_ref().unwrap();

        loop {
            if header.version > 0
                && self.data.position() > (header.tree_length + HEADER_LENGTH as u32).into()
            {
                panic!("Error parsing index.");
            }
            let mut cstr = Vec::new();
            self.data.read_until(b'\x00', &mut cstr).unwrap();
            let Ok(ext) = CString::from_vec_with_nul(cstr) else {return};
            if &ext.to_str().unwrap() == &"" {
                break;
            };

            loop {
                let mut cstr = Vec::new();
                self.data.read_until(b'\x00', &mut cstr).unwrap();
                let Ok(mut path) = CString::from_vec_with_nul(cstr) else {return};
                if &path.to_str().unwrap() == &"" {
                    break;
                };

                if path.to_str().unwrap() != " " {
                    path = CString::new(path.to_str().unwrap().to_owned() + "/").unwrap();
                } else {
                    path = CString::new("").unwrap();
                }
                loop {
                    let mut cstr = Vec::new();
                    self.data.read_until(b'\x00', &mut cstr).unwrap();
                    let Ok(name) = CString::from_vec_with_nul(cstr) else {return};
                    if &name.to_str().unwrap() == &"" {
                        break;
                    };

                    let mut metadata = [b'0'; 18];
                    self.data.read_exact(&mut metadata).unwrap();

                    let path = path.to_str().unwrap().to_owned()
                        + name.to_str().unwrap()
                        + "."
                        + ext.to_str().unwrap();

                    let preload_length = u16::from_le_bytes(metadata[4..6].try_into().unwrap());
                    let mut preload = vec![b'0'; preload_length.into()];
                    self.data.read_exact(&mut preload).unwrap();

                    let mut meta = VPKMetadata {
                        preload,
                        crc32: u32::from_le_bytes(metadata[0..4].try_into().unwrap()),
                        preload_length,
                        archive_index: u16::from_le_bytes(metadata[6..8].try_into().unwrap()),
                        archive_offset: u32::from_le_bytes(metadata[8..12].try_into().unwrap()),
                        file_length: u32::from_le_bytes(metadata[12..16].try_into().unwrap()),
                        suffix: u16::from_le_bytes(metadata[16..18].try_into().unwrap()),
                    };

                    meta.validate(header);
                    self.index.insert(path, meta);
                }
            }
        }
    }

    fn load_file_data(&mut self) {
        for (path, metadata) in &self.index {
            let file_length = metadata.file_length + u32::from(metadata.preload_length);

            self.data.set_position(metadata.archive_offset.into());
            let mut file_data = vec![b'0'; file_length.try_into().unwrap()];
            self.data.read_exact(&mut file_data).unwrap();
            self.files.insert(path.to_string(), file_data);
        }
    }

    fn save_file_data(&self) {
        for (path, file_data) in &self.files {
            let fpath = Path::new(path);
            let fparent = fpath.parent().unwrap();
            create_dir_all(fparent).unwrap();

            std::fs::write(fpath, file_data).unwrap();
        }
    }
}

fn create_vpk(vpk_data: HashMap<String, Vec<u8>>) -> Vec<u8> {
    let mut tree: HashMap<String, HashMap<String, Vec<String>>> = HashMap::new();

    // Create Tree using File List
    for file in vpk_data.keys() {
        let fpath = Path::new(&file);
        let ext = fpath.extension().unwrap().to_str().unwrap().to_owned();
        let dir = fpath.parent().unwrap().to_str().unwrap().to_owned();
        let name = fpath.file_stem().unwrap().to_str().unwrap().to_owned();

        if tree.contains_key(&ext) {
            if tree.get(&ext).unwrap().contains_key(&dir) {
                tree.get_mut(&ext)
                    .unwrap()
                    .get_mut(&dir)
                    .unwrap()
                    .push(name);
            } else {
                tree.get_mut(&ext).unwrap().insert(dir, vec![name]);
            }
        } else {
            tree.insert(ext, HashMap::from([(dir, vec![name])]));
        }
    }

    // Calculate Tree Length
    let mut tree_length: u32 = 1;
    for ext in tree.keys() {
        tree_length += ext.len() as u32 + 2_u32;

        for dir in tree.get(ext).unwrap().keys() {
            tree_length += dir.len() as u32 + 2_u32;

            for file in tree.get(ext).unwrap().get(dir).unwrap() {
                tree_length += file.len() as u32 + 19_u32;
            }
        }
    }

    // Create File Structure
    let mut tree_cursor = Cursor::new(Vec::new());
    let mut data_offset: u32 = tree_length + HEADER_LENGTH as u32; // HEADER_LENGTH + tree_length
    let mut data_cursor = Cursor::new(Vec::new());
    let mut embed_chunk_length: u32 = 0;

    for (ext, dir) in tree {
        tree_cursor.write(format!("{ext}\0").as_bytes()).unwrap();

        for (dirname, files) in dir {
            tree_cursor
                .write(format!("{dirname}\0").as_bytes())
                .unwrap();

            for file in files {
                tree_cursor.write(format!("{file}\0").as_bytes()).unwrap();

                // Write Metadata
                let file_offset = data_offset;
                let filename = if ext.len() > 0 {
                    format!("{file}.{ext}")
                } else {
                    file
                };

                let filedata = vpk_data.get(&format!("{dirname}/{filename}")).unwrap();
                let file_length = filedata.len() as u32;
                let mut data_hash = CRC32.digest();
                data_hash.update(filedata);

                tree_cursor
                    .write(&data_hash.finalize().to_le_bytes())
                    .unwrap(); // crc32
                tree_cursor.write(&0_u16.to_le_bytes()).unwrap(); // preload_length
                tree_cursor.write(&32767_u16.to_le_bytes()).unwrap(); // archive_index
                let archive_offset: u32 = file_offset - tree_length - HEADER_LENGTH as u32;
                tree_cursor.write(&archive_offset.to_le_bytes()).unwrap(); // archive_offset
                tree_cursor.write(&file_length.to_le_bytes()).unwrap(); // file_length
                tree_cursor.write(&65535_u16.to_le_bytes()).unwrap();

                embed_chunk_length += file_length;
                data_offset += file_length;
                data_cursor.write(filedata).unwrap();
            }
            // Next dir
            tree_cursor.write("\0".as_bytes()).unwrap();
        }
        // Next ext
        tree_cursor.write("\0".as_bytes()).unwrap();
    }
    // End of tree
    tree_cursor.write("\0".as_bytes()).unwrap();

    // Create Header
    let mut header_cursor = Cursor::new(Vec::new());
    header_cursor.write(&0x55aa1234_u32.to_le_bytes()).unwrap(); // signature
    header_cursor.write(&2_u32.to_le_bytes()).unwrap(); // version
    header_cursor.write(&tree_length.to_le_bytes()).unwrap(); // tree_length
    header_cursor
        .write(&embed_chunk_length.to_le_bytes())
        .unwrap(); // embed_chunk_length
    header_cursor.write(&0_u32.to_le_bytes()).unwrap(); // chunk_hashes_length
    header_cursor.write(&48_u32.to_le_bytes()).unwrap(); // self_hashes_length
    header_cursor.write(&0_u32.to_le_bytes()).unwrap(); // signature_length

    // Calculate Hashes
    let mut tree_checksum = Md5::new();
    let mut file_checksum = Md5::new();
    let chunk_hashes_checksum = Md5::new();
    tree_checksum.update(tree_cursor.get_ref());
    file_checksum.update(header_cursor.get_ref());
    file_checksum.update(tree_cursor.get_ref());
    file_checksum.update(data_cursor.get_ref());
    let tree_digest = tree_checksum.finalize();
    let chunk_hashes_checksum_digest = chunk_hashes_checksum.finalize();
    file_checksum.update(&tree_digest);
    file_checksum.update(&chunk_hashes_checksum_digest);
    let mut hashes = tree_digest.to_vec();
    hashes.append(&mut chunk_hashes_checksum_digest.to_vec());
    hashes.append(&mut file_checksum.finalize().to_vec());

    // Combine Cursors to file
    let mut file = header_cursor.into_inner();
    file.append(&mut tree_cursor.into_inner());
    file.append(&mut data_cursor.into_inner());
    file.append(&mut hashes);

    file
}

fn patch_vpk(
    base: HashMap<String, Vec<u8>>,
    mut target: HashMap<String, Vec<u8>>,
) -> HashMap<String, Vec<u8>> {
    // Rename vmap_c in target to dota.vmap_c
    let mut target_vmap = String::new();
    for fpath in target.keys() {
        if fpath.ends_with(".vmap_c") {
            target_vmap.push_str(fpath);
            break;
        }
    }
    let vmap_data = target.remove(&target_vmap).unwrap();
    target.insert(
        Path::new(&target_vmap)
            .with_file_name("dota.vmap_c")
            .to_str()
            .unwrap()
            .to_string(),
        vmap_data,
    );

    // Add files from base to target
    for (fpath, data) in base {
        if !target.contains_key(&fpath) {
            target.insert(fpath, data);
        }
    }

    target
}

pub fn create_terrain(base_path: PathBuf, target_path: PathBuf) -> Vec<u8> {
    let (tx, rx) = mpsc::channel();
    let mut base_vpk = VPK::new(base_path);
    thread::spawn(move || {
        let mut target_vpk = VPK::new(target_path);
        target_vpk.read();
        tx.send(target_vpk).unwrap();
    });
    let target_vpk = rx.recv().unwrap();
    base_vpk.read();

    let out_data = patch_vpk(base_vpk.files, target_vpk.files);
    create_vpk(out_data)
}

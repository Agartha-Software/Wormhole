extern crate wormhole;
use serial_test::parallel;
use std::{
    collections::HashMap,
    time::{Duration, SystemTime},
};
use wormhole::pods::{
    arbo::{Arbo, FsEntry, Inode, Metadata, BLOCK_SIZE, ROOT},
    filesystem::fs_interface::SimpleFileType,
};

fn arbo_values(inode: &Inode, expected_result: Inode) {
    let tolerance = Duration::from_millis(100); // Tolérance de 100 ms

    // Check that the inode is correct
    assert_eq!(inode.id, expected_result.id);
    assert_eq!(inode.parent, expected_result.parent);
    assert_eq!(inode.name, expected_result.name);
    assert_eq!(inode.entry, expected_result.entry);

    // Check that the metadata is correct
    assert_eq!(inode.meta.ino, expected_result.meta.ino);
    assert_eq!(inode.meta.size, expected_result.meta.size);
    assert_eq!(inode.meta.blocks, expected_result.meta.blocks);
    assert_eq!(inode.meta.kind, expected_result.meta.kind);
    assert_eq!(inode.meta.perm, expected_result.meta.perm);
    assert_eq!(inode.meta.nlink, expected_result.meta.nlink);
    assert_eq!(inode.meta.uid, expected_result.meta.uid);
    assert_eq!(inode.meta.gid, expected_result.meta.gid);
    assert_eq!(inode.meta.rdev, expected_result.meta.rdev);
    assert_eq!(inode.meta.blksize, expected_result.meta.blksize);
    assert_eq!(inode.meta.flags, expected_result.meta.flags);

    // Check that the timestamps are correct
    let now = SystemTime::now();

    assert!(now.duration_since(inode.meta.atime).unwrap() < tolerance);
    assert!(now.duration_since(inode.meta.mtime).unwrap() < tolerance);
    assert!(now.duration_since(inode.meta.ctime).unwrap() < tolerance);
    assert!(now.duration_since(inode.meta.crtime).unwrap() < tolerance);
}

#[parallel]
#[test]
fn test_inserting_and_retreiving_files() {
    let mut arbo = Arbo::new();

    assert!(
        arbo.add_inode_from_parameters(
            "file1".to_owned(),
            10,
            ROOT,
            FsEntry::File(Vec::new()),
            0o777
        )
        .is_ok(),
        "can't add file1 in / folder"
    );
    assert!(
        arbo.add_inode_from_parameters(
            "file2".to_owned(),
            11,
            ROOT,
            FsEntry::File(Vec::new()),
            0o777
        )
        .is_ok(),
        "can't add file2 in / folder"
    );

    let result_one = Inode {
        id: 10,
        parent: 1,
        name: "file1".to_owned(),
        entry: FsEntry::File(Vec::new()),
        meta: Metadata {
            ino: 10,
            size: 0,
            blocks: 0,
            atime: SystemTime::now(),
            mtime: SystemTime::now(),
            ctime: SystemTime::now(),
            crtime: SystemTime::now(),
            kind: SimpleFileType::File,
            perm: 0o777,
            nlink: 0,
            uid: 0,
            gid: 0,
            rdev: 0,
            blksize: BLOCK_SIZE as u32,
            flags: 0,
        },
        xattrs: HashMap::new(),
    };

    let result_two = Inode {
        id: 11,
        parent: 1,
        name: "file2".to_owned(),
        entry: FsEntry::File(Vec::new()),
        meta: Metadata {
            ino: 11,
            size: 0,
            blocks: 0,
            atime: SystemTime::now(),
            mtime: SystemTime::now(),
            ctime: SystemTime::now(),
            crtime: SystemTime::now(),
            kind: SimpleFileType::File,
            perm: 0o777,
            nlink: 0,
            uid: 0,
            gid: 0,
            rdev: 0,
            blksize: BLOCK_SIZE as u32,
            flags: 0,
        },
        xattrs: HashMap::new(),
    };
    arbo_values(&arbo.get_inode(10).unwrap(), result_one);
    arbo_values(&arbo.get_inode(11).unwrap(), result_two);
}

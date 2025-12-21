#[cfg(target_os = "linux")]
use std::os::unix::fs::PermissionsExt;

use assert_fs::{
    assert::PathAssert,
    prelude::{PathChild, PathCreateDir},
};
use wormhole::pods::disk_managers::{unix_disk_manager::UnixDiskManager, DiskManager};

pub fn test_generic_disk<D: DiskManager, A: PathAssert + PathChild + AsRef<std::path::Path>>(
    disk: &D,
    temp_dir: &A,
) {
    // NEW
    {
        disk.new_file(&"file".into(), 0o644).expect("new_file");
        temp_dir.child("file").assert(predicates::path::is_file());

        disk.new_dir(&"folder".into(), 0o775).expect("new_dir");
        temp_dir.child("folder").assert(predicates::path::is_dir());
    }

    // EXISTS
    {
        assert!(disk.file_exists(&"file".into()), "file exists");
    }

    // REMOVE
    {
        disk.new_file(&"file2".into(), 0o644).expect("new_file");
        disk.remove_file(&"file2".into()).expect("remove_file");
        temp_dir.child("file2").assert(predicates::path::missing());

        disk.new_dir(&"dir2".into(), 0o755).expect("new_dir");
        disk.remove_dir(&"dir2".into()).expect("remove_dir");
        temp_dir.child("dir2").assert(predicates::path::missing());

        disk.new_dir(&"dir2".into(), 0o755).expect("new_dir");
        temp_dir.child("dir2").assert(predicates::path::is_dir());
    }

    // WRITE

    let contents = b"lorem ipsum\nGogi\x01to Ergo Sum";
    disk.write_file(&"file".into(), contents, 0)
        .expect("write_file");

    assert_eq!(
        std::fs::read(temp_dir.child(&"file").path())
            .expect("reading file")
            .as_slice(),
        contents,
        "contents written correctly"
    );

    // READ
    {
        let mut buf = [0; 28];
        let len = disk
            .read_file(&"file".into(), 8, &mut buf)
            .expect("read_file");

        assert_eq!(
            &buf[..len],
            &contents[8..(len + 8)],
            "contents read correctly"
        );
    }

    // set_file_size
    {
        disk.set_file_size(&"file".into(), 19)
            .expect("set_file_size");

        assert_eq!(
            std::fs::read(temp_dir.child(&"file").path())
                .expect("reading file")
                .as_slice(),
            &contents[..19],
            "contents resized correctly"
        );

        disk.new_file(&"file2".into(), 0o644).expect("new_file");

        disk.set_file_size(&"file2".into(), 256)
            .expect("set_file_size");

        assert_eq!(
            std::fs::read(temp_dir.child(&"file2").path())
                .expect("reading file")
                .as_slice(),
            &[0; 256],
            "expanded is 0-initialized"
        );

        disk.remove_file(&"file2".into()).expect("remove_file");
    }

    // MV
    {
        assert!(
            disk.mv_file(&"folder".into(), &"".into()).is_err(),
            "moving to root is invalid but doesn't break anything"
        );

        disk.mv_file(&"file".into(), &"folder/file".into())
            .expect("mv_file");

        assert_eq!(
            std::fs::read(temp_dir.child(&"folder").child(&"file").path())
                .expect("reading file")
                .as_slice(),
            &contents[..19],
            "contents remain after move"
        );

        disk.mv_file(&"folder".into(), &"directory".into())
            .expect("mv_file");

        assert_eq!(
            std::fs::read(temp_dir.child(&"directory").child(&"file").path())
                .expect("reading file")
                .as_slice(),
            &contents[..19],
            "contents remain after move"
        );
    }

    // PERMISSIONS
    #[cfg(target_os = "linux")]
    {
        disk.set_permisions(&"".into(), 0o444)
            .expect("set_permission");

        let p = temp_dir
            .as_ref()
            .metadata()
            .expect("metadata")
            .permissions();
        assert_eq!(p.mode() & 0o777, 0o444, "root permission set correctly");

        disk.set_permisions(&"".into(), 0o775)
            .expect("set_permission");

        disk.set_permisions(&"directory/file".into(), 0o666)
            .expect("set_permission");

        let p = temp_dir
            .child("directory")
            .child("file")
            .metadata()
            .expect("metadata")
            .permissions();

        assert_eq!(p.mode() & 0o777, 0o666, "root permission set correctly");
    }
}

#[test]
pub fn test_unix_disk() {
    let temp_dir = assert_fs::TempDir::new().expect("creating temp dir");
    let disk = UnixDiskManager::new(&temp_dir.path().into()).expect("creating disk manager");

    test_generic_disk(&disk, &temp_dir);
}

#[test]
#[cfg(target_os = "windows")]
pub fn test_unix_disk() {
    let temp_dir = assert_fs::TempDir::new().expect("creating temp dir");

    let mountpoint = temp_dir.child("wormhole");
    mountpoint.create_dir_all();
    let disk = UnixDiskManager::new(&mountpoint.path().into()).expect("creating disk manager");
    let temp_dir = temp_dir.child(".wormhole");

    test_generic_disk(&disk, &temp_dir);
}

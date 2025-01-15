extern crate wormhole;
use wormhole::pods::arbo::{Arbo, FsEntry, Inode, ROOT};

#[test]
fn test_inserting_and_retreiving_files() {
    let mut arbo = Arbo::new();

    assert!(
        arbo.add_inode_from_parameters("file1".to_owned(), 1, ROOT, FsEntry::File(Vec::new()),)
            .is_ok(),
        "can't add file1 in / folder"
    );
    assert!(
        arbo.add_inode_from_parameters("file2".to_owned(), 2, ROOT, FsEntry::File(Vec::new()),)
            .is_ok(),
        "can't add file2 in / folder"
    );

    assert_eq!(
        Ok((Inode {
            parent: 0,
            name: "file1".to_owned(),
            entry: FsEntry::File(Vec::new())
        })),
        arbo.get_inode(1)
    );

    assert_eq!(
        Ok((Inode {
            parent: 0,
            name: "file1".to_owned(),
            entry: FsEntry::File(Vec::new())
        })),
        arbo.get_inode(2)
    );
}

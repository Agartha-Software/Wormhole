use std::{io, path::Path};

use libp2p::PeerId;

use crate::{
    config::GlobalConfig,
    pods::{
        disk_managers::DiskManager,
        itree::{
            index_folder_recursive, FsEntry, ITree, Ino,
            GLOBAL_CONFIG_FNAME, GLOBAL_CONFIG_INO, ROOT,
        },
        whpath::WhPath,
    },
};

pub fn generate_itree(mountpoint: &Path, host: &PeerId) -> io::Result<ITree> {
    let mut itree = ITree::new();

    index_folder_recursive(&mut itree, ROOT, mountpoint, host, mountpoint)?;
    Ok(itree)
}

pub fn initiate_itree(
    itree: &ITree,
    global: &GlobalConfig,
    disk: &dyn DiskManager,
) -> io::Result<()> {
    create_all_shared(itree, ROOT, disk)?;
    apply_global_config_to_file(itree, global, disk)
}

fn apply_global_config_to_file(
    itree: &ITree,
    global: &GlobalConfig,
    disk: &dyn DiskManager,
) -> io::Result<()> {
    if let Ok(perms) = itree
        .get_inode(GLOBAL_CONFIG_INO)
        .map(|inode| inode.meta.perm)
    {
        let _ = disk.new_file(&WhPath::try_from(GLOBAL_CONFIG_FNAME).unwrap(), perms);
        disk.write_file(
            &WhPath::try_from(GLOBAL_CONFIG_FNAME).unwrap(),
            toml::to_string(global).expect("infallible").as_bytes(),
            0,
        )?;
    }
    Ok(())
}

/// Create all directories and symlinks present in ITree. (not the files)
///
/// Required at setup to resolve issue #179
/// (files pulling need the parent folder to be already present)
fn create_all_shared(itree: &ITree, from: Ino, disk: &dyn DiskManager) -> io::Result<()> {
    let from = itree.get_inode(from).map_err(|e| e.into_io())?;

    match &from.entry {
        FsEntry::File(_) => Ok(()),
        FsEntry::Symlink(symlink) => {
            let current_path = itree
                .get_path_from_inode_id(from.id)
                .map_err(|e| e.into_io())?;
            disk.new_symlink(&current_path, from.meta.perm, symlink)
                .or_else(|e| {
                    if e.kind() == io::ErrorKind::AlreadyExists {
                        Ok(())
                    } else {
                        Err(e)
                    }
                })
        }
        FsEntry::Directory(children) => {
            let current_path = itree
                .get_path_from_inode_id(from.id)
                .map_err(|e| e.into_io())?;

            // skipping root folder
            if current_path != WhPath::root() {
                disk.new_dir(&current_path, from.meta.perm).or_else(|e| {
                    if e.kind() == io::ErrorKind::AlreadyExists {
                        Ok(())
                    } else {
                        Err(e)
                    }
                })?;
            }

            for child in children {
                create_all_shared(itree, *child, disk)?
            }
            Ok(())
        }
    }
}

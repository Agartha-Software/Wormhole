# XFSTests Test Catalog (Group 'quick')

This document lists all tests included in the `quick` group of xfstests, sorted by main category.
Generated on Fri Jan 23 14:35:04 UTC 2026

## Category Summary

- [XFSTests Test Catalog (Group 'quick')](#xfstests-test-catalog-group-quick)
  - [Category Summary](#category-summary)
  - [Category: acl ](#category-acl-)
  - [Category: aio ](#category-aio-)
  - [Category: atime ](#category-atime-)
  - [Category: attr ](#category-attr-)
  - [Category: bigtime ](#category-bigtime-)
  - [Category: cap ](#category-cap-)
  - [Category: casefold ](#category-casefold-)
  - [Category: clone ](#category-clone-)
  - [Category: copy\_range ](#category-copy_range-)
  - [Category: data ](#category-data-)
  - [Category: dax ](#category-dax-)
  - [Category: dedupe ](#category-dedupe-)
  - [Category: dir ](#category-dir-)
  - [Category: eio ](#category-eio-)
  - [Category: encrypt ](#category-encrypt-)
  - [Category: enospc ](#category-enospc-)
  - [Category: exportfs ](#category-exportfs-)
  - [Category: fiemap ](#category-fiemap-)
  - [Category: fiexchange ](#category-fiexchange-)
  - [Category: freeze ](#category-freeze-)
  - [Category: fsr ](#category-fsr-)
  - [Category: idmapped ](#category-idmapped-)
  - [Category: insert ](#category-insert-)
  - [Category: io\_uring ](#category-io_uring-)
  - [Category: ioctl ](#category-ioctl-)
  - [Category: locks ](#category-locks-)
  - [Category: log ](#category-log-)
  - [Category: metadata ](#category-metadata-)
  - [Category: misc ](#category-misc-)
  - [Category: mkfs ](#category-mkfs-)
  - [Category: mmap ](#category-mmap-)
  - [Category: mount ](#category-mount-)
  - [Category: other ](#category-other-)
  - [Category: pattern ](#category-pattern-)
  - [Category: perms ](#category-perms-)
  - [Category: prealloc ](#category-prealloc-)
  - [Category: punch ](#category-punch-)
  - [Category: quota ](#category-quota-)
  - [Category: recoveryloop ](#category-recoveryloop-)
  - [Category: remount ](#category-remount-)
  - [Category: rename ](#category-rename-)
  - [Category: rw ](#category-rw-)
  - [Category: seek ](#category-seek-)
  - [Category: shutdown ](#category-shutdown-)
  - [Category: swap ](#category-swap-)
  - [Category: swapext ](#category-swapext-)
  - [Category: trim ](#category-trim-)
  - [Category: unlink ](#category-unlink-)
  - [Category: unshare ](#category-unshare-)
  - [Category: verity ](#category-verity-)
  - [Category: volume ](#category-volume-)
  - [Category: zone ](#category-zone-)

## Category: acl <a name="category-acl"></a>

| Test ID | Description |
|---|---|
| **generic/026** | Test out ACL count limits |
| **generic/053** | xfs_repair breaks acls |
| **generic/099** | Test out ACLs. |
| **generic/105** | Test fix of bug: |
| **generic/237** | Check user B can setfacl a file which belongs to user A |
| **generic/307** | Check if ctime is updated and written to disk after setfacl |
| **generic/318** | Check get/set ACLs to/from disk with a user namespace. A new file |
| **generic/319** | Regression test to make sure a directory inherits the default ACL from |
| **generic/375** | Check if SGID is cleared upon chmod / setfacl when the owner is not in the |
| **generic/389** | Test if O_TMPFILE files inherit POSIX Default ACLs when they are linked into |
| **generic/444** | Check if SGID is inherited when creating a subdirectory when the owner is not |
| **generic/449** | Fill the device and set as many extended attributes to a file as |
| **generic/529** | Regression test for a bug where XFS corrupts memory if the listxattr buffer |

## Category: aio <a name="category-aio"></a>

| Test ID | Description |
|---|---|
| **generic/198** | Test that aio+dio into holes does completion at the proper offsets |
| **generic/207** | Run aio-dio-extend-stat - test race in dio aio completion |
| **generic/210** | Run aio-dio-subblock-eof-read - test AIO read of last block of DIO file |
| **generic/212** | Run aio-io-setup-with-nonwritable-context-pointer - |
| **generic/240** | Test that non-block-aligned aio+dio into holes does not leave |
| **generic/427** | Try to trigger a race of free eofblocks and file extending dio writes. |
| **generic/538** | Non-block-aligned direct AIO write test with an initial truncate i_size. |

## Category: atime <a name="category-atime"></a>

| Test ID | Description |
|---|---|
| **generic/003** | Tests the noatime, relatime, strictatime and nodiratime mount options. |
| **generic/633** | Test that idmapped mounts behave correctly. |
| **generic/634** | Make sure we can store and retrieve timestamps on the extremes of the |
| **generic/635** | Make sure we can store and retrieve timestamps on the extremes of the |

## Category: attr <a name="category-attr"></a>

| Test ID | Description |
|---|---|
| **generic/037** | FSQA Test No. 037 |
| **generic/062** | Exercises the getfattr/setfattr tools |
| **generic/066** | This test is motivated by an fsync issue discovered in btrfs. |
| **generic/070** | fsstress incarnation testing extended attributes writes |
| **generic/097** | simple attr tests for EAs: |
| **generic/103** | FSQA Test No. 103 |
| **generic/117** | Attempt to cause filesystem corruption with serial fsstresses doing |
| **generic/337** | FSQA Test No. 337 |
| **generic/377** | FSQA Test No. 377 |
| **generic/403** | Test racing getxattr requests against large xattr add and remove loop. This |
| **generic/425** | Check that FIEMAP produces some output when we require an external |
| **generic/454** | Create xattrs with multiple keys that all appear the same |
| **generic/486** | Ensure that we can XATTR_REPLACE a tiny attr into a large attr. |
| **generic/489** | FSQA Test No. 489 |
| **generic/523** | Check that xattrs can have slashes in their name. |
| **generic/533** | Simple attr smoke tests for user EAs, dereived from generic/097. |
| **generic/605** | Test per-inode DAX flag by mmap direct/buffered IO. |
| **generic/606** | By the following cases, verify if statx() can query S_DAX flag |
| **generic/607** | Verify the inheritance behavior of FS_XFLAG_DAX flag in various combinations. |
| **generic/608** | Toggling FS_XFLAG_DAX on an existing file can make S_DAX on the |
| **generic/611** | Verify that metadata won't get corrupted when extended attribute |
| **generic/618** | Verify that forkoff can be returned as 0 properly if it isn't |
| **generic/728** | Test a bug where the NFS client wasn't sending a post-op GETATTR to the |

## Category: bigtime <a name="category-bigtime"></a>

| Test ID | Description |
|---|---|
| **generic/258** | Test timestamps prior to epoch |

## Category: cap <a name="category-cap"></a>

| Test ID | Description |
|---|---|
| **generic/545** | Check that we can't set the FS_APPEND_FL and FS_IMMUTABLE_FL inode |
| **generic/555** | Check that we can't set FS_XFLAG_APPEND and FS_XFLAG_IMMUTABLE inode |
| **generic/644** | Test that fscaps on idmapped mounts behave correctly. |
| **generic/696** | Test S_ISGID stripping whether works correctly when call process |
| **generic/697** | Test S_ISGID stripping whether works correctly when call process |

## Category: casefold <a name="category-casefold"></a>

| Test ID | Description |
|---|---|
| **generic/556** | Test the basic functionality of filesystems with case-insensitive |

## Category: clone <a name="category-clone"></a>

| Test ID | Description |
|---|---|
| **generic/110** | Tests file clone functionality of btrfs ("reflinks"): |
| **generic/111** | Tests file clone functionality of btrfs ("reflinks") on directory |
| **generic/115** | Moving and deleting cloned ("reflinked") files on btrfs: |
| **generic/116** | Ensure that we can reflink parts of two identical files: |
| **generic/118** | Ensuring that we can reflink non-matching parts of files: |
| **generic/119** | Reflinking two sets of files together: |
| **generic/121** | Ensure that we can dedupe parts of two files: |
| **generic/122** | Ensuring that we cannot dedupe non-matching parts of files: |
| **generic/134** | Ensure that we can reflink the last block of a file whose size isn't |
| **generic/136** | Ensure that we can dedupe the last block of a file whose size isn't |
| **generic/138** | Ensuring that copy on write through the page cache works: |
| **generic/139** | Ensuring that copy on write in direct-io mode works: |
| **generic/140** | Ensuring that mmap copy on write through the page cache works: |
| **generic/142** | Ensure that reflinking a file N times and CoWing the copies leaves the |
| **generic/143** | Ensure that reflinking a file N times and DIO CoWing the copies leaves the |
| **generic/144** | Ensure that fallocate steps around reflinked ranges: |
| **generic/145** | Ensure that collapse range steps around reflinked ranges: |
| **generic/146** | Ensure that punch-hole steps around reflinked ranges: |
| **generic/147** | Ensure that insert range steps around reflinked ranges: |
| **generic/148** | Ensure that truncating the last block in a reflinked file CoWs appropriately: |
| **generic/149** | Ensure that zero-range steps around reflinked ranges: |
| **generic/150** | Ensure that reflinking a file N times doesn't eat a lot of blocks |
| **generic/151** | Ensure that deleting all copies of a file reflinked N times releases the blocks |
| **generic/152** | Ensure that punching all copies of a file reflinked N times releases the blocks |
| **generic/153** | Ensure that collapse-range on all copies of a file reflinked N times releases the blocks |
| **generic/154** | Ensure that CoW on all copies of a file reflinked N times increases block count |
| **generic/155** | Ensure that CoW on all copies of a file reflinked N times increases block count |
| **generic/156** | Ensure that fallocate on reflinked files actually CoWs the shared blocks. |
| **generic/157** | Check that various invalid reflink scenarios are rejected. |
| **generic/158** | Check that various invalid dedupe scenarios are rejected. |
| **generic/159** | Check that we can't reflink immutable files |
| **generic/160** | Check that we can't dedupe immutable files |
| **generic/161** | Test for race between delete a file while rewriting its reflinked twin |
| **generic/162** | Test for race between dedupe and writing the dest file |
| **generic/163** | Test for race between dedupe and writing the source file |
| **generic/171** | Reflink a file, use up the rest of the space, then try to observe ENOSPC |
| **generic/172** | Reflink a file that uses more than half of the space, then try to observe |
| **generic/173** | Reflink a file, use up the rest of the space, then try to observe ENOSPC |
| **generic/174** | Reflink a file, use up the rest of the space, then try to observe ENOSPC |
| **generic/178** | Ensure that punch-hole doesn't clobber CoW. |
| **generic/179** | Ensure that unaligned punch-hole steps around reflinked ranges: |
| **generic/180** | Ensure that unaligned zero-range steps around reflinked ranges: |
| **generic/181** | Test the convention that reflink with length == 0 means "to the end of fileA" |
| **generic/182** | Test the convention that dedupe with length == 0 always returns success. |
| **generic/183** | Ensuring that copy on write in direct-io mode works when the CoW |
| **generic/185** | Ensuring that copy on write in buffered mode works when the CoW |
| **generic/188** | Ensuring that copy on write in direct-io mode works when the CoW |
| **generic/189** | Ensuring that copy on write in buffered mode works when the CoW |
| **generic/190** | Ensuring that copy on write in direct-io mode works when the CoW |
| **generic/191** | Ensuring that copy on write in buffered mode works when the CoW |
| **generic/194** | Ensuring that copy on write in direct-io mode works when the CoW |
| **generic/195** | Ensuring that copy on write in buffered mode works when the CoW |
| **generic/196** | Ensuring that copy on write in direct-io mode works when the CoW |
| **generic/197** | Ensuring that copy on write in buffered mode works when the CoW |
| **generic/199** | Ensuring that copy on write in direct-io mode works when the CoW |
| **generic/200** | Ensuring that copy on write in buffered mode works when the CoW |
| **generic/201** | See what happens if we dirty a lot of pages via CoW and immediately |
| **generic/202** | See what happens if we CoW across not-block-aligned EOF. |
| **generic/203** | See what happens if we DIO CoW across not-block-aligned EOF. |
| **generic/205** | See what happens if we CoW blocks 2-4 of a page's worth of blocks when the |
| **generic/206** | See what happens if we DIO CoW blocks 2-4 of a page's worth of blocks when |
| **generic/216** | See what happens if we CoW blocks 2-4 of a page's worth of blocks when the |
| **generic/217** | See what happens if we DIO CoW blocks 2-4 of a page's worth of blocks when |
| **generic/218** | See what happens if we CoW blocks 2-4 of a page's worth of blocks when the |
| **generic/220** | See what happens if we DIO CoW blocks 2-4 of a page's worth of blocks when |
| **generic/222** | See what happens if we CoW blocks 2-4 of a page's worth of blocks when the |
| **generic/227** | See what happens if we DIO CoW blocks 2-4 of a page's worth of blocks when |
| **generic/229** | See what happens if we CoW blocks 2-4 of a page's worth of blocks when the |
| **generic/238** | See what happens if we DIO CoW blocks 2-4 of a page's worth of blocks when |
| **generic/253** | Truncate a file at midway through a CoW region. |
| **generic/254** | Punch a file at midway through a CoW region. |
| **generic/259** | fzero a file at midway through a CoW region. |
| **generic/261** | fcollapse a file at midway through a CoW region. |
| **generic/262** | finsert a file at midway through a CoW region. |
| **generic/264** | fallocate a file at midway through a CoW region. |
| **generic/265** | Test CoW behavior when the write temporarily fails. |
| **generic/266** | Test CoW behavior when the write permanently fails. |
| **generic/267** | Test CoW behavior when the write temporarily fails and we unmount. |
| **generic/268** | Test CoW behavior when the write temporarily fails but the userspace |
| **generic/271** | Test DIO CoW behavior when the write temporarily fails. |
| **generic/272** | Test DIO CoW behavior when the write permanently fails. |
| **generic/276** | Test DIO CoW behavior when the write temporarily fails and we unmount. |
| **generic/278** | Test DIO CoW behavior when the write temporarily fails but the userspace |
| **generic/279** | Test mmap CoW behavior when the write temporarily fails. |
| **generic/281** | Test mmap CoW behavior when the write permanently fails. |
| **generic/282** | Test mmap CoW behavior when the write temporarily fails and we unmount. |
| **generic/283** | Test mmap CoW behavior when the write temporarily fails but the userspace |
| **generic/284** | Ensuring that copy on write in buffered mode to the source file when the |
| **generic/287** | Ensuring that copy on write in directio mode to the source file when the |
| **generic/289** | Ensuring that copy on write in buffered mode to the source file when the |
| **generic/290** | Ensuring that copy on write in directio mode to the source file when the |
| **generic/291** | Ensuring that copy on write in buffered mode to the source file when the |
| **generic/292** | Ensuring that copy on write in directio mode to the source file when the |
| **generic/293** | Ensuring that copy on write in buffered mode to the source file when the |
| **generic/295** | Ensuring that copy on write in directio mode to the source file when the |
| **generic/296** | - Create two reflinked files a byte longer than a block. |
| **generic/301** | Test fragmentation after a lot of random CoW: |
| **generic/302** | Test fragmentation after a lot of random CoW: |
| **generic/303** | Check that high-offset reflinks work. |
| **generic/304** | Check that high-offset dedupes work. |
| **generic/305** | Ensure that quota charges us for reflinking a file and that we're not |
| **generic/326** | Ensure that quota charges us for reflinking a file and that we're not |
| **generic/327** | Ensure that we can't go over the hard block limit when reflinking. |
| **generic/328** | Ensure that we can't go over the hard block limit when CoWing a file. |
| **generic/329** | Test AIO DIO CoW behavior when the write temporarily fails. |
| **generic/330** | Test AIO DIO CoW behavior. |
| **generic/331** | Test AIO CoW behavior when the write temporarily fails. |
| **generic/332** | Test AIO CoW behavior. |
| **generic/353** | Check if fiemap ioctl returns correct SHARED flag on reflinked file |
| **generic/356** | Check that we can't reflink a swapfile. |
| **generic/357** | Check that we can't swapon a reflinked file. |
| **generic/358** | Share an extent amongst a bunch of files such that the refcount |
| **generic/359** | Make sure that the reference counting mechanism can handle the case |
| **generic/370** | Test that we are able to create and activate a swap file on a file that used |
| **generic/372** | Check that bmap/fiemap accurately report shared extents. |
| **generic/373** | Check that cross-mountpoint reflink works. |
| **generic/374** | Check that cross-mountpoint dedupe works |
| **generic/407** | Verify that mtime is updated when cloning files |
| **generic/408** | Verify that mtime is not updated when deduping files. |
| **generic/414** | Check that reflinking adjacent blocks in a file produces a single |
| **generic/458** | Regression test for xfs leftover CoW extents after truncate |
| **generic/463** | Test racy COW AIO write completions. |
| **generic/501** | Test that if we do a buffered write to a file, fsync it, clone a range from |
| **generic/513** | Ensure that ctime is updated and capabilities are cleared when reflinking. |
| **generic/514** | Ensure that file size resource limits are respected when reflinking. |
| **generic/515** | Ensure that reflinking into a file well beyond EOF zeroes everything between |
| **generic/518** | Test that we can not clone a range from a file A into the middle of a file B |
| **generic/540** | Ensuring that reflinking works when the destination range covers multiple |
| **generic/541** | Ensuring that reflinking works when the source range covers multiple |
| **generic/542** | Ensuring that reflinking works when the destination range covers multiple |
| **generic/543** | Ensuring that reflinking works when the source range covers multiple |
| **generic/544** | Ensure that we can reflink from a file with a higher inode number to a lower |
| **generic/546** | Test when a fs is full we can still: |
| **generic/612** | Regression test for reflink corruption present as of: |
| **generic/651** | See what happens if we MMAP CoW blocks 2-4 of a page's worth of blocks when |
| **generic/652** | See what happens if we MMAP CoW blocks 2-4 of a page's worth of blocks when |
| **generic/653** | See what happens if we MMAP CoW blocks 2-4 of a page's worth of blocks when |
| **generic/654** | See what happens if we MMAP CoW blocks 2-4 of a page's worth of blocks when |
| **generic/655** | See what happens if we MMAP CoW blocks 2-4 of a page's worth of blocks when |
| **generic/657** | Ensuring that copy on write in mmap mode works when the CoW |
| **generic/658** | Ensuring that copy on write in mmap mode works when the CoW |
| **generic/659** | Ensuring that copy on write in mmap mode works when the CoW |
| **generic/660** | Ensuring that copy on write in mmap mode works when the CoW |
| **generic/661** | Ensuring that copy on write in mmap mode works when the CoW |
| **generic/662** | Ensuring that copy on write in mmap mode works when the CoW |
| **generic/663** | Ensuring that copy on write in mmap mode to the source file when the |
| **generic/664** | Ensuring that copy on write in mmap mode to the source file when the |
| **generic/665** | Ensuring that copy on write in mmap mode to the source file when the |
| **generic/666** | Ensuring that copy on write in mmap mode to the source file when the |
| **generic/667** | Ensuring that copy on write in buffered mode works when the CoW |
| **generic/668** | Ensuring that copy on write in direct-io mode works when the CoW |
| **generic/669** | Ensuring that copy on write in mmap mode works when the CoW |
| **generic/673** | Functional test for dropping suid and sgid bits as part of a reflink. |
| **generic/674** | Functional test for dropping suid and sgid bits as part of a deduplication. |
| **generic/675** | Functional test for dropping suid and sgid capabilities as part of a reflink. |
| **generic/683** | Functional test for dropping suid and sgid bits as part of a fallocate. |
| **generic/684** | Functional test for dropping suid and sgid bits as part of a fpunch. |
| **generic/685** | Functional test for dropping suid and sgid bits as part of a fzero. |
| **generic/686** | Functional test for dropping suid and sgid bits as part of a finsert. |
| **generic/687** | Functional test for dropping suid and sgid bits as part of a fcollapse. |
| **generic/702** | Test that if we have two consecutive extents and only one of them is cloned, |

## Category: copy_range <a name="category-copy_range"></a>

| Test ID | Description |
|---|---|
| **generic/430** | Tests vfs_copy_file_range(): |
| **generic/431** | Tests vfs_copy_file_range(): |
| **generic/432** | Tests vfs_copy_file_range(): |
| **generic/433** | Tests vfs_copy_file_range(): |
| **generic/434** | Tests vfs_copy_file_range() error checking |
| **generic/553** | Check that we cannot copy_file_range() to an immutable file |
| **generic/554** | Check that we cannot copy_file_range() to a swapfile |
| **generic/564** | Exercise copy_file_range() syscall error conditions. |
| **generic/565** | Exercise copy_file_range() across devices supported by some |

## Category: data <a name="category-data"></a>

| Test ID | Description |
|---|---|
| **generic/325** | Make some pages/extents of a file dirty, do a ranged fsync that covers |

## Category: dax <a name="category-dax"></a>

| Test ID | Description |
|---|---|
| **generic/413** | mmap direct/buffered io between DAX and non-DAX mountpoints. |
| **generic/428** | This is a regression test for kernel patch: |
| **generic/437** | This is a regression test for kernel patches: |
| **generic/452** | This is a regression test for kernel patch: |
| **generic/462** | This is a regression test for kernel commit |
| **generic/470** | Use dm-log-writes to verify that MAP_SYNC actually syncs metadata during |
| **generic/503** | This is a regression test for kernel patch: |

## Category: dedupe <a name="category-dedupe"></a>

| Test ID | Description |
|---|---|
| **generic/516** | Ensuring that we cannot dedupe non-matching parts of files: |
| **generic/517** | Test that deduplication of an entire file that has a size that is not aligned |

## Category: dir <a name="category-dir"></a>

| Test ID | Description |
|---|---|
| **generic/005** | Test symlinks & ELOOP |
| **generic/006** | permname |
| **generic/007** | drive the src/nametest program |
| **generic/011** | dirstress |
| **generic/245** | Check that directory renames onto non-empty targets fail |
| **generic/257** | Check that no duplicate d_off values are returned and that those |
| **generic/453** | Create a directory with multiple filenames that all appear the same |
| **generic/471** | Test that if names are added to a directory after an opendir(3) call and |
| **generic/637** | Check that directory modifications to an open dir are observed |
| **generic/736** | Test that on a fairly large directory if we keep renaming files while holding |

## Category: eio <a name="category-eio"></a>

| Test ID | Description |
|---|---|
| **generic/441** | Open a file several times, write to it, fsync on all fds and make sure that |
| **generic/484** | Open a file and write to it and fsync. Then, flip the data device to throw |
| **generic/487** | Open a file several times, write to it, fsync on all fds and make sure that |

## Category: encrypt <a name="category-encrypt"></a>

| Test ID | Description |
|---|---|
| **generic/368** | Verify the ciphertext for encryption policies that use a hardware-wrapped |
| **generic/369** | Verify the ciphertext for encryption policies that use a hardware-wrapped |
| **generic/395** | Test setting and getting encryption policies. |
| **generic/396** | Test that FS_IOC_SET_ENCRYPTION_POLICY correctly validates the fscrypt_policy |
| **generic/397** | Test accessing encrypted files and directories, both with and without the |
| **generic/398** | Filesystem encryption is designed to enforce that a consistent encryption |
| **generic/419** | Try to rename files in an encrypted directory, without access to the |
| **generic/421** | Test revoking an encryption key during concurrent I/O.  Regression test for |
| **generic/440** | Test that when the filesystem tries to enforce that all files in a directory |
| **generic/548** | Verify ciphertext for v1 encryption policies that use AES-256-XTS to encrypt |
| **generic/549** | Verify ciphertext for v1 encryption policies that use AES-128-CBC-ESSIV to |
| **generic/550** | Verify ciphertext for v1 encryption policies that use Adiantum to encrypt file |
| **generic/580** | Basic test of the fscrypt filesystem-level encryption keyring |
| **generic/581** | Test non-root use of the fscrypt filesystem-level encryption keyring |
| **generic/582** | Verify ciphertext for v2 encryption policies that use AES-256-XTS to encrypt |
| **generic/583** | Verify ciphertext for v2 encryption policies that use AES-128-CBC-ESSIV to |
| **generic/584** | Verify ciphertext for v2 encryption policies that use Adiantum to encrypt file |
| **generic/592** | Verify ciphertext for v2 encryption policies that use the IV_INO_LBLK_64 flag |
| **generic/593** | Test adding a key to a filesystem's fscrypt keyring via an |
| **generic/595** | Regression test for a bug in the FS_IOC_REMOVE_ENCRYPTION_KEY ioctl fixed by |
| **generic/602** | Verify ciphertext for v2 encryption policies that use the IV_INO_LBLK_32 flag |
| **generic/613** | Test that encryption nonces are unique and random, where randomness is |
| **generic/621** | Test for a race condition where a duplicate filename could be created in an |
| **generic/693** | Verify ciphertext for v2 encryption policies that use AES-256-XTS to encrypt |
| **generic/739** | Verify the on-disk format of encrypted files that use a crypto data unit size |

## Category: enospc <a name="category-enospc"></a>

| Test ID | Description |
|---|---|
| **generic/371** | Run write(2) and fallocate(2) in parallel and the total needed data space |

## Category: exportfs <a name="category-exportfs"></a>

| Test ID | Description |
|---|---|
| **generic/426** | Check stale handles pointing to unlinked files |
| **generic/467** | Check open by file handle. |
| **generic/477** | Check open by file handle after cycle mount. |
| **generic/756** | Check stale handles pointing to unlinked files and non-stale handles pointing |
| **generic/777** | Check open by connectable file handle after cycle mount. |

## Category: fiemap <a name="category-fiemap"></a>

| Test ID | Description |
|---|---|
| **generic/225** | Run the fiemap (file extent mapping) tester |
| **generic/519** | Verify if there's physical address overlap returned by FIBMAP, cover: |
| **generic/742** | Test fiemap into an mmaped buffer of the same file |

## Category: fiexchange <a name="category-fiexchange"></a>

| Test ID | Description |
|---|---|
| **generic/709** | Can we use exchangerange to make the quota accounting incorrect? |
| **generic/710** | Can we use exchangerange to exceed the quota enforcement? |
| **generic/712** | Make sure that exchangerange modifies ctime and not mtime of the file. |
| **generic/713** | Test exchangerange between ranges of two different files. |
| **generic/714** | Test exchangerange between ranges of two different files, when one of the files |
| **generic/715** | Test exchangerange between two files of unlike size. |
| **generic/716** | Test atomic file updates when (a) the length is the same; (b) the length |
| **generic/717** | Try invalid parameters to see if they fail. |
| **generic/718** | Make sure exchangerange honors RLIMIT_FSIZE. |
| **generic/719** | Test atomic file replacement when (a) the length is the same; (b) the length |
| **generic/720** | Stress testing with a lot of extents. |
| **generic/721** | Test non-root atomic file updates when (a) the file contents are cloned into |
| **generic/722** | Test exchangerange with the fsync flag flushes everything to disk before the call |
| **generic/723** | Test exchangerange with the dry run flag doesn't change anything. |
| **generic/724** | Test scatter-gather atomic file writes.  We create a temporary file, write |
| **generic/725** | Test scatter-gather atomic file commits.  Use the startupdate command to |
| **generic/726** | Functional test for dropping suid and sgid bits as part of an atomic file |
| **generic/727** | Functional test for dropping capability bits as part of an atomic file |
| **generic/752** | Make sure that exchangerange won't touch a swap file. |

## Category: freeze <a name="category-freeze"></a>

| Test ID | Description |
|---|---|
| **generic/491** | Test first read with freeze right after mount. |
| **generic/738** | Test possible deadlock of umount and reclaim memory |

## Category: fsr <a name="category-fsr"></a>

| Test ID | Description |
|---|---|
| **generic/018** | Basic defragmentation sanity tests |
| **generic/324** | Sanity check for defrag utility. |

## Category: idmapped <a name="category-idmapped"></a>

| Test ID | Description |
|---|---|
| **generic/645** | Test that idmapped mounts behave correctly with complex user namespaces. |

## Category: insert <a name="category-insert"></a>

| Test ID | Description |
|---|---|
| **generic/404** | Regression test which targets two nasty ext4 bugs in a logic which |
| **generic/485** | Regression test for: |
| **generic/735** | Append writes to a file with logical block numbers close to 0xffffffff |

## Category: io_uring <a name="category-io_uring"></a>

| Test ID | Description |
|---|---|
| **generic/678** | Test doing a read, with io_uring, over a file range that includes multiple |

## Category: ioctl <a name="category-ioctl"></a>

| Test ID | Description |
|---|---|
| **generic/079** | Run the t_immutable test program for immutable/append-only files. |
| **generic/277** | Check if ctime update caused by chattr is written to disk |
| **generic/288** | This check the FITRIM argument handling in the corner case where length is |
| **generic/367** | This test verifies that extent allocation hint setting works correctly on |

## Category: locks <a name="category-locks"></a>

| Test ID | Description |
|---|---|
| **generic/504** | Regression test case for kernel patch: |
| **generic/786** | Test directory delegation support |
| **generic/787** | Test file delegation support |

## Category: log <a name="category-log"></a>

| Test ID | Description |
|---|---|
| **generic/481** | FSQA Test No. 481 |
| **generic/483** | FSQA Test No. 483 |
| **generic/498** | Test that if we create a new hard link for a file which was previously |
| **generic/502** | Test that if we have a file with 2 (or more) hard links in the same parent |
| **generic/509** | Test that if we fsync a tmpfile, without adding a hard link to it, and then |
| **generic/510** | Test that if we move a file from a directory B to a directory A, replace |
| **generic/512** | Test that if we have a very small file, with a size smaller than the block |
| **generic/520** | Test case created by CrashMonkey |
| **generic/526** | Test that after a combination of file renames, linking and creating a new file |
| **generic/527** | Test that after a combination of file renames, deletions, linking and creating |
| **generic/534** | Test that if we truncate a file to reduce its size, rename it and then fsync |
| **generic/535** | This testcase is trying to test recovery flow of generic filesystem, |
| **generic/547** | Run fsstress, fsync every file and directory, simulate a power failure and |
| **generic/552** | FSQA Test No. 552 |
| **generic/557** | FSQA Test No. 557 |
| **generic/588** | FSQA Test No. 588 |
| **generic/640** | FSQA Test No. 640 |
| **generic/677** | Test that after a full fsync of a file with preallocated extents beyond the |
| **generic/690** | Test that if we fsync a directory, create a symlink inside it, rename the |
| **generic/695** | Test that if we punch a hole adjacent to an existing hole, fsync the file and |
| **generic/703** | Test that direct IO writes with io_uring and O_DSYNC are durable if a power |
| **generic/748** | Repeatedly prealloc beyond i_size, set an xattr, direct write into the |
| **generic/764** | Test that if we fsync a file that has no more hard links, power fail and then |
| **generic/771** | Create two files, the first one with some data, and then fsync both files. |
| **generic/779** | Test that if we fsync a directory that has a new symlink, then rename the |
| **generic/782** | Test that if we add a new directory to the root directory, change a file in |
| **generic/784** | Test moving a directory to another location, create a file in the old location |
| **generic/785** | Test that if we fsync a file, create a directory in the same parent directory |

## Category: metadata <a name="category-metadata"></a>

| Test ID | Description |
|---|---|
| **generic/002** | simple inode link count test for a regular file |
| **generic/020** | extended attributes |
| **generic/034** | This test is motivated by a bug found in btrfs when replaying a directory |
| **generic/039** | This test is motivated by an fsync issue discovered in btrfs. |
| **generic/040** | This test is motivated by an fsync issue discovered in btrfs. |
| **generic/041** | This test is motivated by an fsync issue discovered in btrfs. |
| **generic/056** | This test is motivated by an fsync issue discovered in btrfs. |
| **generic/057** | This test is motivated by an fsync issue discovered in btrfs. |
| **generic/059** | This test is motivated by an fsync issue discovered in btrfs. |
| **generic/065** | Test fsync on directories that got new hardlinks added to them and that point |
| **generic/073** | Test file A fsync after moving one other unrelated file B between directories |
| **generic/076** | Test blockdev reads in parallel with filesystem reads/writes |
| **generic/078** | Check renameat2 syscall with RENAME_WHITEOUT flag |
| **generic/084** | Test hardlink to unlinked file. |
| **generic/090** | Test that after syncing the filesystem, adding a hard link to a file, |
| **generic/098** | FSQA Test No. 098 |
| **generic/101** | FSQA Test No. 101 |
| **generic/104** | FSQA Test No. 104 |
| **generic/106** | FSQA Test No. 106 |
| **generic/107** | FSQA Test No. 107 |
| **generic/135** | FSQA Test No. 135 |
| **generic/184** | check mknod makes working nodes. |
| **generic/193** | Test permission checks in ->setattr |
| **generic/215** | Test out c/mtime updates after mapped writes. |
| **generic/221** | Check ctime updates when calling futimens without UTIME_OMIT for the |
| **generic/236** | Check ctime updated or not if file linked |
| **generic/317** | Check uid/gid to/from disk with a user namespace. A new file |
| **generic/321** | Runs various dir fsync tests to cover fsync'ing directory corner cases. |
| **generic/322** | Runs various rename fsync tests to cover some rename fsync corner cases. |
| **generic/335** | FSQA Test No. 335 |
| **generic/336** | FSQA Test No. 336 |
| **generic/341** | FSQA Test No. 341 |
| **generic/342** | FSQA Test No. 342 |
| **generic/343** | FSQA Test No. 343 |
| **generic/348** | FSQA Test No. 348 |
| **generic/360** | Test symlink to very long path, check symlink file contains correct path. |
| **generic/376** | FSQA Test No. 376 |
| **generic/378** | Simple permission check on hard links. |
| **generic/412** | FSQA Test No. 412 |
| **generic/456** | This test is motivated by a bug found in ext4 during random crash |
| **generic/479** | FSQA Test No. 479 |
| **generic/480** | FSQA Test No. 480 |
| **generic/745** | Test that after syncing the filesystem, adding many xattrs to a file, syncing |
| **generic/757** | Test async dio with fsync to test a btrfs bug where a race meant that csums |

## Category: misc <a name="category-misc"></a>

| Test ID | Description |
|---|---|
| **generic/004** | Test O_TMPFILE opens, and linking them back into the namespace. |
| **generic/023** | Check renameat2 syscall without flags |
| **generic/024** | Check renameat2 syscall with RENAME_NOREPLACE flag |
| **generic/025** | Check renameat2 syscall with RENAME_EXCHANGE flag |
| **generic/028** | The following commit introduced a race condition that causes getcwd(2) |
| **generic/035** | Check overwriting rename system call |
| **generic/081** | Test I/O error path by fully filling an dm snapshot. |
| **generic/294** | Tests for EEXIST (not EROFS) for inode creations, if |
| **generic/308** | Regression test for commit: |
| **generic/309** | Test directory mtime and ctime are updated when moving a file onto an |
| **generic/313** | Check ctime and mtime are updated on truncate(2) and ftruncate(2) |
| **generic/361** | Test remount on I/O errors. |
| **generic/362** | Test that doing a direct IO append write to a file when the input buffer was |
| **generic/364** | Test that a program that has 2 threads using the same file descriptor and |
| **generic/394** | Make sure fs honors file size resource limit. |
| **generic/401** | FSQA Test No. 401 |
| **generic/406** | If a larger dio write (size >= 128M) got splitted, the assertion in endio |
| **generic/423** | Test the statx system call |
| **generic/424** | Test the statx stx_attribute flags that can be set with chattr |
| **generic/478** | Test OFD lock. fcntl F_OFD_SETLK to set lock, then F_OFD_GETLK |
| **generic/488** | Test having many file descriptors referring to deleted files open. Regression |
| **generic/492** | Test the online filesystem label set/get ioctls |
| **generic/524** | Test XFS page writeback code for races with the cached file mapping. XFS |
| **generic/528** | Check that statx btime (aka creation time) is plausibly close to when |
| **generic/532** | Regression test for a bug where XFS fails to set statx attributes_mask but |
| **generic/563** | This test verifies that cgroup aware writeback properly accounts I/Os in |
| **generic/571** | FSQA Test No. 571 |
| **generic/596** | Regression test for the bug fixed by commit 10a98cb16d80 ("xfs: clear |
| **generic/676** | Test that filesystem properly handles seeking in directory both to valid |
| **generic/680** | Test for the Dirty Pipe vulnerability (CVE-2022-0847) caused by an |
| **generic/704** | Make sure logical-sector sized O_DIRECT write is allowed |
| **generic/730** | Test proper file system shut down when the block device is removed underneath |
| **generic/731** | Test proper file system shut down when the block device is removed underneath |
| **generic/755** | Create a file, stat it and then unlink it. Does the ctime of the |
| **generic/761** | Making sure direct IO (O_DIRECT) writes won't cause any data checksum mismatch, |
| **generic/763** | test zero-byte writes |
| **generic/780** | Test file_getattr() and file_setattr() syscalls on special files (fifo, |

## Category: mkfs <a name="category-mkfs"></a>

| Test ID | Description |
|---|---|
| **generic/740** | cross check mkfs detection of foreign filesystems |

## Category: mmap <a name="category-mmap"></a>

| Test ID | Description |
|---|---|
| **generic/080** | Verify that mtime is updated when writing to mmap-ed pages |
| **generic/647** | Trigger page faults in the same file during read and write |
| **generic/708** | Test iomap direct_io partial writes. |
| **generic/729** | Trigger page faults in the same file during read and write |

## Category: mount <a name="category-mount"></a>

| Test ID | Description |
|---|---|
| **generic/067** | Some random mount/umount corner case tests |
| **generic/409** | Test mount shared subtrees, verify the bind semantics: |
| **generic/410** | Test mount shared subtrees, verify the state transition when use: |
| **generic/411** | This test cover linux commit 7ae8fd0, kernel two mnt_group_id == 0 |
| **generic/604** | Evicting dirty inodes can take a long time during umount. |
| **generic/620** | Since the test is not specific to ext4, hence adding it to generic. |
| **generic/632** | All Rights Reserved. |
| **generic/783** | Test overlayfs error cases with casefold enabled layers |

## Category: other <a name="category-other"></a>

| Test ID | Description |
|---|---|
| **generic/013** | fsstress |
| **generic/015** | check out-of-space behaviour |
| **generic/120** | Test noatime mount option. |
| **generic/286** | SEEK_DATA/SEEK_HOLE copy tests. |

## Category: pattern <a name="category-pattern"></a>

| Test ID | Description |
|---|---|
| **generic/124** | FSQA Test No. 124 |
| **generic/130** | FSQA Test No. 130 |

## Category: perms <a name="category-perms"></a>

| Test ID | Description |
|---|---|
| **generic/087** | FSQA Test No. 087 |
| **generic/088** | test out CAP_DAC_OVERRIDE and CAP_DAC_SEARCH code in  |
| **generic/123** | FSQA Test No. 123 |
| **generic/126** | FSQA Test No. 126 |
| **generic/128** | FSQA Test No. 128 |
| **generic/131** | FSQA Test No. 131 |
| **generic/314** | Test SGID inheritance on subdirectories |
| **generic/355** | Test clear of suid/sgid on direct write. |
| **generic/597** | Test protected_symlink and protected_hardlink sysctls |
| **generic/598** | Test protected_regular and protected_fifos sysctls |
| **generic/689** | Test that setting POSIX ACLs in userns-mountable filesystems works. |
| **generic/698** | Test that users can changed group ownership of a file they own to a group |
| **generic/699** | This's copied from generic/698, extend it to test overlayfs on top of idmapped |

## Category: prealloc <a name="category-prealloc"></a>

| Test ID | Description |
|---|---|
| **generic/008** | Makes calls to fallocate zero range and checks tossed ranges |
| **generic/009** | Test fallocate FALLOC_FL_ZERO_RANGE |
| **generic/012** | Multi collapse range tests |
| **generic/016** | Delayed allocation multi collapse range tests |
| **generic/021** | Standard collapse range tests  |
| **generic/022** | Delayed allocation collapse range tests |
| **generic/031** | Test non-aligned writes against fcollapse to ensure that partial pages are |
| **generic/058** | Standard insert range tests |
| **generic/060** | Delayed allocation insert range tests |
| **generic/061** | Multi insert range tests |
| **generic/063** | Delayed allocation multi insert range tests |
| **generic/064** | Test multiple fallocate insert/collapse range calls on same file. |
| **generic/071** | Test extent pre-allocation (using fallocate) into a region that already has a |
| **generic/086** | This test excercises the problem with unwritten and delayed extents |
| **generic/092** | fallocate/truncate tests with FALLOC_FL_KEEP_SIZE option. |
| **generic/094** | Run the fiemap (file extent mapping) tester with preallocation enabled |
| **generic/096** | Exercise the situation that cause ext4 to BUG_ON() when we use |
| **generic/177** | FSQA Test No. 177 |
| **generic/223** | File alignment tests |
| **generic/250** | Create an unwritten extent, set up dm-error, try a DIO write, then |
| **generic/252** | Create an unwritten extent, set up dm-error, try an AIO DIO write, then |
| **generic/255** | Test Generic fallocate hole punching |
| **generic/312** | ENOSPC in fallocate(2) could corrupt ext4 when file size > 4G |
| **generic/422** | Test that a filesystem's implementation of the stat(2) system call reports |
| **generic/610** | Test a fallocate() zero range operation against a large file range for which |
| **generic/679** | Test that if we call fallocate against a file range that has a mix of holes |
| **generic/688** | Functional test for dropping capability bits as part of an fallocate. |
| **generic/749** | As per POSIX NOTES mmap(2) maps multiples of the system page size, but if the |

## Category: punch <a name="category-punch"></a>

| Test ID | Description |
|---|---|
| **generic/256** | Test Full File System Hole Punching |
| **generic/316** | Test Generic fallocate hole punching w/o unwritten extent |
| **generic/420** | Verify fallocate(mode=FALLOC_FL_KEEP_SIZE|FALLOC_FL_PUNCH_HOLE) does |
| **generic/439** | Test that if we punch a hole in a file, with either a range that goes beyond |
| **generic/469** | Test that mmap read doesn't see non-zero data past EOF on truncate down. |
| **generic/539** | Check that SEEK_HOLE can find a punched hole. |

## Category: quota <a name="category-quota"></a>

| Test ID | Description |
|---|---|
| **generic/082** | Test quota handling on remount ro failure |
| **generic/219** | Simple quota accounting test for direct/buffered/mmap IO. |
| **generic/230** | Simple quota enforcement test. |
| **generic/235** | Test whether quota gets properly reenabled after remount read-write |
| **generic/244** | test out "sparse" quota ids retrieved by Q_GETNEXTQUOTA |
| **generic/379** | Check behavior of chown with both user and group quota enabled, |
| **generic/380** | To test out pv#940675 crash in xfs_trans_brelse + quotas |
| **generic/381** | Test xfs_quota when user or names beginning with digits. |
| **generic/382** | When default quota is set, all different quota types inherits the |
| **generic/383** | Test xfs_quota when project names beginning with digits. |
| **generic/384** | test to reproduce PV951636: |
| **generic/385** | Make sure renames accross project boundaries are properly rejected |
| **generic/386** | This test checks the project quota values reported by the quota |
| **generic/400** | test out high quota ids retrieved by Q_GETNEXTQUOTA |
| **generic/566** | Regression test for chgrp returning to userspace with ILOCK held after a hard |
| **generic/594** | Test per-type(user, group and project) filesystem quota timers, make sure |
| **generic/600** | Test individual user ID quota grace period extension |
| **generic/601** | Test individual user ID quota grace period extension |
| **generic/603** | Test per-type(user, group and project) filesystem quota timers, make sure |
| **generic/681** | Ensure that unprivileged userspace hits EDQUOT while linking files into a |
| **generic/682** | Ensure that unprivileged userspace hits EDQUOT while moving files into a |

## Category: recoveryloop <a name="category-recoveryloop"></a>

| Test ID | Description |
|---|---|
| **generic/646** | Testcase for kernel commit: |

## Category: remount <a name="category-remount"></a>

| Test ID | Description |
|---|---|
| **generic/599** | All Rights Reserved. |

## Category: rename <a name="category-rename"></a>

| Test ID | Description |
|---|---|
| **generic/626** | Test RENAME_WHITEOUT on filesystem without space to create one more inodes. |
| **generic/700** | Verify selinux label can be kept after RENAME_WHITEOUT. This is |
| **generic/732** | Mount the same export to different mount points and move (rename) |

## Category: rw <a name="category-rw"></a>

| Test ID | Description |
|---|---|
| **generic/001** | Random file copier to produce chains of identical files so the head |
| **generic/014** | truncfile |
| **generic/029** | Test mapped writes against truncate down/up to ensure we get the data |
| **generic/030** | Test mapped writes against remap+truncate down/up to ensure we get the data |
| **generic/032** | This test implements a data corruption scenario on XFS filesystems with |
| **generic/033** | This test stresses indirect block reservation for delayed allocation extents. |
| **generic/069** | Test out writes with O_APPEND flag sets. |
| **generic/075** | fsx (non-AIO variant) |
| **generic/091** | fsx exercising direct IO -- sub-block sizes and concurrent buffered IO |
| **generic/095** | Concurrent mixed I/O (buffer I/O, aiodio, mmap, splice) on the same files |
| **generic/108** | Test partial block device failure. Calls like fsync() should report failure |
| **generic/112** | fsx (AIO variant, based on 075) |
| **generic/113** | aio-stress |
| **generic/114** | Test races while extending past EOF via sub-block AIO writes |
| **generic/129** | FSQA Test No. 129 |
| **generic/141** | FSQA Test No. 141 |
| **generic/169** | FSQA Test No. 169 |
| **generic/211** | Test that overwriting a file with mmap when the filesystem has no more space |
| **generic/213** | Check some unwritten extent boundary conditions, fallocate version. |
| **generic/214** | Basic unwritten extent sanity checks |
| **generic/228** | Check if fallocate respects RLIMIT_FSIZE |
| **generic/246** | Check that truncation after failed writes does not zero too much data. |
| **generic/247** | Test for race between direct I/O and mmap |
| **generic/248** | Test for pwrite hang problem when writing from mmaped buffer of the same page  |
| **generic/249** | simple splice(2) test. |
| **generic/263** | fsx exercising direct IO vs sub-block buffered I/O |
| **generic/306** | Test RW open of a device on a RO fs |
| **generic/315** | fallocate/truncate tests with FALLOC_FL_KEEP_SIZE option. |
| **generic/338** | Test I/O on dm error device. |
| **generic/346** | FSQA Test No. 346 |
| **generic/347** | Test very basic thin device usage, exhaustion, and growth |
| **generic/366** | Test if mixed direct read, direct write and buffered write on the same file will |
| **generic/391** | Test two threads doing non-overlapping direct I/O in the same extents. |
| **generic/393** | Test some small truncations to check inline_data and its cached data are |
| **generic/402** | Test to verify filesystem timestamps for supported ranges. |
| **generic/436** | More SEEK_DATA/SEEK_HOLE sanity tests. |
| **generic/443** | Takes page fault while writev is iterating over the vectors in the IOV |
| **generic/445** | Another SEEK_DATA/SEEK_HOLE sanity test. |
| **generic/446** | Regression test for commit: |
| **generic/448** | Check what happens when SEEK_HOLE/SEEK_DATA are fed negative offsets. |
| **generic/450** | Test read around EOF. If the file offset is at or past the end of file, |
| **generic/451** | Test data integrity when mixing buffered reads and asynchronous |
| **generic/460** | Test that XFS reserves reasonable indirect blocks for delalloc and |
| **generic/465** | Test i_size is updated properly under dio read/write |
| **generic/466** | Check that high-offset reads and writes work. |
| **generic/490** | Check that SEEK_DATA works properly for offsets in the middle of large holes. |
| **generic/499** | Test a specific sequence of fsx operations that causes an mmap read past |
| **generic/511** | Test a specific sequence of fsx operations that causes an mmap read past |
| **generic/525** | All Rights Reserved. |
| **generic/536** | Test a some write patterns for stale data exposure after a crash.  XFS is |
| **generic/567** | Test mapped writes against punch-hole to ensure we get the data |
| **generic/568** | Test that fallocating an unaligned range allocates all blocks |
| **generic/569** | Check that we can't modify a file that's an active swap file. |
| **generic/570** | Check that we can't modify a block device that's an active swap device. |
| **generic/578** | Make sure that we can handle multiple mmap writers to the same file. |
| **generic/586** | Race an appending aio dio write to the second block of a file while |
| **generic/587** | Regression test to ensure that dquots are attached to the inode when we're |
| **generic/591** | Test using splice() to read from pipes. |
| **generic/609** | iomap can call generic_write_sync() if we're O_DSYNC, so write a basic test to |
| **generic/614** | Test that after doing a memory mapped write to an empty file, a call to |
| **generic/615** | Test that if we keep overwriting an entire file, either with buffered writes |
| **generic/628** | Make sure that reflink forces the log out if we open the file with O_SYNC or |
| **generic/629** | Make sure that copy_file_range forces the log out if we open the file with |
| **generic/630** | Make sure that mmap and file writers racing with FIDEDUPERANGE cannot write |
| **generic/638** | This case mmaps several pages of a file, alloc pages, copy data with pages |
| **generic/639** | Open a file and write a little data to it. Unmount (to clean out the cache) |
| **generic/758** | Test mapped writes against zero-range to ensure we get the data |
| **generic/759** | fsx exercising reads/writes from userspace buffers |
| **generic/760** | fsx exercising direct IO reads/writes from userspace buffers |
| **generic/765** | Validate atomic write support |
| **generic/767** | Validate multi-fsblock atomic write support with simulated hardware support |
| **generic/768** | Validate multi-fsblock atomic write support with or without hw support |
| **generic/769** | reflink tests for large atomic writes with mixed mappings |
| **generic/770** | basic tests for large atomic writes with mixed mappings |
| **generic/775** | Atomic write multi-fsblock data integrity tests with mixed mappings |
| **generic/776** | fuzz fsx with atomic writes |

## Category: seek <a name="category-seek"></a>

| Test ID | Description |
|---|---|
| **generic/706** | Test that seeking for data on a 1 byte file works correctly, the returned |

## Category: shutdown <a name="category-shutdown"></a>

| Test ID | Description |
|---|---|
| **generic/042** | Test stale data exposure via writeback using various file allocation |
| **generic/050** | Check out various mount/remount/unmount scenarious on a read-only blockdev. |
| **generic/052** | To test log replay by shutdown of file system |
| **generic/392** | Test inode's metadata after fsync or fdatasync calls. |
| **generic/417** | Test orphan inode / unlinked list processing on RO mount & RW transition |
| **generic/468** | This testcase is a fallocate variant of generic/392, it expands to test |
| **generic/474** | Inspired by syncfs bug of overlayfs which does not sync dirty inodes in |
| **generic/505** | This testcase is trying to test recovery flow of generic filesystem, w/ below |
| **generic/506** | This testcase is trying to test recovery flow of generic filesystem, w/ below |
| **generic/507** | This testcase is trying to test recovery flow of generic filesystem, w/ below |
| **generic/508** | This testcase is trying to test recovery flow of generic filesystem, it needs |
| **generic/530** | Stress test creating a lot of unlinked O_TMPFILE files and recovering them |
| **generic/623** | Test a write fault scenario on a shutdown fs. |
| **generic/737** | Integrity test for O_SYNC with buff-io, dio, aio-dio with sudden shutdown. |
| **generic/766** | Copied from tests generic/050 and adjusted to support testing |

## Category: swap <a name="category-swap"></a>

| Test ID | Description |
|---|---|
| **generic/472** | Test various swapfile activation oddities. |
| **generic/493** | Check that we can't dedupe a swapfile. |
| **generic/494** | Test truncation/hole punching of an active swapfile. |
| **generic/495** | Test invalid swap file (with holes) |
| **generic/496** | Test various swapfile activation oddities on filesystems that support |
| **generic/497** | Test various swapfile activation oddities, having used fcollapse to |
| **generic/636** | Test invalid swap files. |
| **generic/641** | Test small swapfile which doesn't contain even a single page-aligned contiguous |

## Category: swapext <a name="category-swapext"></a>

| Test ID | Description |
|---|---|
| **generic/711** | Make sure that swapext won't touch a swap file. |

## Category: trim <a name="category-trim"></a>

| Test ID | Description |
|---|---|
| **generic/260** | Purpose of this test is to check FITRIM argument handling to make sure |
| **generic/537** | Ensure that we can't call fstrim on filesystems mounted norecovery, because |

## Category: unlink <a name="category-unlink"></a>

| Test ID | Description |
|---|---|
| **generic/531** | Stress test creating a lot of unlinked O_TMPFILE files and closing them |

## Category: unshare <a name="category-unshare"></a>

| Test ID | Description |
|---|---|
| **generic/734** | This is a regression test for the kernel commit noted below.  The stale |

## Category: verity <a name="category-verity"></a>

| Test ID | Description |
|---|---|
| **generic/572** | This is a basic fs-verity test which verifies: |
| **generic/573** | Test access controls on the fs-verity ioctls.  FS_IOC_MEASURE_VERITY is |
| **generic/574** | Test corrupting verity files.  This test corrupts various parts of the |
| **generic/575** | Test that fs-verity is using the correct file digest values.  This test |
| **generic/576** | Test using fs-verity and fscrypt simultaneously.  This primarily verifies |
| **generic/577** | Test the fs-verity built-in signature verification support. |
| **generic/624** | Test retrieving the Merkle tree and fs-verity descriptor of a verity file |
| **generic/625** | Test retrieving the built-in signature of a verity file using |
| **generic/692** | fs-verity requires the filesystem to decide how it stores the Merkle tree, |

## Category: volume <a name="category-volume"></a>

| Test ID | Description |
|---|---|
| **generic/741** | Attempt to mount both the DM physical device and the DM flakey device. |

## Category: zone <a name="category-zone"></a>

| Test ID | Description |
|---|---|
| **generic/781** | Smoke test for FSes with ZBD support on zloop |

use custom_error::custom_error;
use std::{
    io::{Cursor, Read},
    sync::Arc,
};

use serde::{Deserialize, Serialize};

use super::File;

////////////////////////////////////////////////////////////////////////////////
// Traits
////////////////////////////////////////////////////////////////////////////////

/// Delta Trait
/// [`Dlt::patch`] lets you patch a file with a diff
pub trait Dlt {
    type Error;
    fn patch(&self, file: &File) -> Result<File, Self::Error>;
}

/// Signature Trait
/// [`Sig::Error`] is the error type of the underlying implementor
/// [`Sig::new`] is a simple TryFrom of the underlying implementor
/// [`Sig::diff`] creates a corresponding Delta type
pub trait Sig: Sized + PartialEq /* for<'a> TryFrom<&'a File> */ {
    // type Error: for<'a> From< <Self as TryFrom<&'a File>>::Error>;
    // fn new(file: &File) -> Result<Self,  <Self as Sig>::Error> {
    //     Ok(TryFrom::try_from(file)?)
    // }

    type Error;
    fn new(file: &File) -> Result<Self, <Self as Sig>::Error>;

    fn diff(&self, with: &File) -> Result<impl Dlt, <Self as Sig>::Error>;
}

////////////////////////////////////////////////////////////////////////////////
// Implementation with librsync
////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RSyncSig {
    data: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RSyncDelta {
    data: Vec<u8>,
}

impl TryFrom<&File> for RSyncSig {
    type Error = librsync::Error;
    fn try_from(file: &File) -> Result<Self, Self::Error> {
        let mut data = Vec::new();
        librsync::whole::signature(&mut Cursor::new(&file.0[..]), &mut data)?;
        Ok(Self { data })
    }
}

impl Sig for RSyncSig {
    type Error = librsync::Error;

    fn new(file: &File) -> Result<Self, <Self as Sig>::Error> {
        Ok(TryFrom::try_from(file)?)
    }

    #[allow(refining_impl_trait)]
    fn diff(&self, with: &File) -> Result<RSyncDelta, <Self as Sig>::Error> {
        RSyncDelta::diff(self, with)
    }
}

impl Dlt for RSyncDelta {
    type Error = librsync::Error;
    fn patch(&self, file: &File) -> Result<File, librsync::Error> {
        let mut data = Vec::new();
        librsync::Patch::new(
            &mut Cursor::new(&file.0[..]),
            &mut Cursor::new(&self.data[..]),
        )?
        .read_to_end(&mut data)?;
        Ok(File(Arc::new(data)))
    }
}

impl RSyncDelta {
    fn diff(sig: &RSyncSig, with: &File) -> Result<Self, <RSyncSig as Sig>::Error> {
        let mut data = Vec::new();
        librsync::Delta::new(Cursor::new(&with.0[..]), &mut Cursor::new(&sig.data[..]))?
            .read_to_end(&mut data)?;
        Ok(Self { data })
    }
}

////////////////////////////////////////////////////////////////////////////////
// Generic Wrappers
////////////////////////////////////////////////////////////////////////////////

custom_error! {
    #[derive(Clone)]
    pub DiffError
    RSyncError{rsync: Arc<librsync::Error>} = "{rsync}"
}

impl From<librsync::Error> for DiffError {
    fn from(rsync: librsync::Error) -> Self {
        Self::RSyncError {
            rsync: Arc::new(rsync),
        }
    }
}

pub enum Implementors {
    RSync,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Signature {
    RSyncSig(RSyncSig),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Delta {
    RSyncDelta(RSyncDelta),
}

impl Signature {
    pub fn new_using(file: &File, i: Implementors) -> Result<Self, DiffError> {
        match i {
            Implementors::RSync => Ok(Self::RSyncSig(RSyncSig::try_from(file)?)),
        }
    }

    pub fn implementor(&self) -> Implementors {
        match self {
            Signature::RSyncSig(_) => Implementors::RSync,
        }
    }
}

impl Sig for Signature {
    type Error = DiffError;

    fn new(file: &File) -> Result<Self, <Self as Sig>::Error> {
        Ok(Self::RSyncSig(RSyncSig::try_from(file)?))
        // Ok(<RSyncSig as TryFrom<&File>>::try_from(file)?.into())
    }

    #[allow(refining_impl_trait)]
    fn diff(&self, with: &File) -> Result<Delta, <Self as Sig>::Error> {
        match self {
            Signature::RSyncSig(sig) => Ok(Delta::RSyncDelta(sig.diff(with)?)),
        }
    }
}

impl Dlt for Delta {
    type Error = DiffError;

    fn patch(&self, file: &File) -> Result<File, Self::Error> {
        match self {
            Delta::RSyncDelta(delta) => Ok(delta.patch(file)?),
        }
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use rand::Rng;

    use crate::pods::filesystem::{
        diffs::{Dlt, RSyncDelta, RSyncSig, Sig},
        File,
    };

    #[test]
    fn test_rsync_sig_empty() {
        let empty_sig_ref: [u8; 12] = [114, 115, 1, 55, 0, 0, 8, 0, 0, 0, 0, 32];

        let empty = File::empty();
        let sig = RSyncSig::new(&empty).unwrap();

        assert_eq!(&sig.data[..], &empty_sig_ref);
    }

    #[test]
    fn test_rsync_diff() {
        let mut rng = rand::rng();
        let file = {
            let mut data: Vec<u8> = Vec::new();
            data.resize_with(7830, ||rng.random());
            File(Arc::new(data))
        };

        let sig = RSyncSig::new(&file).unwrap();

        let mut change: Vec<u8> = Vec::new();
        change.resize_with(1057, ||rng.random());

        let changed_file = {
            let mut data = (*file.0).clone();
            data.splice(2630..2701, change.iter().cloned());
            File(Arc::new(data))
        };

        let diff: RSyncDelta = sig.diff(&changed_file).unwrap();

        let sig_ser = bincode::serialize(&sig).unwrap();
        let diff_ser = bincode::serialize(&diff).unwrap();

        let sig_deser = bincode::deserialize::<RSyncSig>(&sig_ser).unwrap();
        let diff_deser = bincode::deserialize::<RSyncDelta>(&diff_ser).unwrap();

        let sig_dest = RSyncSig::new(&file).unwrap(); // simulate case where file contents match on the remote

        assert_eq!(sig_deser, sig_dest);

        let patched_file = diff_deser.patch(&file).unwrap();

        assert_eq!(patched_file.0, changed_file.0);

        assert!(diff.data.len() < changed_file.0.len());
    }
}

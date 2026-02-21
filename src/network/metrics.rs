use std::{collections::HashMap, fmt::Write};

use libp2p::PeerId;
use serde::{Deserialize, Serialize};

use crate::network;

fn format_byte(mut num: usize) -> String {
    let units = ["B", "KB", "MB", "GB", "TB", "PB"];

    let mut dec = 0;

    for unit in units {
        if num <= 1024 {
            if dec > 0 {
                return format!("{num}.{dec:02}{unit}");
            } else {
                return format!("{num}{unit}");
            }
        } else {
            if num < 1024 * 1024 {
                dec = 25 * num / 256;
                num >>= 10;
                dec -= num * 100;
            } else {
                num >>= 10;
            }
        }
    }
    if dec > 0 {
        format!("{num}.{dec:02}{}", units.last().expect("non-empty"))
    } else {
        format!("{num}{}", units.last().expect("non-empty"))
    }
}

#[derive(Debug, Default)]
pub struct Metrics {
    pub uploaded_bytes: HashMap<PeerId, usize>,
    pub downloaded_bytes: HashMap<PeerId, usize>,
    pub upload_number: HashMap<PeerId, usize>,
    pub download_number: HashMap<PeerId, usize>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct MetricsData {
    pub uploaded: HashMap<PeerId, (String, usize, usize)>,
    pub downloaded: HashMap<PeerId, (String, usize, usize)>,
}

impl Metrics {
    pub fn show(&self, infos: Option<&HashMap<PeerId, network::PeerInfo>>) -> String {
        let mut string = String::new();
        let _ = writeln!(&mut string, "Uploaded:");
        for (peer, bytes) in &self.uploaded_bytes {
            let peer = infos
                .and_then(|i| i.get(peer))
                .map(|i| i.nickname.clone())
                .unwrap_or_else(|| peer.to_base58());
            let _ = writeln!(&mut string, "{}: {} / {}", peer, format_byte(*bytes), 1);
        }

        let _ = writeln!(&mut string, "Downloaded:");
        for (peer, bytes) in &self.downloaded_bytes {
            let peer = infos
                .and_then(|i| i.get(peer))
                .map(|i| i.nickname.clone())
                .unwrap_or_else(|| peer.to_base58());
            let _ = writeln!(&mut string, "{}: {} / {}", peer, format_byte(*bytes), 1);
        }

        string
    }

    pub fn export(&self, infos: Option<&HashMap<PeerId, network::PeerInfo>>) -> MetricsData {
        if let Some(infos) = infos {
            let key_switcher = |peer: &PeerId| -> String {
                infos
                    .get(peer)
                    .map(|i| i.nickname.clone())
                    .unwrap_or_else(|| peer.to_base58())
            };
            MetricsData {
                downloaded: HashMap::from_iter(infos.keys().map(|peer| {
                    (
                        *peer,
                        (
                            key_switcher(peer),
                            self.downloaded_bytes.get(peer).cloned().unwrap_or_default(),
                            self.download_number.get(peer).cloned().unwrap_or_default(),
                        ),
                    )
                })),
                uploaded: HashMap::from_iter(infos.keys().map(|peer| {
                    (
                        *peer,
                        (
                            key_switcher(peer),
                            self.uploaded_bytes.get(peer).cloned().unwrap_or_default(),
                            self.upload_number.get(peer).cloned().unwrap_or_default(),
                        ),
                    )
                })),
            }
        } else {
            MetricsData {
                downloaded: HashMap::from_iter(self.downloaded_bytes.iter().map(
                    |(peer, value)| {
                        (
                            *peer,
                            (
                                peer.to_base58(),
                                *value,
                                self.download_number.get(peer).cloned().unwrap_or_default(),
                            ),
                        )
                    },
                )),
                uploaded: HashMap::from_iter(self.uploaded_bytes.iter().map(|(peer, value)| {
                    (
                        *peer,
                        (
                            peer.to_base58(),
                            *value,
                            self.upload_number.get(peer).cloned().unwrap_or_default(),
                        ),
                    )
                })),
            }
        }
    }
}

impl MetricsData {
    pub fn show(&self) -> String {
        let mut string = String::new();
        let _ = writeln!(&mut string, "Uploaded:");
        for (name, bytes, count) in self.uploaded.values() {
            let _ = writeln!(&mut string, "{}: {} / {}", name, format_byte(*bytes), count);
        }

        let _ = writeln!(&mut string, "Downloaded:");
        for (name, bytes, count) in self.downloaded.values() {
            let _ = writeln!(&mut string, "{}: {} / {}", name, format_byte(*bytes), count);
        }

        string
    }
}

impl Metrics {
    pub fn inc_download(&mut self, peer: PeerId, size: usize) {
        *self.download_number.entry(peer).or_default() += 1;
        *self.downloaded_bytes.entry(peer).or_default() += size;
    }

    pub fn inc_upload(&mut self, peer: PeerId, size: usize) {
        *self.upload_number.entry(peer).or_default() += 1;
        *self.uploaded_bytes.entry(peer).or_default() += size;
    }
}

// SPDX-License-Identifier: GPL-3.0

use crate::constants::{IMAGE_CACHE_SWEEP_SECS, IMAGE_CACHE_TTL_SECS};
use cosmic::widget::image::Handle;
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

pub struct ImageStore {
    artwork_dir: PathBuf,
    cache: Arc<Mutex<HashMap<PathBuf, CachedImage>>>,
    queue: Arc<Mutex<VecDeque<PathBuf>>>,
    tx: mpsc::Sender<PathBuf>,
}

impl ImageStore {
    pub fn new(artwork_dir: PathBuf) -> Self {
        let (tx, mut rx) = mpsc::channel::<PathBuf>(64);

        let cache = Arc::new(Mutex::new(HashMap::new()));
        let queue = Arc::new(Mutex::new(VecDeque::new()));

        let cache_clone = cache.clone();
        let queue_clone = queue.clone();

        let cache_eviction = cache.clone();

        tokio::spawn(async move {
            while let Some(path) = rx.recv().await {
                // Remove path from queue
                queue_clone.lock().unwrap().retain(|p| p != &path);

                // If path is already in cache, skip loading
                if cache_clone.lock().unwrap().contains_key(&path) {
                    continue;
                }

                let path_for_read = path.clone();
                match tokio::task::spawn_blocking(move || fs::read(&path_for_read)).await {
                    Ok(Ok(data)) => {
                        cache_clone.lock().unwrap().insert(
                            path,
                            CachedImage {
                                handle: Arc::new(cosmic::widget::image::Handle::from_bytes(data)),
                                last_used: Instant::now(),
                            },
                        );
                    }
                    Ok(Err(err)) => {
                        eprintln!("Failed to load image: {:?} {}", path, err);
                    }
                    Err(err) => {
                        eprintln!("Failed to join image load task for {:?}: {}", path, err);
                    }
                }
            }
        });

        tokio::spawn(async move {
            let ttl = Duration::from_secs(IMAGE_CACHE_TTL_SECS);
            let sweep_every = Duration::from_secs(IMAGE_CACHE_SWEEP_SECS);

            loop {
                tokio::time::sleep(sweep_every).await;

                let mut cache = cache_eviction.lock().unwrap();
                let now = Instant::now();

                cache.retain(|_, entry| now.duration_since(entry.last_used) < ttl);
            }
        });

        Self {
            artwork_dir,
            cache,
            queue,
            tx,
        }
    }
}

impl ImageStore {
    pub fn request(&self, path: String) {
        let artwork_path = self.artwork_dir.join(path);

        if self.cache.lock().unwrap().contains_key(&artwork_path) {
            return;
        }

        let mut q = self.queue.lock().unwrap();
        if q.contains(&artwork_path) {
            return;
        }

        if self.tx.try_send(artwork_path.clone()).is_ok() {
            q.push_back(artwork_path);
        }
    }

    pub fn get(&self, path: &String) -> Option<Arc<Handle>> {
        let artwork_path = self.artwork_dir.join(path);
        let mut cache = self.cache.lock().unwrap();

        if let Some(entry) = cache.get_mut(&artwork_path) {
            entry.last_used = Instant::now();
            return Some(entry.handle.clone());
        }

        None
    }
}

struct CachedImage {
    handle: Arc<cosmic::widget::image::Handle>,
    last_used: Instant,
}

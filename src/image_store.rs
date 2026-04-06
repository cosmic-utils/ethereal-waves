// SPDX-License-Identifier: GPL-3.0

use cosmic::widget::image::Handle;
use std::collections::{HashMap, HashSet, VecDeque};
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

                match fs::read(&path) {
                    Ok(data) => {
                        cache_clone.lock().unwrap().insert(
                            path,
                            CachedImage {
                                handle: Arc::new(cosmic::widget::image::Handle::from_bytes(data)),
                                last_used: Instant::now(),
                            },
                        );
                    }
                    Err(err) => {
                        eprintln!("Failed to load image: {:?} {}", path, err);
                    }
                }
            }
        });

        tokio::spawn(async move {
            let ttl = Duration::from_secs(20);
            let sweep_every = Duration::from_secs(10);

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

        q.push_back(artwork_path.clone());
        let _ = self.tx.try_send(artwork_path);
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

    pub fn cleanup_unused(&self, used_filenames: &HashSet<String>) {
        let entries = match fs::read_dir(&self.artwork_dir) {
            Ok(entries) => entries,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return,
            Err(err) => {
                eprintln!(
                    "Failed to read artwork cache directory {:?}: {}",
                    self.artwork_dir, err
                );
                return;
            }
        };

        let mut removed_paths = HashSet::new();

        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };

            if used_filenames.contains(file_name) {
                continue;
            }

            match fs::remove_file(&path) {
                Ok(_) => {
                    removed_paths.insert(path);
                }
                Err(err) => {
                    eprintln!(
                        "Failed to remove unused artwork cache file {:?}: {}",
                        path, err
                    );
                }
            }
        }

        if removed_paths.is_empty() {
            return;
        }

        self.cache
            .lock()
            .unwrap()
            .retain(|path, _| !removed_paths.contains(path));
        self.queue
            .lock()
            .unwrap()
            .retain(|path| !removed_paths.contains(path));
    }
}

struct CachedImage {
    handle: Arc<cosmic::widget::image::Handle>,
    last_used: Instant,
}

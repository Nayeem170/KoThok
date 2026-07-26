use std::collections::HashMap;
use std::rc::Rc;

use kobo_core::audio::Prepared;

const MAX_CACHE_BYTES: usize = 8 * 1024 * 1024;

struct Entry {
    prep: Rc<Prepared>,
    bytes: usize,
}

pub(super) struct PcmCache {
    map: HashMap<(String, String, String), Entry>,
    order: Vec<(String, String, String)>,
    used_bytes: usize,
}

impl PcmCache {
    pub(super) fn new() -> Self {
        Self {
            map: HashMap::new(),
            order: Vec::new(),
            used_bytes: 0,
        }
    }

    pub(super) fn lookup(&mut self, text: &str, voice: &str, rate: &str) -> Option<Rc<Prepared>> {
        let key = (text.to_owned(), voice.to_owned(), rate.to_owned());
        if let Some(entry) = self.map.get_mut(&key) {
            let prep = Rc::clone(&entry.prep);
            self.touch(&key);
            return Some(prep);
        }
        None
    }

    pub(super) fn store(&mut self, key: (String, String, String), prep: Rc<Prepared>) {
        let bytes = prep.stereo.len() * 2;
        if bytes > MAX_CACHE_BYTES {
            return;
        }
        if let Some(old) = self.map.remove(&key) {
            self.used_bytes -= old.bytes;
            self.order.retain(|k| k != &key);
        }
        while self.used_bytes + bytes > MAX_CACHE_BYTES && !self.order.is_empty() {
            if let Some(evicted) = self.order.first().cloned() {
                self.order.remove(0);
                if let Some(entry) = self.map.remove(&evicted) {
                    self.used_bytes -= entry.bytes;
                }
            }
        }
        self.used_bytes += bytes;
        self.order.push(key.clone());
        self.map.insert(key, Entry { prep, bytes });
    }

    fn touch(&mut self, key: &(String, String, String)) {
        self.order.retain(|k| k != key);
        self.order.push(key.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_prep(samples: usize) -> Rc<Prepared> {
        Rc::new(Prepared {
            stereo: vec![0i16; samples],
            bounds: vec![],
        })
    }

    fn key(text: &str) -> (String, String, String) {
        (text.into(), "voice".into(), "rate".into())
    }

    #[test]
    fn cache_miss_returns_none() {
        let mut c = PcmCache::new();
        assert!(c.lookup("hello", "voice", "rate").is_none());
    }

    #[test]
    fn cache_hit_returns_stored_prep() {
        let mut c = PcmCache::new();
        c.store(key("hello"), make_prep(100));
        let hit = c.lookup("hello", "voice", "rate");
        assert!(hit.is_some());
        assert_eq!(hit.unwrap().stereo.len(), 100);
    }

    #[test]
    fn different_keys_are_independent() {
        let mut c = PcmCache::new();
        c.store(key("a"), make_prep(100));
        c.store(key("b"), make_prep(200));
        assert!(c.lookup("a", "voice", "rate").is_some());
        assert!(c.lookup("b", "voice", "rate").is_some());
    }

    #[test]
    fn lru_eviction_on_overflow() {
        let mut c = PcmCache::new();
        let samples = MAX_CACHE_BYTES / 6 + 1;
        c.store(key("first"), make_prep(samples));
        c.store(key("second"), make_prep(samples));
        assert!(c.lookup("first", "voice", "rate").is_some());
        assert!(c.lookup("second", "voice", "rate").is_some());
        c.store(key("third"), make_prep(samples));
        assert!(c.lookup("first", "voice", "rate").is_none());
        assert!(c.lookup("second", "voice", "rate").is_some());
        assert!(c.lookup("third", "voice", "rate").is_some());
    }

    #[test]
    fn oversized_entry_is_rejected() {
        let mut c = PcmCache::new();
        c.store(key("huge"), make_prep(MAX_CACHE_BYTES + 1));
        assert!(c.lookup("huge", "voice", "rate").is_none());
    }

    #[test]
    fn lookup_promotes_lru_order() {
        let mut c = PcmCache::new();
        let small = MAX_CACHE_BYTES / 6;
        c.store(key("a"), make_prep(small));
        c.store(key("b"), make_prep(small));
        c.store(key("c"), make_prep(small));
        c.lookup("a", "voice", "rate");
        c.store(key("d"), make_prep(small));
        assert!(c.lookup("a", "voice", "rate").is_some());
        assert!(c.lookup("d", "voice", "rate").is_some());
        assert!(c.lookup("b", "voice", "rate").is_none());
    }

    #[test]
    fn store_updates_existing_entry() {
        let mut c = PcmCache::new();
        c.store(key("x"), make_prep(100));
        c.store(key("x"), make_prep(200));
        let hit = c.lookup("x", "voice", "rate");
        assert_eq!(hit.unwrap().stereo.len(), 200);
    }
}

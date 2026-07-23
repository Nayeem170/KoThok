use std::collections::HashMap;
use super::ReadyUtt;

const MAX_CACHE_BYTES: usize = 8 * 1024 * 1024;

struct Entry {
    utt: ReadyUtt,
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

    pub(super) fn lookup(&mut self, text: &str, voice: &str, rate: &str) -> Option<ReadyUtt> {
        let key = (text.to_owned(), voice.to_owned(), rate.to_owned());
        if let Some(entry) = self.map.get_mut(&key) {
            let utt = Self::clone_utt(&entry.utt);
            self.touch(&key);
            return Some(utt);
        }
        None
    }

    pub(super) fn store(&mut self, key: (String, String, String), utt: ReadyUtt) {
        let bytes = utt.prep.stereo.len() * 2;
        if bytes > MAX_CACHE_BYTES {
            return;
        }
        if self.map.contains_key(&key) {
            self.used_bytes -= self.map.get(&key).map(|e| e.bytes).unwrap_or(0);
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
        self.map.insert(key, Entry { utt, bytes });
    }

    fn touch(&mut self, key: &(String, String, String)) {
        self.order.retain(|k| k != key);
        self.order.push(key.clone());
    }

    fn clone_utt(utt: &ReadyUtt) -> ReadyUtt {
        ReadyUtt {
            idx: utt.idx,
            prep: kobo_core::audio::Prepared {
                stereo: utt.prep.stereo.clone(),
                bounds: utt.prep.bounds.clone(),
            },
            start: utt.start,
            end: utt.end,
            para_end: utt.para_end,
            page_break: utt.page_break,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_utt(text: &str, samples: usize) -> ReadyUtt {
        ReadyUtt {
            idx: 0,
            prep: kobo_core::audio::Prepared {
                stereo: vec![0i16; samples],
                bounds: vec![],
            },
            start: 0,
            end: text.len(),
            para_end: true,
            page_break: None,
        }
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
    fn cache_hit_returns_stored_utt() {
        let mut c = PcmCache::new();
        let u = make_utt("hello", 100);
        c.store(key("hello"), u.clone());
        let hit = c.lookup("hello", "voice", "rate");
        assert!(hit.is_some());
        assert_eq!(hit.unwrap().prep.stereo.len(), 100);
    }

    #[test]
    fn different_keys_are_independent() {
        let mut c = PcmCache::new();
        c.store(key("a"), make_utt("a", 100));
        c.store(key("b"), make_utt("b", 200));
        assert!(c.lookup("a", "voice", "rate").is_some());
        assert!(c.lookup("b", "voice", "rate").is_some());
    }

    #[test]
    fn lru_eviction_on_overflow() {
        let mut c = PcmCache::new();
        let big = MAX_CACHE_BYTES / 2 + 100;
        c.store(key("first"), make_utt("first", big / 2));
        c.store(key("second"), make_utt("second", big / 2));
        assert!(c.lookup("first", "voice", "rate").is_some());
        assert!(c.lookup("second", "voice", "rate").is_some());
        c.store(key("third"), make_utt("third", big / 2));
        assert!(c.lookup("first", "voice", "rate").is_none());
        assert!(c.lookup("second", "voice", "rate").is_some());
        assert!(c.lookup("third", "voice", "rate").is_some());
    }

    #[test]
    fn oversized_entry_is_rejected() {
        let mut c = PcmCache::new();
        c.store(key("huge"), make_utt("huge", MAX_CACHE_BYTES + 1));
        assert!(c.lookup("huge", "voice", "rate").is_none());
    }

    #[test]
    fn lookup_promotes_lru_order() {
        let mut c = PcmCache::new();
        let small = MAX_CACHE_BYTES / 4;
        c.store(key("a"), make_utt("a", small));
        c.store(key("b"), make_utt("b", small));
        c.store(key("c"), make_utt("c", small));
        c.lookup("a", "voice", "rate");
        c.store(key("d"), make_utt("d", small));
        assert!(c.lookup("a", "voice", "rate").is_some());
        assert!(c.lookup("d", "voice", "rate").is_some());
        assert!(c.lookup("b", "voice", "rate").is_none());
    }

    #[test]
    fn store_updates_existing_entry() {
        let mut c = PcmCache::new();
        c.store(key("x"), make_utt("x", 100));
        c.store(key("x"), make_utt("x", 200));
        let hit = c.lookup("x", "voice", "rate");
        assert_eq!(hit.unwrap().prep.stereo.len(), 200);
    }
}

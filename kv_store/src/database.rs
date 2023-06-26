use crate::kv::{KVCommand, KeyValue};
use rocksdb::{Options, DB};

pub struct Database {
    rocks_db: DB,
}

impl Database {
    pub fn new(path: &str) -> Self {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        let rocks_db = DB::open(&opts, path).unwrap();
        Self { rocks_db }
    }

    pub fn handle_command(&self, command: KVCommand) -> Option<String> {
        match command {
            KVCommand::Put(KeyValue { key, value }) => {
                self.put(&key, &value);
                None
            }
            KVCommand::Delete(key) => {
                self.delete(&key);
                None
            }
            KVCommand::Get(key) => self.get(key.as_str()),
        }
    }

    fn get(&self, key: &str) -> Option<String> {
        match self.rocks_db.get(key.as_bytes()) {
            Ok(Some(value)) => {
                let value = String::from_utf8(value).unwrap();
                Some(value)
            }
            Ok(None) => None,
            Err(e) => panic!("failed to get value: {}", e),
        }
    }

    fn put(&self, key: &str, value: &str) {
        match self.rocks_db.put(key.as_bytes(), value.as_bytes()) {
            Ok(_) => {}
            Err(e) => panic!("failed to put value: {}", e),
        }
    }

    fn delete(&self, key: &str) {
        match self.rocks_db.delete(key.as_bytes()) {
            Ok(_) => {}
            Err(e) => panic!("failed to delete value: {}", e),
        }
    }
}

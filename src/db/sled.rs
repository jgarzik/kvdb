use super::api;

pub struct SledDb {
    db: sled::Db,
}

impl api::Db for SledDb {
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, &'static str> {
        match self.db.get(key) {
            Ok(opt_val) => match opt_val {
                None => Ok(None),
                Some(val) => Ok(Some(val.to_vec())),
            },
            Err(_e) => Err("get failed"),
        }
    }

    fn put(&mut self, key: &[u8], val: &[u8]) -> Result<bool, &'static str> {
        match self.db.insert(key, val) {
            Ok(_old_val) => Ok(true),
            Err(_e) => Err("put failed"),
        }
    }

    fn del(&mut self, key: &[u8]) -> Result<bool, &'static str> {
        match self.db.remove(key) {
            Ok(old_val) => match old_val {
                None => Ok(false),
                Some(_v) => Ok(true),
            },
            Err(_e) => Err("del failed"),
        }
    }

    fn apply_batch(&mut self, batch_in: &api::Batch) -> Result<bool, &'static str> {
        let mut batch = sled::Batch::default();
        for mutation in &batch_in.ops {
            match mutation.op {
                api::MutationOp::Insert => {
                    batch.insert(mutation.key.clone(), mutation.value.clone().unwrap())
                }
                api::MutationOp::Remove => batch.remove(mutation.key.clone()),
            }
        }

        match self.db.apply_batch(batch) {
            Ok(_optval) => Ok(true),
            Err(_e) => Err("batch failed"),
        }
    }
}

pub struct SledDriver {}

impl api::Driver for SledDriver {
    fn start_db(&self, cfg: api::Config) -> Result<Box<dyn api::Db + Send>, &'static str> {
        let sled_db_cfg = sled::ConfigBuilder::new()
            .path(cfg.path)
            .read_only(cfg.read_only)
            .build();

        Ok(Box::new(SledDb {
            db: sled::Db::start(sled_db_cfg).unwrap(),
        }) as Box<dyn api::Db + Send>)
    }
}

pub fn new_driver() -> Box<dyn api::Driver> {
    Box::new(SledDriver {})
}

#[cfg(test)]
use super::api::{Batch, ConfigBuilder};
#[cfg(test)]
use tempdir::TempDir;

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_get_put() {
        let tmp_dir = TempDir::new("tgp").unwrap();
        let tmp_path = tmp_dir.path().to_str().unwrap().to_string();
        let db_config = ConfigBuilder::new().path(tmp_path).read_only(false).build();

        let driver = new_driver();

        let mut db = driver.start_db(db_config).unwrap();

        assert_eq!(db.get(b"name"), Ok(None));
        assert_eq!(db.put(b"name", b"alan"), Ok(true));
        assert_eq!(db.get(b"name"), Ok(Some(Vec::from("alan"))));
        assert_eq!(db.del(b"name"), Ok(true));
        assert_eq!(db.get(b"name"), Ok(None));
        assert_eq!(db.get(b"never_existed"), Ok(None));
    }

    #[test]
    fn test_del() {
        let tmp_dir = TempDir::new("td").unwrap();
        let tmp_path = tmp_dir.path().to_str().unwrap().to_string();
        let db_config = ConfigBuilder::new().path(tmp_path).read_only(false).build();

        let driver = new_driver();

        let mut db = driver.start_db(db_config).unwrap();

        assert_eq!(db.put(b"name", b"alan"), Ok(true));
        assert_eq!(db.del(b"name"), Ok(true));
        assert_eq!(db.del(b"name"), Ok(false));
    }

    #[test]
    fn test_batch() {
        let tmp_dir = TempDir::new("tb").unwrap();
        let tmp_path = tmp_dir.path().to_str().unwrap().to_string();
        let db_config = ConfigBuilder::new().path(tmp_path).read_only(false).build();

        let driver = new_driver();

        let mut db = driver.start_db(db_config).unwrap();

        assert_eq!(db.put(b"name", b"alan"), Ok(true));

        let mut batch = Batch::default();
        batch.insert(b"age", b"25");
        batch.insert(b"city", b"anytown");
        batch.remove(b"name");
        assert_eq!(db.apply_batch(&batch), Ok(true));

        assert_eq!(db.get(b"name"), Ok(None));
        assert_eq!(db.get(b"age"), Ok(Some(Vec::from("25"))));
        assert_eq!(db.get(b"city"), Ok(Some(Vec::from("anytown"))));
    }
}

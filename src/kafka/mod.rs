mod kafka_ops;
mod consumer_ops;
mod producer_ops;
mod admin_ops;

use sled::{Db, Tree};
pub use kafka_ops::{KafkaOps, Pref};
pub use consumer_ops::ConsumerOps;
pub use producer_ops::ProducerOps;
pub use admin_ops::AdminOps;

use thiserror::Error;
use anyhow::Result;
use bson::Document;
use openssl::rand::rand_bytes;
use openssl::symm::{Cipher, decrypt, encrypt};
use rdkafka::config::RDKafkaLogLevel;
use rdkafka::message::FromBytes;
use serde::{Serialize, Deserialize};

#[derive(Error, Debug)]
pub enum KafkaOpsError {
    #[error("the client is not open. You must call open() first")]
    ClientNotOpen,
    #[error("server configuration for {0} not found")]
    ServerConfigNotFound(String),
    #[error("unknown kafka operations error")]
    Unknown,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Password {
    len: usize,
    pwd_data: Vec<u8>,
    iv: Vec<u8>,
}

impl Password {
    pub fn new<T: AsRef<str>>(password: T) -> Result<Self> {
        Password::encrypt_password(password)
    }

    fn generate_random_bytes(length: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0; length];
        rand_bytes(&mut buf)?;
        Ok(buf)
    }

    fn encrypt_password<T: AsRef<str>>(password: T) -> Result<Password> {
        let cipher = Cipher::bf_cfb64();
        let key = "master password";
        let mut key_bytes = vec![0; cipher.key_len()];
        for (dst, src) in key_bytes.iter_mut().zip(key.as_bytes()) {
            *dst = *src
        }

        let password_bytes = password.as_ref().as_bytes();
        let mut data = vec![0; password_bytes.len() + cipher.block_size()];
        for (dst, src) in data.iter_mut().zip(password_bytes) {
            *dst = *src
        }

        let iv = Password::generate_random_bytes(cipher.iv_len().unwrap_or(16))?;
        let ciphertext = encrypt(
            cipher,
            &key_bytes,
            Some(&iv),
            &data)?;

        Ok(Password {
            len: password_bytes.len(),
            pwd_data: ciphertext,
            iv,
        })
    }

    pub fn decrypt(&self) -> Result<Vec<u8>> {
        let cipher = Cipher::bf_cfb64();
        let key = "master password";
        let mut key_bytes = vec![0; cipher.key_len()];
        for (dst, src) in key_bytes.iter_mut().zip(key.as_bytes()) {
            *dst = *src
        }

        let mut ciphertext = decrypt(
            cipher,
            &key_bytes,
            Some(&self.iv),
            &self.pwd_data)?;

        ciphertext.truncate(self.len);
        Ok(ciphertext)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct KafkaServer {
    bootstrap: String,
    ssl_verification: bool,
    ssl_keystore_location: Option<String>,
    ssl_keystore_password: Option<Password>,
    ssl_security_protocol: String,
    message_max_bytes: u64,
    fetch_max_bytes: u64,
}

const GROUP_NAME: &str = "kator-rs-v1";

impl KafkaServer {
    pub fn from_db<T: AsRef<str>>(db: &Tree, name: T) -> Result<Self> {
        let name_str = name.as_ref();
        let found = db.get(name_str)?;
        match found {
            None => Err(KafkaOpsError::ServerConfigNotFound(name_str.to_string()).into()),
            Some(data) => {
                bson::from_reader(data.as_ref())
                    .map_err(|e| e.into())
            }
        }
    }

    pub fn new(bootstrap: String,
               ssl_verification: bool,
               ssl_keystore_location: Option<String>,
               ssl_keystore_password: Option<Password>,
               ssl_security_protocol: String,
               message_max_bytes: u64,
               fetch_max_bytes: u64) -> Self {
        Self {
            bootstrap,
            ssl_verification,
            ssl_keystore_location,
            ssl_keystore_password,
            ssl_security_protocol,
            message_max_bytes,
            fetch_max_bytes
        }
    }

    pub fn open(&self) -> Result<KafkaOps> {
        let mut builder = KafkaOps::builder()
            .with_prop("enable.auto.commit", "false")
            .with_bootstrap(&self.bootstrap)
            .with_ssl_cert_verification(self.ssl_verification)
            .with_security_protocol(&self.ssl_security_protocol)
            .with_group_id("kators-g-1")
            .with_message_max_bytes(self.message_max_bytes)
            .with_fetch_max_bytes(self.fetch_max_bytes);

        builder = match (&self.ssl_keystore_location, &self.ssl_keystore_password) {
            (Some(location), Some(password)) => builder
                .with_ssl_keystore_location(location)
                .with_ssl_keystore_password(str::from_bytes(&password.decrypt()?)?),
            (_, _) => builder,
        };

        builder
            .open(RDKafkaLogLevel::Info)
            .map_err(|e| e.into())
    }
}

#[cfg(test)]
mod test {
    use rdkafka::message::FromBytes;
    use crate::kafka::{KafkaServer, Password};

    #[test]
    fn test_password() {
        let pwd_text = "test$_пароль";
        let pwd = Password::new("test$_пароль").unwrap();
        assert_eq!(pwd_text, str::from_bytes(&pwd.decrypt().unwrap()).unwrap())
    }

    #[test]
    fn test_db_store() {
        let db: sled::Db = sled::open("my_test_db").unwrap();
        let servers = db.open_tree("servers").unwrap();

        let ks = KafkaServer::new(
            "10.53.91.21:9093".to_string(),
            false,
            Some("/Users/sbt-sementsov-av/projects/certs/kafka.p12".to_string()),
            Some(Password::new("123456").unwrap()),
            "SSL".to_string(),
            100,
            102400
        );

        let data = bson::to_vec(&ks).unwrap();
        servers.insert("my_server", data);

        // Read from servers
        let ks_read = KafkaServer::from_db(&servers, "my_server").unwrap();
        assert_eq!("123456", str::from_bytes(&ks_read.ssl_keystore_password.unwrap().decrypt().unwrap()).unwrap());

        db.drop_tree("servers").unwrap();
    }
}
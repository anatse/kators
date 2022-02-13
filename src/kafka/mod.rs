mod admin_ops;
mod consumer_ops;
mod kafka_ops;
mod producer_ops;

pub use admin_ops::AdminCommand;
pub use admin_ops::AdminOps;
pub use consumer_ops::ConsumerOps;
pub use kafka_ops::{KafkaOps, Pref};
pub use producer_ops::ProducerOps;
use sled::Tree;

use anyhow::Result;
use log::error;
use openssl::rand::rand_bytes;
use openssl::symm::{decrypt, encrypt, Cipher};
use rdkafka::config::RDKafkaLogLevel;
use rdkafka::message::{FromBytes, ToBytes};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Describes errors for kafka operations
#[derive(Error, Debug)]
pub enum KafkaOpsError {
    #[error("the client is not open. You must call open() first")]
    ClientNotOpen,
    #[error("server configuration for {0} not found")]
    ServerConfigNotFound(String),
    #[error("unknown kafka operations error")]
    Unknown,
}

/// Structure contains fields to manipulate encrypted passwords
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Password {
    /// Length of original password
    len: usize,
    /// Encrypted data
    pwd_data: Vec<u8>,
    /// Initialization vector
    iv: Vec<u8>,
}

/// Implementation crypto functions
impl Password {
    /// Create new password based on string representation
    /// # Arguments
    ///   - password - string representation of password
    pub fn new<T: AsRef<str>>(password: T) -> Result<Self> {
        Password::encrypt_password(password)
    }

    /// Internal function& Used to generate random array of bytes
    /// # Arguments
    ///   - length - length of result array
    fn generate_random_bytes(length: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0; length];
        rand_bytes(&mut buf)?;
        Ok(buf)
    }

    /// Encrypt password using blowfish crypto algorithm from openssl library
    /// # Arguments
    ///    - password - string representation of password
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
        let ciphertext = encrypt(cipher, &key_bytes, Some(&iv), &data)?;

        Ok(Password {
            len: password_bytes.len(),
            pwd_data: ciphertext,
            iv,
        })
    }

    /// Decrypt password and return plain representation of password as array of bytes
    pub fn decrypt(&self) -> Result<Vec<u8>> {
        let cipher = Cipher::bf_cfb64();
        let key = "master password";
        let mut key_bytes = vec![0; cipher.key_len()];
        for (dst, src) in key_bytes.iter_mut().zip(key.as_bytes()) {
            *dst = *src
        }

        let mut ciphertext = decrypt(cipher, &key_bytes, Some(&self.iv), &self.pwd_data)?;

        ciphertext.truncate(self.len);
        Ok(ciphertext)
    }
}

/// Kafka server parameters
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KafkaServer {
    pub name: String,
    pub bootstrap: String,
    pub ssl_verification: bool,
    pub ssl_keystore_location: Option<String>,
    pub ssl_keystore_password: Option<Password>,
    pub ssl_security_protocol: String,
    pub message_max_bytes: u64,
    pub fetch_max_bytes: u64,
}

impl ToString for KafkaServer {
    fn to_string(&self) -> String {
        self.name.clone()
    }
}

impl PartialEq<Self> for KafkaServer {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for KafkaServer {}

/// Default group name for this application
const GROUP_NAME: &str = "kator-rs-v1";

/// Implementation functions to store and retrieve kafka server parameters from
/// preferences database. The `sled` database is used as storage to manage preferences
impl KafkaServer {
    pub fn default() -> Self {
        Self {
            name: "".to_string(),
            bootstrap: "".to_string(),
            ssl_verification: false,
            ssl_keystore_location: None,
            ssl_keystore_password: None,
            ssl_security_protocol: "".to_string(),
            message_max_bytes: 0,
            fetch_max_bytes: 0,
        }
    }

    /// Load KafkaServer object from database table (tree) with the given name
    /// # Arguments
    ///   - db - sled tree object
    ///   - name - name of the server
    ///
    /// # Example
    /// ```
    /// let db: sled::Db = sled::open("my_test_db").unwrap();
    /// let servers = db.open_tree("servers").unwrap();
    /// let server = KafkaServer::from_db(&servers, "my server").unwrap();
    /// ```
    pub fn from_db<T: AsRef<str>>(db: &Tree, name: T) -> Result<Self> {
        let name_str = name.as_ref();
        let found = db.get(name_str)?;
        match found {
            None => Err(KafkaOpsError::ServerConfigNotFound(name_str.to_string()).into()),
            Some(data) => bson::from_reader(data.as_ref()).map_err(|e| e.into()),
        }
    }

    /// Create new KafkaServer using parameters
    /// # Arguments
    ///   - bootstrap - kafka brokers definition in format `host1:port1,host2:port2`, etc
    ///   - ssl_verification - flag specifies whether or not to check the server certificate
    ///   - ssl_keystore_location - keystore location in PKCS#12 format
    ///   - ssl_keystore_password - password as [Password]
    ///   - ssl_security_protocol - must be either PLAINTEXT or SSL
    ///   - message_max_bytes - see kafka parameters
    ///   - fetch_max_bytes - see kafka parameters
    pub fn new(
        name: String,
        bootstrap: String,
        ssl_verification: bool,
        ssl_keystore_location: Option<String>,
        ssl_keystore_password: Option<Password>,
        ssl_security_protocol: String,
        message_max_bytes: u64,
        fetch_max_bytes: u64,
    ) -> Self {
        Self {
            name,
            bootstrap,
            ssl_verification,
            ssl_keystore_location,
            ssl_keystore_password,
            ssl_security_protocol,
            message_max_bytes,
            fetch_max_bytes,
        }
    }

    /// Opens connection to kafka and returns KafkaOps object
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

        builder.open(RDKafkaLogLevel::Info)
    }

    pub fn store(&self, db: &Tree) -> Result<()> {
        let data = bson::to_vec(&self)?;
        let _ = db.insert(&self.name, data)?;
        Ok(())
    }

    pub fn all(db: &Tree) -> Vec<Self> {
        db.iter()
            .filter_map(|v| match v {
                Ok((iv_key, iv_item)) => match str::from_bytes(iv_key.to_bytes()) {
                    Ok(key) => match bson::from_reader(iv_item.as_ref()) {
                        Ok(srv) => Some(srv),
                        Err(e) => {
                            error!("Error parsing server definition for {}: {}", key, e);
                            None
                        }
                    },
                    Err(e) => {
                        error!("Error parsing server name: {}", e);
                        None
                    }
                },
                Err(e) => {
                    error!("Error loading server definition: {}", e);
                    None
                }
            })
            .collect::<Vec<Self>>()
    }
}

#[cfg(test)]
mod test {
    use crate::kafka::{KafkaServer, Password};
    use log::info;
    use rdkafka::message::FromBytes;

    fn create_ks() -> KafkaServer {
        KafkaServer::new(
            "my_server".to_string(),
            "10.53.91.21:9093".to_string(),
            false,
            Some("/Users/sbt-sementsov-av/projects/certs/kafka.p12".to_string()),
            Some(Password::new("123456").unwrap()),
            "SSL".to_string(),
            100,
            102400,
        )
    }

    fn test_get_all(db: &sled::Db) {
        let servers = db.open_tree("servers").unwrap();
        let ks = create_ks();
        let data = bson::to_vec(&ks).unwrap();
        servers.insert("my_server", data).unwrap();

        let kss = KafkaServer::all(&servers);
        assert_eq!(1, kss.len());
        assert_eq!(
            "123456",
            str::from_bytes(
                &kss[0]
                    .ssl_keystore_password
                    .as_ref()
                    .unwrap()
                    .decrypt()
                    .unwrap()
            )
            .unwrap()
        );
    }

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

        let ks = create_ks();
        let data = bson::to_vec(&ks).unwrap();
        servers.insert("my_server", data).unwrap();

        // Read from servers
        let ks_read = KafkaServer::from_db(&servers, "my_server").unwrap();
        assert_eq!(
            "123456",
            str::from_bytes(&ks_read.ssl_keystore_password.unwrap().decrypt().unwrap()).unwrap()
        );

        db.drop_tree("servers").unwrap();

        // get all
        test_get_all(&db);
    }
}

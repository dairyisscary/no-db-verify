use crate::user::{User, UserId};
use hmac::Mac;
use serde::{Deserialize, Serialize};

type HmacSha3_256 = hmac::Hmac<sha3::Sha3_256>;
type UtcDateTime = chrono::DateTime<chrono::Utc>;

const SECRET_KEY: &[u8; 19] = b"my super secret key";

fn as_base64<S: serde::Serializer>(key: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&base64::encode(key))
}

fn from_base64<'d, D: serde::Deserializer<'d>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
    String::deserialize(deserializer).and_then(|string| {
        base64::decode(string).map_err(|err| serde::de::Error::custom(err.to_string()))
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateParams {
    email: String,
    #[serde(serialize_with = "as_base64", deserialize_with = "from_base64")]
    token: Vec<u8>,
}

impl CreateParams {
    fn accum_mac(email: &str) -> HmacSha3_256 {
        let mut mac = HmacSha3_256::new_varkey(SECRET_KEY).unwrap();
        mac.input(email.as_bytes());
        mac
    }

    pub fn email(&self) -> &str {
        &self.email
    }

    pub fn verify(email: &str, params: &Self) -> bool {
        let mac = Self::accum_mac(email);
        mac.verify(params.token.as_slice()).is_ok()
    }
}

impl From<&str> for CreateParams {
    fn from(email: &str) -> Self {
        let mac = CreateParams::accum_mac(email);
        let token = Vec::from(mac.result().code().as_slice());
        CreateParams {
            email: email.to_string(),
            token,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResetParams {
    user_id: UserId,
    expires: UtcDateTime,
    #[serde(serialize_with = "as_base64", deserialize_with = "from_base64")]
    token: Vec<u8>,
}

impl ResetParams {
    fn accum_mac(user: &User, expires: &UtcDateTime) -> HmacSha3_256 {
        let mut mac = HmacSha3_256::new_varkey(SECRET_KEY).unwrap();
        mac.input(&user.id.to_string().into_bytes());
        mac.input(&expires.to_string().into_bytes());
        mac
    }

    pub fn user_id(&self) -> UserId {
        self.user_id
    }

    pub fn verify(user: &User, params: &Self) -> bool {
        let expires = params.expires;
        if chrono::Utc::now() > expires {
            return false;
        }
        let mac = Self::accum_mac(user, &expires);
        mac.verify(params.token.as_slice()).is_ok()
    }
}

impl From<&User> for ResetParams {
    fn from(user: &User) -> Self {
        let expires = chrono::Utc::now() + chrono::Duration::hours(3);
        let mac = Self::accum_mac(user, &expires);
        let token = Vec::from(mac.result().code().as_slice());
        ResetParams {
            user_id: user.id,
            expires,
            token,
        }
    }
}
